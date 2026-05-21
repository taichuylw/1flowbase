use std::{convert::Infallible, sync::Arc, time::Duration};

use axum::response::{
    sse::{Event, KeepAlive, Sse},
    IntoResponse, Response,
};
use control_plane::{
    application_public_api::{compat::openai::response_id_from_run_id, native::NativeRunResult},
    orchestration_runtime::{
        debug_stream_events, OrchestrationRuntimeService, StartPublishedFlowRunCommand,
    },
    ports::RuntimeEventEnvelope,
};
use serde_json::{json, Value};
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

use crate::{
    app_state::ApiState,
    provider_runtime::ApiProviderRuntime,
    routes::application_public_api::{
        native::{service_error, NativeApiError},
        stream_terminal_fallback::{
            load_latest_native_run_for_terminal_fallback, terminal_runtime_event_from_native_run,
        },
        tool_callback_ids::{
            encode_anthropic_callback_tool_use_id, encode_openai_callback_tool_call_id,
        },
    },
};

type CompatRunSseStream = tokio_stream::wrappers::ReceiverStream<Result<Event, Infallible>>;

pub(crate) async fn start_openai_run_stream(
    state: Arc<ApiState>,
    run: NativeRunResult,
    model: String,
) -> Result<Response, NativeApiError> {
    let chat_completion_id = openai_chat_completion_id_from_run_id(run.id);
    start_compatible_run_stream(state, run, move |run, envelope| {
        openai_runtime_event_to_sse(run, &model, &chat_completion_id, envelope)
    })
    .await
}

pub(crate) async fn start_openai_response_stream(
    state: Arc<ApiState>,
    run: NativeRunResult,
    model: String,
    previous_response_id: Option<String>,
) -> Result<Response, NativeApiError> {
    start_compatible_run_stream(state, run, move |run, envelope| {
        openai_response_runtime_event_to_sse(run, &model, previous_response_id.as_deref(), envelope)
    })
    .await
}

pub(crate) async fn start_anthropic_run_stream(
    state: Arc<ApiState>,
    run: NativeRunResult,
    model: String,
) -> Result<Response, NativeApiError> {
    let mut stateful_mapper = AnthropicStreamMapper::new(model);
    start_compatible_run_stream(state, run, move |run, envelope| {
        stateful_mapper.runtime_event_to_sse(run, envelope)
    })
    .await
}

pub(crate) fn completed_openai_chat_stream(
    run: NativeRunResult,
    model: String,
    chat_completion_id: String,
) -> Response {
    completed_compatible_stream(openai_completed_run_to_sse(
        &run,
        &model,
        &chat_completion_id,
    ))
}

pub(crate) fn completed_openai_response_stream(
    run: NativeRunResult,
    model: String,
    previous_response_id: Option<String>,
) -> Response {
    completed_compatible_stream(openai_response_completed_run_to_sse(
        &run,
        &model,
        previous_response_id.as_deref(),
    ))
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

async fn send_compatible_runtime_event_stream<F>(
    state: Arc<ApiState>,
    initial_run: NativeRunResult,
    sender: mpsc::Sender<Result<Event, Infallible>>,
    mut mapper: F,
) where
    F: FnMut(&NativeRunResult, RuntimeEventEnvelope) -> Vec<Result<Event, Infallible>>,
{
    let stream = state.runtime_event_stream.clone();
    let Ok(mut subscription) = stream.subscribe(initial_run.id, None).await else {
        warn!(
            flow_run_id = %initial_run.id,
            application_id = %initial_run.application_id,
            "failed to subscribe compatible public API runtime event stream"
        );
        return;
    };

    let mut emitted_public_event = false;
    for event in subscription.replay {
        let is_terminal = is_public_terminal_runtime_event(&event.event_type);
        let events = mapper(&initial_run, event);
        emitted_public_event |= !events.is_empty();
        if !send_compatible_sse_events(&sender, events).await {
            debug!(
                flow_run_id = %initial_run.id,
                application_id = %initial_run.application_id,
                "compatible public API stream client disconnected during replay"
            );
            return;
        }
        if is_terminal {
            debug!(
                flow_run_id = %initial_run.id,
                application_id = %initial_run.application_id,
                "compatible public API stream replay reached terminal event"
            );
            return;
        }
    }

    while let Some(event) = subscription.live_events.recv().await {
        let event_type = event.event_type.clone();
        let is_terminal = is_public_terminal_runtime_event(&event_type);
        let events = mapper(&initial_run, event);
        emitted_public_event |= !events.is_empty();
        if !send_compatible_sse_events(&sender, events).await {
            debug!(
                flow_run_id = %initial_run.id,
                application_id = %initial_run.application_id,
                "compatible public API stream client disconnected"
            );
            return;
        }
        if is_terminal {
            debug!(
                flow_run_id = %initial_run.id,
                application_id = %initial_run.application_id,
                event_type = %event_type,
                "compatible public API stream reached terminal event"
            );
            return;
        }
    }

    emit_compatible_terminal_fallback(
        &state,
        &initial_run,
        &sender,
        &mut mapper,
        emitted_public_event,
    )
    .await;
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

async fn emit_compatible_terminal_fallback<F>(
    state: &ApiState,
    initial_run: &NativeRunResult,
    sender: &mpsc::Sender<Result<Event, Infallible>>,
    mapper: &mut F,
    emitted_public_event: bool,
) where
    F: FnMut(&NativeRunResult, RuntimeEventEnvelope) -> Vec<Result<Event, Infallible>>,
{
    let latest_run = load_latest_native_run_for_terminal_fallback(state, initial_run).await;
    let Some(terminal_event) = terminal_runtime_event_from_native_run(&latest_run) else {
        warn!(
            flow_run_id = %initial_run.id,
            application_id = %initial_run.application_id,
            status = ?latest_run.status,
            "compatible public API stream closed without terminal event before durable run reached a terminal state"
        );
        return;
    };

    warn!(
        flow_run_id = %initial_run.id,
        application_id = %initial_run.application_id,
        status = ?latest_run.status,
        "compatible public API stream closed without terminal event; emitting durable terminal fallback"
    );

    if !emitted_public_event {
        let started_event = RuntimeEventEnvelope::new(
            latest_run.id,
            0,
            debug_stream_events::flow_started(latest_run.id),
        );
        if !send_compatible_sse_events(sender, mapper(&latest_run, started_event)).await {
            return;
        }
    }
    let _ = send_compatible_sse_events(sender, mapper(&latest_run, terminal_event)).await;
}

fn is_public_terminal_runtime_event(event_type: &str) -> bool {
    matches!(
        event_type,
        "flow_finished" | "flow_failed" | "flow_cancelled" | "waiting_human" | "waiting_callback"
    )
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
        "text_delta" | "reasoning_delta" => openai_delta_chunk_payload(
            initial_run,
            model,
            chat_completion_id,
            envelope.event_type.as_str(),
            envelope.text.unwrap_or_default(),
        )
        .map(json_sse)
        .into_iter()
        .collect(),
        "flow_finished" => vec![
            json_sse(json!({
                "id": chat_completion_id,
                "object": "chat.completion.chunk",
                "created": initial_run.created_at.unix_timestamp(),
                "model": model,
                "choices": [{
                    "index": 0,
                    "delta": {},
                    "finish_reason": "stop"
                }]
            })),
            done_sse(),
        ],
        "flow_failed" => vec![
            json_sse(json!({
                "error": {
                    "message": runtime_error_message(&envelope.payload),
                    "type": "server_error",
                    "param": null,
                    "code": "runtime_error"
                }
            })),
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
        "text_delta" => vec![event_json_sse(
            "response.output_text.delta",
            json!({
                "type": "response.output_text.delta",
                "response_id": response_id_from_run_id(initial_run.id),
                "item_id": format!("msg_{}", initial_run.id),
                "output_index": 0,
                "content_index": 0,
                "delta": envelope.text.unwrap_or_default()
            }),
        )],
        "reasoning_delta" => vec![event_json_sse(
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
        "flow_finished" => vec![event_json_sse(
            "response.completed",
            json!({
                "type": "response.completed",
                "response": openai_response_stream_snapshot(
                    initial_run,
                    model,
                    previous_response_id,
                    "completed"
                )
            }),
        )],
        "flow_failed" => vec![event_json_sse(
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
        )],
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
            "delta": {},
            "finish_reason": finish_reason
        }]
    })
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
    json!({
        "id": response_id_from_run_id(initial_run.id),
        "object": "response",
        "created_at": initial_run.created_at.unix_timestamp(),
        "status": status,
        "model": model,
        "output": output,
        "output_text": "",
        "previous_response_id": previous_response_id
    })
}

fn openai_completed_run_to_sse(
    run: &NativeRunResult,
    model: &str,
    chat_completion_id: &str,
) -> Vec<Result<Event, Infallible>> {
    let mut events = vec![json_sse(json!({
        "id": chat_completion_id,
        "object": "chat.completion.chunk",
        "created": run.created_at.unix_timestamp(),
        "model": model,
        "choices": [{
            "index": 0,
            "delta": { "role": "assistant" },
            "finish_reason": null
        }]
    }))];
    if let Some(payload) = waiting_payload_from_run(run) {
        if let Some(tool_call_payload) =
            openai_tool_call_chunk_payload(run, model, chat_completion_id, &payload)
        {
            events.push(json_sse(tool_call_payload));
            events.push(json_sse(openai_finish_chunk_payload(
                run,
                model,
                chat_completion_id,
                "tool_calls",
            )));
            events.push(done_sse());
            return events;
        }
    }
    if let Some(answer) = run.answer.as_ref().filter(|answer| !answer.is_empty()) {
        if let Some(payload) =
            openai_delta_chunk_payload(run, model, chat_completion_id, "text_delta", answer.clone())
        {
            events.push(json_sse(payload));
        }
    }
    events.push(json_sse(openai_finish_chunk_payload(
        run,
        model,
        chat_completion_id,
        "stop",
    )));
    events.push(done_sse());
    events
}

fn openai_response_completed_run_to_sse(
    run: &NativeRunResult,
    model: &str,
    previous_response_id: Option<&str>,
) -> Vec<Result<Event, Infallible>> {
    let mut events = vec![event_json_sse(
        "response.created",
        json!({
            "type": "response.created",
            "response": openai_response_stream_snapshot(
                run,
                model,
                previous_response_id,
                "in_progress"
            )
        }),
    )];
    if let Some(payload) = waiting_payload_from_run(run) {
        if let Some(output) = openai_response_function_call_output_items(&payload) {
            events.extend(openai_response_function_call_sse(
                run,
                model,
                previous_response_id,
                output,
            ));
            return events;
        }
    }
    if let Some(answer) = run.answer.as_ref().filter(|answer| !answer.is_empty()) {
        events.push(event_json_sse(
            "response.output_text.delta",
            json!({
                "type": "response.output_text.delta",
                "response_id": response_id_from_run_id(run.id),
                "item_id": format!("msg_{}", run.id),
                "output_index": 0,
                "content_index": 0,
                "delta": answer
            }),
        ));
    }
    events.push(event_json_sse(
        "response.completed",
        json!({
            "type": "response.completed",
            "response": openai_response_stream_snapshot(
                run,
                model,
                previous_response_id,
                "completed"
            )
        }),
    ));
    events
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
        if let Some(tool_events) = mapper.anthropic_tool_use_events(&payload) {
            events.extend(tool_events);
            return events;
        }
    }
    if let Some(answer) = run.answer.as_ref().filter(|answer| !answer.is_empty()) {
        events.extend(mapper.runtime_event_to_sse(
            run,
            RuntimeEventEnvelope::new(
                run.id,
                1,
                debug_stream_events::text_delta("assistant", run.id, answer.clone()),
            ),
        ));
    }
    events.extend(mapper.anthropic_stop_events());
    events
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
    active_content: Option<AnthropicContentBlockKind>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AnthropicContentBlockKind {
    Text,
    Thinking,
}

impl AnthropicStreamMapper {
    fn new(model: String) -> Self {
        Self {
            model,
            next_content_index: 0,
            active_content: None,
        }
    }

    fn runtime_event_to_sse(
        &mut self,
        initial_run: &NativeRunResult,
        envelope: RuntimeEventEnvelope,
    ) -> Vec<Result<Event, Infallible>> {
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
                        "usage": {"input_tokens": 0, "output_tokens": 0}
                    }
                }),
            )],
            "reasoning_delta" => {
                let mut events =
                    self.ensure_anthropic_content_block(AnthropicContentBlockKind::Thinking);
                let (event_name, payload) = anthropic_delta_payload(
                    self.active_content_index(),
                    "reasoning_delta",
                    envelope.text.unwrap_or_default(),
                )
                .expect("reasoning_delta should map to Anthropic thinking_delta");
                events.push(event_json_sse(event_name, payload));
                events
            }
            "text_delta" => {
                let mut events =
                    self.ensure_anthropic_content_block(AnthropicContentBlockKind::Text);
                let (event_name, payload) = anthropic_delta_payload(
                    self.active_content_index(),
                    "text_delta",
                    envelope.text.unwrap_or_default(),
                )
                .expect("text_delta should map to Anthropic text_delta");
                events.push(event_json_sse(event_name, payload));
                events
            }
            "flow_finished" => self.anthropic_stop_events(),
            "waiting_callback" => self
                .anthropic_tool_use_events(&envelope.payload)
                .unwrap_or_else(required_action_not_supported_anthropic_sse),
            "waiting_human" => required_action_not_supported_anthropic_sse(),
            "flow_failed" => vec![event_json_sse(
                "error",
                json!({
                    "type": "error",
                    "error": {
                        "type": "api_error",
                        "message": runtime_error_message(&envelope.payload)
                    }
                }),
            )],
            "flow_cancelled" => self.anthropic_stop_events(),
            _ => Vec::new(),
        }
    }

    fn anthropic_stop_events(&mut self) -> Vec<Result<Event, Infallible>> {
        let mut events = Vec::new();
        if self.active_content.is_none() && self.next_content_index == 0 {
            events.extend(self.ensure_anthropic_content_block(AnthropicContentBlockKind::Text));
        }
        events.extend(self.close_active_anthropic_content_block());
        events.push(event_json_sse(
            "message_delta",
            json!({
                "type": "message_delta",
                "delta": {"stop_reason": "end_turn"},
                "usage": {"output_tokens": 0}
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
    ) -> Option<Vec<Result<Event, Infallible>>> {
        let blocks = anthropic_tool_use_blocks_from_waiting_payload(payload)?;
        let mut events = self.close_active_anthropic_content_block();
        for block in blocks {
            let index = self.next_content_index;
            self.next_content_index += 1;
            events.push(event_json_sse(
                "content_block_start",
                json!({
                    "type": "content_block_start",
                    "index": index,
                    "content_block": block
                }),
            ));
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
                "usage": {"output_tokens": 0}
            }),
        ));
        events.push(event_json_sse(
            "message_stop",
            json!({"type": "message_stop"}),
        ));
        Some(events)
    }

    fn ensure_anthropic_content_block(
        &mut self,
        kind: AnthropicContentBlockKind,
    ) -> Vec<Result<Event, Infallible>> {
        if self.active_content == Some(kind) {
            return Vec::new();
        }

        let mut events = self.close_active_anthropic_content_block();
        let index = self.next_content_index;
        self.next_content_index += 1;
        self.active_content = Some(kind);
        let content_block = match kind {
            AnthropicContentBlockKind::Text => json!({"type": "text", "text": ""}),
            AnthropicContentBlockKind::Thinking => {
                json!({"type": "thinking", "thinking": "", "signature": ""})
            }
        };
        events.push(event_json_sse(
            "content_block_start",
            json!({
                "type": "content_block_start",
                "index": index,
                "content_block": content_block
            }),
        ));
        events
    }

    fn close_active_anthropic_content_block(&mut self) -> Vec<Result<Event, Infallible>> {
        if self.active_content.is_none() {
            return Vec::new();
        }
        let index = self.active_content_index();
        self.active_content = None;
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
        "reasoning_delta" => json!({
            "type": "thinking_delta",
            "thinking": text
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
    use control_plane::application_public_api::native::NativeRunStatus;
    use serde_json::json;
    use time::OffsetDateTime;
    use uuid::Uuid;

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
    fn anthropic_delta_payload_maps_reasoning_to_thinking_delta() {
        let (event_name, payload) =
            anthropic_delta_payload(0, "reasoning_delta", "先分析用户问题".to_string())
                .expect("reasoning delta should map to an Anthropic thinking delta");

        assert_eq!(event_name, "content_block_delta");
        assert_eq!(payload["delta"]["type"], json!("thinking_delta"));
        assert_eq!(payload["delta"]["thinking"], json!("先分析用户问题"));
        assert_eq!(payload["delta"].get("text"), None);
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
}
