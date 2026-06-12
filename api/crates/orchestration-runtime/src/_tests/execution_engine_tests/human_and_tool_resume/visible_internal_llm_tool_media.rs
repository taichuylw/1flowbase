use super::visible_internal_llm_tool_fixtures::*;
use super::*;

fn configure_image_llm_tool(plan: &mut CompiledPlan) {
    let main_llm = plan
        .nodes
        .get_mut("node-llm")
        .expect("main llm node should exist");
    main_llm.config["visible_internal_llm_tools"][0]["tool_name"] = json!("image_llm");
    main_llm.config["visible_internal_llm_tools"][0]["connector_id"] = json!("image_llm");
    main_llm.config["visible_internal_llm_tools"][0]["input_schema"] = json!({
        "type": "object",
        "properties": {
            "task": { "type": "string" },
            "media": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "kind": { "type": "string", "enum": ["image"] },
                        "source": { "type": "string", "enum": ["workspace_path"] },
                        "path": { "type": "string" }
                    },
                    "required": ["kind", "source", "path"]
                }
            }
        },
        "required": ["task"]
    });
    let mounted_llm = plan
        .nodes
        .get_mut("node-mounted-llm")
        .expect("mounted llm node should exist");
    mounted_llm.bindings = BTreeMap::from([(
        "prompt_messages".to_string(),
        CompiledBinding {
            kind: "prompt_messages".to_string(),
            selector_paths: vec![vec![
                "visible_internal_llm_tool".to_string(),
                "arguments".to_string(),
                "task".to_string(),
            ]],
            raw_value: json!([
                {
                    "id": "mounted-user",
                    "role": "user",
                    "content": {
                        "kind": "templated_text",
                        "value": "{{ visible_internal_llm_tool.arguments.task }}"
                    }
                }
            ]),
        },
    )]);
}

#[tokio::test]
async fn missing_workspace_image_path_waits_for_client_read_without_invoking_image_llm() {
    let (invoker, captured_inputs) = sequential_tool_invoker(vec![tool_call_response(vec![
        ProviderToolCall {
            id: "call_image".to_string(),
            name: "image_llm".to_string(),
            arguments: json!({
                "task": "描述这张图片",
                "media": [
                    {
                        "kind": "image",
                        "source": "workspace_path",
                        "path": "uploads/windows-only.png"
                    }
                ]
            }),
            provider_metadata: json!({}),
        },
        ProviderToolCall {
            id: "call_read".to_string(),
            name: "Read".to_string(),
            arguments: json!({ "file_path": "E:\\code\\project\\uploads\\windows-only.png" }),
            provider_metadata: json!({}),
        },
    ])]);
    let mut plan = visible_internal_llm_tool_plan();
    configure_image_llm_tool(&mut plan);

    let outcome = start_flow_debug_run(
        &plan,
        &json!({
            "node-start": {
                "query": "uploads\\windows-only.png 找一下这幅图相关代码",
                "history": [],
                "tools": [
                    {
                        "name": "Read",
                        "description": "Read a file",
                        "input_schema": {
                            "type": "object",
                            "properties": {
                                "file_path": { "type": "string" }
                            },
                            "required": ["file_path"]
                        }
                    }
                ]
            }
        }),
        &invoker,
    )
    .await
    .unwrap();

    match outcome.stop_reason {
        ExecutionStopReason::WaitingCallback(ref pending) => {
            assert_eq!(pending.node_id, "node-llm");
            let pending_tool_names = pending.request_payload["tool_calls"]
                .as_array()
                .expect("pending request should include external tool calls")
                .iter()
                .filter_map(|tool_call| tool_call["name"].as_str())
                .collect::<Vec<_>>();
            assert_eq!(pending_tool_names, vec!["Read"]);
        }
        other => panic!("expected external Read callback wait, got {other:?}"),
    }

    let captured = captured_inputs
        .lock()
        .expect("captured inputs mutex poisoned")
        .clone();
    assert_eq!(
        captured.len(),
        1,
        "missing server-side workspace media must not invoke the mounted image LLM before client Read returns media"
    );

    let main_trace = outcome
        .node_traces
        .iter()
        .find(|trace| trace.node_id == "node-llm")
        .expect("main llm trace should exist");
    let route_events = main_trace.debug_payload["visible_internal_llm_tool_events"]
        .as_array()
        .expect("main debug payload should include route events");
    assert!(route_events.iter().any(|event| {
        event["event_type"] == json!("visible_internal_llm_tool_failed")
            && event["error_payload"]["details"]["error_code"]
                == json!("visible_internal_llm_tool_media_unavailable")
    }));
}

#[tokio::test]
async fn missing_workspace_image_path_reuses_inherited_image_content_blocks() {
    let (invoker, captured_inputs) = sequential_tool_invoker(vec![
        ProviderInvocationResult {
            final_content: Some("main-before ".to_string()),
            tool_calls: vec![ProviderToolCall {
                id: "call_image".to_string(),
                name: "image_llm".to_string(),
                arguments: json!({
                    "task": "描述这张图片",
                    "media": [
                        {
                            "kind": "image",
                            "source": "workspace_path",
                            "path": "uploads/windows-only.png"
                        }
                    ]
                }),
                provider_metadata: json!({}),
            }],
            finish_reason: Some(ProviderFinishReason::ToolCall),
            ..ProviderInvocationResult::default()
        },
        final_llm_response("mounted-visible "),
        final_llm_response("main-after"),
    ]);
    let mut plan = visible_internal_llm_tool_plan();
    configure_image_llm_tool(&mut plan);

    start_flow_debug_run(
        &plan,
        &json!({
            "node-start": {
                "query": "uploads\\windows-only.png 找一下这幅图相关代码",
                "history": [
                    {
                        "role": "user",
                        "content": "uploads\\windows-only.png 找一下这幅图相关代码"
                    },
                    {
                        "role": "assistant",
                        "content": "",
                        "tool_calls": [
                            {
                                "id": "call_read",
                                "name": "Read",
                                "arguments": {
                                    "file_path": "E:\\code\\project\\uploads\\windows-only.png"
                                }
                            }
                        ]
                    },
                    {
                        "role": "tool",
                        "tool_call_id": "call_read",
                        "name": "Read",
                        "content": "",
                        "content_blocks": [
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
        }),
        &invoker,
    )
    .await
    .unwrap();

    let captured = captured_inputs
        .lock()
        .expect("captured inputs mutex poisoned")
        .clone();
    assert_eq!(captured.len(), 3);
    let mounted_input = &captured[1];
    let media_blocks = mounted_input
        .messages
        .iter()
        .filter_map(|message| message.content_blocks.as_ref())
        .flat_map(|content_blocks| content_blocks.as_array().into_iter().flatten())
        .collect::<Vec<_>>();
    assert!(
        media_blocks.iter().any(|block| {
            block["type"] == json!("image")
                && block["source"]["media_type"] == json!("image/png")
                && block["source"]["data"] == json!("aW1hZ2U=")
        }),
        "mounted image LLM should receive inherited image content blocks when workspace_path is not readable by the server, got {:?}",
        mounted_input.messages
    );
}
