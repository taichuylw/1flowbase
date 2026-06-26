use super::visible_internal_llm_tool_fixtures::*;
use super::*;

#[tokio::test]
async fn visible_internal_llm_tool_provider_error_returns_error_tool_result_to_main_llm() {
    let provider_error_message = "429 Too Many Requests: rate limit exceeded";
    let (invoker, captured_inputs) = sequential_tool_output_invoker(vec![
        provider_output(ProviderInvocationResult {
            final_content: Some("main-before ".to_string()),
            tool_calls: vec![ProviderToolCall {
                id: "call_visible".to_string(),
                name: "inspect_visible_context".to_string(),
                arguments: json!({ "query": "image?" }),
                provider_metadata: json!({}),
            }],
            finish_reason: Some(ProviderFinishReason::ToolCall),
            ..ProviderInvocationResult::default()
        }),
        ProviderInvocationOutput {
            events: vec![ProviderStreamEvent::Error {
                error: ProviderRuntimeError {
                    kind: ProviderRuntimeErrorKind::RateLimited,
                    message: provider_error_message.to_string(),
                    provider_summary: None,
                    provider_details: None,
                },
            }],
            result: ProviderInvocationResult {
                finish_reason: Some(ProviderFinishReason::Error),
                ..ProviderInvocationResult::default()
            },
            first_token_at: None,
            time_to_first_token_ms: None,
        },
        provider_output(final_llm_response("main-after-error")),
    ]);
    let plan = visible_internal_llm_tool_plan();

    let outcome = start_flow_debug_run(
        &plan,
        &json!({ "node-start": { "query": "describe the picture", "history": [] } }),
        &invoker,
    )
    .await
    .unwrap();

    assert!(
        matches!(outcome.stop_reason, ExecutionStopReason::Completed),
        "expected completed run, got {:?}",
        outcome.stop_reason
    );
    assert_eq!(
        outcome.variable_pool["node-answer"]["answer"],
        json!("main-before main-after-error")
    );

    let captured = captured_inputs
        .lock()
        .expect("captured inputs mutex poisoned")
        .clone();
    assert_eq!(captured.len(), 3);
    let tool_result = captured[2]
        .messages
        .iter()
        .find(|message| {
            message.role == ProviderMessageRole::Tool
                && message.tool_call_id.as_deref() == Some("call_visible")
        })
        .expect("main llm recall should receive the internal tool failure result");
    assert_eq!(tool_result.is_error, Some(true));
    assert_eq!(tool_result.content, provider_error_message);
    assert!(!tool_result
        .content
        .contains("visible internal LLM tool branch node failed"));
    assert!(!tool_result.content.contains("\"details\""));

    let recall_trace = outcome
        .node_traces
        .iter()
        .find(|trace| trace.node_id == "node-llm")
        .expect("main llm trace should exist");
    assert_eq!(
        recall_trace.debug_payload["llm_rounds"][0]["tool_results"][0]["is_error"],
        json!(true)
    );
    let route_events = recall_trace.debug_payload["visible_internal_llm_tool_events"]
        .as_array()
        .expect("main debug payload should include route events");
    assert!(route_events.iter().any(|event| {
        event["event_type"] == json!("visible_internal_llm_tool_failed")
            && event["error_payload"]["message"]
                == json!("visible internal LLM tool branch node failed")
            && event["error_payload"]["details"]["message"] == json!(provider_error_message)
    }));
}
