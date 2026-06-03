use anyhow::{anyhow, Result};
use observability::RuntimeEventBus;
use plugin_framework::provider_contract::ProviderStreamEvent;
use serde_json::{json, Value};
use time::{format_description::well_known::Rfc3339, Duration, OffsetDateTime};
use uuid::Uuid;

use crate::{
    capability_runtime::{host_tool_capability_id, mcp_tool_capability_id},
    ports::{
        AppendCapabilityInvocationInput, AppendContextProjectionInput,
        AppendModelFailoverAttemptLedgerInput, AppendRunEventInput, AppendUsageLedgerInput,
        CreateCallbackTaskInput, CreateCheckpointInput, CreateNodeRunInput,
        LinkUsageLedgerToModelFailoverAttemptInput, OrchestrationRuntimeRepository,
        UpdateFlowRunInput, UpdateNodeRunInput,
    },
    runtime_observability::{
        append_host_event, append_host_span, append_provider_stream_events_raw,
        coalesce_provider_stream_events,
        projection::{estimate_tokens_for_text, model_input_hash},
        AppendHostSpanInput, PROVIDER_DELTA_COALESCE_MAX_BYTES,
    },
    state_transition::{ensure_flow_run_transition, ensure_node_run_transition},
};

mod node_traces;
#[cfg(test)]
mod tests;

use super::{
    debug_stream_events,
    llm_observability_refs::{apply_llm_debug_observability_refs, LlmDebugObservabilityRefs},
    payloads::persisted_node_output_payload,
    runtime_event_persister,
};
use node_traces::persist_flow_debug_node_traces;

pub(super) struct WaitingNodeResumeUpdate {
    pub(super) node_run_id: Uuid,
    pub(super) from_status: domain::NodeRunStatus,
    pub(super) output_payload: Value,
    pub(super) metrics_payload: Value,
    pub(super) debug_payload: Value,
}

pub(super) struct PersistFlowDebugOutcomeInput<'a> {
    pub(super) application_id: Uuid,
    pub(super) flow_run: &'a domain::FlowRunRecord,
    pub(super) compiled_plan: Option<&'a orchestration_runtime::compiled_plan::CompiledPlan>,
    pub(super) outcome: &'a orchestration_runtime::execution_state::FlowDebugExecutionOutcome,
    pub(super) trigger_event_type: &'a str,
    pub(super) trigger_event_payload: Value,
    pub(super) base_started_at: OffsetDateTime,
    pub(super) waiting_node_resume: Option<WaitingNodeResumeUpdate>,
}

pub(super) struct PersistedFlowDebugOutcome {
    pub(super) detail: domain::ApplicationRunDetail,
    pub(super) answer_presentation_events: Vec<crate::ports::RuntimeEventPayload>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CheckpointLocatorPayload {
    node_id: String,
    next_node_index: usize,
    active_node_ids: Vec<String>,
}

impl CheckpointLocatorPayload {
    pub(super) fn from_snapshot(
        node_id: &str,
        snapshot: &orchestration_runtime::execution_state::CheckpointSnapshot,
    ) -> Self {
        Self {
            node_id: node_id.to_string(),
            next_node_index: snapshot.next_node_index,
            active_node_ids: snapshot.active_node_ids.clone(),
        }
    }

    pub(super) fn from_runtime_position(
        node_id: &str,
        next_node_index: usize,
        active_node_ids: Vec<String>,
    ) -> Self {
        Self {
            node_id: node_id.to_string(),
            next_node_index,
            active_node_ids,
        }
    }

    pub(super) fn from_record(checkpoint: &domain::CheckpointRecord) -> Result<Self> {
        let node_id = checkpoint
            .locator_payload
            .get("node_id")
            .and_then(Value::as_str)
            .map(str::to_string)
            .ok_or_else(|| anyhow!("checkpoint is missing node_id"))?;
        let next_node_index = checkpoint
            .locator_payload
            .get("next_node_index")
            .and_then(Value::as_u64)
            .ok_or_else(|| anyhow!("checkpoint is missing next_node_index"))?;
        let next_node_index = usize::try_from(next_node_index)
            .map_err(|_| anyhow!("checkpoint next_node_index is too large"))?;
        let active_node_ids = checkpoint
            .locator_payload
            .get("active_node_ids")
            .and_then(Value::as_array)
            .ok_or_else(|| anyhow!("checkpoint is missing active_node_ids"))?
            .iter()
            .map(|value| {
                value
                    .as_str()
                    .map(str::to_string)
                    .ok_or_else(|| anyhow!("checkpoint active_node_ids must be strings"))
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(Self {
            node_id,
            next_node_index,
            active_node_ids,
        })
    }

    pub(super) fn into_json(self) -> Value {
        json!({
            "node_id": self.node_id,
            "next_node_index": self.next_node_index,
            "active_node_ids": self.active_node_ids,
        })
    }

    pub(super) fn into_checkpoint_snapshot(
        self,
        variable_snapshot: &Value,
    ) -> Result<orchestration_runtime::execution_state::CheckpointSnapshot> {
        Ok(orchestration_runtime::execution_state::CheckpointSnapshot {
            next_node_index: self.next_node_index,
            variable_pool: variable_snapshot
                .as_object()
                .cloned()
                .ok_or_else(|| anyhow!("checkpoint variable_snapshot must be an object"))?,
            active_node_ids: self.active_node_ids,
        })
    }

    pub(super) fn into_node_id(self) -> String {
        self.node_id
    }
}

pub(super) async fn persist_flow_debug_outcome<R>(
    repository: &R,
    input: PersistFlowDebugOutcomeInput<'_>,
) -> Result<PersistedFlowDebugOutcome>
where
    R: OrchestrationRuntimeRepository,
{
    let PersistFlowDebugOutcomeInput {
        application_id,
        flow_run,
        compiled_plan,
        outcome,
        trigger_event_type,
        trigger_event_payload,
        base_started_at,
        waiting_node_resume,
    } = input;
    let flow_span = append_host_span(
        repository,
        AppendHostSpanInput {
            flow_run_id: flow_run.id,
            node_run_id: None,
            parent_span_id: None,
            kind: domain::RuntimeSpanKind::Flow,
            name: "debug flow".to_string(),
            started_at: base_started_at,
            metadata: json!({
                "application_id": application_id,
                "run_mode": flow_run.run_mode.as_str(),
                "trigger_event_type": trigger_event_type,
            }),
        },
    )
    .await?;
    let answer_presentation_events;
    repository
        .append_run_event(&AppendRunEventInput {
            flow_run_id: flow_run.id,
            node_run_id: waiting_node_resume.as_ref().map(|value| value.node_run_id),
            event_type: trigger_event_type.to_string(),
            payload: trigger_event_payload,
        })
        .await?;

    if let Some(waiting_node_resume) = waiting_node_resume {
        ensure_node_run_transition(
            waiting_node_resume.from_status,
            domain::NodeRunStatus::Succeeded,
            "resume_waiting_node",
        )?;
        repository
            .update_node_run(&UpdateNodeRunInput {
                node_run_id: waiting_node_resume.node_run_id,
                status: domain::NodeRunStatus::Succeeded,
                output_payload: waiting_node_resume.output_payload,
                error_payload: None,
                metrics_payload: waiting_node_resume.metrics_payload,
                debug_payload: waiting_node_resume.debug_payload,
                finished_at: Some(OffsetDateTime::now_utc()),
            })
            .await?;
    }

    let waiting_node_run = persist_flow_debug_node_traces(
        repository,
        flow_run.id,
        Some(flow_span.id),
        outcome,
        base_started_at,
    )
    .await?;

    match &outcome.stop_reason {
        orchestration_runtime::execution_state::ExecutionStopReason::WaitingHuman(wait) => {
            let snapshot = outcome
                .checkpoint_snapshot
                .as_ref()
                .ok_or_else(|| anyhow!("waiting_human outcome is missing checkpoint"))?;
            let waiting_node_run = waiting_node_run
                .ok_or_else(|| anyhow!("waiting_human outcome is missing node run"))?;
            let answer_output_payload = materialize_ready_answer_node_run(
                repository,
                flow_run.id,
                compiled_plan,
                outcome,
                base_started_at + Duration::seconds(outcome.node_traces.len() as i64),
            )
            .await?
            .unwrap_or_else(|| json!({}));
            repository
                .create_checkpoint(&CreateCheckpointInput {
                    flow_run_id: flow_run.id,
                    node_run_id: Some(waiting_node_run.id),
                    status: "waiting_human".to_string(),
                    reason: "等待人工输入".to_string(),
                    locator_payload: CheckpointLocatorPayload::from_snapshot(
                        &wait.node_id,
                        snapshot,
                    )
                    .into_json(),
                    variable_snapshot: Value::Object(snapshot.variable_pool.clone()),
                    external_ref_payload: Some(json!({ "prompt": wait.prompt })),
                })
                .await?;
            repository
                .update_flow_run(&UpdateFlowRunInput {
                    flow_run_id: flow_run.id,
                    status: {
                        ensure_flow_run_transition(
                            flow_run.status,
                            domain::FlowRunStatus::WaitingHuman,
                            "persist_flow_waiting_human",
                        )?;
                        domain::FlowRunStatus::WaitingHuman
                    },
                    output_payload: answer_output_payload,
                    error_payload: None,
                    finished_at: None,
                })
                .await?;
            answer_presentation_events = append_ready_answer_presentation_prefix(
                repository,
                flow_run.id,
                compiled_plan,
                outcome,
            )
            .await?;
            runtime_event_persister::persist_runtime_event_payload(
                repository,
                flow_run.id,
                &debug_stream_events::waiting_human(
                    flow_run.id,
                    waiting_node_run.id,
                    &wait.node_id,
                ),
            )
            .await?;
        }
        orchestration_runtime::execution_state::ExecutionStopReason::WaitingCallback(wait) => {
            let snapshot = outcome
                .checkpoint_snapshot
                .as_ref()
                .ok_or_else(|| anyhow!("waiting_callback outcome is missing checkpoint"))?;
            let waiting_node_run = waiting_node_run
                .ok_or_else(|| anyhow!("waiting_callback outcome is missing node run"))?;
            let answer_output_payload = materialize_ready_answer_node_run(
                repository,
                flow_run.id,
                compiled_plan,
                outcome,
                base_started_at + Duration::seconds(outcome.node_traces.len() as i64),
            )
            .await?
            .unwrap_or_else(|| json!({}));
            repository
                .create_checkpoint(&CreateCheckpointInput {
                    flow_run_id: flow_run.id,
                    node_run_id: Some(waiting_node_run.id),
                    status: "waiting_callback".to_string(),
                    reason: "等待 callback 回填".to_string(),
                    locator_payload: CheckpointLocatorPayload::from_snapshot(
                        &wait.node_id,
                        snapshot,
                    )
                    .into_json(),
                    variable_snapshot: Value::Object(snapshot.variable_pool.clone()),
                    external_ref_payload: Some(wait.request_payload.clone()),
                })
                .await?;
            let callback_task = repository
                .create_callback_task(&CreateCallbackTaskInput {
                    flow_run_id: flow_run.id,
                    node_run_id: waiting_node_run.id,
                    callback_kind: wait.callback_kind.clone(),
                    request_payload: wait.request_payload.clone(),
                    external_ref_payload: Some(wait.request_payload.clone()),
                })
                .await?;
            repository
                .update_flow_run(&UpdateFlowRunInput {
                    flow_run_id: flow_run.id,
                    status: {
                        ensure_flow_run_transition(
                            flow_run.status,
                            domain::FlowRunStatus::WaitingCallback,
                            "persist_flow_waiting_callback",
                        )?;
                        domain::FlowRunStatus::WaitingCallback
                    },
                    output_payload: answer_output_payload,
                    error_payload: None,
                    finished_at: None,
                })
                .await?;
            answer_presentation_events = append_ready_answer_presentation_prefix(
                repository,
                flow_run.id,
                compiled_plan,
                outcome,
            )
            .await?;
            runtime_event_persister::persist_runtime_event_payload(
                repository,
                flow_run.id,
                &debug_stream_events::waiting_callback_with_task(
                    flow_run.id,
                    waiting_node_run.id,
                    &wait.node_id,
                    &callback_task,
                ),
            )
            .await?;
        }
        orchestration_runtime::execution_state::ExecutionStopReason::Completed => {
            ensure_flow_run_transition(
                flow_run.status,
                domain::FlowRunStatus::Succeeded,
                "persist_flow_completed",
            )?;
            let output_payload = final_flow_output_payload(outcome);
            answer_presentation_events = append_answer_presentation_suffix(
                repository,
                flow_run.id,
                answer_node_id(outcome),
                &output_payload,
            )
            .await?;
            repository
                .update_flow_run(&UpdateFlowRunInput {
                    flow_run_id: flow_run.id,
                    status: domain::FlowRunStatus::Succeeded,
                    output_payload: output_payload.clone(),
                    error_payload: None,
                    finished_at: Some(OffsetDateTime::now_utc()),
                })
                .await?;
            repository
                .append_run_event(&AppendRunEventInput {
                    flow_run_id: flow_run.id,
                    node_run_id: None,
                    event_type: "flow_run_completed".to_string(),
                    payload: output_payload.clone(),
                })
                .await?;
            runtime_event_persister::persist_runtime_event_payload(
                repository,
                flow_run.id,
                &debug_stream_events::flow_finished(flow_run.id, output_payload),
            )
            .await?;
        }
        orchestration_runtime::execution_state::ExecutionStopReason::Failed(failure) => {
            ensure_flow_run_transition(
                flow_run.status,
                domain::FlowRunStatus::Failed,
                "persist_flow_failed",
            )?;
            let output_payload = final_flow_output_payload(outcome);
            let error_payload = failure.error_payload.clone();
            answer_presentation_events = append_answer_presentation_suffix(
                repository,
                flow_run.id,
                answer_node_id(outcome),
                &output_payload,
            )
            .await?;
            repository
                .update_flow_run(&UpdateFlowRunInput {
                    flow_run_id: flow_run.id,
                    status: domain::FlowRunStatus::Failed,
                    output_payload,
                    error_payload: Some(error_payload.clone()),
                    finished_at: Some(OffsetDateTime::now_utc()),
                })
                .await?;
            repository
                .append_run_event(&AppendRunEventInput {
                    flow_run_id: flow_run.id,
                    node_run_id: None,
                    event_type: "flow_run_failed".to_string(),
                    payload: error_payload.clone(),
                })
                .await?;
            runtime_event_persister::persist_runtime_event_payload(
                repository,
                flow_run.id,
                &debug_stream_events::flow_failed(flow_run.id, error_payload),
            )
            .await?;
        }
    }

    let detail = repository
        .get_application_run_detail(application_id, flow_run.id)
        .await?
        .ok_or_else(|| anyhow!("persisted flow run detail not found"))?;
    Ok(PersistedFlowDebugOutcome {
        detail,
        answer_presentation_events,
    })
}

pub(super) async fn persist_preview_events<R>(
    repository: &R,
    flow_run: &domain::FlowRunRecord,
    node_run: &domain::NodeRunRecord,
    preview: &orchestration_runtime::preview_executor::NodePreviewOutcome,
) -> Result<Vec<domain::RunEventRecord>>
where
    R: OrchestrationRuntimeRepository,
{
    let mut events = Vec::new();
    let started = repository
        .append_run_event(&AppendRunEventInput {
            flow_run_id: flow_run.id,
            node_run_id: Some(node_run.id),
            event_type: "node_preview_started".to_string(),
            payload: json!({
                "target_node_id": preview.target_node_id,
                "input_payload": flow_run.input_payload,
            }),
        })
        .await?;
    events.push(started);
    append_host_event(
        repository,
        flow_run.id,
        Some(node_run.id),
        None,
        "node_preview_started",
        domain::RuntimeEventLayer::Diagnostic,
        json!({
            "target_node_id": preview.target_node_id,
            "input_payload": flow_run.input_payload,
        }),
    )
    .await?;
    events.extend(
        append_provider_stream_events(
            repository,
            flow_run.id,
            Some(node_run.id),
            None,
            &preview.provider_events,
        )
        .await?,
    );
    let completed = repository
        .append_run_event(&AppendRunEventInput {
            flow_run_id: flow_run.id,
            node_run_id: Some(node_run.id),
            event_type: if preview.is_failed() {
                "node_preview_failed".to_string()
            } else {
                "node_preview_completed".to_string()
            },
            payload: preview.as_payload(),
        })
        .await?;
    events.push(completed);
    append_host_event(
        repository,
        flow_run.id,
        Some(node_run.id),
        None,
        if preview.is_failed() {
            "node_preview_failed"
        } else {
            "node_preview_completed"
        },
        domain::RuntimeEventLayer::Diagnostic,
        preview.as_payload(),
    )
    .await?;

    Ok(events)
}

pub(super) fn checkpoint_snapshot_from_record(
    checkpoint: &domain::CheckpointRecord,
) -> Result<orchestration_runtime::execution_state::CheckpointSnapshot> {
    CheckpointLocatorPayload::from_record(checkpoint)?
        .into_checkpoint_snapshot(&checkpoint.variable_snapshot)
}

pub(super) fn checkpoint_node_id(checkpoint: &domain::CheckpointRecord) -> Result<String> {
    Ok(CheckpointLocatorPayload::from_record(checkpoint)?.into_node_id())
}

pub(super) fn next_node_started_at(detail: &domain::ApplicationRunDetail) -> OffsetDateTime {
    detail
        .node_runs
        .iter()
        .map(|record| record.started_at)
        .max()
        .map(|value| value + Duration::seconds(1))
        .unwrap_or_else(OffsetDateTime::now_utc)
}

async fn materialize_ready_answer_node_run<R>(
    repository: &R,
    flow_run_id: Uuid,
    compiled_plan: Option<&orchestration_runtime::compiled_plan::CompiledPlan>,
    outcome: &orchestration_runtime::execution_state::FlowDebugExecutionOutcome,
    started_at: OffsetDateTime,
) -> Result<Option<Value>>
where
    R: OrchestrationRuntimeRepository,
{
    let Some(compiled_plan) = compiled_plan else {
        return Ok(None);
    };
    let variable_pool = outcome
        .checkpoint_snapshot
        .as_ref()
        .map(|snapshot| &snapshot.variable_pool)
        .unwrap_or(&outcome.variable_pool);
    let Some(ready) = super::answer_presentation::ready_answer_output_from_variable_pool(
        compiled_plan,
        variable_pool,
    ) else {
        return Ok(None);
    };
    let Some(answer_node) = compiled_plan.nodes.get(&ready.answer_node_id) else {
        return Ok(None);
    };
    let output_payload =
        super::answer_presentation::ready_answer_output_payload(&ready, variable_pool);
    let node_run = repository
        .create_node_run(&CreateNodeRunInput {
            flow_run_id,
            node_id: answer_node.node_id.clone(),
            node_type: answer_node.node_type.clone(),
            node_alias: answer_node.alias.clone(),
            status: domain::NodeRunStatus::Running,
            input_payload: json!({
                "presentation": {
                    "kind": "answer",
                    "complete": ready.complete,
                    "materialized_from": "waiting_prefix"
                }
            }),
            debug_payload: json!({}),
            started_at,
        })
        .await?;
    ensure_node_run_transition(
        domain::NodeRunStatus::Running,
        domain::NodeRunStatus::Succeeded,
        "materialize_waiting_answer_node",
    )?;
    repository
        .update_node_run(&UpdateNodeRunInput {
            node_run_id: node_run.id,
            status: domain::NodeRunStatus::Succeeded,
            output_payload: output_payload.clone(),
            error_payload: None,
            metrics_payload: json!({
                "preview_mode": true,
                "answer_presentation": {
                    "partial": !ready.complete,
                    "materialized_from": "waiting_prefix"
                }
            }),
            debug_payload: json!({
                "answer_presentation": {
                    "partial": !ready.complete,
                    "materialized_from": "waiting_prefix"
                }
            }),
            finished_at: Some(started_at),
        })
        .await?;

    if ready.text.is_empty() {
        Ok(None)
    } else {
        Ok(Some(output_payload))
    }
}

async fn append_answer_presentation_suffix<R>(
    repository: &R,
    flow_run_id: Uuid,
    answer_node_id: &str,
    output_payload: &Value,
) -> Result<Vec<crate::ports::RuntimeEventPayload>>
where
    R: OrchestrationRuntimeRepository,
{
    let Some(answer) = output_payload.get("answer").and_then(Value::as_str) else {
        return Ok(Vec::new());
    };
    if answer.is_empty() {
        return Ok(Vec::new());
    }

    let existing = existing_answer_presentation_text(repository, flow_run_id, "text_delta").await?;
    let suffix = answer.strip_prefix(&existing).unwrap_or(answer);
    if suffix.is_empty() {
        return Ok(Vec::new());
    }

    let event = debug_stream_events::answer_text_delta(
        answer_node_id,
        suffix.to_string(),
        0,
        None,
        None,
        None,
    );
    runtime_event_persister::persist_runtime_event_payload(repository, flow_run_id, &event).await?;
    Ok(vec![event])
}

async fn append_ready_answer_presentation_prefix<R>(
    repository: &R,
    flow_run_id: Uuid,
    compiled_plan: Option<&orchestration_runtime::compiled_plan::CompiledPlan>,
    outcome: &orchestration_runtime::execution_state::FlowDebugExecutionOutcome,
) -> Result<Vec<crate::ports::RuntimeEventPayload>>
where
    R: OrchestrationRuntimeRepository,
{
    let Some(compiled_plan) = compiled_plan else {
        return Ok(Vec::new());
    };
    let Some(mut cursor) =
        super::answer_presentation::AnswerPresentationCursor::from_plan(compiled_plan)
    else {
        return Ok(Vec::new());
    };
    let variable_pool = outcome
        .checkpoint_snapshot
        .as_ref()
        .map(|snapshot| &snapshot.variable_pool)
        .unwrap_or(&outcome.variable_pool);
    let mut candidate_events = Vec::new();

    for node_id in &compiled_plan.topological_order {
        let Some(output_payload) = variable_pool.get(node_id) else {
            continue;
        };
        candidate_events.extend(cursor.complete_node_with_run_id(node_id, None, output_payload));
    }

    append_missing_answer_presentation_events(repository, flow_run_id, candidate_events).await
}

async fn append_missing_answer_presentation_events<R>(
    repository: &R,
    flow_run_id: Uuid,
    events: Vec<crate::ports::RuntimeEventPayload>,
) -> Result<Vec<crate::ports::RuntimeEventPayload>>
where
    R: OrchestrationRuntimeRepository,
{
    let existing_text =
        existing_answer_presentation_text(repository, flow_run_id, "text_delta").await?;
    let existing_reasoning =
        existing_answer_presentation_text(repository, flow_run_id, "reasoning_delta").await?;
    let candidate_text = answer_presentation_event_text(&events, "text_delta");
    let candidate_reasoning = answer_presentation_event_text(&events, "reasoning_delta");
    let mut skip_text_bytes = if candidate_text.starts_with(&existing_text) {
        existing_text.len()
    } else {
        0
    };
    let mut skip_reasoning_bytes = if candidate_reasoning.starts_with(&existing_reasoning) {
        existing_reasoning.len()
    } else {
        0
    };
    let mut appended = Vec::new();

    for mut event in events {
        let Some(text) = event.payload.get("text").and_then(Value::as_str) else {
            continue;
        };
        let skip_bytes = match event.event_type.as_str() {
            "text_delta" => &mut skip_text_bytes,
            "reasoning_delta" => &mut skip_reasoning_bytes,
            _ => continue,
        };
        let missing = missing_answer_delta_text(skip_bytes, text);
        if missing.is_empty() {
            continue;
        }
        if let Some(payload) = event.payload.as_object_mut() {
            payload.insert("text".to_string(), Value::String(missing));
        }
        runtime_event_persister::persist_runtime_event_payload(repository, flow_run_id, &event)
            .await?;
        appended.push(event);
    }

    Ok(appended)
}

fn answer_presentation_event_text(
    events: &[crate::ports::RuntimeEventPayload],
    event_type: &str,
) -> String {
    events
        .iter()
        .filter(|event| event.event_type == event_type)
        .filter_map(|event| event.payload.get("text").and_then(Value::as_str))
        .collect()
}

async fn existing_answer_presentation_text<R>(
    repository: &R,
    flow_run_id: Uuid,
    event_type: &str,
) -> Result<String>
where
    R: OrchestrationRuntimeRepository,
{
    Ok(repository
        .list_runtime_events(flow_run_id, 0)
        .await?
        .into_iter()
        .filter(|event| event.event_type == event_type)
        .filter(|event| debug_stream_events::is_answer_presentation_delta_payload(&event.payload))
        .filter_map(|event| {
            event
                .payload
                .get("text")
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .collect::<String>())
}

fn missing_answer_delta_text(skip_bytes: &mut usize, next_delta: &str) -> String {
    if *skip_bytes >= next_delta.len() {
        *skip_bytes -= next_delta.len();
        return String::new();
    }
    if *skip_bytes == 0 {
        return next_delta.to_string();
    }
    let missing = next_delta
        .get(*skip_bytes..)
        .unwrap_or(next_delta)
        .to_string();
    *skip_bytes = 0;
    missing
}

fn answer_node_id(
    outcome: &orchestration_runtime::execution_state::FlowDebugExecutionOutcome,
) -> &str {
    outcome
        .node_traces
        .iter()
        .rev()
        .find(|trace| trace.node_type == "answer")
        .map(|trace| trace.node_id.as_str())
        .unwrap_or("assistant")
}

fn final_flow_output_payload(
    outcome: &orchestration_runtime::execution_state::FlowDebugExecutionOutcome,
) -> Value {
    if matches!(
        outcome.stop_reason,
        orchestration_runtime::execution_state::ExecutionStopReason::Failed(_)
    ) {
        if let Some(answer_payload) = outcome
            .node_traces
            .iter()
            .rev()
            .find(|trace| trace.node_type == "answer" && !is_empty_object(&trace.output_payload))
            .map(|trace| trace.output_payload.clone())
        {
            return answer_payload;
        }

        return outcome
            .node_traces
            .iter()
            .rev()
            .find(|trace| trace.error_payload.is_none() && !is_empty_object(&trace.output_payload))
            .map(|trace| trace.output_payload.clone())
            .unwrap_or_else(|| json!({}));
    }

    outcome
        .node_traces
        .last()
        .map(|trace| trace.output_payload.clone())
        .unwrap_or_else(|| json!({}))
}

fn is_empty_object(value: &Value) -> bool {
    value.as_object().is_some_and(|object| object.is_empty())
}

async fn append_provider_stream_events<R>(
    repository: &R,
    flow_run_id: Uuid,
    node_run_id: Option<Uuid>,
    span_id: Option<Uuid>,
    events: &[ProviderStreamEvent],
) -> Result<Vec<domain::RunEventRecord>>
where
    R: OrchestrationRuntimeRepository,
{
    let runtime_bus = RuntimeEventBus::new((events.len() + 4).max(16));
    let events =
        coalesce_provider_stream_events(&runtime_bus, events, PROVIDER_DELTA_COALESCE_MAX_BYTES)?;
    let records =
        append_provider_stream_events_raw(repository, flow_run_id, node_run_id, span_id, &events)
            .await?;
    for event in &events {
        append_provider_capability_intent(repository, flow_run_id, node_run_id, span_id, event)
            .await?;
    }
    Ok(records)
}

async fn append_provider_capability_intent<R>(
    repository: &R,
    flow_run_id: Uuid,
    node_run_id: Option<Uuid>,
    span_id: Option<Uuid>,
    event: &ProviderStreamEvent,
) -> Result<()>
where
    R: OrchestrationRuntimeRepository,
{
    let (capability_id, call) = match event {
        ProviderStreamEvent::ToolCallCommit { call } => (
            host_tool_capability_id(&call.name),
            serde_json::to_value(call)?,
        ),
        ProviderStreamEvent::McpCallCommit { call } => (
            mcp_tool_capability_id(&call.server, &call.method),
            serde_json::to_value(call)?,
        ),
        _ => return Ok(()),
    };

    let event = append_host_event(
        repository,
        flow_run_id,
        node_run_id,
        span_id,
        "capability_call_requested",
        domain::RuntimeEventLayer::Capability,
        json!({
            "provider_only_intent": true,
            "capability_id": capability_id,
            "requested_by": "model",
            "call": call,
        }),
    )
    .await?;
    repository
        .append_capability_invocation(&AppendCapabilityInvocationInput {
            flow_run_id,
            span_id,
            capability_id,
            requested_by_span_id: span_id,
            requester_kind: "model".to_string(),
            arguments_ref: Some(format!("runtime_artifact:inline:{}", event.id)),
            authorization_status: "requested".to_string(),
            authorization_reason: None,
            result_ref: None,
            normalized_result: None,
            started_at: None,
            finished_at: None,
            error_payload: None,
        })
        .await?;

    Ok(())
}

async fn persist_llm_context_observability<R>(
    repository: &R,
    flow_run_id: Uuid,
    node_run_id: Uuid,
    span_id: Uuid,
    trace: &orchestration_runtime::execution_state::NodeExecutionTrace,
) -> Result<LlmDebugObservabilityRefs>
where
    R: OrchestrationRuntimeRepository,
{
    let model_input = json!({
        "node_input": trace.input_payload,
        "provider": trace.metrics_payload.get("provider_code").cloned().unwrap_or(Value::Null),
        "model": trace.metrics_payload.get("model").cloned().unwrap_or(Value::Null),
    });
    let model_input_hash = model_input_hash(&model_input);
    let projection = repository
        .append_context_projection(&AppendContextProjectionInput {
            flow_run_id,
            node_run_id: Some(node_run_id),
            llm_turn_span_id: Some(span_id),
            projection_kind: "managed_full".to_string(),
            merge_stage_ref: None,
            source_transcript_ref: None,
            source_item_refs: json!([]),
            compaction_event_id: None,
            summary_version: None,
            model_input_ref: format!("runtime_artifact:inline:{model_input_hash}"),
            model_input_hash,
            compacted_summary_ref: None,
            previous_projection_id: None,
            token_estimate: Some(estimate_tokens_for_text(&model_input.to_string())),
            provider_continuation_metadata: json!({}),
        })
        .await?;

    let usage = trace.metrics_payload.get("usage").cloned();
    let raw_usage = usage.clone().unwrap_or_else(|| json!({}));
    let usage_status = if usage.is_some() && trace.error_payload.is_none() {
        domain::UsageLedgerStatus::Recorded
    } else {
        domain::UsageLedgerStatus::UnavailableError
    };

    let attempts = append_model_attempts_from_metrics(
        repository,
        flow_run_id,
        node_run_id,
        span_id,
        &projection,
        &trace.metrics_payload,
        trace.error_payload.as_ref(),
    )
    .await?;
    let usage_attempt_id = winner_attempt_id(&attempts);

    let usage_ledger = repository
        .append_usage_ledger(&AppendUsageLedgerInput {
            flow_run_id,
            node_run_id: Some(node_run_id),
            span_id: Some(span_id),
            failover_attempt_id: usage_attempt_id,
            provider_instance_id: trace
                .metrics_payload
                .get("provider_instance_id")
                .and_then(Value::as_str)
                .and_then(|value| Uuid::parse_str(value).ok()),
            gateway_route_id: None,
            model_id: trace
                .metrics_payload
                .get("model")
                .and_then(Value::as_str)
                .map(str::to_string),
            upstream_model_id: trace
                .metrics_payload
                .get("model")
                .and_then(Value::as_str)
                .map(str::to_string),
            upstream_request_id: None,
            input_tokens: usage_i64(&raw_usage, "input_tokens"),
            cached_input_tokens: usage_i64(&raw_usage, "cached_input_tokens"),
            output_tokens: usage_i64(&raw_usage, "output_tokens"),
            reasoning_output_tokens: usage_i64(&raw_usage, "reasoning_tokens"),
            total_tokens: usage_i64(&raw_usage, "total_tokens"),
            input_cache_hit_tokens: usage_i64(&raw_usage, "input_cache_hit_tokens"),
            input_cache_miss_tokens: usage_i64(&raw_usage, "input_cache_miss_tokens"),
            cache_read_tokens: usage_i64(&raw_usage, "cache_read_tokens"),
            cache_write_tokens: usage_i64(&raw_usage, "cache_write_tokens"),
            price_snapshot: None,
            cost_snapshot: None,
            usage_status,
            raw_usage: raw_usage.clone(),
            normalized_usage: raw_usage,
        })
        .await?;
    if let Some(failover_attempt_id) = usage_attempt_id {
        repository
            .link_usage_ledger_to_model_failover_attempt(
                &LinkUsageLedgerToModelFailoverAttemptInput {
                    failover_attempt_id,
                    usage_ledger_id: usage_ledger.id,
                },
            )
            .await?;
    }

    Ok(LlmDebugObservabilityRefs::from_records(
        &projection,
        &attempts,
    ))
}

async fn append_model_attempts_from_metrics<R>(
    repository: &R,
    flow_run_id: Uuid,
    node_run_id: Uuid,
    span_id: Uuid,
    projection: &domain::ContextProjectionRecord,
    metrics_payload: &Value,
    error_payload: Option<&Value>,
) -> Result<Vec<domain::ModelFailoverAttemptLedgerRecord>>
where
    R: OrchestrationRuntimeRepository,
{
    let mut attempt_payloads = metrics_payload
        .get("attempts")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    if attempt_payloads.is_empty() {
        attempt_payloads.push(json!({
            "attempt_index": 0,
            "provider_instance_id": metrics_payload.get("provider_instance_id").cloned().unwrap_or(Value::Null),
            "provider_code": metrics_payload.get("provider_code").cloned().unwrap_or(Value::Null),
            "protocol": metrics_payload.get("protocol").cloned().unwrap_or(Value::Null),
            "upstream_model_id": metrics_payload.get("model").cloned().unwrap_or(Value::Null),
            "status": if error_payload.is_some() { "failed" } else { "succeeded" },
            "failed_after_first_token": false,
        }));
    }

    let mut records = Vec::with_capacity(attempt_payloads.len());
    for selected_attempt in attempt_payloads {
        let status = selected_attempt
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or(if error_payload.is_some() {
                "failed"
            } else {
                "succeeded"
            });

        let record = repository
            .append_model_failover_attempt_ledger(&AppendModelFailoverAttemptLedgerInput {
                flow_run_id,
                node_run_id: Some(node_run_id),
                llm_turn_span_id: Some(span_id),
                queue_snapshot_id: metrics_payload
                    .get("queue_snapshot_id")
                    .and_then(Value::as_str)
                    .and_then(|value| Uuid::parse_str(value).ok()),
                attempt_index: selected_attempt
                    .get("attempt_index")
                    .and_then(Value::as_i64)
                    .unwrap_or(records.len() as i64) as i32,
                provider_instance_id: selected_attempt
                    .get("provider_instance_id")
                    .and_then(Value::as_str)
                    .and_then(|value| Uuid::parse_str(value).ok()),
                provider_code: selected_attempt
                    .get("provider_code")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
                    .to_string(),
                upstream_model_id: selected_attempt
                    .get("upstream_model_id")
                    .or_else(|| selected_attempt.get("model"))
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
                    .to_string(),
                protocol: selected_attempt
                    .get("protocol")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
                    .to_string(),
                request_ref: Some(projection.model_input_ref.clone()),
                request_hash: Some(projection.model_input_hash.clone()),
                started_at: OffsetDateTime::now_utc(),
                first_token_at: parse_attempt_first_token_at(&selected_attempt),
                finished_at: Some(OffsetDateTime::now_utc()),
                status: status.to_string(),
                failed_after_first_token: selected_attempt
                    .get("failed_after_first_token")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                upstream_request_id: selected_attempt
                    .get("upstream_request_id")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                error_code: selected_attempt
                    .get("error_code")
                    .and_then(Value::as_str)
                    .map(str::to_string)
                    .or_else(|| {
                        (status != "succeeded").then(|| {
                            error_payload
                                .and_then(|payload| payload.get("error_kind"))
                                .and_then(Value::as_str)
                                .unwrap_or("provider_error")
                                .to_string()
                        })
                    }),
                error_message_ref: selected_attempt
                    .get("error_message_ref")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                usage_ledger_id: None,
                cost_ledger_id: None,
                response_ref: selected_attempt
                    .get("response_ref")
                    .and_then(Value::as_str)
                    .map(str::to_string),
            })
            .await?;
        records.push(record);
    }

    Ok(records)
}

fn winner_attempt_id(attempts: &[domain::ModelFailoverAttemptLedgerRecord]) -> Option<Uuid> {
    attempts
        .iter()
        .find(|attempt| attempt.status == "succeeded")
        .map(|attempt| attempt.id)
}

fn parse_attempt_first_token_at(attempt: &Value) -> Option<OffsetDateTime> {
    attempt
        .get("first_token_at")
        .and_then(Value::as_str)
        .and_then(|value| OffsetDateTime::parse(value, &Rfc3339).ok())
}

fn usage_i64(usage: &Value, field: &str) -> Option<i64> {
    usage.get(field).and_then(Value::as_i64)
}
