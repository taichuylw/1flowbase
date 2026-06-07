use super::*;
use control_plane::application_public_api::native::{NativeRequiredAction, NativeRunStatus};
use time::OffsetDateTime;
use uuid::Uuid;

#[test]
fn anthropic_response_projects_native_tool_calls() {
    let run = NativeRunResult {
        id: Uuid::nil(),
        application_id: Uuid::nil(),
        api_key_id: Uuid::nil(),
        publication_version_id: Uuid::nil(),
        status: NativeRunStatus::Succeeded,
        node_input_payload: json!({}),
        metadata: json!({}),
        answer: None,
        required_action: None,
        tool_calls: Some(json!([
            {
                "id": "toolu_123",
                "name": "lookup_order",
                "arguments": {"order_id": "order_123"}
            }
        ])),
        usage: None,
        error: None,
        created_at: OffsetDateTime::UNIX_EPOCH,
    };

    let payload = serde_json::to_value(to_anthropic_response(run, "provider/model".into()))
        .expect("anthropic response serializes");

    assert_eq!(payload["stop_reason"], json!("tool_use"));
    assert_eq!(payload["content"][0]["type"], json!("tool_use"));
    assert_eq!(payload["content"][0]["name"], json!("lookup_order"));
    assert_eq!(
        payload["content"][0]["input"]["order_id"],
        json!("order_123")
    );
}

#[test]
fn anthropic_response_filters_internal_visible_llm_tool_calls() {
    let callback_task_id = Uuid::from_u128(0xcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcd);
    let run = NativeRunResult {
        id: Uuid::nil(),
        application_id: Uuid::nil(),
        api_key_id: Uuid::nil(),
        publication_version_id: Uuid::nil(),
        status: NativeRunStatus::Waiting,
        node_input_payload: json!({}),
        metadata: json!({}),
        answer: Some("visible internal LLM output".to_string()),
        required_action: Some(NativeRequiredAction {
            action_type: "submit_tool_outputs".to_string(),
            payload: json!({
                "callback_task_id": callback_task_id,
                "callback_kind": "llm_tool_calls"
            }),
        }),
        tool_calls: Some(json!([
            {
                "id": "toolu_internal",
                "type": "visible_internal_llm_tool",
                "name": "inspect_visible_context",
                "arguments": {"query": "visible"}
            }
        ])),
        usage: None,
        error: None,
        created_at: OffsetDateTime::UNIX_EPOCH,
    };

    let payload = serde_json::to_value(to_anthropic_response(run, "provider/model".into()))
        .expect("anthropic response serializes");

    assert_eq!(payload["stop_reason"], json!("end_turn"));
    assert_eq!(payload["content"][0]["type"], json!("text"));
    assert_eq!(
        payload["content"][0]["text"],
        json!("visible internal LLM output")
    );
    assert!(payload["content"]
        .as_array()
        .unwrap()
        .iter()
        .all(|block| block["type"] != json!("tool_use")));
}

#[test]
fn anthropic_response_projects_only_visible_assistant_text() {
    let run = NativeRunResult {
            id: Uuid::nil(),
            application_id: Uuid::nil(),
            api_key_id: Uuid::nil(),
            publication_version_id: Uuid::nil(),
            status: NativeRunStatus::Succeeded,
            node_input_payload: json!({}),
            metadata: json!({}),
            answer: Some(
                "<think>private reasoning</think>raw draft<tool_call>{}</tool_call>\n\n---\n\n下面是美化后内容\n\nVisible answer"
                    .to_string(),
            ),
            required_action: None,
            tool_calls: None,
            usage: None,
            error: None,
            created_at: OffsetDateTime::UNIX_EPOCH,
        };

    let payload = serde_json::to_value(to_anthropic_response(run, "provider/model".into()))
        .expect("anthropic response serializes");

    assert_eq!(payload["content"][0]["text"], json!("Visible answer"));
}

#[test]
fn anthropic_count_tokens_estimates_messages_and_tools() {
    let without_tools = anthropic_count_input_tokens(&json!({
        "model": "1flowbase",
        "messages": [{"role": "user", "content": "hello"}]
    }));
    let with_tools = anthropic_count_input_tokens(&json!({
        "model": "1flowbase",
        "messages": [{"role": "user", "content": "hello"}],
        "tools": [{
            "name": "lookup_order",
            "description": "Find an order",
            "input_schema": {"type": "object"}
        }]
    }));

    assert!(without_tools > 0);
    assert!(with_tools > without_tools);
}

#[test]
fn anthropic_probe_response_only_handles_known_lightweight_probes() {
    assert!(anthropic_probe_response(
        &json!({
            "model": "1flowbase",
            "max_tokens": 1,
            "messages": [{"role": "user", "content": "test"}]
        }),
        "1flowbase",
    )
    .is_some());
    assert!(anthropic_probe_response(
        &json!({
            "model": "1flowbase",
            "max_tokens": 1,
            "messages": [{"role": "user", "content": "hi ?"}]
        }),
        "1flowbase",
    )
    .is_none());
}

#[test]
fn anthropic_structured_title_response_returns_json_title() {
    let run = anthropic_structured_output_run(&json!({
        "model": "1flowbase",
        "messages": [{"role": "user", "content": "帮我找找这个代码位置"}],
        "output_config": {
            "format": {
                "type": "json_schema",
                "schema": {
                    "type": "object",
                    "properties": {
                        "title": { "type": "string" }
                    },
                    "required": ["title"],
                    "additionalProperties": false
                }
            }
        }
    }))
    .expect("structured output request should parse")
    .expect("title schema should be handled locally");
    let response = to_anthropic_response(run, "1flowbase".to_string());

    assert_eq!(
        response.content[0]["text"],
        json!("{\"title\":\"帮我找找这个代码位置\"}")
    );
}

#[test]
fn anthropic_structured_title_response_detects_session_title_prompt() {
    let run = anthropic_structured_output_run(&json!({
            "model": "1flowbase",
            "stream": true,
            "system": "Generate a concise, sentence-case title (3-7 words) that captures the main topic or goal of this coding session.\n\nReturn JSON with a single \"title\" field.",
            "messages": [{"role": "user", "content": "uploads/agent-flow-preview-debug.png 描述一下这幅图说什么？"}]
        }))
        .expect("structured output request should parse")
        .expect("session title prompt should be handled locally");
    let response = to_anthropic_response(run, "1flowbase".to_string());

    assert_eq!(
        response.content[0]["text"],
        json!("{\"title\":\"uploads/agent-flow-preview-debug.png 描述一下这幅图说什么？\"}")
    );
}

#[test]
fn claude_code_session_header_fills_missing_metadata_session_id() {
    let mut request = json!({
        "model": "1flowbase",
        "messages": [{"role": "user", "content": "hi"}]
    });
    let mut headers = HeaderMap::new();
    headers.insert(
        "x-claude-code-session-id",
        "header-session-123".parse().unwrap(),
    );

    merge_claude_code_session_header(&mut request, &headers);

    assert_eq!(
        request["metadata"]["session_id"],
        json!("header-session-123")
    );
}

#[test]
fn anthropic_response_encodes_callback_task_id_into_tool_use_ids() {
    let callback_task_id = Uuid::from_u128(0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee);
    let run = NativeRunResult {
        id: Uuid::nil(),
        application_id: Uuid::nil(),
        api_key_id: Uuid::nil(),
        publication_version_id: Uuid::nil(),
        status: NativeRunStatus::Waiting,
        node_input_payload: json!({}),
        metadata: json!({}),
        answer: None,
        required_action: Some(NativeRequiredAction {
            action_type: "submit_tool_outputs".to_string(),
            payload: json!({ "callback_task_id": callback_task_id, "callback_kind": "llm_tool_calls" }),
        }),
        tool_calls: Some(json!([
            {
                "id": "toolu_123",
                "name": "lookup_order",
                "arguments": {"order_id": "order_123"}
            }
        ])),
        usage: None,
        error: None,
        created_at: OffsetDateTime::UNIX_EPOCH,
    };

    let payload = serde_json::to_value(to_anthropic_response(run, "provider/model".into()))
        .expect("anthropic response serializes");

    let tool_use_id = payload["content"][0]["id"]
        .as_str()
        .expect("tool_use id should be encoded");
    assert_eq!(
        decode_anthropic_callback_tool_use_id(tool_use_id),
        Some((callback_task_id, "toolu_123".to_string()))
    );
}

#[test]
fn anthropic_tool_resume_request_decodes_tool_result_blocks() {
    let callback_task_id = Uuid::from_u128(0xffffffffffffffffffffffffffffffff);
    let tool_use_id = encode_anthropic_callback_tool_use_id(callback_task_id, "toolu_123");

    let resume = anthropic_tool_resume_request(&json!({
        "model": "1flowbase",
        "messages": [
            {
                "role": "assistant",
                "content": [{
                    "type": "tool_use",
                    "id": tool_use_id,
                    "name": "lookup_order",
                    "input": {}
                }]
            },
            {
                "role": "user",
                "content": [
                    {
                        "type": "tool_result",
                        "tool_use_id": tool_use_id,
                        "content": [{"type": "text", "text": "{\"order\":\"ready\"}"}]
                    }
                ]
            }
        ]
    }))
    .expect("tool_result should parse")
    .expect("encoded tool_result should resume callback");

    assert_eq!(resume.callback_task_id, callback_task_id);
    assert_eq!(resume.tool_results[0]["tool_call_id"], json!("toolu_123"));
    assert_eq!(
        resume.tool_results[0]["content"],
        json!("{\"order\":\"ready\"}")
    );
}

#[test]
fn anthropic_tool_resume_request_ignores_orphan_trailing_tool_result() {
    let callback_task_id = Uuid::from_u128(0xffffffffffffffffffffffffffffffff);
    let tool_use_id = encode_anthropic_callback_tool_use_id(callback_task_id, "toolu_123");

    let resume = anthropic_tool_resume_request(&json!({
        "model": "1flowbase",
        "messages": [
            {
                "role": "user",
                "content": [
                    {
                        "type": "tool_result",
                        "tool_use_id": tool_use_id,
                        "content": "stale result"
                    }
                ]
            }
        ]
    }))
    .expect("orphan tool_result should parse");

    assert!(resume.is_none());
}

#[test]
fn anthropic_tool_resume_request_preserves_media_tool_result_content() {
    let callback_task_id = Uuid::from_u128(0x99999999999999999999999999999999);
    let tool_use_id = encode_anthropic_callback_tool_use_id(callback_task_id, "toolu_image");

    let resume = anthropic_tool_resume_request(&json!({
        "model": "1flowbase",
        "messages": [
            {
                "role": "assistant",
                "content": [{
                    "type": "tool_use",
                    "id": tool_use_id,
                    "name": "Read",
                    "input": {}
                }]
            },
            {
                "role": "user",
                "content": [
                    {
                        "type": "tool_result",
                        "tool_use_id": tool_use_id,
                        "content": [
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
            }
        ]
    }))
    .expect("tool_result should parse")
    .expect("encoded tool_result should resume callback");

    assert_eq!(resume.callback_task_id, callback_task_id);
    assert_eq!(resume.tool_results[0]["tool_call_id"], json!("toolu_image"));
    assert_eq!(resume.tool_results[0]["content"][0]["type"], json!("image"));
}

#[test]
fn anthropic_tool_resume_request_uses_latest_trailing_tool_result_message() {
    let previous_callback_task_id = Uuid::from_u128(0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa);
    let current_callback_task_id = Uuid::from_u128(0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb);
    let previous_tool_use_id =
        encode_anthropic_callback_tool_use_id(previous_callback_task_id, "toolu_previous");
    let current_tool_use_id =
        encode_anthropic_callback_tool_use_id(current_callback_task_id, "toolu_current");

    let resume = anthropic_tool_resume_request(&json!({
        "model": "1flowbase",
        "messages": [
            {"role": "user", "content": "first"},
            {
                "role": "assistant",
                "content": [{
                    "type": "tool_use",
                    "id": previous_tool_use_id,
                    "name": "lookup_previous",
                    "input": {}
                }]
            },
            {
                "role": "user",
                "content": [{
                    "type": "tool_result",
                    "tool_use_id": previous_tool_use_id,
                    "content": "old result"
                }]
            },
            {"role": "assistant", "content": "old answer"},
            {"role": "user", "content": "next"},
            {
                "role": "assistant",
                "content": [{
                    "type": "tool_use",
                    "id": current_tool_use_id,
                    "name": "lookup_current",
                    "input": {}
                }]
            },
            {
                "role": "user",
                "content": [{
                    "type": "tool_result",
                    "tool_use_id": current_tool_use_id,
                    "content": "new result"
                }]
            }
        ]
    }))
    .expect("resume request should parse")
    .expect("trailing tool_result should resume callback");

    assert_eq!(resume.callback_task_id, current_callback_task_id);
    assert_eq!(resume.tool_results.as_array().unwrap().len(), 1);
    assert_eq!(
        resume.tool_results[0]["tool_call_id"],
        json!("toolu_current")
    );
    assert_eq!(resume.tool_results[0]["content"], json!("new result"));
}

#[test]
fn anthropic_tool_resume_request_ignores_historical_tool_results_before_latest_user_text() {
    let callback_task_id = Uuid::from_u128(0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa);
    let tool_use_id = encode_anthropic_callback_tool_use_id(callback_task_id, "toolu_previous");

    let resume = anthropic_tool_resume_request(&json!({
        "model": "1flowbase",
        "messages": [
            {"role": "user", "content": "first"},
            {
                "role": "assistant",
                "content": [{
                    "type": "tool_use",
                    "id": tool_use_id,
                    "name": "lookup_previous",
                    "input": {}
                }]
            },
            {
                "role": "user",
                "content": [{
                    "type": "tool_result",
                    "tool_use_id": tool_use_id,
                    "content": "old result"
                }]
            },
            {"role": "assistant", "content": "old answer"},
            {"role": "user", "content": "next question"}
        ]
    }))
    .expect("historical tool_result should parse");

    assert!(resume.is_none());
}

#[test]
fn anthropic_tool_resume_request_ignores_tool_result_mixed_with_new_user_text() {
    let callback_task_id = Uuid::from_u128(0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa);
    let tool_use_id = encode_anthropic_callback_tool_use_id(callback_task_id, "toolu_previous");

    let resume = anthropic_tool_resume_request(&json!({
        "model": "1flowbase",
        "messages": [
            {
                "role": "assistant",
                "content": [{
                    "type": "tool_use",
                    "id": tool_use_id,
                    "name": "Read",
                    "input": {}
                }]
            },
            {
                "role": "user",
                "content": [
                    {
                        "type": "tool_result",
                        "tool_use_id": tool_use_id,
                        "content": [{
                            "type": "image",
                            "source": {
                                "type": "base64",
                                "media_type": "image/png",
                                "data": "aW1hZ2U="
                            }
                        }]
                    },
                    {
                        "type": "text",
                        "text": "帮我找找这个代码位置"
                    }
                ]
            }
        ]
    }))
    .expect("mixed tool_result and text should parse");

    assert!(resume.is_none());
}

#[test]
fn anthropic_tool_resume_request_uses_latest_callback_from_contiguous_tool_results() {
    let previous_callback_task_id = Uuid::from_u128(0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa);
    let current_callback_task_id = Uuid::from_u128(0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb);
    let previous_tool_use_id =
        encode_anthropic_callback_tool_use_id(previous_callback_task_id, "toolu_previous");
    let current_tool_use_id =
        encode_anthropic_callback_tool_use_id(current_callback_task_id, "toolu_current");

    let resume = anthropic_tool_resume_request(&json!({
        "model": "1flowbase",
        "messages": [
            {"role": "user", "content": "first"},
            {
                "role": "assistant",
                "content": [{
                    "type": "tool_use",
                    "id": previous_tool_use_id,
                    "name": "lookup_previous",
                    "input": {}
                }]
            },
            {
                "role": "user",
                "content": [{
                    "type": "tool_result",
                    "tool_use_id": previous_tool_use_id,
                    "content": "old result"
                }]
            },
            {
                "role": "assistant",
                "content": [{
                    "type": "tool_use",
                    "id": current_tool_use_id,
                    "name": "lookup_current",
                    "input": {}
                }]
            },
            {
                "role": "user",
                "content": [{
                    "type": "tool_result",
                    "tool_use_id": previous_tool_use_id,
                    "content": "old result replayed"
                }]
            },
            {
                "role": "user",
                "content": [{
                    "type": "tool_result",
                    "tool_use_id": current_tool_use_id,
                    "content": "new result"
                }]
            }
        ]
    }))
    .expect("mixed trailing tool_result history should parse")
    .expect("latest callback should be resumed");

    assert_eq!(resume.callback_task_id, current_callback_task_id);
    assert_eq!(resume.tool_results.as_array().unwrap().len(), 1);
    assert_eq!(
        resume.tool_results[0]["tool_call_id"],
        json!("toolu_current")
    );
    assert_eq!(resume.tool_results[0]["content"], json!("new result"));
}
