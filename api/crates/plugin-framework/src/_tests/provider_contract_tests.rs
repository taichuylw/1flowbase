use plugin_framework::{
    installation::PluginTaskStatus,
    provider_contract::{
        ModelDiscoveryMode, ProviderBalanceInfo, ProviderBalanceResult, ProviderInvocationInput,
        ProviderInvocationResult, ProviderMessage, ProviderMessageRole, ProviderRuntimeError,
        ProviderRuntimeErrorKind, ProviderRuntimeLine, ProviderStdioMethod, ProviderStdioRequest,
        ProviderStdioResponse, ProviderStreamEvent, ProviderToolCall, ProviderUsage,
    },
};
use serde_json::json;

#[test]
fn model_discovery_mode_accepts_all_supported_wire_values() {
    assert_eq!(
        ModelDiscoveryMode::try_from("static").unwrap(),
        ModelDiscoveryMode::Static
    );
    assert_eq!(
        ModelDiscoveryMode::try_from("dynamic").unwrap(),
        ModelDiscoveryMode::Dynamic
    );
    assert_eq!(
        ModelDiscoveryMode::try_from("hybrid").unwrap(),
        ModelDiscoveryMode::Hybrid
    );
    assert!(ModelDiscoveryMode::try_from("unknown").is_err());
}

#[test]
fn provider_usage_total_tokens_falls_back_to_known_segments() {
    let usage = ProviderUsage {
        input_tokens: Some(120),
        input_cache_hit_tokens: Some(80),
        input_cache_miss_tokens: Some(40),
        output_tokens: Some(45),
        reasoning_tokens: Some(12),
        cache_read_tokens: Some(9),
        cache_write_tokens: Some(3),
        total_tokens: None,
    };

    assert_eq!(usage.total_tokens(), Some(177));
}

#[test]
fn provider_usage_serializes_input_cache_hit_and_miss_tokens() {
    let usage = ProviderUsage {
        input_tokens: Some(100),
        input_cache_hit_tokens: Some(40),
        input_cache_miss_tokens: Some(60),
        output_tokens: Some(12),
        total_tokens: Some(112),
        ..ProviderUsage::default()
    };

    let payload = serde_json::to_value(&usage).unwrap();

    assert_eq!(payload["input_tokens"], 100);
    assert_eq!(payload["input_cache_hit_tokens"], 40);
    assert_eq!(payload["input_cache_miss_tokens"], 60);
    assert_eq!(payload["output_tokens"], 12);
    assert_eq!(payload["total_tokens"], 112);
}

#[test]
fn provider_runtime_error_normalizes_common_vendor_failures() {
    let auth_failed = ProviderRuntimeError::normalize(
        "invalid_api_key",
        "401 unauthorized",
        Some("upstream rejected api key"),
    );
    assert_eq!(auth_failed.kind, ProviderRuntimeErrorKind::AuthFailed);

    let endpoint_unreachable =
        ProviderRuntimeError::normalize("upstream_timeout", "connect timeout", None);
    assert_eq!(
        endpoint_unreachable.kind,
        ProviderRuntimeErrorKind::EndpointUnreachable
    );

    let rate_limited = ProviderRuntimeError::normalize("quota_exceeded", "429", None);
    assert_eq!(rate_limited.kind, ProviderRuntimeErrorKind::RateLimited);

    let unknown = ProviderRuntimeError::normalize("unexpected_shape", "bad payload", None);
    assert_eq!(
        unknown.kind,
        ProviderRuntimeErrorKind::ProviderInvalidResponse
    );
}

#[test]
fn plugin_task_status_marks_only_terminal_states() {
    assert!(!PluginTaskStatus::Pending.is_terminal());
    assert!(!PluginTaskStatus::Running.is_terminal());
    assert!(PluginTaskStatus::Success.is_terminal());
    assert!(PluginTaskStatus::Failed.is_terminal());
    assert!(PluginTaskStatus::Canceled.is_terminal());
    assert!(PluginTaskStatus::TimedOut.is_terminal());
}

#[test]
fn provider_stdio_contract_uses_snake_case_methods_and_result_payloads() {
    let request = ProviderStdioRequest {
        method: ProviderStdioMethod::ListModels,
        input: json!({
            "api_key": "secret"
        }),
    };

    let request_payload = serde_json::to_value(&request).unwrap();
    assert_eq!(request_payload["method"], "list_models");
    assert_eq!(request_payload["input"]["api_key"], "secret");

    let response: ProviderStdioResponse = serde_json::from_value(json!({
        "ok": true,
        "result": [
            {
                "model_id": "fixture_dynamic"
            }
        ]
    }))
    .unwrap();
    assert!(response.ok);
    assert_eq!(response.result[0]["model_id"], "fixture_dynamic");
}

#[test]
fn provider_balance_stdio_method_serializes_balance() {
    let request = ProviderStdioRequest {
        method: ProviderStdioMethod::Balance,
        input: json!({ "api_key": "secret" }),
    };

    assert_eq!(
        serde_json::to_value(request).unwrap(),
        json!({
            "method": "balance",
            "input": { "api_key": "secret" }
        })
    );
}

#[test]
fn provider_invocation_input_preserves_tool_message_metadata() {
    let input = ProviderInvocationInput {
        previous_response_id: Some("resp_previous".to_string()),
        messages: vec![
            ProviderMessage {
                role: ProviderMessageRole::Assistant,
                content: String::new(),
                name: None,
                tool_call_id: None,
                is_error: None,
                tool_calls: Some(json!([
                    {
                        "id": "call-1",
                        "type": "function",
                        "function": {
                            "name": "lookup_order",
                            "arguments": "{\"order_id\":\"A-1\"}"
                        }
                    }
                ])),
                content_blocks: None,
            },
            ProviderMessage {
                role: ProviderMessageRole::Tool,
                content: "{\"status\":\"shipped\"}".to_string(),
                name: None,
                tool_call_id: Some("call-1".to_string()),
                is_error: Some(true),
                tool_calls: None,
                content_blocks: None,
            },
        ],
        tools: vec![json!({
            "type": "function",
            "function": { "name": "lookup_order" }
        })],
        ..ProviderInvocationInput::default()
    };

    let payload = serde_json::to_value(input).unwrap();

    assert_eq!(payload["tools"][0]["function"]["name"], "lookup_order");
    assert_eq!(payload["previous_response_id"], "resp_previous");
    assert_eq!(payload["messages"][0]["tool_calls"][0]["id"], "call-1");
    assert_eq!(payload["messages"][1]["role"], "tool");
    assert_eq!(payload["messages"][1]["tool_call_id"], "call-1");
    assert_eq!(payload["messages"][1]["is_error"], true);
}

#[test]
fn provider_invocation_result_exposes_native_response_cursor() {
    let result = ProviderInvocationResult {
        final_content: Some("hello".to_string()),
        response_id: Some("resp_current".to_string()),
        ..ProviderInvocationResult::default()
    };

    let payload = serde_json::to_value(result).unwrap();
    let decoded: ProviderInvocationResult = serde_json::from_value(payload.clone()).unwrap();

    assert_eq!(payload["response_id"], "resp_current");
    assert_eq!(decoded.response_id.as_deref(), Some("resp_current"));
}

#[test]
fn provider_balance_result_serializes_deepseek_shape() {
    let result = ProviderBalanceResult {
        is_available: true,
        balance_infos: vec![ProviderBalanceInfo {
            currency: "CNY".to_string(),
            total_balance: "110.00".to_string(),
            granted_balance: Some("10.00".to_string()),
            topped_up_balance: Some("100.00".to_string()),
        }],
        provider_metadata: json!({ "provider": "deepseek" }),
    };

    let payload = serde_json::to_value(result).unwrap();

    assert_eq!(payload["is_available"], true);
    assert_eq!(payload["balance_infos"][0]["currency"], "CNY");
    assert_eq!(payload["balance_infos"][0]["total_balance"], "110.00");
    assert_eq!(payload["balance_infos"][0]["granted_balance"], "10.00");
    assert_eq!(payload["balance_infos"][0]["topped_up_balance"], "100.00");
    assert_eq!(payload["provider_metadata"]["provider"], "deepseek");
}

#[test]
fn provider_runtime_line_result_is_not_a_stream_event() {
    let line = ProviderRuntimeLine::Result {
        result: ProviderInvocationResult {
            final_content: Some("hello".into()),
            ..ProviderInvocationResult::default()
        },
    };

    assert_eq!(line.into_stream_event(), None);
}

#[test]
fn provider_runtime_line_text_maps_to_stream_event() {
    let line = ProviderRuntimeLine::TextDelta {
        delta: "hello".into(),
    };

    assert_eq!(
        line.into_stream_event(),
        Some(ProviderStreamEvent::TextDelta {
            delta: "hello".into()
        })
    );
}

#[test]
fn provider_runtime_line_tool_commit_preserves_arguments() {
    let line = ProviderRuntimeLine::ToolCallCommit {
        call: ProviderToolCall {
            id: "call-1".into(),
            name: "lookup_order".into(),
            arguments: json!({ "order_id": "A-1" }),
            provider_metadata: json!({}),
        },
    };

    match line.into_stream_event() {
        Some(ProviderStreamEvent::ToolCallCommit { call }) => {
            assert_eq!(call.arguments, json!({ "order_id": "A-1" }));
        }
        other => panic!("expected tool call commit stream event, got {other:?}"),
    }
}
