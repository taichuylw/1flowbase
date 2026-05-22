use control_plane::{
    application_public_api::{
        native::{NativeRunResult, NativeRunStatus},
        run_service::{native_result_from_run_detail, ApplicationPublishedRunControlRepository},
    },
    orchestration_runtime::debug_stream_events,
    ports::{
        RuntimeEventDurability, RuntimeEventEnvelope, RuntimeEventPayload, RuntimeEventSource,
    },
};
use serde_json::{json, Value};
use tracing::warn;

use crate::app_state::ApiState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TerminalAnswerDeltaKind {
    Reasoning,
    Text,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TerminalAnswerDelta {
    pub kind: TerminalAnswerDeltaKind,
    pub text: String,
}

pub(crate) async fn load_latest_native_run_for_terminal_fallback(
    state: &ApiState,
    initial_run: &NativeRunResult,
) -> NativeRunResult {
    match state
        .store
        .get_published_run_detail(initial_run.application_id, initial_run.id)
        .await
    {
        Ok(Some(detail)) => native_result_from_run_detail(&detail, initial_run.metadata.clone()),
        Ok(None) => {
            warn!(
                flow_run_id = %initial_run.id,
                application_id = %initial_run.application_id,
                "compatible/native stream closed without terminal event and no durable run detail was found"
            );
            initial_run.clone()
        }
        Err(error) => {
            warn!(
                flow_run_id = %initial_run.id,
                application_id = %initial_run.application_id,
                error = %error,
                "failed to load durable run detail for stream terminal fallback"
            );
            initial_run.clone()
        }
    }
}

pub(crate) fn terminal_runtime_event_from_native_run(
    run: &NativeRunResult,
) -> Option<RuntimeEventEnvelope> {
    let payload = match run.status {
        NativeRunStatus::Succeeded => {
            debug_stream_events::flow_finished(run.id, terminal_output_payload(run))
        }
        NativeRunStatus::Failed => {
            debug_stream_events::flow_failed(run.id, terminal_error_payload(run))
        }
        NativeRunStatus::Cancelled => debug_stream_events::flow_cancelled(run.id),
        NativeRunStatus::Waiting => waiting_callback_payload(run)?,
        NativeRunStatus::Created | NativeRunStatus::Queued | NativeRunStatus::Running => {
            return None;
        }
    };
    Some(RuntimeEventEnvelope::new(run.id, 0, payload))
}

fn terminal_output_payload(run: &NativeRunResult) -> Value {
    json!({
        "answer": run.answer,
        "tool_calls": run.tool_calls,
        "usage": run.usage,
    })
}

fn terminal_error_payload(run: &NativeRunResult) -> Value {
    run.error
        .as_ref()
        .and_then(|error| serde_json::to_value(error).ok())
        .unwrap_or_else(|| json!({ "message": "published run failed" }))
}

fn waiting_callback_payload(run: &NativeRunResult) -> Option<RuntimeEventPayload> {
    let action = run.required_action.as_ref()?;
    let callback_task_id = action
        .payload
        .get("callback_task_id")
        .cloned()
        .unwrap_or(Value::Null);
    let callback_kind = action
        .payload
        .get("callback_kind")
        .cloned()
        .unwrap_or(Value::Null);
    let tool_calls = run
        .tool_calls
        .clone()
        .or_else(|| action.payload.get("tool_calls").cloned())
        .unwrap_or(Value::Null);

    Some(RuntimeEventPayload {
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
            "callback_kind": callback_kind,
            "node_run_id": action
                .payload
                .get("node_run_id")
                .cloned()
                .unwrap_or(Value::Null),
            "request_payload": action
                .payload
                .get("request_payload")
                .cloned()
                .unwrap_or(Value::Null),
            "tool_calls": tool_calls,
            "required_action": action,
        }),
    })
}

pub(crate) fn terminal_answer_deltas_from_payload(payload: &Value) -> Vec<TerminalAnswerDelta> {
    payload
        .get("output")
        .and_then(|output| output.get("answer"))
        .or_else(|| payload.get("answer"))
        .and_then(Value::as_str)
        .filter(|answer| !answer.is_empty())
        .map(split_terminal_answer_deltas)
        .unwrap_or_default()
}

pub(crate) fn split_terminal_answer_deltas(answer: &str) -> Vec<TerminalAnswerDelta> {
    let mut remaining = answer;
    let mut inside_think = false;
    let mut deltas = Vec::new();

    while !remaining.is_empty() {
        let tag = if inside_think { "</think>" } else { "<think>" };
        let Some(tag_index) = remaining.find(tag) else {
            push_terminal_answer_delta(&mut deltas, inside_think, remaining);
            break;
        };

        push_terminal_answer_delta(&mut deltas, inside_think, &remaining[..tag_index]);
        remaining = &remaining[tag_index + tag.len()..];
        inside_think = !inside_think;
    }

    deltas
}

fn push_terminal_answer_delta(deltas: &mut Vec<TerminalAnswerDelta>, reasoning: bool, text: &str) {
    if text.is_empty() {
        return;
    }
    deltas.push(TerminalAnswerDelta {
        kind: if reasoning {
            TerminalAnswerDeltaKind::Reasoning
        } else {
            TerminalAnswerDeltaKind::Text
        },
        text: text.to_string(),
    });
}

#[cfg(test)]
mod tests {
    use control_plane::application_public_api::native::{
        NativeRequiredAction, NativeRunResult, NativeRunStatus,
    };
    use serde_json::json;
    use time::OffsetDateTime;
    use uuid::Uuid;

    use super::{
        split_terminal_answer_deltas, terminal_runtime_event_from_native_run,
        TerminalAnswerDeltaKind,
    };

    fn native_run(status: NativeRunStatus) -> NativeRunResult {
        NativeRunResult {
            id: Uuid::from_u128(0x11111111111111111111111111111111),
            application_id: Uuid::from_u128(0x22222222222222222222222222222222),
            api_key_id: Uuid::from_u128(0x33333333333333333333333333333333),
            publication_version_id: Uuid::from_u128(0x44444444444444444444444444444444),
            status,
            node_input_payload: json!({}),
            metadata: json!({}),
            answer: Some("done".to_string()),
            required_action: None,
            tool_calls: None,
            usage: None,
            error: None,
            created_at: OffsetDateTime::UNIX_EPOCH,
        }
    }

    #[test]
    fn terminal_fallback_maps_succeeded_native_run_to_flow_finished() {
        let event = terminal_runtime_event_from_native_run(&native_run(NativeRunStatus::Succeeded))
            .expect("succeeded run should synthesize a terminal runtime event");

        assert_eq!(event.event_type, "flow_finished");
        assert_eq!(event.payload["output"]["answer"], json!("done"));
    }

    #[test]
    fn terminal_fallback_ignores_non_terminal_native_run() {
        assert!(
            terminal_runtime_event_from_native_run(&native_run(NativeRunStatus::Running)).is_none()
        );
    }

    #[test]
    fn terminal_fallback_maps_waiting_native_run_to_callback_event() {
        let mut run = native_run(NativeRunStatus::Waiting);
        run.required_action = Some(NativeRequiredAction {
            action_type: "submit_tool_outputs".to_string(),
            payload: json!({
                "callback_task_id": "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa",
                "callback_kind": "llm_tool_calls",
                "node_run_id": "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb",
                "tool_calls": [{"id": "call_read", "name": "Read"}],
            }),
        });
        run.tool_calls = Some(json!([{"id": "call_read", "name": "Read"}]));

        let event = terminal_runtime_event_from_native_run(&run)
            .expect("waiting run should synthesize required-action terminal event");

        assert_eq!(event.event_type, "waiting_callback");
        assert_eq!(event.payload["callback_kind"], json!("llm_tool_calls"));
        assert_eq!(event.payload["tool_calls"][0]["name"], json!("Read"));
    }

    #[test]
    fn split_terminal_answer_deltas_recovers_native_reasoning_and_text() {
        let deltas = split_terminal_answer_deltas("开头<think>先分析</think>\n最终回答");

        assert_eq!(deltas.len(), 3);
        assert_eq!(deltas[0].kind, TerminalAnswerDeltaKind::Text);
        assert_eq!(deltas[0].text, "开头");
        assert_eq!(deltas[1].kind, TerminalAnswerDeltaKind::Reasoning);
        assert_eq!(deltas[1].text, "先分析");
        assert_eq!(deltas[2].kind, TerminalAnswerDeltaKind::Text);
        assert_eq!(deltas[2].text, "\n最终回答");
    }
}
