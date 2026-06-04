use control_plane::application_public_api::compat::anthropic::{
    map_messages_request, AnthropicCompatError,
};
use serde_json::{json, Value};

fn base_request() -> Value {
    json!({
        "model": "claude-compatible-custom",
        "max_tokens": 512,
        "messages": [
            {"role": "user", "content": "Earlier question"},
            {"role": "assistant", "content": "Earlier answer"},
            {"role": "user", "content": "Final question"}
        ]
    })
}

fn assert_unsupported_feature(request: Value) {
    let error = map_messages_request(request).unwrap_err();

    assert_anthropic_unsupported_feature(error);
}

fn assert_anthropic_unsupported_feature(error: AnthropicCompatError) {
    assert_eq!(error.error_type, "unsupported_feature");
    assert!(error.message.contains("is not supported by this endpoint"));
}

#[test]
fn system_maps_to_native_system_context() {
    let mut request = base_request();
    request["system"] = json!("Use the support playbook.");

    let native = map_messages_request(request).unwrap();

    assert_eq!(native.system.as_deref(), Some("Use the support playbook."));
    assert_eq!(
        native.history,
        vec![
            json!({"role": "user", "content": "Earlier question"}),
            json!({"role": "assistant", "content": "Earlier answer"})
        ]
    );
}

#[test]
fn last_user_text_maps_to_native_query() {
    let native = map_messages_request(base_request()).unwrap();

    assert_eq!(native.query, "Final question");
}

#[test]
fn prior_messages_map_to_native_history() {
    let native = map_messages_request(base_request()).unwrap();

    assert_eq!(
        native.history,
        vec![
            json!({"role": "user", "content": "Earlier question"}),
            json!({"role": "assistant", "content": "Earlier answer"})
        ]
    );
}

#[test]
fn stream_true_maps_to_native_streaming_response_mode() {
    let mut request = base_request();
    request["stream"] = json!(true);

    let native = map_messages_request(request).unwrap();

    assert_eq!(native.response_mode.as_deref(), Some("streaming"));
}

#[test]
fn metadata_expand_id_maps_to_native_conversation_user() {
    let mut request = base_request();
    request["metadata"] = json!({
        "expand_id": "external-user-123"
    });

    let native = map_messages_request(request).unwrap();

    assert_eq!(
        native.conversation.get("user"),
        Some(&json!("external-user-123"))
    );
}

#[test]
fn metadata_user_id_json_maps_session_to_native_conversation() {
    let mut request = base_request();
    request["metadata"] = json!({
        "user_id": "{\"device_id\":\"device-123\",\"account_uuid\":\"\",\"session_id\":\"session-456\"}"
    });

    let native = map_messages_request(request).unwrap();

    assert_eq!(native.conversation.get("user"), Some(&json!("device-123")));
    assert_eq!(native.conversation.get("id"), Some(&json!("session-456")));
}

#[test]
fn metadata_plain_user_id_session_suffix_maps_session_to_native_conversation() {
    let mut request = base_request();
    request["metadata"] = json!({
        "user_id": "user_31fb5a_account__session_3e7058c2-3120-4222-bb14-c99ec85e1c0f"
    });

    let native = map_messages_request(request).unwrap();

    assert_eq!(
        native.conversation.get("user"),
        Some(&json!(
            "user_31fb5a_account__session_3e7058c2-3120-4222-bb14-c99ec85e1c0f"
        ))
    );
    assert_eq!(
        native.conversation.get("id"),
        Some(&json!("3e7058c2-3120-4222-bb14-c99ec85e1c0f"))
    );
}

#[test]
fn metadata_session_id_maps_to_native_conversation_id() {
    let mut request = base_request();
    request["metadata"] = json!({
        "expand_id": "external-user-123",
        "session_id": "header-session-789"
    });

    let native = map_messages_request(request).unwrap();

    assert_eq!(
        native.conversation.get("user"),
        Some(&json!("external-user-123"))
    );
    assert_eq!(
        native.conversation.get("id"),
        Some(&json!("header-session-789"))
    );
}

#[test]
fn model_maps_exactly_without_validation() {
    let mut request = base_request();
    request["model"] = json!("unregistered/anthropic:model.with/slashes");

    let native = map_messages_request(request).unwrap();

    assert_eq!(
        native.model.as_deref(),
        Some("unregistered/anthropic:model.with/slashes")
    );
}

#[test]
fn tools_are_accepted_for_agent_framework_compatibility() {
    let mut request = base_request();
    request["tools"] = json!([
        {
            "name": "lookup_order",
            "description": "Find an order",
            "input_schema": {"type": "object"}
        }
    ]);

    let native = map_messages_request(request).unwrap();

    assert_eq!(native.query, "Final question");
    assert_eq!(native.model.as_deref(), Some("claude-compatible-custom"));
    assert_eq!(
        native.inputs.as_value()["tools"][0]["name"],
        json!("lookup_order")
    );
}

#[test]
fn tool_choice_is_accepted_for_agent_framework_compatibility() {
    let mut request = base_request();
    request["tool_choice"] = json!({
        "type": "tool",
        "name": "lookup_order"
    });

    let native = map_messages_request(request).unwrap();

    assert_eq!(native.query, "Final question");
    assert_eq!(
        native.inputs.as_value()["tool_choice"]["name"],
        json!("lookup_order")
    );
}

#[test]
fn tool_use_and_tool_result_blocks_map_to_native_history_and_query() {
    let mut request = base_request();
    request["messages"] = json!([
        {"role": "user", "content": "Find order"},
        {
            "role": "assistant",
            "content": [
                {
                    "type": "tool_use",
                    "id": "toolu_123",
                    "name": "lookup_order",
                    "input": {"order_id": "order_123"}
                }
            ]
        },
        {
            "role": "user",
            "content": [
                {
                    "type": "tool_result",
                    "tool_use_id": "toolu_123",
                    "content": "Order found"
                }
            ]
        }
    ]);

    let native = map_messages_request(request).unwrap();

    assert_eq!(native.query, "Order found");
    assert_eq!(
        native.history,
        vec![
            json!({"role": "user", "content": "Find order"}),
            json!({
                "role": "assistant",
                "content": "",
                "content_blocks": [
                    {
                        "type": "tool_use",
                        "id": "toolu_123",
                        "name": "lookup_order",
                        "input": {"order_id": "order_123"}
                    }
                ]
            })
        ]
    );
}

#[test]
fn last_user_multimodal_content_maps_query_text_and_preserves_media_blocks() {
    let native = map_messages_request(json!({
        "model": "claude-compatible-custom",
        "messages": [
            {
                "role": "user",
                "content": [
                    {"type": "text", "text": "Describe this image"},
                    {
                        "type": "image",
                        "source": {
                            "type": "base64",
                            "media_type": "image/png",
                            "data": "aW1hZ2U="
                        }
                    }
                ]
            }
        ]
    }))
    .unwrap();

    assert_eq!(native.query, "Describe this image");
    assert_eq!(native.history.len(), 1);
    assert_eq!(native.history[0]["role"], json!("user"));
    assert_eq!(native.history[0]["content"], json!(""));
    assert_eq!(
        native.history[0]["content_blocks"][0]["type"],
        json!("image")
    );
    assert_eq!(
        native.history[0]["content_blocks"][0]["source"]["media_type"],
        json!("image/png")
    );
}

#[test]
fn last_user_mixed_tool_result_and_text_uses_visible_text_as_query() {
    let native = map_messages_request(json!({
        "model": "claude-compatible-custom",
        "messages": [
            {"role": "user", "content": "uploads/agent-flow-preview-debug.png 描述一下这幅图说什么？"},
            {
                "role": "assistant",
                "content": [
                    {
                        "type": "tool_use",
                        "id": "toolu_read",
                        "name": "Read",
                        "input": {"file_path": "uploads/agent-flow-preview-debug.png"}
                    }
                ]
            },
            {
                "role": "user",
                "content": [
                    {
                        "type": "tool_result",
                        "tool_use_id": "toolu_read",
                        "content": "<tool_use_error>old tool payload</tool_use_error>\nold image output"
                    },
                    {"type": "text", "text": "帮我找找这个代码位置"}
                ]
            }
        ]
    }))
    .unwrap();

    assert_eq!(native.query, "帮我找找这个代码位置");
    assert!(!native.query.contains("old image output"));
}

#[test]
fn assistant_thinking_history_is_ignored_for_claude_code_replay() {
    let native = map_messages_request(json!({
        "model": "claude-compatible-custom",
        "messages": [
            {"role": "user", "content": "hi ?"},
            {
                "role": "assistant",
                "content": [
                    {"type": "thinking", "thinking": "internal reasoning", "signature": ""}
                ]
            },
            {
                "role": "assistant",
                "content": [
                    {"type": "text", "text": "Hello!"}
                ]
            },
            {"role": "user", "content": "next question"}
        ]
    }))
    .unwrap();

    assert_eq!(native.query, "next question");
    assert_eq!(
        native.history,
        vec![
            json!({"role": "user", "content": "hi ?"}),
            json!({"role": "assistant", "content": "Hello!"})
        ]
    );
}

#[test]
fn computer_use_returns_unsupported_feature() {
    let mut request = base_request();
    request["messages"] = json!([
        {
            "role": "assistant",
            "content": [
                {
                    "type": "tool_use",
                    "id": "toolu_computer",
                    "name": "computer",
                    "input": {"action": "screenshot"}
                }
            ]
        },
        {"role": "user", "content": "What is on screen?"}
    ]);

    assert_unsupported_feature(request);
}
