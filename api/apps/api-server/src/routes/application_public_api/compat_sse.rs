use std::{convert::Infallible, sync::Arc, time::Duration};

use axum::response::{
    sse::{Event, KeepAlive, Sse},
    IntoResponse, Response,
};
use control_plane::{
    application_public_api::{
        compat::openai::response_id_from_run_id,
        native::{NativeRunResult, NativeRunStatus, NativeUsage},
        run_service::native_result_from_run_detail,
    },
    orchestration_runtime::{
        debug_stream_events, CompleteCallbackTaskCommand, OrchestrationRuntimeService,
        StartPublishedFlowRunCommand,
    },
    ports::{OrchestrationRuntimeRepository, RuntimeEventEnvelope, RuntimeEventPayload},
};
use serde_json::{json, Value};
use tokio::sync::{mpsc, oneshot};
use tracing::{debug, info, warn};

use crate::{
    app_state::ApiState,
    provider_runtime::ApiProviderRuntime,
    routes::application_public_api::{
        native::{service_error, NativeApiError},
        stream_terminal_fallback::{
            enrich_terminal_runtime_event_with_durable_answer,
            load_latest_native_run_for_terminal_fallback, terminal_answer_deltas_from_payload,
            terminal_answer_text_from_payload, terminal_runtime_event_from_native_run,
            TerminalAnswerDelta, TerminalAnswerDeltaKind,
        },
        tool_callback_ids::{
            encode_anthropic_callback_tool_use_id, encode_openai_callback_tool_call_id,
        },
    },
};

type CompatRunSseStream = tokio_stream::wrappers::ReceiverStream<Result<Event, Infallible>>;

const OPENAI_CHAT_SSE_PROJECTION: &str = "openai_chat";
const OPENAI_RESPONSES_SSE_PROJECTION: &str = "openai_responses";
const ANTHROPIC_SSE_PROJECTION: &str = "anthropic";

#[derive(Debug, Default)]
struct CompatibleStreamStats {
    emitted_public_event: bool,
    emitted_content_bytes: usize,
    emitted_text_content: bool,
    emitted_reasoning_content: bool,
}

impl CompatibleStreamStats {
    fn emitted_content(&self) -> bool {
        self.emitted_content_bytes > 0
    }

    fn record_sent_runtime_event(
        &mut self,
        run: &NativeRunResult,
        event: &RuntimeEventEnvelope,
        emitted_public_event: bool,
    ) {
        self.emitted_public_event |= emitted_public_event;
        if is_answer_presentation_delta(event) {
            if !emitted_public_event {
                return;
            }
            let Some(text) = event.text.as_deref().filter(|text| !text.is_empty()) else {
                return;
            };
            match event.event_type.as_str() {
                "reasoning_delta" => self.record_reasoning_content(text),
                "text_delta" => self.record_text_content(text),
                _ => {}
            }
            return;
        }

        if !matches!(event.event_type.as_str(), "flow_finished" | "flow_failed") {
            return;
        }
        for delta in terminal_answer_deltas_from_run_or_payload(run, &event.payload) {
            match delta.kind {
                TerminalAnswerDeltaKind::Reasoning if !self.emitted_reasoning_content => {
                    self.record_reasoning_content(&delta.text);
                }
                TerminalAnswerDeltaKind::Text if !self.emitted_text_content => {
                    self.record_text_content(&delta.text);
                }
                _ => {}
            }
        }
    }

    fn record_text_content(&mut self, text: &str) {
        self.emitted_text_content = true;
        self.emitted_content_bytes += text.len();
    }

    fn record_reasoning_content(&mut self, text: &str) {
        self.emitted_reasoning_content = true;
        self.emitted_content_bytes += text.len();
    }
}

pub(crate) async fn start_openai_run_stream(
    state: Arc<ApiState>,
    run: NativeRunResult,
    model: String,
) -> Result<Response, NativeApiError> {
    let mut mapper =
        OpenAiChatStreamMapper::new(model, openai_chat_completion_id_from_run_id(run.id), true);
    start_compatible_run_stream(
        state,
        run,
        OPENAI_CHAT_SSE_PROJECTION,
        move |run, envelope| mapper.runtime_event_to_sse(run, envelope),
    )
    .await
}

pub(crate) async fn start_openai_response_stream(
    state: Arc<ApiState>,
    run: NativeRunResult,
    model: String,
    previous_response_id: Option<String>,
) -> Result<Response, NativeApiError> {
    let mut mapper = OpenAiResponseStreamMapper::new(model, previous_response_id, true);
    start_compatible_run_stream(
        state,
        run,
        OPENAI_RESPONSES_SSE_PROJECTION,
        move |run, envelope| mapper.runtime_event_to_sse(run, envelope),
    )
    .await
}

pub(crate) async fn start_openai_chat_resume_stream(
    state: Arc<ApiState>,
    run: NativeRunResult,
    model: String,
    chat_completion_id: String,
    command: CompleteCallbackTaskCommand,
) -> Result<Response, NativeApiError> {
    let mut mapper = OpenAiChatStreamMapper::new(model, chat_completion_id, true);
    start_compatible_resume_stream(
        state,
        run,
        command,
        OPENAI_CHAT_SSE_PROJECTION,
        move |run, envelope| mapper.runtime_event_to_sse(run, envelope),
    )
    .await
}

pub(crate) async fn start_openai_response_resume_stream(
    state: Arc<ApiState>,
    run: NativeRunResult,
    model: String,
    previous_response_id: Option<String>,
    command: CompleteCallbackTaskCommand,
) -> Result<Response, NativeApiError> {
    let mut mapper = OpenAiResponseStreamMapper::new(model, previous_response_id, true);
    start_compatible_resume_stream(
        state,
        run,
        command,
        OPENAI_RESPONSES_SSE_PROJECTION,
        move |run, envelope| mapper.runtime_event_to_sse(run, envelope),
    )
    .await
}

pub(crate) async fn start_anthropic_run_stream(
    state: Arc<ApiState>,
    run: NativeRunResult,
    model: String,
) -> Result<Response, NativeApiError> {
    let mut stateful_mapper = AnthropicStreamMapper::new(model);
    start_compatible_run_stream(
        state,
        run,
        ANTHROPIC_SSE_PROJECTION,
        move |run, envelope| stateful_mapper.runtime_event_to_sse(run, envelope),
    )
    .await
}

pub(crate) fn completed_anthropic_stream(run: NativeRunResult, model: String) -> Response {
    completed_compatible_stream(anthropic_completed_run_to_sse(&run, &model))
}

fn completed_compatible_stream(events: Vec<Result<Event, Infallible>>) -> Response {
    Sse::new(tokio_stream::iter(events))
        .keep_alive(
            KeepAlive::new()
                .interval(Duration::from_secs(10))
                .text("heartbeat"),
        )
        .into_response()
}

async fn start_compatible_run_stream<F>(
    state: Arc<ApiState>,
    run: NativeRunResult,
    sse_projection: &'static str,
    mut mapper: F,
) -> Result<Response, NativeApiError>
where
    F: FnMut(&NativeRunResult, RuntimeEventEnvelope) -> Vec<Result<Event, Infallible>>
        + Send
        + 'static,
{
    if let Err(error) = state
        .runtime_event_stream
        .open_run(
            run.id,
            control_plane::ports::RuntimeEventStreamPolicy::debug_default(),
        )
        .await
    {
        warn!(
            flow_run_id = %run.id,
            application_id = %run.application_id,
            error = %error,
            "failed to open compatible public API runtime event stream"
        );
        return Err(service_error(error));
    }

    let (sender, receiver) = mpsc::channel(32);
    tokio::spawn(send_compatible_runtime_event_stream(
        state.clone(),
        run.clone(),
        sse_projection,
        None,
        None,
        sender,
        move |run, envelope| mapper(run, envelope),
    ));

    let background_state = state.clone();
    tokio::spawn(async move {
        let runtime_service = OrchestrationRuntimeService::new(
            background_state.store.clone(),
            ApiProviderRuntime::new(background_state.provider_runtime.clone()),
            background_state.runtime_engine.clone(),
            background_state.provider_secret_master_key.clone(),
        )
        .with_runtime_event_stream(background_state.runtime_event_stream.clone());
        if let Err(runtime_error) = runtime_service
            .start_published_flow_run(StartPublishedFlowRunCommand {
                application_id: run.application_id,
                flow_run_id: run.id,
            })
            .await
        {
            warn!(
                flow_run_id = %run.id,
                application_id = %run.application_id,
                error = %runtime_error,
                "compatible public API streamed run failed"
            );
            let _ = background_state
                .runtime_event_stream
                .append(
                    run.id,
                    debug_stream_events::flow_failed(
                        run.id,
                        json!({ "message": runtime_error.to_string() }),
                    ),
                )
                .await;
            let _ = background_state
                .runtime_event_stream
                .close_run(
                    run.id,
                    control_plane::ports::RuntimeEventCloseReason::Failed,
                )
                .await;
        }
    });

    info!(
        flow_run_id = %run.id,
        application_id = %run.application_id,
        sse_projection = %sse_projection,
        heartbeat_interval_secs = 10_u64,
        heartbeat_text = "heartbeat",
        "compatible public API stream opened"
    );

    Ok(Sse::new(CompatRunSseStream::new(receiver))
        .keep_alive(
            KeepAlive::new()
                .interval(Duration::from_secs(10))
                .text("heartbeat"),
        )
        .into_response())
}

async fn start_compatible_resume_stream<F>(
    state: Arc<ApiState>,
    run: NativeRunResult,
    command: CompleteCallbackTaskCommand,
    sse_projection: &'static str,
    mut mapper: F,
) -> Result<Response, NativeApiError>
where
    F: FnMut(&NativeRunResult, RuntimeEventEnvelope) -> Vec<Result<Event, Infallible>>
        + Send
        + 'static,
{
    state
        .runtime_event_stream
        .open_run(
            run.id,
            control_plane::ports::RuntimeEventStreamPolicy::debug_default(),
        )
        .await
        .map_err(service_error)?;
    let resume_started = state
        .runtime_event_stream
        .append(run.id, debug_stream_events::flow_started(run.id))
        .await
        .map_err(service_error)?;

    let (sender, receiver) = mpsc::channel(32);
    let (resume_done_sender, resume_done_receiver) = oneshot::channel::<()>();
    let resume_done_guard_sender = sender.clone();
    tokio::spawn(async move {
        let _ = resume_done_receiver.await;
        drop(resume_done_guard_sender);
    });
    tokio::spawn(send_compatible_runtime_event_stream(
        state.clone(),
        run.clone(),
        sse_projection,
        Some(resume_started.sequence.saturating_sub(1)),
        Some(command.callback_task_id),
        sender,
        move |run, envelope| mapper(run, envelope),
    ));

    let background_state = state.clone();
    tokio::spawn(async move {
        let runtime_service = OrchestrationRuntimeService::new(
            background_state.store.clone(),
            ApiProviderRuntime::new(background_state.provider_runtime.clone()),
            background_state.runtime_engine.clone(),
            background_state.provider_secret_master_key.clone(),
        )
        .with_runtime_event_stream(background_state.runtime_event_stream.clone());
        match runtime_service.complete_callback_task(command).await {
            Ok(detail) => {
                append_compatible_resume_terminal_event(&background_state, &detail).await;
            }
            Err(error) => {
                let _ = background_state
                    .runtime_event_stream
                    .append(
                        run.id,
                        debug_stream_events::flow_failed(
                            run.id,
                            json!({ "message": error.to_string() }),
                        ),
                    )
                    .await;
                let _ = background_state
                    .runtime_event_stream
                    .close_run(
                        run.id,
                        control_plane::ports::RuntimeEventCloseReason::Failed,
                    )
                    .await;
            }
        }
        let _ = resume_done_sender.send(());
    });

    Ok(Sse::new(CompatRunSseStream::new(receiver))
        .keep_alive(
            KeepAlive::new()
                .interval(Duration::from_secs(10))
                .text("heartbeat"),
        )
        .into_response())
}

async fn send_compatible_runtime_event_stream<F>(
    state: Arc<ApiState>,
    initial_run: NativeRunResult,
    sse_projection: &'static str,
    from_sequence: Option<i64>,
    ignored_waiting_callback_task_id: Option<uuid::Uuid>,
    sender: mpsc::Sender<Result<Event, Infallible>>,
    mut mapper: F,
) where
    F: FnMut(&NativeRunResult, RuntimeEventEnvelope) -> Vec<Result<Event, Infallible>>,
{
    let mut stats = CompatibleStreamStats::default();
    let stream = state.runtime_event_stream.clone();
    let Ok(mut subscription) = stream.subscribe(initial_run.id, from_sequence).await else {
        warn!(
            flow_run_id = %initial_run.id,
            application_id = %initial_run.application_id,
            "failed to subscribe compatible public API runtime event stream"
        );
        log_compatible_sse_closed(
            sse_projection,
            &initial_run,
            &stats,
            "subscribe_failed",
            "subscribe",
            false,
        );
        return;
    };

    let mut last_forwarded_sequence = from_sequence.unwrap_or(0);
    let mut last_forwarded_durable_sequence = durable_sequence_for_ignored_waiting_callback(
        state.as_ref(),
        initial_run.id,
        ignored_waiting_callback_task_id,
    )
    .await
    .unwrap_or(0);
    match forward_compatible_runtime_events(CompatibleRuntimeEventsForward {
        state: &state,
        initial_run: &initial_run,
        sender: &sender,
        mapper: &mut mapper,
        stats: &mut stats,
        ignored_waiting_callback_task_id,
        last_forwarded_sequence: &mut last_forwarded_sequence,
        resume_durable_sequence_before_terminal: Some(&mut last_forwarded_durable_sequence),
        events: subscription.replay,
    })
    .await
    {
        CompatibleForwardOutcome::Terminal { event_type } => {
            debug!(
                flow_run_id = %initial_run.id,
                application_id = %initial_run.application_id,
                "compatible public API stream replay reached terminal event"
            );
            log_compatible_sse_closed(
                sse_projection,
                &initial_run,
                &stats,
                &event_type,
                "replay",
                false,
            );
            return;
        }
        CompatibleForwardOutcome::ClientDisconnected => {
            debug!(
                flow_run_id = %initial_run.id,
                application_id = %initial_run.application_id,
                "compatible public API stream client disconnected during replay"
            );
            log_compatible_sse_closed(
                sse_projection,
                &initial_run,
                &stats,
                "client_disconnected",
                "replay",
                true,
            );
            return;
        }
        CompatibleForwardOutcome::Open { .. } => {}
    }

    let mut durable_terminal_check = tokio::time::interval(Duration::from_millis(500));
    durable_terminal_check.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        tokio::select! {
            maybe_event = subscription.live_events.recv() => {
                let Some(event) = maybe_event else {
                    break;
                };
                let event_type = event.event_type.clone();
                match forward_compatible_runtime_events(CompatibleRuntimeEventsForward {
                    state: &state,
                    initial_run: &initial_run,
                    sender: &sender,
                    mapper: &mut mapper,
                    stats: &mut stats,
                    ignored_waiting_callback_task_id,
                    last_forwarded_sequence: &mut last_forwarded_sequence,
                    resume_durable_sequence_before_terminal: Some(
                        &mut last_forwarded_durable_sequence,
                    ),
                    events: vec![event],
                })
                .await
                {
                    CompatibleForwardOutcome::Terminal { event_type: _ } => {
                        debug!(
                            flow_run_id = %initial_run.id,
                            application_id = %initial_run.application_id,
                            event_type = %event_type,
                            "compatible public API stream reached terminal event"
                        );
                        log_compatible_sse_closed(
                            sse_projection,
                            &initial_run,
                            &stats,
                            &event_type,
                            "live",
                            false,
                        );
                        return;
                    }
                    CompatibleForwardOutcome::ClientDisconnected => {
                        debug!(
                            flow_run_id = %initial_run.id,
                            application_id = %initial_run.application_id,
                            "compatible public API stream client disconnected"
                        );
                        log_compatible_sse_closed(
                            sse_projection,
                            &initial_run,
                            &stats,
                            "client_disconnected",
                            "live",
                            true,
                        );
                        return;
                    }
                    CompatibleForwardOutcome::Open { .. } => {}
                }
            }
            _ = durable_terminal_check.tick() => {
                if let Ok(events) = stream.replay(
                    initial_run.id,
                    Some(last_forwarded_sequence),
                    usize::MAX,
                )
                .await
                {
                    match forward_compatible_runtime_events(CompatibleRuntimeEventsForward {
                        state: &state,
                        initial_run: &initial_run,
                        sender: &sender,
                        mapper: &mut mapper,
                        stats: &mut stats,
                        ignored_waiting_callback_task_id,
                        last_forwarded_sequence: &mut last_forwarded_sequence,
                        resume_durable_sequence_before_terminal: Some(
                            &mut last_forwarded_durable_sequence,
                        ),
                        events,
                    })
                    .await
                    {
                        CompatibleForwardOutcome::Terminal { event_type } => {
                            debug!(
                                flow_run_id = %initial_run.id,
                                application_id = %initial_run.application_id,
                                trigger = "durable_poll",
                                "compatible public API stream drained runtime terminal event before durable fallback"
                            );
                            log_compatible_sse_closed(
                                sse_projection,
                                &initial_run,
                                &stats,
                                &event_type,
                                "durable_poll_stream",
                                false,
                            );
                            return;
                        }
                        CompatibleForwardOutcome::ClientDisconnected => {
                            log_compatible_sse_closed(
                                sse_projection,
                                &initial_run,
                                &stats,
                                "client_disconnected",
                                "durable_poll_stream",
                                true,
                            );
                            return;
                        }
                        CompatibleForwardOutcome::Open { saw_event: true } => continue,
                        CompatibleForwardOutcome::Open { saw_event: false } => {}
                    }
                }

                if ignored_waiting_callback_task_id.is_some()
                    && last_forwarded_durable_sequence == 0
                {
                    if let Some(sequence) = durable_sequence_for_ignored_waiting_callback(
                        state.as_ref(),
                        initial_run.id,
                        ignored_waiting_callback_task_id,
                    )
                    .await
                    {
                        last_forwarded_durable_sequence = sequence;
                    } else {
                        continue;
                    }
                }

                if let Ok(records) = state
                    .store
                    .list_runtime_events(initial_run.id, last_forwarded_durable_sequence)
                    .await
                {
                    let saw_durable_record = !records.is_empty();
                    let events = records
                        .into_iter()
                        .map(durable_record_to_runtime_event_envelope)
                        .collect::<Vec<_>>();
                    match forward_compatible_runtime_events(CompatibleRuntimeEventsForward {
                        state: &state,
                        initial_run: &initial_run,
                        sender: &sender,
                        mapper: &mut mapper,
                        stats: &mut stats,
                        ignored_waiting_callback_task_id,
                        last_forwarded_sequence: &mut last_forwarded_durable_sequence,
                        resume_durable_sequence_before_terminal: None,
                        events,
                    })
                    .await
                    {
                        CompatibleForwardOutcome::Terminal { event_type } => {
                            debug!(
                                flow_run_id = %initial_run.id,
                                application_id = %initial_run.application_id,
                                trigger = "durable_poll",
                                "compatible public API stream drained durable terminal event before fallback"
                            );
                            log_compatible_sse_closed(
                                sse_projection,
                                &initial_run,
                                &stats,
                                &event_type,
                                "durable_poll_records",
                                false,
                            );
                            return;
                        }
                        CompatibleForwardOutcome::ClientDisconnected => {
                            log_compatible_sse_closed(
                                sse_projection,
                                &initial_run,
                                &stats,
                                "client_disconnected",
                                "durable_poll_records",
                                true,
                            );
                            return;
                        }
                        CompatibleForwardOutcome::Open { saw_event: true } => continue,
                        CompatibleForwardOutcome::Open { saw_event: false } => {
                            if ignored_waiting_callback_task_id.is_some() && !saw_durable_record {
                                continue;
                            }
                        }
                    }
                }

                match emit_compatible_terminal_fallback(CompatibleTerminalFallback {
                    state: &state,
                    initial_run: &initial_run,
                    sender: &sender,
                    mapper: &mut mapper,
                    stats: &mut stats,
                    trigger: "durable_poll",
                    warn_if_not_terminal: false,
                    ignored_waiting_callback_task_id,
                })
                .await
                {
                    CompatibleTerminalFallbackOutcome::Sent { event_type } => {
                        log_compatible_sse_closed(
                            sse_projection,
                            &initial_run,
                            &stats,
                            &event_type,
                            "durable_terminal_fallback",
                            false,
                        );
                        return;
                    }
                    CompatibleTerminalFallbackOutcome::ClientDisconnected { event_type } => {
                        let terminal_reason =
                            event_type.as_deref().unwrap_or("client_disconnected");
                        log_compatible_sse_closed(
                            sse_projection,
                            &initial_run,
                            &stats,
                            terminal_reason,
                            "durable_terminal_fallback",
                            true,
                        );
                        return;
                    }
                    CompatibleTerminalFallbackOutcome::NotTerminal
                    | CompatibleTerminalFallbackOutcome::IgnoredWaitingCallback => {}
                }
            }
        }
    }

    match emit_compatible_terminal_fallback(CompatibleTerminalFallback {
        state: &state,
        initial_run: &initial_run,
        sender: &sender,
        mapper: &mut mapper,
        stats: &mut stats,
        trigger: "stream_closed",
        warn_if_not_terminal: true,
        ignored_waiting_callback_task_id,
    })
    .await
    {
        CompatibleTerminalFallbackOutcome::Sent { event_type } => {
            log_compatible_sse_closed(
                sse_projection,
                &initial_run,
                &stats,
                &event_type,
                "stream_closed_terminal_fallback",
                false,
            );
        }
        CompatibleTerminalFallbackOutcome::ClientDisconnected { event_type } => {
            let terminal_reason = event_type.as_deref().unwrap_or("client_disconnected");
            log_compatible_sse_closed(
                sse_projection,
                &initial_run,
                &stats,
                terminal_reason,
                "stream_closed_terminal_fallback",
                true,
            );
        }
        CompatibleTerminalFallbackOutcome::IgnoredWaitingCallback => {
            log_compatible_sse_closed(
                sse_projection,
                &initial_run,
                &stats,
                "ignored_waiting_callback",
                "stream_closed_terminal_fallback",
                false,
            );
        }
        CompatibleTerminalFallbackOutcome::NotTerminal => {
            log_compatible_sse_closed(
                sse_projection,
                &initial_run,
                &stats,
                "stream_closed_before_terminal",
                "stream_closed",
                false,
            );
        }
    }
}

async fn durable_sequence_for_ignored_waiting_callback(
    state: &ApiState,
    run_id: uuid::Uuid,
    ignored_waiting_callback_task_id: Option<uuid::Uuid>,
) -> Option<i64> {
    let ignored_task_id = ignored_waiting_callback_task_id?;
    let records = state.store.list_runtime_events(run_id, 0).await.ok()?;
    records
        .into_iter()
        .filter(|record| {
            record
                .payload
                .get("callback_task_id")
                .and_then(Value::as_str)
                .and_then(|value| uuid::Uuid::parse_str(value).ok())
                == Some(ignored_task_id)
        })
        .map(|record| record.sequence)
        .max()
}

fn log_compatible_sse_closed(
    sse_projection: &'static str,
    run: &NativeRunResult,
    stats: &CompatibleStreamStats,
    terminal_reason: &str,
    close_trigger: &str,
    client_disconnected: bool,
) {
    info!(
        flow_run_id = %run.id,
        application_id = %run.application_id,
        sse_projection = %sse_projection,
        emitted_content = stats.emitted_content(),
        content_bytes = stats.emitted_content_bytes,
        terminal_reason = %terminal_reason,
        close_trigger = %close_trigger,
        client_disconnected = client_disconnected,
        "compatible public API SSE stream closed"
    );
}

fn durable_record_to_runtime_event_envelope(
    record: domain::RuntimeEventRecord,
) -> RuntimeEventEnvelope {
    let text = compat_payload_string(&record.payload, "text")
        .or_else(|| compat_payload_string(&record.payload, "delta"));
    let delta_index = compat_payload_i64(&record.payload, "delta_index")
        .or_else(|| compat_payload_i64(&record.payload, "sequence_start"));
    let content_type = compat_payload_string(&record.payload, "content_type");
    RuntimeEventEnvelope {
        run_id: record.flow_run_id,
        node_run_id: record.node_run_id,
        sequence: record.sequence,
        event_id: format!("{}:{}", record.flow_run_id, record.sequence),
        event_type: record.event_type,
        occurred_at: record.created_at,
        delta_index,
        content_type,
        text,
        source: match record.source {
            domain::RuntimeEventSource::ProviderPlugin => {
                control_plane::ports::RuntimeEventSource::Provider
            }
            _ => control_plane::ports::RuntimeEventSource::Runtime,
        },
        durability: match record.durability {
            domain::RuntimeEventDurability::Durable => {
                control_plane::ports::RuntimeEventDurability::DurableRequired
            }
            domain::RuntimeEventDurability::Ephemeral | domain::RuntimeEventDurability::Sampled => {
                control_plane::ports::RuntimeEventDurability::Ephemeral
            }
        },
        persist_required: true,
        trace_visible: true,
        payload: record.payload,
    }
}

fn compat_payload_i64(payload: &Value, key: &str) -> Option<i64> {
    payload.get(key).and_then(|value| {
        value
            .as_i64()
            .or_else(|| value.as_str().and_then(|text| text.parse().ok()))
    })
}

fn compat_payload_string(payload: &Value, key: &str) -> Option<String> {
    payload
        .get(key)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

struct CompatibleRuntimeEventsForward<'a, F> {
    state: &'a ApiState,
    initial_run: &'a NativeRunResult,
    sender: &'a mpsc::Sender<Result<Event, Infallible>>,
    mapper: &'a mut F,
    stats: &'a mut CompatibleStreamStats,
    ignored_waiting_callback_task_id: Option<uuid::Uuid>,
    last_forwarded_sequence: &'a mut i64,
    resume_durable_sequence_before_terminal: Option<&'a mut i64>,
    events: Vec<RuntimeEventEnvelope>,
}

enum CompatibleForwardOutcome {
    Open { saw_event: bool },
    Terminal { event_type: String },
    ClientDisconnected,
}

async fn forward_compatible_runtime_events<F>(
    forward: CompatibleRuntimeEventsForward<'_, F>,
) -> CompatibleForwardOutcome
where
    F: FnMut(&NativeRunResult, RuntimeEventEnvelope) -> Vec<Result<Event, Infallible>>,
{
    let CompatibleRuntimeEventsForward {
        state,
        initial_run,
        sender,
        mapper,
        stats,
        ignored_waiting_callback_task_id,
        last_forwarded_sequence,
        resume_durable_sequence_before_terminal,
        events,
    } = forward;
    let mut saw_event = false;
    let mut resume_durable_sequence_before_terminal = resume_durable_sequence_before_terminal;

    for event in events {
        if event.sequence <= *last_forwarded_sequence {
            continue;
        }
        *last_forwarded_sequence = event.sequence;
        saw_event = true;

        if is_ignored_waiting_callback(&event, ignored_waiting_callback_task_id) {
            continue;
        }

        let is_terminal = is_public_terminal_runtime_event(&event.event_type);
        if is_terminal && ignored_waiting_callback_task_id.is_some() {
            if let Some(last_forwarded_durable_sequence) =
                resume_durable_sequence_before_terminal.as_deref_mut()
            {
                match drain_compatible_durable_runtime_events(
                    CompatibleDurableRuntimeEventsForward {
                        state,
                        initial_run,
                        sender,
                        mapper,
                        stats,
                        ignored_waiting_callback_task_id,
                        last_forwarded_durable_sequence,
                    },
                )
                .await
                {
                    CompatibleForwardOutcome::Terminal { event_type } => {
                        return CompatibleForwardOutcome::Terminal { event_type };
                    }
                    CompatibleForwardOutcome::ClientDisconnected => {
                        return CompatibleForwardOutcome::ClientDisconnected;
                    }
                    CompatibleForwardOutcome::Open { .. } => {}
                }
            }
        }
        if let Some(last_forwarded_durable_sequence) =
            resume_durable_sequence_before_terminal.as_deref_mut()
        {
            advance_durable_cursor_for_forwarded_event(
                state,
                initial_run.id,
                &event,
                last_forwarded_durable_sequence,
            )
            .await;
        }
        match forward_single_compatible_runtime_event(
            state,
            initial_run,
            sender,
            mapper,
            stats,
            event,
        )
        .await
        {
            CompatibleForwardOutcome::Terminal { event_type } => {
                return CompatibleForwardOutcome::Terminal { event_type };
            }
            CompatibleForwardOutcome::ClientDisconnected => {
                return CompatibleForwardOutcome::ClientDisconnected;
            }
            CompatibleForwardOutcome::Open { .. } => {}
        }
    }

    CompatibleForwardOutcome::Open { saw_event }
}

async fn advance_durable_cursor_for_forwarded_event(
    state: &ApiState,
    run_id: uuid::Uuid,
    event: &RuntimeEventEnvelope,
    last_forwarded_durable_sequence: &mut i64,
) {
    if !event_can_match_durable_cursor(event) {
        return;
    }
    let Ok(records) = state
        .store
        .list_runtime_events(run_id, *last_forwarded_durable_sequence)
        .await
    else {
        return;
    };
    let Some(record) = records.into_iter().find(|record| {
        record.sequence > *last_forwarded_durable_sequence
            && durable_record_matches_forwarded_event(record, event)
    }) else {
        return;
    };

    *last_forwarded_durable_sequence = record.sequence;
}

fn event_can_match_durable_cursor(event: &RuntimeEventEnvelope) -> bool {
    event.event_type == "flow_started" || is_answer_presentation_delta(event)
}

fn durable_record_matches_forwarded_event(
    record: &domain::RuntimeEventRecord,
    event: &RuntimeEventEnvelope,
) -> bool {
    if record.event_type != event.event_type {
        return false;
    }
    if is_answer_presentation_delta(event) {
        return durable_record_matches_answer_delta(record, event);
    }
    if event.event_type == "flow_started" {
        return record.payload.get("type").and_then(Value::as_str) == Some("flow_started")
            && event.payload.get("type").and_then(Value::as_str) == Some("flow_started");
    }
    false
}

fn durable_record_matches_answer_delta(
    record: &domain::RuntimeEventRecord,
    event: &RuntimeEventEnvelope,
) -> bool {
    record.event_type == event.event_type
        && debug_stream_events::is_answer_presentation_delta_payload(&record.payload)
        && answer_delta_payload_field(&record.payload, "text")
            == answer_delta_payload_field(&event.payload, "text")
        && answer_delta_presentation_field(&record.payload, "answer_node_id")
            == answer_delta_presentation_field(&event.payload, "answer_node_id")
        && answer_delta_presentation_field(&record.payload, "segment_index")
            == answer_delta_presentation_field(&event.payload, "segment_index")
        && answer_delta_presentation_field(&record.payload, "source_node_id")
            == answer_delta_presentation_field(&event.payload, "source_node_id")
        && answer_delta_presentation_field(&record.payload, "source_output_key")
            == answer_delta_presentation_field(&event.payload, "source_output_key")
}

fn answer_delta_payload_field(payload: &Value, key: &str) -> Option<Value> {
    payload.get(key).cloned()
}

fn answer_delta_presentation_field(payload: &Value, key: &str) -> Option<Value> {
    payload
        .get("presentation")
        .and_then(Value::as_object)
        .and_then(|presentation| presentation.get(key))
        .cloned()
}

async fn forward_compatible_runtime_events_without_resume_durable_prefix<F>(
    forward: CompatibleRuntimeEventsForward<'_, F>,
) -> CompatibleForwardOutcome
where
    F: FnMut(&NativeRunResult, RuntimeEventEnvelope) -> Vec<Result<Event, Infallible>>,
{
    let CompatibleRuntimeEventsForward {
        state,
        initial_run,
        sender,
        mapper,
        stats,
        ignored_waiting_callback_task_id,
        last_forwarded_sequence,
        resume_durable_sequence_before_terminal: _,
        events,
    } = forward;
    let mut saw_event = false;

    for event in events {
        if event.sequence <= *last_forwarded_sequence {
            continue;
        }
        *last_forwarded_sequence = event.sequence;
        saw_event = true;

        if is_ignored_waiting_callback(&event, ignored_waiting_callback_task_id) {
            continue;
        }

        match forward_single_compatible_runtime_event(
            state,
            initial_run,
            sender,
            mapper,
            stats,
            event,
        )
        .await
        {
            CompatibleForwardOutcome::Terminal { event_type } => {
                return CompatibleForwardOutcome::Terminal { event_type };
            }
            CompatibleForwardOutcome::ClientDisconnected => {
                return CompatibleForwardOutcome::ClientDisconnected;
            }
            CompatibleForwardOutcome::Open { .. } => {}
        }
    }

    CompatibleForwardOutcome::Open { saw_event }
}

async fn forward_single_compatible_runtime_event<F>(
    state: &ApiState,
    initial_run: &NativeRunResult,
    sender: &mpsc::Sender<Result<Event, Infallible>>,
    mapper: &mut F,
    stats: &mut CompatibleStreamStats,
    event: RuntimeEventEnvelope,
) -> CompatibleForwardOutcome
where
    F: FnMut(&NativeRunResult, RuntimeEventEnvelope) -> Vec<Result<Event, Infallible>>,
{
    let is_terminal = is_public_terminal_runtime_event(&event.event_type);
    let terminal_run;
    let run = if is_terminal {
        terminal_run = load_latest_native_run_for_terminal_fallback(state, initial_run).await;
        &terminal_run
    } else {
        initial_run
    };
    let event = if is_terminal {
        enrich_terminal_runtime_event_with_durable_answer(state, run, event).await
    } else {
        event
    };
    let event_type = event.event_type.clone();
    let events = mapper(run, event.clone());
    let emitted_public_event = !events.is_empty();
    if !send_compatible_sse_events(sender, events).await {
        return CompatibleForwardOutcome::ClientDisconnected;
    }
    stats.record_sent_runtime_event(run, &event, emitted_public_event);
    if is_terminal {
        return CompatibleForwardOutcome::Terminal { event_type };
    }
    CompatibleForwardOutcome::Open { saw_event: true }
}

struct CompatibleDurableRuntimeEventsForward<'a, F> {
    state: &'a ApiState,
    initial_run: &'a NativeRunResult,
    sender: &'a mpsc::Sender<Result<Event, Infallible>>,
    mapper: &'a mut F,
    stats: &'a mut CompatibleStreamStats,
    ignored_waiting_callback_task_id: Option<uuid::Uuid>,
    last_forwarded_durable_sequence: &'a mut i64,
}

async fn drain_compatible_durable_runtime_events<F>(
    forward: CompatibleDurableRuntimeEventsForward<'_, F>,
) -> CompatibleForwardOutcome
where
    F: FnMut(&NativeRunResult, RuntimeEventEnvelope) -> Vec<Result<Event, Infallible>>,
{
    let CompatibleDurableRuntimeEventsForward {
        state,
        initial_run,
        sender,
        mapper,
        stats,
        ignored_waiting_callback_task_id,
        last_forwarded_durable_sequence,
    } = forward;

    if ignored_waiting_callback_task_id.is_some() && *last_forwarded_durable_sequence == 0 {
        if let Some(sequence) = durable_sequence_for_ignored_waiting_callback(
            state,
            initial_run.id,
            ignored_waiting_callback_task_id,
        )
        .await
        {
            *last_forwarded_durable_sequence = sequence;
        } else {
            return CompatibleForwardOutcome::Open { saw_event: false };
        }
    }

    let records = match state
        .store
        .list_runtime_events(initial_run.id, *last_forwarded_durable_sequence)
        .await
    {
        Ok(records) => records,
        Err(error) => {
            warn!(
                flow_run_id = %initial_run.id,
                application_id = %initial_run.application_id,
                error = %error,
                "failed to drain compatible public API durable runtime events"
            );
            return CompatibleForwardOutcome::Open { saw_event: false };
        }
    };
    let events = records
        .into_iter()
        .map(durable_record_to_runtime_event_envelope)
        .collect::<Vec<_>>();

    forward_compatible_runtime_events_without_resume_durable_prefix(
        CompatibleRuntimeEventsForward {
            state,
            initial_run,
            sender,
            mapper,
            stats,
            ignored_waiting_callback_task_id,
            last_forwarded_sequence: last_forwarded_durable_sequence,
            resume_durable_sequence_before_terminal: None,
            events,
        },
    )
    .await
}

async fn send_compatible_sse_events(
    sender: &mpsc::Sender<Result<Event, Infallible>>,
    events: Vec<Result<Event, Infallible>>,
) -> bool {
    for sse in events {
        if sender.send(sse).await.is_err() {
            return false;
        }
    }
    true
}

async fn append_compatible_resume_terminal_event(
    state: &ApiState,
    detail: &domain::ApplicationRunDetail,
) {
    let run = native_result_from_run_detail(detail, resume_metadata_from_detail(detail));
    let Some(event) = terminal_runtime_event_from_native_run(&run) else {
        return;
    };
    let close_reason = match run.status {
        NativeRunStatus::Succeeded => control_plane::ports::RuntimeEventCloseReason::Finished,
        NativeRunStatus::Failed => control_plane::ports::RuntimeEventCloseReason::Failed,
        NativeRunStatus::Cancelled => control_plane::ports::RuntimeEventCloseReason::Cancelled,
        NativeRunStatus::Waiting => control_plane::ports::RuntimeEventCloseReason::WaitingCallback,
        NativeRunStatus::Created | NativeRunStatus::Queued | NativeRunStatus::Running => return,
    };
    let _ = state
        .runtime_event_stream
        .append(run.id, runtime_event_payload_from_envelope(event))
        .await;
    let _ = state
        .runtime_event_stream
        .close_run(run.id, close_reason)
        .await;
}

fn runtime_event_payload_from_envelope(envelope: RuntimeEventEnvelope) -> RuntimeEventPayload {
    RuntimeEventPayload {
        event_type: envelope.event_type,
        source: envelope.source,
        durability: envelope.durability,
        persist_required: envelope.persist_required,
        trace_visible: envelope.trace_visible,
        payload: envelope.payload,
    }
}

fn resume_metadata_from_detail(detail: &domain::ApplicationRunDetail) -> Value {
    json!({
        "external_user": detail.flow_run.external_user,
        "external_conversation_id": detail.flow_run.external_conversation_id,
        "external_trace_id": detail.flow_run.external_trace_id,
        "compatibility_mode": detail.flow_run.compatibility_mode,
        "idempotency_key": detail.flow_run.idempotency_key,
        "request": {
            "conversation": {
                "id": detail.flow_run.external_conversation_id,
                "user": detail.flow_run.external_user,
            }
        }
    })
}

struct CompatibleTerminalFallback<'a, F> {
    state: &'a ApiState,
    initial_run: &'a NativeRunResult,
    sender: &'a mpsc::Sender<Result<Event, Infallible>>,
    mapper: &'a mut F,
    stats: &'a mut CompatibleStreamStats,
    trigger: &'static str,
    warn_if_not_terminal: bool,
    ignored_waiting_callback_task_id: Option<uuid::Uuid>,
}

enum CompatibleTerminalFallbackOutcome {
    NotTerminal,
    Sent { event_type: String },
    ClientDisconnected { event_type: Option<String> },
    IgnoredWaitingCallback,
}

async fn emit_compatible_terminal_fallback<F>(
    fallback: CompatibleTerminalFallback<'_, F>,
) -> CompatibleTerminalFallbackOutcome
where
    F: FnMut(&NativeRunResult, RuntimeEventEnvelope) -> Vec<Result<Event, Infallible>>,
{
    let CompatibleTerminalFallback {
        state,
        initial_run,
        sender,
        mapper,
        stats,
        trigger,
        warn_if_not_terminal,
        ignored_waiting_callback_task_id,
    } = fallback;

    let latest_run = load_latest_native_run_for_terminal_fallback(state, initial_run).await;
    let Some(terminal_event) = terminal_runtime_event_from_native_run(&latest_run) else {
        if warn_if_not_terminal {
            warn!(
                flow_run_id = %initial_run.id,
                application_id = %initial_run.application_id,
                status = ?latest_run.status,
                trigger = %trigger,
                "compatible public API stream ended before durable run reached a terminal state"
            );
        }
        return CompatibleTerminalFallbackOutcome::NotTerminal;
    };

    warn!(
        flow_run_id = %initial_run.id,
        application_id = %initial_run.application_id,
        status = ?latest_run.status,
        trigger = %trigger,
        "compatible public API stream missing runtime terminal event; emitting durable terminal fallback"
    );
    if is_ignored_waiting_callback(&terminal_event, ignored_waiting_callback_task_id) {
        debug!(
            flow_run_id = %initial_run.id,
            application_id = %initial_run.application_id,
            trigger = %trigger,
            "compatible public API resume stream ignored stale waiting callback terminal fallback"
        );
        return CompatibleTerminalFallbackOutcome::IgnoredWaitingCallback;
    }

    if !stats.emitted_public_event {
        let started_event = RuntimeEventEnvelope::new(
            latest_run.id,
            0,
            debug_stream_events::flow_started(latest_run.id),
        );
        let events = mapper(&latest_run, started_event.clone());
        let emitted_public_event = !events.is_empty();
        if !send_compatible_sse_events(sender, events).await {
            return CompatibleTerminalFallbackOutcome::ClientDisconnected { event_type: None };
        }
        stats.record_sent_runtime_event(&latest_run, &started_event, emitted_public_event);
    }
    let terminal_event =
        enrich_terminal_runtime_event_with_durable_answer(state, &latest_run, terminal_event).await;
    let event_type = terminal_event.event_type.clone();
    let events = mapper(&latest_run, terminal_event.clone());
    let emitted_public_event = !events.is_empty();
    if !send_compatible_sse_events(sender, events).await {
        return CompatibleTerminalFallbackOutcome::ClientDisconnected {
            event_type: Some(event_type),
        };
    }
    stats.record_sent_runtime_event(&latest_run, &terminal_event, emitted_public_event);
    CompatibleTerminalFallbackOutcome::Sent { event_type }
}

fn is_ignored_waiting_callback(
    event: &RuntimeEventEnvelope,
    ignored_waiting_callback_task_id: Option<uuid::Uuid>,
) -> bool {
    if event.event_type != "waiting_callback" {
        return false;
    }
    let Some(ignored_task_id) = ignored_waiting_callback_task_id else {
        return false;
    };
    event
        .payload
        .get("callback_task_id")
        .and_then(Value::as_str)
        .and_then(|value| uuid::Uuid::parse_str(value).ok())
        == Some(ignored_task_id)
}

fn is_public_terminal_runtime_event(event_type: &str) -> bool {
    matches!(
        event_type,
        "flow_finished" | "flow_failed" | "flow_cancelled" | "waiting_human" | "waiting_callback"
    )
}

fn is_answer_presentation_delta(envelope: &RuntimeEventEnvelope) -> bool {
    matches!(
        envelope.event_type.as_str(),
        "reasoning_delta" | "text_delta"
    ) && debug_stream_events::is_answer_presentation_delta_payload(&envelope.payload)
}

pub(crate) fn openai_chat_completion_id_from_run_id(run_id: uuid::Uuid) -> String {
    format!("chatcmpl-{run_id}")
}

pub(crate) fn openai_chat_completion_id_from_callback_task(
    run_id: uuid::Uuid,
    callback_task_id: uuid::Uuid,
) -> String {
    format!("chatcmpl-{run_id}-{callback_task_id}")
}

struct OpenAiChatStreamMapper {
    model: String,
    chat_completion_id: String,
    terminal_answer_fallback: bool,
    emitted_reasoning_delta: bool,
    emitted_text_delta: bool,
}

impl OpenAiChatStreamMapper {
    fn new(model: String, chat_completion_id: String, terminal_answer_fallback: bool) -> Self {
        Self {
            model,
            chat_completion_id,
            terminal_answer_fallback,
            emitted_reasoning_delta: false,
            emitted_text_delta: false,
        }
    }

    fn runtime_event_to_sse(
        &mut self,
        initial_run: &NativeRunResult,
        envelope: RuntimeEventEnvelope,
    ) -> Vec<Result<Event, Infallible>> {
        let is_answer_presentation_delta = is_answer_presentation_delta(&envelope);
        match envelope.event_type.as_str() {
            "reasoning_delta"
                if is_answer_presentation_delta
                    && envelope
                        .text
                        .as_deref()
                        .is_some_and(|text| !text.is_empty()) =>
            {
                self.emitted_reasoning_delta = true;
            }
            "text_delta"
                if is_answer_presentation_delta
                    && envelope
                        .text
                        .as_deref()
                        .is_some_and(|text| !text.is_empty()) =>
            {
                self.emitted_text_delta = true;
            }
            _ => {}
        }

        let terminal_deltas = if self.terminal_answer_fallback
            && matches!(
                envelope.event_type.as_str(),
                "flow_finished" | "flow_failed"
            ) {
            terminal_answer_deltas_from_run_or_payload(initial_run, &envelope.payload)
        } else {
            Vec::new()
        };

        let mut events = Vec::new();
        let had_reasoning_delta = self.emitted_reasoning_delta;
        let had_text_delta = self.emitted_text_delta;
        for delta in terminal_deltas {
            match delta.kind {
                TerminalAnswerDeltaKind::Reasoning if !had_reasoning_delta => {
                    if let Some(payload) = openai_delta_chunk_payload(
                        initial_run,
                        &self.model,
                        &self.chat_completion_id,
                        "reasoning_delta",
                        delta.text,
                    ) {
                        events.push(json_sse(payload));
                        self.emitted_reasoning_delta = true;
                    }
                }
                TerminalAnswerDeltaKind::Text if !had_text_delta => {
                    if let Some(payload) = openai_delta_chunk_payload(
                        initial_run,
                        &self.model,
                        &self.chat_completion_id,
                        "text_delta",
                        delta.text,
                    ) {
                        events.push(json_sse(payload));
                        self.emitted_text_delta = true;
                    }
                }
                _ => {}
            }
        }
        events.extend(openai_runtime_event_to_sse(
            initial_run,
            &self.model,
            &self.chat_completion_id,
            envelope,
        ));
        events
    }
}

struct OpenAiResponseStreamMapper {
    model: String,
    previous_response_id: Option<String>,
    terminal_answer_fallback: bool,
    emitted_reasoning_delta: bool,
    emitted_text_delta: bool,
}

impl OpenAiResponseStreamMapper {
    fn new(
        model: String,
        previous_response_id: Option<String>,
        terminal_answer_fallback: bool,
    ) -> Self {
        Self {
            model,
            previous_response_id,
            terminal_answer_fallback,
            emitted_reasoning_delta: false,
            emitted_text_delta: false,
        }
    }

    fn runtime_event_to_sse(
        &mut self,
        initial_run: &NativeRunResult,
        envelope: RuntimeEventEnvelope,
    ) -> Vec<Result<Event, Infallible>> {
        let is_answer_presentation_delta = is_answer_presentation_delta(&envelope);
        match envelope.event_type.as_str() {
            "reasoning_delta"
                if is_answer_presentation_delta
                    && envelope
                        .text
                        .as_deref()
                        .is_some_and(|text| !text.is_empty()) =>
            {
                self.emitted_reasoning_delta = true;
            }
            "text_delta"
                if is_answer_presentation_delta
                    && envelope
                        .text
                        .as_deref()
                        .is_some_and(|text| !text.is_empty()) =>
            {
                self.emitted_text_delta = true;
            }
            _ => {}
        }

        let terminal_deltas = if self.terminal_answer_fallback
            && matches!(
                envelope.event_type.as_str(),
                "flow_finished" | "flow_failed"
            ) {
            terminal_answer_deltas_from_run_or_payload(initial_run, &envelope.payload)
        } else {
            Vec::new()
        };

        let mut events = Vec::new();
        let had_reasoning_delta = self.emitted_reasoning_delta;
        let had_text_delta = self.emitted_text_delta;
        for delta in terminal_deltas {
            match delta.kind {
                TerminalAnswerDeltaKind::Reasoning if !had_reasoning_delta => {
                    events.push(event_json_sse(
                        "response.reasoning_text.delta",
                        openai_response_reasoning_text_delta_payload(initial_run, delta.text),
                    ));
                    self.emitted_reasoning_delta = true;
                }
                TerminalAnswerDeltaKind::Text if !had_text_delta => {
                    events.push(event_json_sse(
                        "response.output_text.delta",
                        openai_response_output_text_delta_payload(initial_run, delta.text),
                    ));
                    self.emitted_text_delta = true;
                }
                _ => {}
            }
        }
        events.extend(openai_response_runtime_event_to_sse(
            initial_run,
            &self.model,
            self.previous_response_id.as_deref(),
            envelope,
        ));
        events
    }
}

fn terminal_answer_text(run: &NativeRunResult, payload: &Value) -> Option<String> {
    terminal_answer_text_from_payload(payload).or_else(|| {
        run.answer
            .as_ref()
            .filter(|answer| !answer.is_empty())
            .cloned()
    })
}

fn terminal_answer_deltas_from_run_or_payload(
    run: &NativeRunResult,
    payload: &Value,
) -> Vec<TerminalAnswerDelta> {
    terminal_answer_text(run, payload)
        .map(|answer| terminal_answer_deltas_from_payload(&json!({ "answer": answer })))
        .unwrap_or_default()
}

fn openai_runtime_event_to_sse(
    initial_run: &NativeRunResult,
    model: &str,
    chat_completion_id: &str,
    envelope: RuntimeEventEnvelope,
) -> Vec<Result<Event, Infallible>> {
    match envelope.event_type.as_str() {
        "flow_started" => vec![json_sse(json!({
            "id": chat_completion_id,
            "object": "chat.completion.chunk",
            "created": initial_run.created_at.unix_timestamp(),
            "model": model,
            "choices": [{
                "index": 0,
                "delta": { "role": "assistant" },
                "finish_reason": null
            }]
        }))],
        "text_delta" | "reasoning_delta" if is_answer_presentation_delta(&envelope) => {
            openai_delta_chunk_payload(
                initial_run,
                model,
                chat_completion_id,
                envelope.event_type.as_str(),
                envelope.text.unwrap_or_default(),
            )
            .map(json_sse)
            .into_iter()
            .collect()
        }
        "text_delta" | "reasoning_delta" => Vec::new(),
        "flow_finished" => vec![
            json_sse(openai_finish_chunk_payload(
                initial_run,
                model,
                chat_completion_id,
                "stop",
            )),
            done_sse(),
        ],
        "flow_failed" => vec![
            if terminal_answer_text(initial_run, &envelope.payload).is_some() {
                json_sse(openai_finish_chunk_payload(
                    initial_run,
                    model,
                    chat_completion_id,
                    "stop",
                ))
            } else {
                json_sse(json!({
                    "error": {
                        "message": runtime_error_message(&envelope.payload),
                        "type": "server_error",
                        "param": null,
                        "code": "runtime_error"
                    }
                }))
            },
            done_sse(),
        ],
        "flow_cancelled" => vec![done_sse()],
        "waiting_callback" => openai_tool_call_chunk_payload(
            initial_run,
            model,
            chat_completion_id,
            &envelope.payload,
        )
        .map(|payload| {
            vec![
                json_sse(payload),
                json_sse(openai_finish_chunk_payload(
                    initial_run,
                    model,
                    chat_completion_id,
                    "tool_calls",
                )),
                done_sse(),
            ]
        })
        .unwrap_or_else(required_action_not_supported_openai_sse),
        "waiting_human" => required_action_not_supported_openai_sse(),
        _ => Vec::new(),
    }
}

fn openai_response_runtime_event_to_sse(
    initial_run: &NativeRunResult,
    model: &str,
    previous_response_id: Option<&str>,
    envelope: RuntimeEventEnvelope,
) -> Vec<Result<Event, Infallible>> {
    match envelope.event_type.as_str() {
        "flow_started" => vec![event_json_sse(
            "response.created",
            json!({
                "type": "response.created",
                "response": openai_response_stream_snapshot(
                    initial_run,
                    model,
                    previous_response_id,
                    "in_progress"
                )
            }),
        )],
        "text_delta" if is_answer_presentation_delta(&envelope) => vec![event_json_sse(
            "response.output_text.delta",
            openai_response_output_text_delta_payload(
                initial_run,
                envelope.text.unwrap_or_default(),
            ),
        )],
        "reasoning_delta" if is_answer_presentation_delta(&envelope) => vec![event_json_sse(
            "response.reasoning_text.delta",
            json!({
                "type": "response.reasoning_text.delta",
                "response_id": response_id_from_run_id(initial_run.id),
                "item_id": format!("msg_{}", initial_run.id),
                "output_index": 0,
                "content_index": 0,
                "delta": envelope.text.unwrap_or_default()
            }),
        )],
        "text_delta" | "reasoning_delta" => Vec::new(),
        "flow_finished" => vec![event_json_sse(
            "response.completed",
            json!({
                "type": "response.completed",
                "response": openai_response_completed_snapshot(
                    initial_run,
                    model,
                    previous_response_id
                )
            }),
        )],
        "flow_failed" => {
            if terminal_answer_text(initial_run, &envelope.payload).is_some() {
                vec![event_json_sse(
                    "response.completed",
                    json!({
                        "type": "response.completed",
                        "response": openai_response_completed_snapshot(
                            initial_run,
                            model,
                            previous_response_id
                        )
                    }),
                )]
            } else {
                vec![event_json_sse(
                    "response.failed",
                    json!({
                        "type": "response.failed",
                        "response": openai_response_stream_snapshot(
                            initial_run,
                            model,
                            previous_response_id,
                            "failed"
                        ),
                        "error": {
                            "message": runtime_error_message(&envelope.payload),
                            "type": "server_error",
                            "param": null,
                            "code": "runtime_error"
                        }
                    }),
                )]
            }
        }
        "flow_cancelled" => vec![event_json_sse(
            "response.failed",
            json!({
                "type": "response.failed",
                "response": openai_response_stream_snapshot(
                    initial_run,
                    model,
                    previous_response_id,
                    "failed"
                ),
                "error": {
                    "message": "published run cancelled",
                    "type": "invalid_request_error",
                    "param": null,
                    "code": "run_cancelled"
                }
            }),
        )],
        "waiting_callback" => openai_response_function_call_output_items(&envelope.payload)
            .map(|items| {
                openai_response_function_call_sse(initial_run, model, previous_response_id, items)
            })
            .unwrap_or_else(|| {
                required_action_not_supported_openai_response_sse(
                    initial_run,
                    model,
                    previous_response_id,
                )
            }),
        "waiting_human" => required_action_not_supported_openai_response_sse(
            initial_run,
            model,
            previous_response_id,
        ),
        _ => Vec::new(),
    }
}

fn openai_response_stream_snapshot(
    initial_run: &NativeRunResult,
    model: &str,
    previous_response_id: Option<&str>,
    status: &'static str,
) -> Value {
    json!({
        "id": response_id_from_run_id(initial_run.id),
        "object": "response",
        "created_at": initial_run.created_at.unix_timestamp(),
        "status": status,
        "model": model,
        "output": [],
        "output_text": "",
        "previous_response_id": previous_response_id
    })
}

fn openai_response_completed_snapshot(
    initial_run: &NativeRunResult,
    model: &str,
    previous_response_id: Option<&str>,
) -> Value {
    let mut response =
        openai_response_stream_snapshot(initial_run, model, previous_response_id, "completed");
    response["usage"] = openai_responses_usage_payload(initial_run.usage.as_ref());
    response
}

fn openai_responses_usage_payload(usage: Option<&NativeUsage>) -> Value {
    let Some(usage) = usage else {
        return json!({
            "input_tokens": 0,
            "output_tokens": 0,
            "total_tokens": 0
        });
    };

    json!({
        "input_tokens": usage.prompt_tokens.unwrap_or_default(),
        "output_tokens": usage.completion_tokens.unwrap_or_default(),
        "total_tokens": usage.total_tokens.unwrap_or_default()
    })
}

fn openai_response_output_text_delta_payload(initial_run: &NativeRunResult, text: String) -> Value {
    json!({
        "type": "response.output_text.delta",
        "response_id": response_id_from_run_id(initial_run.id),
        "item_id": format!("msg_{}", initial_run.id),
        "output_index": 0,
        "content_index": 0,
        "delta": text
    })
}

fn openai_response_reasoning_text_delta_payload(
    initial_run: &NativeRunResult,
    text: String,
) -> Value {
    json!({
        "type": "response.reasoning_text.delta",
        "response_id": response_id_from_run_id(initial_run.id),
        "item_id": format!("msg_{}", initial_run.id),
        "output_index": 0,
        "content_index": 0,
        "delta": text
    })
}

fn openai_delta_chunk_payload(
    initial_run: &NativeRunResult,
    model: &str,
    chat_completion_id: &str,
    event_type: &str,
    text: String,
) -> Option<Value> {
    let delta = match event_type {
        "text_delta" => json!({ "content": text }),
        "reasoning_delta" => json!({ "reasoning_content": text }),
        _ => return None,
    };

    Some(json!({
        "id": chat_completion_id,
        "object": "chat.completion.chunk",
        "created": initial_run.created_at.unix_timestamp(),
        "model": model,
        "choices": [{
            "index": 0,
            "delta": delta,
            "finish_reason": null
        }]
    }))
}

fn openai_tool_call_chunk_payload(
    initial_run: &NativeRunResult,
    model: &str,
    chat_completion_id: &str,
    payload: &Value,
) -> Option<Value> {
    let callback_task_id = llm_tool_callback_task_id(payload)?;
    let calls = llm_tool_calls(payload)?;
    let tool_calls = calls
        .iter()
        .enumerate()
        .filter_map(|(index, call)| {
            let name = call.get("name").and_then(Value::as_str)?;
            let original_id = call
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or("tool_call")
                .to_string();
            let arguments = call.get("arguments").cloned().unwrap_or_else(|| json!({}));
            Some(json!({
                "index": index,
                "id": encode_openai_callback_tool_call_id(callback_task_id, &original_id),
                "type": "function",
                "function": {
                    "name": name,
                    "arguments": tool_call_arguments_string(arguments)
                }
            }))
        })
        .collect::<Vec<_>>();
    if tool_calls.is_empty() {
        return None;
    }

    Some(json!({
        "id": chat_completion_id,
        "object": "chat.completion.chunk",
        "created": initial_run.created_at.unix_timestamp(),
        "model": model,
        "choices": [{
            "index": 0,
            "delta": { "tool_calls": tool_calls },
            "finish_reason": null
        }]
    }))
}

fn openai_finish_chunk_payload(
    initial_run: &NativeRunResult,
    model: &str,
    chat_completion_id: &str,
    finish_reason: &'static str,
) -> Value {
    json!({
        "id": chat_completion_id,
        "object": "chat.completion.chunk",
        "created": initial_run.created_at.unix_timestamp(),
        "model": model,
        "choices": [{
            "index": 0,
            "delta": {
                "content": "",
                "role": null
            },
            "finish_reason": finish_reason
        }],
        "usage": openai_chat_usage_payload(initial_run.usage.as_ref())
    })
}

fn openai_chat_usage_payload(usage: Option<&NativeUsage>) -> Value {
    let Some(usage) = usage else {
        return json!({
            "prompt_tokens": 0,
            "completion_tokens": 0,
            "total_tokens": 0
        });
    };

    json!({
        "prompt_tokens": usage.prompt_tokens.unwrap_or_default(),
        "completion_tokens": usage.completion_tokens.unwrap_or_default(),
        "total_tokens": usage.total_tokens.unwrap_or_default()
    })
}

fn anthropic_message_start_usage_payload(usage: Option<&NativeUsage>) -> Value {
    let Some(usage) = usage else {
        return json!({
            "input_tokens": 0,
            "cache_creation_input_tokens": 0,
            "cache_read_input_tokens": 0,
            "output_tokens": 0
        });
    };

    json!({
        "input_tokens": usage.prompt_tokens.unwrap_or_default(),
        "cache_creation_input_tokens": usage.cache_write_tokens.unwrap_or_default(),
        "cache_read_input_tokens": anthropic_cache_read_input_tokens(usage),
        "output_tokens": 0
    })
}

fn anthropic_message_delta_usage_payload(usage: Option<&NativeUsage>) -> Value {
    let Some(usage) = usage else {
        return json!({
            "input_tokens": 0,
            "cache_creation_input_tokens": 0,
            "cache_read_input_tokens": 0,
            "output_tokens": 0
        });
    };

    json!({
        "input_tokens": usage.prompt_tokens.unwrap_or_default(),
        "cache_creation_input_tokens": usage.cache_write_tokens.unwrap_or_default(),
        "cache_read_input_tokens": anthropic_cache_read_input_tokens(usage),
        "output_tokens": usage.completion_tokens.unwrap_or_default()
    })
}

fn anthropic_cache_read_input_tokens(usage: &NativeUsage) -> u64 {
    usage
        .cache_read_tokens
        .or(usage.input_cache_hit_tokens)
        .unwrap_or_default()
}

fn openai_response_function_call_output_items(payload: &Value) -> Option<Vec<Value>> {
    let callback_task_id = llm_tool_callback_task_id(payload)?;
    let calls = llm_tool_calls(payload)?;
    let output = calls
        .iter()
        .filter_map(|call| {
            let name = call.get("name").and_then(Value::as_str)?;
            let original_id = call
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or("tool_call")
                .to_string();
            let arguments = call.get("arguments").cloned().unwrap_or_else(|| json!({}));
            Some(json!({
                "id": format!("fc_{}", original_id),
                "type": "function_call",
                "call_id": encode_openai_callback_tool_call_id(callback_task_id, &original_id),
                "name": name,
                "arguments": tool_call_arguments_string(arguments),
                "status": "completed"
            }))
        })
        .collect::<Vec<_>>();
    (!output.is_empty()).then_some(output)
}

fn openai_response_function_call_sse(
    initial_run: &NativeRunResult,
    model: &str,
    previous_response_id: Option<&str>,
    output: Vec<Value>,
) -> Vec<Result<Event, Infallible>> {
    let mut events = output
        .iter()
        .enumerate()
        .map(|(index, item)| {
            event_json_sse(
                "response.output_item.added",
                json!({
                    "type": "response.output_item.added",
                    "response_id": response_id_from_run_id(initial_run.id),
                    "output_index": index,
                    "item": item
                }),
            )
        })
        .collect::<Vec<_>>();
    events.push(event_json_sse(
        "response.completed",
        json!({
            "type": "response.completed",
            "response": openai_response_stream_snapshot_with_output(
                initial_run,
                model,
                previous_response_id,
                "completed",
                output
            )
        }),
    ));
    events
}

fn openai_response_stream_snapshot_with_output(
    initial_run: &NativeRunResult,
    model: &str,
    previous_response_id: Option<&str>,
    status: &'static str,
    output: Vec<Value>,
) -> Value {
    let mut response = json!({
        "id": response_id_from_run_id(initial_run.id),
        "object": "response",
        "created_at": initial_run.created_at.unix_timestamp(),
        "status": status,
        "model": model,
        "output": output,
        "output_text": "",
        "previous_response_id": previous_response_id
    });
    response["usage"] = openai_responses_usage_payload(initial_run.usage.as_ref());
    response
}

fn anthropic_tool_use_blocks_from_waiting_payload(payload: &Value) -> Option<Vec<Value>> {
    let callback_task_id = llm_tool_callback_task_id(payload)?;
    let calls = llm_tool_calls(payload)?;
    let blocks = calls
        .iter()
        .filter_map(|call| {
            let name = call.get("name").and_then(Value::as_str)?;
            let original_id = call
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or("toolu_call")
                .to_string();
            let input = call.get("arguments").cloned().unwrap_or_else(|| json!({}));
            Some(json!({
                "type": "tool_use",
                "id": encode_anthropic_callback_tool_use_id(callback_task_id, &original_id),
                "name": name,
                "input": input
            }))
        })
        .collect::<Vec<_>>();
    (!blocks.is_empty()).then_some(blocks)
}

fn anthropic_completed_run_to_sse(
    run: &NativeRunResult,
    model: &str,
) -> Vec<Result<Event, Infallible>> {
    let mut mapper = AnthropicStreamMapper::new(model.to_string());
    let mut events = mapper.runtime_event_to_sse(
        run,
        RuntimeEventEnvelope::new(run.id, 0, debug_stream_events::flow_started(run.id)),
    );
    if let Some(payload) = waiting_payload_from_run(run) {
        if let Some(tool_events) = mapper.anthropic_tool_use_events(&payload, run.usage.as_ref()) {
            events.extend(tool_events);
            return events;
        }
    }
    if let Some(answer) = run.answer.as_ref().filter(|answer| !answer.is_empty()) {
        let deltas = terminal_answer_deltas_from_payload(&json!({ "answer": answer }));
        for (index, delta) in deltas.into_iter().enumerate() {
            let event = terminal_answer_delta_to_runtime_event(run, index as i64 + 1, delta);
            events.extend(mapper.runtime_event_to_sse(run, event));
        }
    }
    events.extend(mapper.anthropic_stop_events(run.usage.as_ref()));
    events
}

fn terminal_answer_delta_to_runtime_event(
    run: &NativeRunResult,
    sequence: i64,
    delta: TerminalAnswerDelta,
) -> RuntimeEventEnvelope {
    let payload = match delta.kind {
        TerminalAnswerDeltaKind::Reasoning => debug_stream_events::answer_reasoning_delta(
            "assistant",
            delta.text,
            sequence as usize,
            None,
            None,
            None,
        ),
        TerminalAnswerDeltaKind::Text => debug_stream_events::answer_text_delta(
            "assistant",
            delta.text,
            sequence as usize,
            None,
            None,
            None,
        ),
    };
    RuntimeEventEnvelope::new(run.id, sequence, payload)
}

fn waiting_payload_from_run(run: &NativeRunResult) -> Option<Value> {
    let action = run.required_action.as_ref()?;
    Some(json!({
        "callback_kind": action.payload.get("callback_kind").cloned().unwrap_or(Value::Null),
        "callback_task_id": action.payload.get("callback_task_id").cloned().unwrap_or(Value::Null),
        "tool_calls": run.tool_calls.clone().unwrap_or(Value::Null),
    }))
}

fn llm_tool_callback_task_id(payload: &Value) -> Option<uuid::Uuid> {
    if payload.get("callback_kind").and_then(Value::as_str) != Some("llm_tool_calls") {
        return None;
    }
    payload
        .get("callback_task_id")
        .and_then(Value::as_str)
        .and_then(|value| uuid::Uuid::parse_str(value).ok())
}

fn llm_tool_calls(payload: &Value) -> Option<&Vec<Value>> {
    payload
        .get("tool_calls")
        .and_then(Value::as_array)
        .or_else(|| {
            payload
                .get("request_payload")
                .and_then(|request| request.get("tool_calls"))
                .and_then(Value::as_array)
        })
        .filter(|calls| !calls.is_empty())
}

fn tool_call_arguments_string(arguments: Value) -> String {
    match arguments {
        Value::String(value) => value,
        value => value.to_string(),
    }
}

fn required_action_not_supported_openai_sse() -> Vec<Result<Event, Infallible>> {
    vec![
        json_sse(json!({
            "error": {
                "message": "waiting states are not supported by compatible endpoints; use the Native API to inspect and resume required_action runs",
                "type": "invalid_request_error",
                "param": null,
                "code": "required_action_not_supported"
            }
        })),
        done_sse(),
    ]
}

fn required_action_not_supported_openai_response_sse(
    initial_run: &NativeRunResult,
    model: &str,
    previous_response_id: Option<&str>,
) -> Vec<Result<Event, Infallible>> {
    vec![event_json_sse(
        "response.failed",
        json!({
            "type": "response.failed",
            "response": openai_response_stream_snapshot(
                initial_run,
                model,
                previous_response_id,
                "failed"
            ),
            "error": {
                "message": "waiting states are not supported by compatible endpoints; use the Native API to inspect and resume required_action runs",
                "type": "invalid_request_error",
                "param": null,
                "code": "required_action_not_supported"
            }
        }),
    )]
}

fn required_action_not_supported_anthropic_sse() -> Vec<Result<Event, Infallible>> {
    vec![event_json_sse(
        "error",
        json!({
            "type": "error",
            "error": {
                "type": "required_action_not_supported",
                "message": "waiting states are not supported by compatible endpoints; use the Native API to inspect and resume required_action runs"
            }
        }),
    )]
}

struct AnthropicStreamMapper {
    model: String,
    next_content_index: u32,
    active_content: bool,
    emitted_text_delta: bool,
}

impl AnthropicStreamMapper {
    fn new(model: String) -> Self {
        Self {
            model,
            next_content_index: 0,
            active_content: false,
            emitted_text_delta: false,
        }
    }

    fn runtime_event_to_sse(
        &mut self,
        initial_run: &NativeRunResult,
        envelope: RuntimeEventEnvelope,
    ) -> Vec<Result<Event, Infallible>> {
        let is_answer_presentation_delta = is_answer_presentation_delta(&envelope);
        match envelope.event_type.as_str() {
            "flow_started" => vec![event_json_sse(
                "message_start",
                json!({
                    "type": "message_start",
                    "message": {
                        "id": format!("msg_{}", initial_run.id),
                        "type": "message",
                        "role": "assistant",
                        "model": self.model,
                        "content": [],
                        "stop_reason": null,
                        "usage": anthropic_message_start_usage_payload(initial_run.usage.as_ref())
                    }
                }),
            )],
            "reasoning_delta" if is_answer_presentation_delta => Vec::new(),
            "text_delta"
                if is_answer_presentation_delta
                    && envelope
                        .text
                        .as_deref()
                        .is_some_and(|text| !text.is_empty()) =>
            {
                self.emitted_text_delta = true;
                let mut events = self.ensure_anthropic_content_block();
                let (event_name, payload) = anthropic_delta_payload(
                    self.active_content_index(),
                    "text_delta",
                    envelope.text.unwrap_or_default(),
                )
                .expect("text_delta should map to Anthropic text_delta");
                events.push(event_json_sse(event_name, payload));
                events
            }
            "text_delta" | "reasoning_delta" => Vec::new(),
            "flow_finished" => self.anthropic_terminal_events(initial_run, &envelope.payload),
            "waiting_callback" => self
                .anthropic_tool_use_events(&envelope.payload, initial_run.usage.as_ref())
                .unwrap_or_else(required_action_not_supported_anthropic_sse),
            "waiting_human" => required_action_not_supported_anthropic_sse(),
            "flow_failed" => {
                if terminal_answer_text(initial_run, &envelope.payload).is_some() {
                    self.anthropic_terminal_events(initial_run, &envelope.payload)
                } else {
                    vec![event_json_sse(
                        "error",
                        json!({
                            "type": "error",
                            "error": {
                                "type": "api_error",
                                "message": runtime_error_message(&envelope.payload)
                            }
                        }),
                    )]
                }
            }
            "flow_cancelled" => self.anthropic_stop_events(initial_run.usage.as_ref()),
            _ => Vec::new(),
        }
    }

    fn anthropic_terminal_events(
        &mut self,
        initial_run: &NativeRunResult,
        payload: &Value,
    ) -> Vec<Result<Event, Infallible>> {
        let mut events = Vec::new();
        let had_text_delta = self.emitted_text_delta;
        for delta in terminal_answer_deltas_from_run_or_payload(initial_run, payload) {
            match delta.kind {
                TerminalAnswerDeltaKind::Text if !had_text_delta => {
                    events.extend(self.anthropic_delta_events("text_delta", delta.text));
                    self.emitted_text_delta = true;
                }
                _ => {}
            }
        }
        events.extend(self.anthropic_stop_events(initial_run.usage.as_ref()));
        events
    }

    fn anthropic_delta_events(
        &mut self,
        event_type: &str,
        text: String,
    ) -> Vec<Result<Event, Infallible>> {
        let mut events = self.ensure_anthropic_content_block();
        let (event_name, payload) =
            anthropic_delta_payload(self.active_content_index(), event_type, text)
                .expect("known Anthropic delta event type should map");
        events.push(event_json_sse(event_name, payload));
        events
    }

    fn anthropic_stop_events(
        &mut self,
        usage: Option<&NativeUsage>,
    ) -> Vec<Result<Event, Infallible>> {
        let mut events = Vec::new();
        if !self.active_content && self.next_content_index == 0 {
            events.extend(self.ensure_anthropic_content_block());
        }
        events.extend(self.close_active_anthropic_content_block());
        events.push(event_json_sse(
            "message_delta",
            json!({
                "type": "message_delta",
                "delta": {"stop_reason": "end_turn"},
                "usage": anthropic_message_delta_usage_payload(usage)
            }),
        ));
        events.push(event_json_sse(
            "message_stop",
            json!({"type": "message_stop"}),
        ));
        events
    }

    fn anthropic_tool_use_events(
        &mut self,
        payload: &Value,
        usage: Option<&NativeUsage>,
    ) -> Option<Vec<Result<Event, Infallible>>> {
        let blocks = anthropic_tool_use_blocks_from_waiting_payload(payload)?;
        let mut events = self.close_active_anthropic_content_block();
        for block in blocks {
            let index = self.next_content_index;
            self.next_content_index += 1;
            let input = block.get("input").cloned().unwrap_or_else(|| json!({}));
            let mut start_block = block;
            if let Some(object) = start_block.as_object_mut() {
                object.insert("input".to_string(), json!({}));
            }
            events.push(event_json_sse(
                "content_block_start",
                json!({
                    "type": "content_block_start",
                    "index": index,
                    "content_block": start_block
                }),
            ));
            if input != json!({}) {
                events.push(event_json_sse(
                    "content_block_delta",
                    json!({
                        "type": "content_block_delta",
                        "index": index,
                        "delta": {
                            "type": "input_json_delta",
                            "partial_json": serde_json::to_string(&input)
                                .expect("tool input JSON should serialize")
                        }
                    }),
                ));
            }
            events.push(event_json_sse(
                "content_block_stop",
                json!({"type": "content_block_stop", "index": index}),
            ));
        }
        events.push(event_json_sse(
            "message_delta",
            json!({
                "type": "message_delta",
                "delta": {"stop_reason": "tool_use"},
                "usage": anthropic_message_delta_usage_payload(usage)
            }),
        ));
        events.push(event_json_sse(
            "message_stop",
            json!({"type": "message_stop"}),
        ));
        Some(events)
    }

    fn ensure_anthropic_content_block(&mut self) -> Vec<Result<Event, Infallible>> {
        if self.active_content {
            return Vec::new();
        }

        let mut events = self.close_active_anthropic_content_block();
        let index = self.next_content_index;
        self.next_content_index += 1;
        self.active_content = true;
        events.push(event_json_sse(
            "content_block_start",
            json!({
                "type": "content_block_start",
                "index": index,
                "content_block": {"type": "text", "text": ""}
            }),
        ));
        events
    }

    fn close_active_anthropic_content_block(&mut self) -> Vec<Result<Event, Infallible>> {
        if !self.active_content {
            return Vec::new();
        }
        let index = self.active_content_index();
        self.active_content = false;
        vec![event_json_sse(
            "content_block_stop",
            json!({"type": "content_block_stop", "index": index}),
        )]
    }

    fn active_content_index(&self) -> u32 {
        self.next_content_index.saturating_sub(1)
    }
}

fn anthropic_delta_payload(
    index: u32,
    event_type: &str,
    text: String,
) -> Option<(&'static str, Value)> {
    let delta = match event_type {
        "text_delta" => json!({
            "type": "text_delta",
            "text": text
        }),
        _ => return None,
    };

    Some((
        "content_block_delta",
        json!({
            "type": "content_block_delta",
            "index": index,
            "delta": delta
        }),
    ))
}

fn runtime_error_message(payload: &Value) -> String {
    payload
        .get("error")
        .or_else(|| payload.get("message"))
        .and_then(Value::as_str)
        .unwrap_or("published run failed")
        .to_string()
}

fn json_sse(payload: Value) -> Result<Event, Infallible> {
    Ok(Event::default()
        .json_data(payload)
        .expect("compatible SSE payload should serialize"))
}

fn event_json_sse(event_name: &'static str, payload: Value) -> Result<Event, Infallible> {
    Ok(Event::default()
        .event(event_name)
        .json_data(payload)
        .expect("compatible SSE payload should serialize"))
}

fn done_sse() -> Result<Event, Infallible> {
    Ok(Event::default().data("[DONE]"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use control_plane::{
        application_public_api::native::{NativeRequiredAction, NativeRunStatus},
        ports::{
            AppendRuntimeEventInput, OrchestrationRuntimeRepository, RuntimeEventCloseReason,
            RuntimeEventDurability, RuntimeEventPayload, RuntimeEventSource, RuntimeEventStream,
            RuntimeEventStreamPolicy, RuntimeEventSubscription, RuntimeEventTrimPolicy,
        },
    };
    use serde_json::json;
    use std::sync::{Arc, Mutex};
    use time::OffsetDateTime;
    use tokio::sync::mpsc;
    use uuid::Uuid;

    use crate::routes::application_public_api::stream_terminal_fallback::recover_terminal_answer_deltas_from_durable_runtime_events;

    struct ReplayBeforeFallbackRuntimeEventStream {
        events: Vec<RuntimeEventEnvelope>,
        subscription_replay: Vec<RuntimeEventEnvelope>,
        live_senders: Mutex<Vec<mpsc::UnboundedSender<RuntimeEventEnvelope>>>,
    }

    impl ReplayBeforeFallbackRuntimeEventStream {
        fn new(events: Vec<RuntimeEventEnvelope>) -> Self {
            Self {
                events,
                subscription_replay: Vec::new(),
                live_senders: Mutex::new(Vec::new()),
            }
        }

        fn with_subscription_replay(
            subscription_replay: Vec<RuntimeEventEnvelope>,
            events: Vec<RuntimeEventEnvelope>,
        ) -> Self {
            Self {
                events,
                subscription_replay,
                live_senders: Mutex::new(Vec::new()),
            }
        }
    }

    #[async_trait]
    impl RuntimeEventStream for ReplayBeforeFallbackRuntimeEventStream {
        async fn open_run(
            &self,
            _run_id: Uuid,
            _policy: RuntimeEventStreamPolicy,
        ) -> anyhow::Result<()> {
            Ok(())
        }

        async fn append(
            &self,
            run_id: Uuid,
            event: RuntimeEventPayload,
        ) -> anyhow::Result<RuntimeEventEnvelope> {
            Ok(RuntimeEventEnvelope::new(run_id, 0, event))
        }

        async fn subscribe(
            &self,
            _run_id: Uuid,
            _from_sequence: Option<i64>,
        ) -> anyhow::Result<RuntimeEventSubscription> {
            let (sender, live_events) = mpsc::unbounded_channel();
            self.live_senders
                .lock()
                .expect("live sender lock poisoned")
                .push(sender);
            Ok(RuntimeEventSubscription {
                replay: self.subscription_replay.clone(),
                live_events,
            })
        }

        async fn replay(
            &self,
            _run_id: Uuid,
            from_sequence: Option<i64>,
            limit: usize,
        ) -> anyhow::Result<Vec<RuntimeEventEnvelope>> {
            let from_sequence = from_sequence.unwrap_or(0);
            Ok(self
                .events
                .iter()
                .filter(|event| event.sequence > from_sequence)
                .take(limit)
                .cloned()
                .collect())
        }

        async fn close_run(
            &self,
            _run_id: Uuid,
            _reason: RuntimeEventCloseReason,
        ) -> anyhow::Result<()> {
            Ok(())
        }

        async fn trim(&self, _run_id: Uuid, _policy: RuntimeEventTrimPolicy) -> anyhow::Result<()> {
            Ok(())
        }
    }

    fn native_run() -> NativeRunResult {
        NativeRunResult {
            id: Uuid::from_u128(0x11111111111111111111111111111111),
            application_id: Uuid::from_u128(0x22222222222222222222222222222222),
            api_key_id: Uuid::from_u128(0x33333333333333333333333333333333),
            publication_version_id: Uuid::from_u128(0x44444444444444444444444444444444),
            status: NativeRunStatus::Running,
            node_input_payload: json!({}),
            metadata: json!({}),
            answer: None,
            required_action: None,
            tool_calls: None,
            usage: None,
            error: None,
            created_at: OffsetDateTime::UNIX_EPOCH,
        }
    }

    async fn seed_flow_run_for_compat_sse_test(state: &ApiState, run: &NativeRunResult) {
        let pool = state.store.pool();
        let user_id: Uuid = sqlx::query_scalar("select id from users where account = 'root'")
            .fetch_one(pool)
            .await
            .unwrap();
        let workspace_id: Uuid = sqlx::query_scalar("select id from workspaces limit 1")
            .fetch_one(pool)
            .await
            .unwrap();
        let flow_id = Uuid::now_v7();
        let flow_draft_id = Uuid::now_v7();
        let compiled_plan_id = Uuid::now_v7();

        sqlx::query(
            r#"
            insert into applications (
                id, workspace_id, application_type, name, description, created_by
            ) values ($1, $2, 'agent_flow', 'compat sse test', '', $3)
            "#,
        )
        .bind(run.application_id)
        .bind(workspace_id)
        .bind(user_id)
        .execute(pool)
        .await
        .unwrap();
        sqlx::query("insert into flows (id, application_id, created_by, updated_by) values ($1, $2, $3, $3)")
            .bind(flow_id)
            .bind(run.application_id)
            .bind(user_id)
            .execute(pool)
            .await
            .unwrap();
        sqlx::query(
            "insert into flow_drafts (id, flow_id, schema_version, document, updated_by) values ($1, $2, '1flowbase.flow/v2', '{}'::jsonb, $3)",
        )
        .bind(flow_draft_id)
        .bind(flow_id)
        .bind(user_id)
        .execute(pool)
        .await
        .unwrap();
        sqlx::query(
            r#"
            insert into flow_compiled_plans (
                id, flow_id, flow_draft_id, schema_version, document_hash,
                document_updated_at, plan, created_by
            ) values (
                $1, $2, $3, '1flowbase.flow/v2', 'compat-sse-test',
                now(), '{}'::jsonb, $4
            )
            "#,
        )
        .bind(compiled_plan_id)
        .bind(flow_id)
        .bind(flow_draft_id)
        .bind(user_id)
        .execute(pool)
        .await
        .unwrap();
        sqlx::query(
            r#"
            insert into flow_runs (
                id, application_id, flow_id, flow_draft_id, compiled_plan_id,
                run_mode, status, input_payload, output_payload, created_by,
                started_at, debug_session_id, flow_schema_version, document_hash, title
            ) values (
                $1, $2, $3, $4, $5,
                'published_api_run', 'waiting_callback', '{}'::jsonb, '{}'::jsonb, $6,
                now(), 'compat-sse-test', '1flowbase.flow/v2', 'compat-sse-test',
                'compat sse test'
            )
            "#,
        )
        .bind(run.id)
        .bind(run.application_id)
        .bind(flow_id)
        .bind(flow_draft_id)
        .bind(compiled_plan_id)
        .bind(user_id)
        .execute(pool)
        .await
        .unwrap();
    }

    async fn append_compat_sse_runtime_event(
        state: &ApiState,
        run_id: Uuid,
        event_type: &str,
        payload: Value,
    ) {
        state
            .store
            .append_runtime_event(&AppendRuntimeEventInput {
                flow_run_id: run_id,
                node_run_id: None,
                span_id: None,
                parent_span_id: None,
                event_type: event_type.to_string(),
                layer: if is_public_terminal_runtime_event(event_type) {
                    domain::RuntimeEventLayer::AgentTransition
                } else {
                    domain::RuntimeEventLayer::RuntimeItem
                },
                source: domain::RuntimeEventSource::Host,
                trust_level: domain::RuntimeTrustLevel::HostFact,
                item_id: None,
                ledger_ref: None,
                payload,
                visibility: domain::RuntimeEventVisibility::Workspace,
                durability: domain::RuntimeEventDurability::Durable,
            })
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn forwarded_answer_delta_advances_matching_durable_cursor() {
        let run = native_run();
        let node_run_id = Uuid::from_u128(0x55555555555555555555555555555555);
        let (base_state, _) = crate::_tests::support::test_api_state_with_database_url().await;
        seed_flow_run_for_compat_sse_test(&base_state, &run).await;
        append_compat_sse_runtime_event(
            &base_state,
            run.id,
            "text_delta",
            json!({
                "type": "text_delta",
                "node_id": "node-answer",
                "text": "prior node answer",
                "presentation": {
                    "kind": "answer",
                    "answer_node_id": "node-answer",
                    "source_node_id": "node-llm",
                    "source_output_key": "text",
                    "segment_index": 0
                }
            }),
        )
        .await;
        let event = RuntimeEventEnvelope::new(
            run.id,
            7,
            debug_stream_events::answer_text_delta(
                "node-answer",
                "prior node answer".to_string(),
                0,
                Some("node-llm"),
                Some(node_run_id),
                Some("text"),
            ),
        );
        let mut durable_sequence = 0;

        advance_durable_cursor_for_forwarded_event(
            &base_state,
            run.id,
            &event,
            &mut durable_sequence,
        )
        .await;

        assert!(
            durable_sequence > 0,
            "forwarded answer delta should mark matching durable record as consumed"
        );
    }

    #[tokio::test]
    async fn anthropic_live_flow_started_is_not_duplicated_by_durable_drain() {
        let mut run = native_run();
        let node_run_id = Uuid::from_u128(0x77777777777777777777777777777777);
        let callback_task_id = Uuid::from_u128(0x99999999999999999999999999999999);
        run.status = NativeRunStatus::Waiting;
        run.tool_calls = Some(json!([
            {
                "id": "toolu_next",
                "name": "lookup_next",
                "arguments": { "query": "next" }
            }
        ]));
        run.required_action = Some(NativeRequiredAction {
            action_type: "submit_tool_outputs".to_string(),
            payload: json!({
                "callback_task_id": callback_task_id,
                "callback_kind": "llm_tool_calls",
                "node_run_id": node_run_id,
                "tool_calls": run.tool_calls.clone().unwrap()
            }),
        });

        let (base_state, _) = crate::_tests::support::test_api_state_with_database_url().await;
        seed_flow_run_for_compat_sse_test(&base_state, &run).await;
        append_compat_sse_runtime_event(
            &base_state,
            run.id,
            "flow_started",
            json!({
                "type": "flow_started",
                "run_id": run.id,
                "status": "running"
            }),
        )
        .await;
        append_compat_sse_runtime_event(
            &base_state,
            run.id,
            "text_delta",
            json!({
                "type": "text_delta",
                "event_type": "text_delta",
                "node_id": "node-answer",
                "text": "prior node answer",
                "presentation": {
                    "kind": "answer",
                    "answer_node_id": "node-answer",
                    "source_node_id": "node-llm",
                    "source_node_run_id": node_run_id,
                    "source_output_key": "text",
                    "segment_index": 0
                }
            }),
        )
        .await;
        append_compat_sse_runtime_event(
            &base_state,
            run.id,
            "waiting_callback",
            json!({
                "type": "waiting_callback",
                "run_id": run.id,
                "status": "waiting_callback",
                "callback_task_id": callback_task_id,
                "callback_kind": "llm_tool_calls",
                "node_run_id": node_run_id,
                "tool_calls": run.tool_calls.clone().unwrap()
            }),
        )
        .await;

        let runtime_event_stream = Arc::new(
            ReplayBeforeFallbackRuntimeEventStream::with_subscription_replay(
                vec![RuntimeEventEnvelope::new(
                    run.id,
                    1,
                    debug_stream_events::flow_started(run.id),
                )],
                Vec::new(),
            ),
        );
        let state = Arc::new(ApiState {
            store: base_state.store.clone(),
            infrastructure: base_state.infrastructure.clone(),
            file_storage_registry: base_state.file_storage_registry.clone(),
            runtime_engine: base_state.runtime_engine.clone(),
            provider_runtime: base_state.provider_runtime.clone(),
            process_started_at: base_state.process_started_at,
            runtime_activity: base_state.runtime_activity.clone(),
            api_runtime_profile: base_state.api_runtime_profile.clone(),
            plugin_runner_system: base_state.plugin_runner_system.clone(),
            official_plugin_source: base_state.official_plugin_source.clone(),
            provider_install_root: base_state.provider_install_root.clone(),
            provider_secret_master_key: base_state.provider_secret_master_key.clone(),
            host_extension_dropin_root: base_state.host_extension_dropin_root.clone(),
            allow_unverified_filesystem_dropins: base_state.allow_unverified_filesystem_dropins,
            allow_uploaded_host_extensions: base_state.allow_uploaded_host_extensions,
            session_store: base_state.session_store.clone(),
            runtime_event_stream,
            api_docs: base_state.api_docs.clone(),
            cookie_name: base_state.cookie_name.clone(),
            session_ttl_days: base_state.session_ttl_days,
            bootstrap_workspace_name: base_state.bootstrap_workspace_name.clone(),
        });
        let (sender, mut receiver) = mpsc::channel(32);
        let mut mapper = AnthropicStreamMapper::new("1flowbase".to_string());

        tokio::time::timeout(
            Duration::from_secs(2),
            send_compatible_runtime_event_stream(
                state,
                run.clone(),
                ANTHROPIC_SSE_PROJECTION,
                Some(0),
                None,
                sender,
                move |run, envelope| mapper.runtime_event_to_sse(run, envelope),
            ),
        )
        .await
        .expect("compatible stream should stop at durable waiting callback");

        let mut events = Vec::new();
        while let Some(event) = receiver.recv().await {
            events.push(event);
        }
        let response = completed_compatible_stream(events);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body = String::from_utf8(body.to_vec()).unwrap();

        assert_eq!(body.matches("event: message_start").count(), 1, "{body}");
        assert!(body.contains("prior node answer"), "{body}");
        assert!(body.contains("lookup_next"), "{body}");
        assert!(body.contains("\"stop_reason\":\"tool_use\""), "{body}");
    }

    #[test]
    fn openai_delta_chunk_maps_reasoning_to_reasoning_content() {
        let chat_completion_id = "chatcmpl-test";
        let payload = openai_delta_chunk_payload(
            &native_run(),
            "deepseek-v4-pro",
            chat_completion_id,
            "reasoning_delta",
            "先分析用户问题".to_string(),
        )
        .expect("reasoning delta should map to an OpenAI-compatible chunk");

        assert_eq!(payload["id"], json!(chat_completion_id));
        assert_eq!(
            payload["choices"][0]["delta"]["reasoning_content"],
            json!("先分析用户问题")
        );
        assert_eq!(payload["choices"][0]["delta"].get("content"), None);
    }

    #[test]
    fn anthropic_delta_payload_ignores_reasoning_delta() {
        let payload = anthropic_delta_payload(0, "reasoning_delta", "先分析用户问题".to_string());

        assert_eq!(payload, None);
    }

    #[tokio::test]
    async fn anthropic_completed_stream_suppresses_thinking_and_streams_visible_text() {
        let mut run = native_run();
        run.status = NativeRunStatus::Succeeded;
        run.answer = Some("<think>先分析</think>\n最终回答".to_string());
        let response = completed_compatible_stream(anthropic_completed_run_to_sse(&run, "claude"));
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body = String::from_utf8(body.to_vec()).unwrap();

        assert!(body.contains("\"type\":\"text_delta\""), "{body}");
        assert!(body.contains("\"text\":\"\\n最终回答\""), "{body}");
        assert!(!body.contains("\"type\":\"thinking\""), "{body}");
        assert!(!body.contains("\"type\":\"thinking_delta\""), "{body}");
        assert!(!body.contains("<think>"), "{body}");
    }

    #[tokio::test]
    async fn anthropic_live_answer_reasoning_delta_is_not_projected() {
        let run = native_run();
        let mut mapper = AnthropicStreamMapper::new("1flowbase".to_string());
        let reasoning_events = mapper.runtime_event_to_sse(
            &run,
            RuntimeEventEnvelope::new(
                run.id,
                1,
                debug_stream_events::answer_reasoning_delta(
                    "node-answer",
                    "private reasoning".to_string(),
                    0,
                    Some("node-llm"),
                    None,
                    Some("text"),
                ),
            ),
        );
        let text_events = mapper.runtime_event_to_sse(
            &run,
            RuntimeEventEnvelope::new(
                run.id,
                2,
                debug_stream_events::answer_text_delta(
                    "node-answer",
                    "visible answer".to_string(),
                    1,
                    Some("node-llm"),
                    None,
                    Some("text"),
                ),
            ),
        );

        let response = completed_compatible_stream([reasoning_events, text_events].concat());
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body = String::from_utf8(body.to_vec()).unwrap();

        assert!(body.contains("\"text\":\"visible answer\""), "{body}");
        assert!(!body.contains("private reasoning"), "{body}");
        assert_eq!(body.matches("event: content_block_start").count(), 1);
    }

    #[test]
    fn anthropic_projects_answer_presentation_delta_not_provider_raw_delta() {
        let run = native_run();
        let mut mapper = AnthropicStreamMapper::new("1flowbase".to_string());
        let provider_events = mapper.runtime_event_to_sse(
            &run,
            RuntimeEventEnvelope::new(
                run.id,
                1,
                debug_stream_events::text_delta(
                    "node-llm",
                    Uuid::from_u128(0x55555555555555555555555555555555),
                    "provider raw".to_string(),
                ),
            ),
        );
        let presentation_events = mapper.runtime_event_to_sse(
            &run,
            RuntimeEventEnvelope::new(
                run.id,
                2,
                debug_stream_events::answer_text_delta(
                    "node-answer",
                    "answer presentation".to_string(),
                    0,
                    Some("node-llm"),
                    None,
                    Some("text"),
                ),
            ),
        );

        assert!(provider_events.is_empty());
        assert_eq!(presentation_events.len(), 2);
    }

    #[tokio::test]
    async fn anthropic_terminal_answer_fallback_emits_text_before_stop() {
        let run = native_run();
        let mut mapper = AnthropicStreamMapper::new("1flowbase".to_string());
        let events = mapper.runtime_event_to_sse(
            &run,
            RuntimeEventEnvelope::new(
                run.id,
                1,
                debug_stream_events::flow_finished(run.id, json!({ "answer": "最终回答" })),
            ),
        );

        let response = completed_compatible_stream(events);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body = String::from_utf8(body.to_vec()).unwrap();

        assert!(body.contains("\"type\":\"text_delta\""), "{body}");
        assert!(body.contains("\"text\":\"最终回答\""), "{body}");
        assert!(body.contains("event: message_stop"), "{body}");
    }

    #[tokio::test]
    async fn anthropic_failed_terminal_with_answer_finishes_without_error_event() {
        let mut run = native_run();
        run.status = NativeRunStatus::Failed;
        run.answer = Some("工具失败后的回答".to_string());
        let mut mapper = AnthropicStreamMapper::new("1flowbase".to_string());
        let events = mapper.runtime_event_to_sse(
            &run,
            RuntimeEventEnvelope::new(
                run.id,
                1,
                debug_stream_events::flow_failed(
                    run.id,
                    json!({ "message": "tool callback failed" }),
                ),
            ),
        );

        let response = completed_compatible_stream(events);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body = String::from_utf8(body.to_vec()).unwrap();

        assert!(body.contains("工具失败后的回答"), "{body}");
        assert!(body.contains("event: message_stop"), "{body}");
        assert!(!body.contains("event: error"), "{body}");
    }

    #[test]
    fn openai_waiting_callback_maps_to_tool_call_chunk() {
        let callback_task_id = Uuid::from_u128(0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa);
        let chat_completion_id = "chatcmpl-tool-test";
        let payload = openai_tool_call_chunk_payload(
            &native_run(),
            "1flowbase",
            chat_completion_id,
            &json!({
                "callback_kind": "llm_tool_calls",
                "callback_task_id": callback_task_id,
                "tool_calls": [
                    {
                        "id": "call_weather",
                        "name": "lookup_weather",
                        "arguments": {"city": "Hangzhou"}
                    }
                ]
            }),
        )
        .expect("LLM callback should map to OpenAI tool call chunk");

        assert_eq!(payload["id"], json!(chat_completion_id));
        assert_eq!(
            payload["choices"][0]["delta"]["tool_calls"][0]["function"]["name"],
            json!("lookup_weather")
        );
        assert_eq!(
            payload["choices"][0]["delta"]["tool_calls"][0]["function"]["arguments"],
            json!("{\"city\":\"Hangzhou\"}")
        );
        let call_id = payload["choices"][0]["delta"]["tool_calls"][0]["id"]
            .as_str()
            .expect("tool call id should be encoded");
        assert!(call_id.contains("call_weather"));
    }

    #[test]
    fn openai_chat_completion_id_changes_for_callback_resume() {
        let run_id = Uuid::from_u128(0x11111111111111111111111111111111);
        let callback_task_id = Uuid::from_u128(0x22222222222222222222222222222222);

        assert_ne!(
            openai_chat_completion_id_from_run_id(run_id),
            openai_chat_completion_id_from_callback_task(run_id, callback_task_id)
        );
    }

    #[test]
    fn openai_responses_waiting_callback_maps_to_function_call_item() {
        let callback_task_id = Uuid::from_u128(0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb);
        let output = openai_response_function_call_output_items(&json!({
            "callback_kind": "llm_tool_calls",
            "callback_task_id": callback_task_id,
            "tool_calls": [
                {
                    "id": "call_inventory",
                    "name": "lookup_inventory",
                    "arguments": {"sku": "sku_123"}
                }
            ]
        }))
        .expect("LLM callback should map to Responses function_call output");

        assert_eq!(output[0]["type"], json!("function_call"));
        assert_eq!(output[0]["name"], json!("lookup_inventory"));
        assert_eq!(output[0]["arguments"], json!("{\"sku\":\"sku_123\"}"));
        assert!(output[0]["call_id"]
            .as_str()
            .expect("call id should be encoded")
            .contains("call_inventory"));
    }

    #[test]
    fn openai_finish_chunk_uses_deepseek_compatible_terminal_shape() {
        let payload =
            openai_finish_chunk_payload(&native_run(), "1flowbase", "chatcmpl-test", "stop");

        assert_eq!(payload["choices"][0]["delta"]["content"], json!(""));
        assert_eq!(payload["choices"][0]["delta"]["role"], Value::Null);
        assert_eq!(payload["choices"][0]["finish_reason"], json!("stop"));
        assert_eq!(payload["usage"]["prompt_tokens"], json!(0));
        assert_eq!(payload["usage"]["completion_tokens"], json!(0));
        assert_eq!(payload["usage"]["total_tokens"], json!(0));
    }

    #[test]
    fn openai_chat_resume_terminal_answer_fallback_emits_content_before_finish() {
        let run = native_run();
        let mut mapper =
            OpenAiChatStreamMapper::new("1flowbase".to_string(), "chatcmpl-test".to_string(), true);
        let events = mapper.runtime_event_to_sse(
            &run,
            RuntimeEventEnvelope::new(
                run.id,
                1,
                debug_stream_events::flow_finished(run.id, json!({ "answer": "最终回答" })),
            ),
        );

        assert_eq!(events.len(), 3);
    }

    #[tokio::test]
    async fn openai_chat_resume_terminal_answer_fallback_projects_thinking_delta() {
        let run = native_run();
        let mut mapper =
            OpenAiChatStreamMapper::new("1flowbase".to_string(), "chatcmpl-test".to_string(), true);

        let events = mapper.runtime_event_to_sse(
            &run,
            RuntimeEventEnvelope::new(
                run.id,
                1,
                debug_stream_events::flow_finished(
                    run.id,
                    json!({ "answer": "<think>先分析</think>\n最终回答" }),
                ),
            ),
        );

        let response = completed_compatible_stream(events);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body = String::from_utf8(body.to_vec()).unwrap();

        assert!(body.contains("\"reasoning_content\":\"先分析\""), "{body}");
        assert!(body.contains("\"content\":\"\\n最终回答\""), "{body}");
        assert!(!body.contains("<think>"), "{body}");
        assert!(body.contains("[DONE]"), "{body}");
    }

    #[tokio::test]
    async fn openai_responses_resume_terminal_answer_fallback_projects_thinking_delta() {
        let run = native_run();
        let mut mapper = OpenAiResponseStreamMapper::new("1flowbase".to_string(), None, true);
        let events = mapper.runtime_event_to_sse(
            &run,
            RuntimeEventEnvelope::new(
                run.id,
                1,
                debug_stream_events::flow_finished(
                    run.id,
                    json!({ "answer": "<think>先分析</think>\n最终回答" }),
                ),
            ),
        );

        let response = completed_compatible_stream(events);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body = String::from_utf8(body.to_vec()).unwrap();

        assert!(
            body.contains("event: response.reasoning_text.delta"),
            "{body}"
        );
        assert!(body.contains("\"delta\":\"先分析\""), "{body}");
        assert!(body.contains("event: response.output_text.delta"), "{body}");
        assert!(body.contains("\"delta\":\"\\n最终回答\""), "{body}");
        assert!(!body.contains("<think>"), "{body}");
        assert!(body.contains("event: response.completed"), "{body}");
    }

    #[tokio::test]
    async fn openai_response_completed_event_includes_usage() {
        let mut run = native_run();
        run.usage = Some(NativeUsage {
            prompt_tokens: Some(11),
            completion_tokens: Some(7),
            total_tokens: Some(18),
            ..Default::default()
        });
        let mut mapper = OpenAiResponseStreamMapper::new("1flowbase".to_string(), None, true);
        let events = mapper.runtime_event_to_sse(
            &run,
            RuntimeEventEnvelope::new(
                run.id,
                1,
                debug_stream_events::flow_finished(run.id, json!({ "answer": "Final answer" })),
            ),
        );

        let response = completed_compatible_stream(events);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body = String::from_utf8(body.to_vec()).unwrap();

        assert!(body.contains("event: response.completed"), "{body}");
        assert!(body.contains("\"usage\""), "{body}");
        assert!(body.contains("\"input_tokens\":11"), "{body}");
        assert!(body.contains("\"output_tokens\":7"), "{body}");
        assert!(body.contains("\"total_tokens\":18"), "{body}");
    }

    #[tokio::test]
    async fn anthropic_completed_stream_includes_usage_for_claude_code_cost_and_context() {
        let mut run = native_run();
        run.status = NativeRunStatus::Succeeded;
        run.answer = Some("Final answer".to_string());
        run.usage = Some(NativeUsage {
            prompt_tokens: Some(11),
            completion_tokens: Some(7),
            total_tokens: Some(18),
            input_cache_hit_tokens: Some(3),
            cache_write_tokens: Some(2),
            ..Default::default()
        });

        let response =
            completed_compatible_stream(anthropic_completed_run_to_sse(&run, "1flowbase"));
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body = String::from_utf8(body.to_vec()).unwrap();

        assert!(body.contains("event: message_start"), "{body}");
        assert!(body.contains("\"input_tokens\":11"), "{body}");
        assert!(body.contains("\"cache_read_input_tokens\":3"), "{body}");
        assert!(body.contains("\"cache_creation_input_tokens\":2"), "{body}");
        assert!(body.contains("event: message_delta"), "{body}");
        assert!(body.contains("\"output_tokens\":7"), "{body}");
    }

    #[tokio::test]
    async fn openai_chat_terminal_answer_fallback_decodes_artifact_preview_answer() {
        let run = native_run();
        let mut mapper =
            OpenAiChatStreamMapper::new("1flowbase".to_string(), "chatcmpl-test".to_string(), true);
        let events = mapper.runtime_event_to_sse(
            &run,
            RuntimeEventEnvelope::new(
                run.id,
                1,
                debug_stream_events::flow_finished(
                    run.id,
                    json!({
                        "answer": {
                            "__runtime_debug_artifact": true,
                            "artifact_ref": "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa",
                            "is_truncated": false,
                            "preview": "\"最终回答\""
                        }
                    }),
                ),
            ),
        );

        let response = completed_compatible_stream(events);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body = String::from_utf8(body.to_vec()).unwrap();

        assert!(body.contains("\"content\":\"最终回答\""), "{body}");
        assert!(body.contains("\"finish_reason\":\"stop\""), "{body}");
        assert!(body.contains("[DONE]"), "{body}");
    }

    #[test]
    fn openai_chat_terminal_answer_fallback_ignores_provider_raw_delta() {
        let run = native_run();
        let mut mapper =
            OpenAiChatStreamMapper::new("1flowbase".to_string(), "chatcmpl-test".to_string(), true);
        let text_events = mapper.runtime_event_to_sse(
            &run,
            RuntimeEventEnvelope::new(
                run.id,
                1,
                debug_stream_events::text_delta("node-llm", run.id, "已流式输出".to_string()),
            ),
        );
        let terminal_events = mapper.runtime_event_to_sse(
            &run,
            RuntimeEventEnvelope::new(
                run.id,
                2,
                debug_stream_events::flow_finished(run.id, json!({ "answer": "最终回答" })),
            ),
        );

        assert!(text_events.is_empty());
        assert_eq!(terminal_events.len(), 3);
    }

    #[test]
    fn compatible_stream_stats_count_answer_content_bytes_once_for_terminal_fallback() {
        let run = native_run();
        let mut stats = CompatibleStreamStats::default();
        let answer_delta = RuntimeEventEnvelope::new(
            run.id,
            1,
            debug_stream_events::answer_text_delta(
                "node-answer",
                "已输出".to_string(),
                0,
                Some("node-llm"),
                None,
                Some("text"),
            ),
        );
        stats.record_sent_runtime_event(&run, &answer_delta, true);

        let terminal_event = RuntimeEventEnvelope::new(
            run.id,
            2,
            debug_stream_events::flow_finished(run.id, json!({ "answer": "最终回答" })),
        );
        stats.record_sent_runtime_event(&run, &terminal_event, true);

        assert!(stats.emitted_content());
        assert_eq!(stats.emitted_content_bytes, "已输出".len());
    }

    #[test]
    fn openai_chat_projects_answer_presentation_delta_not_provider_raw_delta() {
        let run = native_run();
        let mut mapper =
            OpenAiChatStreamMapper::new("1flowbase".to_string(), "chatcmpl-test".to_string(), true);
        let provider_events = mapper.runtime_event_to_sse(
            &run,
            RuntimeEventEnvelope::new(
                run.id,
                1,
                debug_stream_events::text_delta(
                    "node-llm",
                    Uuid::from_u128(0x55555555555555555555555555555555),
                    "provider raw".to_string(),
                ),
            ),
        );
        let presentation_events = mapper.runtime_event_to_sse(
            &run,
            RuntimeEventEnvelope::new(
                run.id,
                2,
                RuntimeEventPayload {
                    event_type: "text_delta".to_string(),
                    source: RuntimeEventSource::Runtime,
                    durability: RuntimeEventDurability::DurableRequired,
                    persist_required: true,
                    trace_visible: false,
                    payload: json!({
                        "type": "text_delta",
                        "node_run_id": Uuid::from_u128(0x66666666666666666666666666666666),
                        "node_id": "node-answer",
                        "text": "answer presentation",
                        "presentation": { "kind": "answer" }
                    }),
                },
            ),
        );

        assert!(provider_events.is_empty());
        assert_eq!(presentation_events.len(), 1);
    }

    #[tokio::test]
    async fn openai_chat_durable_waiting_callback_fallback_drains_text_delta_first() {
        let mut run = native_run();
        let node_run_id = Uuid::from_u128(0x55555555555555555555555555555555);
        let callback_task_id = Uuid::from_u128(0x66666666666666666666666666666666);
        run.status = NativeRunStatus::Waiting;
        run.tool_calls = Some(json!([
            {
                "id": "call_next",
                "name": "lookup_next",
                "arguments": { "query": "next" }
            }
        ]));
        run.required_action = Some(NativeRequiredAction {
            action_type: "submit_tool_outputs".to_string(),
            payload: json!({
                "callback_task_id": callback_task_id,
                "callback_kind": "llm_tool_calls",
                "node_run_id": node_run_id,
                "tool_calls": run.tool_calls.clone().unwrap()
            }),
        });

        let stream_events = vec![
            RuntimeEventEnvelope::new(
                run.id,
                1,
                debug_stream_events::answer_text_delta(
                    "node-answer",
                    "prior node answer".to_string(),
                    0,
                    Some("node-llm"),
                    Some(node_run_id),
                    Some("text"),
                ),
            ),
            RuntimeEventEnvelope::new(
                run.id,
                2,
                RuntimeEventPayload {
                    event_type: "waiting_callback".to_string(),
                    source: RuntimeEventSource::Runtime,
                    durability: RuntimeEventDurability::DurableRequired,
                    persist_required: true,
                    trace_visible: true,
                    payload: json!({
                        "type": "waiting_callback",
                        "run_id": run.id,
                        "status": "waiting_callback",
                        "callback_task_id": callback_task_id,
                        "callback_kind": "llm_tool_calls",
                        "node_run_id": node_run_id,
                        "tool_calls": run.tool_calls.clone().unwrap()
                    }),
                },
            ),
        ];
        let runtime_event_stream =
            Arc::new(ReplayBeforeFallbackRuntimeEventStream::new(stream_events));
        let (base_state, _) = crate::_tests::support::test_api_state_with_database_url().await;
        let state = Arc::new(ApiState {
            store: base_state.store.clone(),
            infrastructure: base_state.infrastructure.clone(),
            file_storage_registry: base_state.file_storage_registry.clone(),
            runtime_engine: base_state.runtime_engine.clone(),
            provider_runtime: base_state.provider_runtime.clone(),
            process_started_at: base_state.process_started_at,
            runtime_activity: base_state.runtime_activity.clone(),
            api_runtime_profile: base_state.api_runtime_profile.clone(),
            plugin_runner_system: base_state.plugin_runner_system.clone(),
            official_plugin_source: base_state.official_plugin_source.clone(),
            provider_install_root: base_state.provider_install_root.clone(),
            provider_secret_master_key: base_state.provider_secret_master_key.clone(),
            host_extension_dropin_root: base_state.host_extension_dropin_root.clone(),
            allow_unverified_filesystem_dropins: base_state.allow_unverified_filesystem_dropins,
            allow_uploaded_host_extensions: base_state.allow_uploaded_host_extensions,
            session_store: base_state.session_store.clone(),
            runtime_event_stream,
            api_docs: base_state.api_docs.clone(),
            cookie_name: base_state.cookie_name.clone(),
            session_ttl_days: base_state.session_ttl_days,
            bootstrap_workspace_name: base_state.bootstrap_workspace_name.clone(),
        });
        let (sender, mut receiver) = mpsc::channel(32);
        let mut mapper =
            OpenAiChatStreamMapper::new("1flowbase".to_string(), "chatcmpl-test".to_string(), true);

        tokio::time::timeout(
            Duration::from_secs(2),
            send_compatible_runtime_event_stream(
                state,
                run.clone(),
                OPENAI_CHAT_SSE_PROJECTION,
                Some(0),
                None,
                sender,
                move |run, envelope| mapper.runtime_event_to_sse(run, envelope),
            ),
        )
        .await
        .expect("compatible stream should stop at replayed waiting callback");

        let mut events = Vec::new();
        while let Some(event) = receiver.recv().await {
            events.push(event);
        }
        let response = completed_compatible_stream(events);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body = String::from_utf8(body.to_vec()).unwrap();

        assert!(body.contains("prior node answer"), "{body}");
        assert!(body.contains("lookup_next"), "{body}");
        assert!(body.contains("\"finish_reason\":\"tool_calls\""), "{body}");
        assert!(body.contains("[DONE]"), "{body}");
    }

    #[tokio::test]
    async fn terminal_answer_recovery_prefers_durable_answer_presentation() {
        let run = native_run();
        let (base_state, _) = crate::_tests::support::test_api_state_with_database_url().await;
        seed_flow_run_for_compat_sse_test(&base_state, &run).await;
        append_compat_sse_runtime_event(
            &base_state,
            run.id,
            "text_delta",
            json!({
                "type": "text_delta",
                "event_type": "text_delta",
                "node_id": "node-answer",
                "text": "durable presentation answer",
                "presentation": {
                    "kind": "answer",
                    "answer_node_id": "node-answer",
                    "source_node_id": "node-llm",
                    "source_output_key": "text",
                    "segment_index": 0
                }
            }),
        )
        .await;
        append_compat_sse_runtime_event(
            &base_state,
            run.id,
            "flow_finished",
            json!({
                "type": "flow_finished",
                "run_id": run.id,
                "status": "succeeded",
                "output": { "answer": "terminal output answer" }
            }),
        )
        .await;

        let deltas =
            recover_terminal_answer_deltas_from_durable_runtime_events(&base_state, &run).await;

        assert_eq!(deltas.len(), 1);
        assert_eq!(deltas[0].kind, TerminalAnswerDeltaKind::Text);
        assert_eq!(deltas[0].text, "durable presentation answer");
    }

    #[tokio::test]
    async fn openai_chat_resume_replay_terminal_drains_durable_text_before_tool_call() {
        let mut run = native_run();
        let node_run_id = Uuid::from_u128(0x77777777777777777777777777777777);
        let previous_callback_task_id = Uuid::from_u128(0x88888888888888888888888888888888);
        let next_callback_task_id = Uuid::from_u128(0x99999999999999999999999999999999);
        run.status = NativeRunStatus::Waiting;
        run.tool_calls = Some(json!([
            {
                "id": "call_next",
                "name": "lookup_next",
                "arguments": { "query": "next" }
            }
        ]));
        run.required_action = Some(NativeRequiredAction {
            action_type: "submit_tool_outputs".to_string(),
            payload: json!({
                "callback_task_id": next_callback_task_id,
                "callback_kind": "llm_tool_calls",
                "node_run_id": node_run_id,
                "tool_calls": run.tool_calls.clone().unwrap()
            }),
        });

        let (base_state, _) = crate::_tests::support::test_api_state_with_database_url().await;
        seed_flow_run_for_compat_sse_test(&base_state, &run).await;
        append_compat_sse_runtime_event(
            &base_state,
            run.id,
            "waiting_callback",
            json!({
                "type": "waiting_callback",
                "run_id": run.id,
                "status": "waiting_callback",
                "callback_task_id": previous_callback_task_id,
                "callback_kind": "llm_tool_calls",
                "node_run_id": node_run_id,
                "tool_calls": [
                    {
                        "id": "call_previous",
                        "name": "lookup_previous",
                        "arguments": { "query": "previous" }
                    }
                ]
            }),
        )
        .await;
        append_compat_sse_runtime_event(
            &base_state,
            run.id,
            "text_delta",
            json!({
                "type": "text_delta",
                "event_type": "text_delta",
                "node_id": "node-answer",
                "text": "prior node answer",
                "presentation": {
                    "kind": "answer",
                    "answer_node_id": "node-answer",
                    "source_node_id": "node-llm",
                    "source_node_run_id": node_run_id,
                    "source_output_key": "text",
                    "segment_index": 0
                },
                "stream_sequence": 2,
                "sequence_start": 2,
                "sequence_end": 2
            }),
        )
        .await;
        append_compat_sse_runtime_event(
            &base_state,
            run.id,
            "waiting_callback",
            json!({
                "type": "waiting_callback",
                "run_id": run.id,
                "status": "waiting_callback",
                "callback_task_id": next_callback_task_id,
                "callback_kind": "llm_tool_calls",
                "node_run_id": node_run_id,
                "tool_calls": run.tool_calls.clone().unwrap()
            }),
        )
        .await;

        let subscription_replay = vec![
            RuntimeEventEnvelope::new(run.id, 1, debug_stream_events::flow_started(run.id)),
            RuntimeEventEnvelope::new(
                run.id,
                2,
                RuntimeEventPayload {
                    event_type: "waiting_callback".to_string(),
                    source: RuntimeEventSource::Runtime,
                    durability: RuntimeEventDurability::DurableRequired,
                    persist_required: true,
                    trace_visible: true,
                    payload: json!({
                        "type": "waiting_callback",
                        "run_id": run.id,
                        "status": "waiting_callback",
                        "callback_task_id": next_callback_task_id,
                        "callback_kind": "llm_tool_calls",
                        "node_run_id": node_run_id,
                        "tool_calls": run.tool_calls.clone().unwrap()
                    }),
                },
            ),
        ];
        let runtime_event_stream = Arc::new(
            ReplayBeforeFallbackRuntimeEventStream::with_subscription_replay(
                subscription_replay,
                Vec::new(),
            ),
        );
        let state = Arc::new(ApiState {
            store: base_state.store.clone(),
            infrastructure: base_state.infrastructure.clone(),
            file_storage_registry: base_state.file_storage_registry.clone(),
            runtime_engine: base_state.runtime_engine.clone(),
            provider_runtime: base_state.provider_runtime.clone(),
            process_started_at: base_state.process_started_at,
            runtime_activity: base_state.runtime_activity.clone(),
            api_runtime_profile: base_state.api_runtime_profile.clone(),
            plugin_runner_system: base_state.plugin_runner_system.clone(),
            official_plugin_source: base_state.official_plugin_source.clone(),
            provider_install_root: base_state.provider_install_root.clone(),
            provider_secret_master_key: base_state.provider_secret_master_key.clone(),
            host_extension_dropin_root: base_state.host_extension_dropin_root.clone(),
            allow_unverified_filesystem_dropins: base_state.allow_unverified_filesystem_dropins,
            allow_uploaded_host_extensions: base_state.allow_uploaded_host_extensions,
            session_store: base_state.session_store.clone(),
            runtime_event_stream,
            api_docs: base_state.api_docs.clone(),
            cookie_name: base_state.cookie_name.clone(),
            session_ttl_days: base_state.session_ttl_days,
            bootstrap_workspace_name: base_state.bootstrap_workspace_name.clone(),
        });
        let (sender, mut receiver) = mpsc::channel(32);
        let mut mapper =
            OpenAiChatStreamMapper::new("1flowbase".to_string(), "chatcmpl-test".to_string(), true);

        tokio::time::timeout(
            Duration::from_secs(2),
            send_compatible_runtime_event_stream(
                state,
                run.clone(),
                OPENAI_CHAT_SSE_PROJECTION,
                Some(0),
                Some(previous_callback_task_id),
                sender,
                move |run, envelope| mapper.runtime_event_to_sse(run, envelope),
            ),
        )
        .await
        .expect("compatible stream should stop at replayed waiting callback");

        let mut events = Vec::new();
        while let Some(event) = receiver.recv().await {
            events.push(event);
        }
        let response = completed_compatible_stream(events);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body = String::from_utf8(body.to_vec()).unwrap();

        let text_index = body.find("prior node answer").unwrap_or_else(|| {
            panic!("resume stream should include prior LLM text before tool call: {body}")
        });
        let tool_index = body
            .find("lookup_next")
            .unwrap_or_else(|| panic!("resume stream should include next tool call: {body}"));
        assert!(
            text_index < tool_index,
            "prior LLM text should be projected before the next tool call: {body}"
        );
        assert!(body.contains("\"finish_reason\":\"tool_calls\""), "{body}");
        assert!(body.contains("[DONE]"), "{body}");
    }

    #[tokio::test]
    async fn openai_chat_live_answer_delta_is_not_duplicated_by_durable_drain() {
        let mut run = native_run();
        let node_run_id = Uuid::from_u128(0x77777777777777777777777777777777);
        let callback_task_id = Uuid::from_u128(0x99999999999999999999999999999999);
        run.status = NativeRunStatus::Waiting;
        run.tool_calls = Some(json!([
            {
                "id": "call_next",
                "name": "lookup_next",
                "arguments": { "query": "next" }
            }
        ]));
        run.required_action = Some(NativeRequiredAction {
            action_type: "submit_tool_outputs".to_string(),
            payload: json!({
                "callback_task_id": callback_task_id,
                "callback_kind": "llm_tool_calls",
                "node_run_id": node_run_id,
                "tool_calls": run.tool_calls.clone().unwrap()
            }),
        });

        let (base_state, _) = crate::_tests::support::test_api_state_with_database_url().await;
        seed_flow_run_for_compat_sse_test(&base_state, &run).await;
        append_compat_sse_runtime_event(
            &base_state,
            run.id,
            "text_delta",
            json!({
                "type": "text_delta",
                "event_type": "text_delta",
                "node_id": "node-answer",
                "text": "prior node answer",
                "presentation": {
                    "kind": "answer",
                    "answer_node_id": "node-answer",
                    "source_node_id": "node-llm",
                    "source_node_run_id": node_run_id,
                    "source_output_key": "text",
                    "segment_index": 0
                }
            }),
        )
        .await;
        append_compat_sse_runtime_event(
            &base_state,
            run.id,
            "waiting_callback",
            json!({
                "type": "waiting_callback",
                "run_id": run.id,
                "status": "waiting_callback",
                "callback_task_id": callback_task_id,
                "callback_kind": "llm_tool_calls",
                "node_run_id": node_run_id,
                "tool_calls": run.tool_calls.clone().unwrap()
            }),
        )
        .await;

        let subscription_replay = vec![
            RuntimeEventEnvelope::new(run.id, 1, debug_stream_events::flow_started(run.id)),
            RuntimeEventEnvelope::new(
                run.id,
                2,
                debug_stream_events::answer_text_delta(
                    "node-answer",
                    "prior node answer".to_string(),
                    0,
                    Some("node-llm"),
                    Some(node_run_id),
                    Some("text"),
                ),
            ),
            RuntimeEventEnvelope::new(
                run.id,
                3,
                RuntimeEventPayload {
                    event_type: "waiting_callback".to_string(),
                    source: RuntimeEventSource::Runtime,
                    durability: RuntimeEventDurability::DurableRequired,
                    persist_required: true,
                    trace_visible: true,
                    payload: json!({
                        "type": "waiting_callback",
                        "run_id": run.id,
                        "status": "waiting_callback",
                        "callback_task_id": callback_task_id,
                        "callback_kind": "llm_tool_calls",
                        "node_run_id": node_run_id,
                        "tool_calls": run.tool_calls.clone().unwrap()
                    }),
                },
            ),
        ];
        let runtime_event_stream = Arc::new(
            ReplayBeforeFallbackRuntimeEventStream::with_subscription_replay(
                subscription_replay,
                Vec::new(),
            ),
        );
        let state = Arc::new(ApiState {
            store: base_state.store.clone(),
            infrastructure: base_state.infrastructure.clone(),
            file_storage_registry: base_state.file_storage_registry.clone(),
            runtime_engine: base_state.runtime_engine.clone(),
            provider_runtime: base_state.provider_runtime.clone(),
            process_started_at: base_state.process_started_at,
            runtime_activity: base_state.runtime_activity.clone(),
            api_runtime_profile: base_state.api_runtime_profile.clone(),
            plugin_runner_system: base_state.plugin_runner_system.clone(),
            official_plugin_source: base_state.official_plugin_source.clone(),
            provider_install_root: base_state.provider_install_root.clone(),
            provider_secret_master_key: base_state.provider_secret_master_key.clone(),
            host_extension_dropin_root: base_state.host_extension_dropin_root.clone(),
            allow_unverified_filesystem_dropins: base_state.allow_unverified_filesystem_dropins,
            allow_uploaded_host_extensions: base_state.allow_uploaded_host_extensions,
            session_store: base_state.session_store.clone(),
            runtime_event_stream,
            api_docs: base_state.api_docs.clone(),
            cookie_name: base_state.cookie_name.clone(),
            session_ttl_days: base_state.session_ttl_days,
            bootstrap_workspace_name: base_state.bootstrap_workspace_name.clone(),
        });
        let (sender, mut receiver) = mpsc::channel(32);
        let mut mapper =
            OpenAiChatStreamMapper::new("1flowbase".to_string(), "chatcmpl-test".to_string(), true);

        tokio::time::timeout(
            Duration::from_secs(2),
            send_compatible_runtime_event_stream(
                state,
                run.clone(),
                OPENAI_CHAT_SSE_PROJECTION,
                Some(0),
                None,
                sender,
                move |run, envelope| mapper.runtime_event_to_sse(run, envelope),
            ),
        )
        .await
        .expect("compatible stream should stop at replayed waiting callback");

        let mut events = Vec::new();
        while let Some(event) = receiver.recv().await {
            events.push(event);
        }
        let response = completed_compatible_stream(events);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body = String::from_utf8(body.to_vec()).unwrap();

        assert_eq!(body.matches("prior node answer").count(), 1, "{body}");
        assert!(body.contains("lookup_next"), "{body}");
        assert!(body.contains("\"finish_reason\":\"tool_calls\""), "{body}");
        assert!(body.contains("[DONE]"), "{body}");
    }

    #[test]
    fn openai_responses_resume_terminal_answer_fallback_emits_output_delta() {
        let run = native_run();
        let mut mapper = OpenAiResponseStreamMapper::new("1flowbase".to_string(), None, true);
        let events = mapper.runtime_event_to_sse(
            &run,
            RuntimeEventEnvelope::new(
                run.id,
                1,
                debug_stream_events::flow_finished(run.id, json!({ "answer": "最终回答" })),
            ),
        );

        assert_eq!(events.len(), 2);
    }

    #[tokio::test]
    async fn openai_chat_failed_terminal_with_answer_finishes_without_error_event() {
        let mut run = native_run();
        run.status = NativeRunStatus::Failed;
        run.answer = Some("工具失败后的回答".to_string());
        let mut mapper =
            OpenAiChatStreamMapper::new("1flowbase".to_string(), "chatcmpl-test".to_string(), true);
        let events = mapper.runtime_event_to_sse(
            &run,
            RuntimeEventEnvelope::new(
                run.id,
                1,
                debug_stream_events::flow_failed(
                    run.id,
                    json!({ "message": "tool callback failed" }),
                ),
            ),
        );

        let response = completed_compatible_stream(events);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body = String::from_utf8(body.to_vec()).unwrap();

        assert!(body.contains("工具失败后的回答"), "{body}");
        assert!(body.contains("\"finish_reason\":\"stop\""), "{body}");
        assert!(!body.contains("\"error\""), "{body}");
        assert!(body.contains("[DONE]"), "{body}");
    }

    #[tokio::test]
    async fn openai_responses_failed_terminal_with_answer_completes_without_failed_event() {
        let mut run = native_run();
        run.status = NativeRunStatus::Failed;
        run.answer = Some("工具失败后的回答".to_string());
        let mut mapper = OpenAiResponseStreamMapper::new("1flowbase".to_string(), None, true);
        let events = mapper.runtime_event_to_sse(
            &run,
            RuntimeEventEnvelope::new(
                run.id,
                1,
                debug_stream_events::flow_failed(
                    run.id,
                    json!({ "message": "tool callback failed" }),
                ),
            ),
        );

        let response = completed_compatible_stream(events);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body = String::from_utf8(body.to_vec()).unwrap();

        assert!(body.contains("event: response.output_text.delta"), "{body}");
        assert!(body.contains("工具失败后的回答"), "{body}");
        assert!(body.contains("event: response.completed"), "{body}");
        assert!(!body.contains("event: response.failed"), "{body}");
    }

    #[test]
    fn anthropic_waiting_callback_maps_to_tool_use_block() {
        let callback_task_id = Uuid::from_u128(0xcccccccccccccccccccccccccccccccc);
        let blocks = anthropic_tool_use_blocks_from_waiting_payload(&json!({
            "callback_kind": "llm_tool_calls",
            "callback_task_id": callback_task_id,
            "tool_calls": [
                {
                    "id": "toolu_weather",
                    "name": "lookup_weather",
                    "arguments": {"city": "Hangzhou"}
                }
            ]
        }))
        .expect("LLM callback should map to Anthropic tool_use blocks");

        assert_eq!(blocks[0]["type"], json!("tool_use"));
        assert_eq!(blocks[0]["name"], json!("lookup_weather"));
        assert_eq!(blocks[0]["input"]["city"], json!("Hangzhou"));
        assert!(blocks[0]["id"]
            .as_str()
            .expect("tool_use id should be encoded")
            .contains("toolu_weather"));
    }

    #[tokio::test]
    async fn anthropic_waiting_callback_streams_tool_input_json_delta() {
        let callback_task_id = Uuid::from_u128(0xcccccccccccccccccccccccccccccccc);
        let mut mapper = AnthropicStreamMapper::new("1flowbase".to_string());
        let events = mapper
            .anthropic_tool_use_events(
                &json!({
                    "callback_kind": "llm_tool_calls",
                    "callback_task_id": callback_task_id,
                    "tool_calls": [
                        {
                            "id": "toolu_bash",
                            "name": "Bash",
                            "arguments": {
                                "command": "pwd && ls -la",
                                "description": "List files"
                            }
                        }
                    ]
                }),
                None,
            )
            .expect("LLM callback should map to Anthropic tool_use stream events");
        let response = completed_compatible_stream(events);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body = String::from_utf8(body.to_vec()).unwrap();

        assert!(body.contains("\"input\":{}"), "{body}");
        assert!(body.contains("\"type\":\"input_json_delta\""), "{body}");
        assert!(
            body.contains("\\\"command\\\":\\\"pwd && ls -la\\\""),
            "{body}"
        );
    }
}
