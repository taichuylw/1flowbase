use orchestration_runtime::execution_state::{
    ExecutionStopReason, FlowDebugExecutionOutcome, NodeExecutionFailure, NodeExecutionTrace,
};
use serde_json::{json, Map, Value};

use super::final_flow_output_payload;

fn typed_trace(
    node_id: &str,
    node_type: &str,
    output_payload: Value,
    error_payload: Option<Value>,
) -> NodeExecutionTrace {
    NodeExecutionTrace {
        node_id: node_id.to_string(),
        node_type: node_type.to_string(),
        node_alias: node_id.to_string(),
        input_payload: json!({}),
        output_payload,
        error_payload,
        metrics_payload: json!({}),
        debug_payload: json!({}),
        provider_events: Vec::new(),
    }
}

fn trace(node_id: &str, output_payload: Value, error_payload: Option<Value>) -> NodeExecutionTrace {
    typed_trace(node_id, "llm", output_payload, error_payload)
}

#[test]
fn failed_flow_output_keeps_last_successful_node_payload() {
    let outcome = FlowDebugExecutionOutcome {
        stop_reason: ExecutionStopReason::Failed(NodeExecutionFailure {
            node_id: "llm-2".to_string(),
            node_alias: "LLM2".to_string(),
            error_payload: json!({ "message": "provider worker ended without result line" }),
        }),
        variable_pool: Map::new(),
        checkpoint_snapshot: None,
        node_traces: vec![
            trace("start", json!({}), None),
            trace("llm-1", json!({ "text": "first answer" }), None),
            trace(
                "llm-2",
                json!({}),
                Some(json!({ "message": "provider worker ended without result line" })),
            ),
        ],
    };

    assert_eq!(
        final_flow_output_payload(&outcome),
        json!({ "text": "first answer" })
    );
}

#[test]
fn completed_flow_output_uses_terminal_node_payload() {
    let outcome = FlowDebugExecutionOutcome {
        stop_reason: ExecutionStopReason::Completed,
        variable_pool: Map::new(),
        checkpoint_snapshot: None,
        node_traces: vec![
            trace("llm-1", json!({ "text": "first answer" }), None),
            trace("answer", json!({ "answer": "final answer" }), None),
        ],
    };

    assert_eq!(
        final_flow_output_payload(&outcome),
        json!({ "answer": "final answer" })
    );
}

#[test]
fn failed_flow_output_uses_terminal_answer_payload_even_when_answer_has_error() {
    let answer_error = json!({
        "error_kind": "prompt_template_unresolved",
        "message": "Answer node rendered with unresolved template selectors",
    });
    let answer_output = json!({
        "answer": "partial final answer",
        "error": answer_error.clone(),
    });
    let outcome = FlowDebugExecutionOutcome {
        stop_reason: ExecutionStopReason::Failed(NodeExecutionFailure {
            node_id: "answer".to_string(),
            node_alias: "Answer".to_string(),
            error_payload: answer_error.clone(),
        }),
        variable_pool: Map::new(),
        checkpoint_snapshot: None,
        node_traces: vec![
            trace("llm-1", json!({ "text": "partial final answer" }), None),
            typed_trace(
                "answer",
                "answer",
                answer_output.clone(),
                Some(answer_error),
            ),
        ],
    };

    assert_eq!(final_flow_output_payload(&outcome), answer_output);
}
