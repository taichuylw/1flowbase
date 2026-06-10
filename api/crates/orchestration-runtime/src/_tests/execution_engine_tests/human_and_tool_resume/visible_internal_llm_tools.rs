use super::visible_internal_llm_tool_fixtures::*;
use super::*;

#[tokio::test]
async fn visible_internal_llm_tool_outputs_visible_text_and_recalls_main_llm() {
    let (invoker, captured_inputs) = sequential_tool_invoker(vec![
        ProviderInvocationResult {
            final_content: Some("main-before ".to_string()),
            tool_calls: vec![ProviderToolCall {
                id: "call_visible".to_string(),
                name: "inspect_visible_context".to_string(),
                arguments: json!({ "query": "image?" }),
                provider_metadata: json!({}),
            }],
            finish_reason: Some(ProviderFinishReason::ToolCall),
            ..ProviderInvocationResult::default()
        },
        final_llm_response("mounted-visible "),
        final_llm_response("main-after"),
    ]);
    let plan = visible_internal_llm_tool_plan();

    let outcome = start_flow_debug_run(
        &plan,
        &json!({
            "node-start": {
                "query": "describe the picture",
                "history": [
                    {
                        "role": "user",
                        "content": "describe the picture",
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

    assert!(
        matches!(outcome.stop_reason, ExecutionStopReason::Completed),
        "expected completed run, got {:?}",
        outcome.stop_reason
    );
    assert_eq!(
        outcome.variable_pool["node-answer"]["answer"],
        json!("main-before mounted-visible main-after")
    );
    assert!(outcome
        .node_traces
        .iter()
        .all(|trace| trace.node_id != "node-mounted-llm"));

    let captured = captured_inputs
        .lock()
        .expect("captured inputs mutex poisoned")
        .clone();
    assert_eq!(captured.len(), 3);
    assert_eq!(
        captured[0].tools[0]["function"]["name"],
        json!("inspect_visible_context")
    );
    assert_eq!(
        captured[1].messages[0]
            .content_blocks
            .as_ref()
            .expect("mounted llm should receive media content blocks")[0]["type"],
        json!("image")
    );
    assert!(captured[2].messages.iter().any(|message| {
        message
            .tool_calls
            .as_ref()
            .and_then(|tool_calls| tool_calls.get(0))
            .and_then(|tool_call| tool_call.get("id"))
            == Some(&json!("call_visible"))
    }));
    let tool_result = captured[2]
        .messages
        .iter()
        .find(|message| {
            message.role == ProviderMessageRole::Tool
                && message.tool_call_id.as_deref() == Some("call_visible")
        })
        .expect("main llm recall should include the internal tool result");
    assert_eq!(tool_result.content, "mounted-visible ");
}

#[tokio::test]
async fn visible_internal_llm_tool_executes_composed_connector_branch() {
    let (invoker, captured_inputs) = sequential_tool_invoker(vec![
        ProviderInvocationResult {
            final_content: Some("main-before ".to_string()),
            tool_calls: vec![ProviderToolCall {
                id: "call_visible".to_string(),
                name: "inspect_visible_context".to_string(),
                arguments: json!({ "query": "image?" }),
                provider_metadata: json!({}),
            }],
            finish_reason: Some(ProviderFinishReason::ToolCall),
            ..ProviderInvocationResult::default()
        },
        final_llm_response("mounted-visible "),
        final_llm_response("main-after"),
    ]);
    let plan = visible_internal_llm_tool_chain_plan();

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
        json!("main-before mounted-visible main-after")
    );
    assert!(
        outcome
            .node_traces
            .iter()
            .all(|trace| trace.node_id != "node-tool-transform"
                && trace.node_id != "node-mounted-llm")
    );

    let captured = captured_inputs
        .lock()
        .expect("captured inputs mutex poisoned")
        .clone();
    assert_eq!(captured.len(), 3);
    assert_eq!(
        captured[1].messages[0].content,
        "Inspect transformed image?"
    );
    let tool_result = captured[2]
        .messages
        .iter()
        .find(|message| {
            message.role == ProviderMessageRole::Tool
                && message.tool_call_id.as_deref() == Some("call_visible")
        })
        .expect("main llm recall should include the internal tool result");
    assert_eq!(tool_result.content, "mounted-visible ");
}

#[tokio::test]
async fn visible_internal_llm_tool_returns_tool_result_node_content() {
    let (invoker, captured_inputs) = sequential_tool_invoker(vec![
        ProviderInvocationResult {
            final_content: Some("main-before ".to_string()),
            tool_calls: vec![ProviderToolCall {
                id: "call_visible".to_string(),
                name: "inspect_visible_context".to_string(),
                arguments: json!({ "query": "image?" }),
                provider_metadata: json!({}),
            }],
            finish_reason: Some(ProviderFinishReason::ToolCall),
            ..ProviderInvocationResult::default()
        },
        final_llm_response("mounted-visible "),
        final_llm_response("main-after"),
    ]);
    let plan = visible_internal_llm_tool_plan_with_result();

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
        json!("main-before tool-result: mounted-visible main-after")
    );

    let captured = captured_inputs
        .lock()
        .expect("captured inputs mutex poisoned")
        .clone();
    let tool_result = captured[2]
        .messages
        .iter()
        .find(|message| {
            message.role == ProviderMessageRole::Tool
                && message.tool_call_id.as_deref() == Some("call_visible")
        })
        .expect("main llm recall should include the explicit tool result");
    assert_eq!(tool_result.content, "tool-result: mounted-visible ");
}

#[tokio::test]
async fn visible_internal_llm_tool_branch_llm_can_wait_for_external_tool_callback() {
    let (waiting_invoker, waiting_inputs) = sequential_tool_invoker(vec![
        ProviderInvocationResult {
            final_content: Some("main-before ".to_string()),
            tool_calls: vec![ProviderToolCall {
                id: "call_visible".to_string(),
                name: "inspect_visible_context".to_string(),
                arguments: json!({ "query": "image?" }),
                provider_metadata: json!({}),
            }],
            finish_reason: Some(ProviderFinishReason::ToolCall),
            ..ProviderInvocationResult::default()
        },
        tool_call_response(vec![ProviderToolCall {
            id: "call_bash".to_string(),
            name: "Bash".to_string(),
            arguments: json!({ "command": "file tmp/frontstage-layout-preview.png" }),
            provider_metadata: json!({}),
        }]),
    ]);
    let plan = visible_internal_llm_tool_plan();

    let waiting = start_flow_debug_run(
        &plan,
        &json!({
            "node-start": {
                "query": "describe the picture",
                "history": [],
                "tools": [
                    {
                        "name": "Bash",
                        "description": "Run a shell command",
                        "input_schema": {
                            "type": "object",
                            "properties": {
                                "command": { "type": "string" }
                            },
                            "required": ["command"]
                        }
                    }
                ]
            }
        }),
        &waiting_invoker,
    )
    .await
    .unwrap();

    match waiting.stop_reason {
        ExecutionStopReason::WaitingCallback(ref pending) => {
            assert_eq!(pending.node_id, "node-mounted-llm");
            assert_eq!(pending.callback_kind, "llm_tool_calls");
            assert_eq!(
                pending.request_payload["tool_calls"][0]["name"],
                json!("Bash")
            );
        }
        other => panic!("expected mounted llm external tool callback wait, got {other:?}"),
    }

    let checkpoint = waiting
        .checkpoint_snapshot
        .clone()
        .expect("mounted llm tool wait should have checkpoint");
    let main_wait_trace = waiting
        .node_traces
        .iter()
        .find(|trace| trace.node_id == "node-llm")
        .expect("main llm waiting trace should exist");
    let route_events = main_wait_trace.debug_payload["visible_internal_llm_tool_events"]
        .as_array()
        .expect("main waiting trace should include visible internal route events");
    assert!(route_events.iter().any(|event| {
        event["event_type"] == json!("visible_internal_llm_tool_waiting_callback")
            && event["waiting_node_id"] == json!("node-mounted-llm")
            && event["request_payload"]["tool_calls"][0]["name"] == json!("Bash")
    }));
    let captured_waiting = waiting_inputs
        .lock()
        .expect("captured inputs mutex poisoned")
        .clone();
    assert_eq!(captured_waiting.len(), 2);
    let mounted_tool_names = captured_waiting[1]
        .tools
        .iter()
        .filter_map(|tool| tool["function"]["name"].as_str())
        .collect::<Vec<_>>();
    assert!(mounted_tool_names.contains(&"Bash"));

    let (resume_invoker, resumed_inputs) = sequential_tool_invoker(vec![
        final_llm_response("mounted-after-tool "),
        final_llm_response("main-after"),
    ]);
    let resumed = resume_flow_debug_run(
        &plan,
        &checkpoint,
        "node-mounted-llm",
        &json!({
            "tool_results": [
                {
                    "tool_call_id": "call_bash",
                    "content": "tmp/frontstage-layout-preview.png: PNG image data"
                }
            ]
        }),
        &resume_invoker,
    )
    .await
    .unwrap();

    assert!(
        matches!(resumed.stop_reason, ExecutionStopReason::Completed),
        "expected completed run, got {:?}",
        resumed.stop_reason
    );
    assert_eq!(
        resumed.variable_pool["node-answer"]["answer"],
        json!("main-before mounted-after-tool main-after")
    );

    let captured_resumed = resumed_inputs
        .lock()
        .expect("captured inputs mutex poisoned")
        .clone();
    assert_eq!(captured_resumed.len(), 2);
    assert_eq!(
        captured_resumed[0].messages.last().unwrap().role,
        ProviderMessageRole::Tool
    );
    assert_eq!(
        captured_resumed[0]
            .messages
            .last()
            .unwrap()
            .tool_call_id
            .as_deref(),
        Some("call_bash")
    );
    let main_internal_tool_result = captured_resumed[1]
        .messages
        .iter()
        .find(|message| {
            message.role == ProviderMessageRole::Tool
                && message.tool_call_id.as_deref() == Some("call_visible")
        })
        .expect("main llm recall should include mounted llm output as hidden tool result");
    assert_eq!(main_internal_tool_result.content, "mounted-after-tool ");
}

#[tokio::test]
async fn visible_internal_llm_tool_branch_inherits_run_context_query_when_argument_is_only_task() {
    let (invoker, captured_inputs) = sequential_tool_invoker(vec![
        ProviderInvocationResult {
            final_content: Some("main-before ".to_string()),
            tool_calls: vec![ProviderToolCall {
                id: "call_visible".to_string(),
                name: "inspect_visible_context".to_string(),
                arguments: json!({ "task": "describe the image" }),
                provider_metadata: json!({}),
            }],
            finish_reason: Some(ProviderFinishReason::ToolCall),
            ..ProviderInvocationResult::default()
        },
        final_llm_response("mounted-visible "),
        final_llm_response("main-after"),
    ]);
    let mut plan = visible_internal_llm_tool_plan();
    let mounted_llm = plan
        .nodes
        .get_mut("node-mounted-llm")
        .expect("mounted llm node should exist");
    mounted_llm.config = json!({
        "model_provider": {
            "provider_code": "fixture_provider",
            "model_id": "gpt-5.4-mini"
        },
        "context_policy": {
            "integration_context": "enabled"
        }
    });
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

    start_flow_debug_run(
        &plan,
        &json!({
            "node-start": {
                "query": "调用 image_llm 看看 tmp/frontstage-layout-preview.png 内容是什么",
                "history": [],
                "files": [
                    {
                        "path": "tmp/frontstage-layout-preview.png",
                        "media_type": "image/png"
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
    let mounted_messages = &captured[1].messages;
    assert!(
        mounted_messages.iter().any(|message| message
            .content
            .contains("tmp/frontstage-layout-preview.png")),
        "mounted LLM should inherit original run query/files context, got {mounted_messages:?}"
    );
}

#[tokio::test]
async fn visible_internal_image_llm_tool_injects_workspace_path_media_blocks() {
    let media_dir = std::env::current_dir()
        .expect("test current dir should be available")
        .join("target")
        .join("visible-internal-media");
    tokio::fs::create_dir_all(&media_dir)
        .await
        .expect("test media dir should be created");
    let image_path = media_dir.join("sample.png");
    tokio::fs::write(&image_path, b"image")
        .await
        .expect("test image should be written");
    let relative_image_path = "target/visible-internal-media/sample.png";

    let (invoker, captured_inputs) = sequential_tool_invoker(vec![
        ProviderInvocationResult {
            final_content: Some("main-before ".to_string()),
            tool_calls: vec![ProviderToolCall {
                id: "call_visible".to_string(),
                name: "inspect_visible_context".to_string(),
                arguments: json!({
                    "task": "看一下这幅图内容是什么",
                    "media": [
                        {
                            "kind": "image",
                            "source": "workspace_path",
                            "path": relative_image_path
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
    let main_llm = plan
        .nodes
        .get_mut("node-llm")
        .expect("main llm node should exist");
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

    start_flow_debug_run(
        &plan,
        &json!({
            "node-start": {
                "query": format!("看{} 看一下这幅图内容是什么", relative_image_path),
                "history": [],
                "tools": [
                    {
                        "name": "Bash",
                        "description": "Run a shell command",
                        "input_schema": {
                            "type": "object",
                            "properties": {
                                "command": { "type": "string" }
                            },
                            "required": ["command"]
                        }
                    },
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

    let captured = captured_inputs
        .lock()
        .expect("captured inputs mutex poisoned")
        .clone();
    assert_eq!(captured.len(), 3);
    let main_tool_names = captured[0]
        .tools
        .iter()
        .filter_map(|tool| tool["function"]["name"].as_str())
        .collect::<Vec<_>>();
    assert_eq!(main_tool_names, vec!["inspect_visible_context"]);
    let image_tool_schema = captured[0]
        .tools
        .iter()
        .find(|tool| tool["function"]["name"] == json!("inspect_visible_context"))
        .map(|tool| &tool["function"]["parameters"])
        .expect("visible internal media tool schema should be registered");
    assert_eq!(
        image_tool_schema["properties"]["media"]["items"]["properties"]["source"]["enum"][0],
        json!("workspace_path")
    );
    let mounted_input = &captured[1];
    assert!(
        mounted_input.tools.is_empty(),
        "mounted image LLM should not inherit outer client tools when media is present"
    );
    let media_blocks = mounted_input.messages[0]
        .content_blocks
        .as_ref()
        .expect("mounted image LLM should receive media content blocks")
        .as_array()
        .expect("content blocks should be an array");
    assert!(media_blocks.iter().any(|block| {
        block["type"] == json!("image_url")
            && block["image_url"]["url"]
                .as_str()
                .is_some_and(|url| url.starts_with("data:image/png;base64,"))
    }));
}

#[tokio::test]
async fn visible_internal_image_llm_tool_preserves_visible_media_arguments() {
    let media_dir = std::env::current_dir()
        .expect("test current dir should be available")
        .join("target")
        .join("visible-internal-media");
    tokio::fs::create_dir_all(&media_dir)
        .await
        .expect("test media dir should be created");
    let image_path = media_dir.join("sanitize.png");
    tokio::fs::write(&image_path, b"image")
        .await
        .expect("test image should be written");
    let relative_image_path = "target/visible-internal-media/sanitize.png";

    let (invoker, captured_inputs) = sequential_tool_invoker(vec![
        ProviderInvocationResult {
            final_content: Some("main-before ".to_string()),
            tool_calls: vec![ProviderToolCall {
                id: "call_visible".to_string(),
                name: "image_llm".to_string(),
                arguments: json!({
                    "task": "看一下这幅图内容是什么",
                    "media": [
                        {
                            "kind": "image",
                            "source": "workspace_path",
                            "path": image_path.to_string_lossy(),
                            "media_type": "image/png",
                            "custom_note": "keep-me"
                        },
                        {
                            "kind": "image",
                            "source": "url",
                            "url": "https://example.test/image.png"
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
    let main_llm = plan
        .nodes
        .get_mut("node-llm")
        .expect("main llm node should exist");
    main_llm.config["visible_internal_llm_tools"][0]["tool_name"] = json!("image_llm");
    main_llm.config["visible_internal_llm_tools"][0]["connector_id"] = json!("image_llm");
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

    let outcome = start_flow_debug_run(
        &plan,
        &json!({
            "node-start": {
                "query": format!("看{} 看一下这幅图内容是什么", relative_image_path),
                "history": []
            }
        }),
        &invoker,
    )
    .await
    .unwrap();

    let main_trace = outcome
        .node_traces
        .iter()
        .find(|trace| trace.node_id == "node-llm")
        .expect("main llm trace should exist");
    let route_events = main_trace.debug_payload["visible_internal_llm_tool_events"]
        .as_array()
        .expect("main debug payload should include route events");
    assert_eq!(
        route_events[0]["arguments"]["media"],
        json!([
            {
                "kind": "image",
                "source": "workspace_path",
                "path": image_path.to_string_lossy(),
                "media_type": "image/png",
                "custom_note": "keep-me"
            },
            {
                "kind": "image",
                "source": "url",
                "url": "https://example.test/image.png"
            }
        ])
    );
    let persisted_main_payload = serde_json::to_string(&json!([
        main_trace.output_payload,
        main_trace.debug_payload
    ]))
    .expect("trace payload should serialize");
    assert!(persisted_main_payload.contains("keep-me"));
    assert!(persisted_main_payload.contains("media_type"));
    assert!(persisted_main_payload.contains(image_path.to_string_lossy().as_ref()));

    let captured = captured_inputs
        .lock()
        .expect("captured inputs mutex poisoned")
        .clone();
    assert_eq!(captured.len(), 3);
    assert!(captured[1].tools.is_empty());
    let media_blocks = captured[1].messages[0]
        .content_blocks
        .as_ref()
        .expect("mounted image LLM should receive media content blocks")
        .as_array()
        .expect("content blocks should be an array");
    assert!(media_blocks.iter().any(|block| {
        block["type"] == json!("image_url")
            && block["image_url"]["url"]
                .as_str()
                .is_some_and(|url| url.starts_with("data:image/png;base64,"))
    }));
}

#[tokio::test]
async fn visible_internal_image_llm_tool_schema_does_not_synthesize_media_contract() {
    let (invoker, captured_inputs) =
        sequential_tool_invoker(vec![final_llm_response("main-after")]);
    let mut plan = visible_internal_llm_tool_plan();
    let main_llm = plan
        .nodes
        .get_mut("node-llm")
        .expect("main llm node should exist");
    main_llm.config["visible_internal_llm_tools"][0]["tool_name"] = json!("image_llm");
    main_llm.config["visible_internal_llm_tools"][0]["connector_id"] = json!("image_llm");
    main_llm.config["visible_internal_llm_tools"][0]["input_schema"] = json!({
        "type": "object",
        "properties": {
            "task": { "type": "string" }
        },
        "required": ["task"]
    });

    start_flow_debug_run(
        &plan,
        &json!({
            "node-start": {
                "query": "看uploads/image_aionui_1781014667000.png 看一下这幅图内容是什么",
                "history": []
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
    assert_eq!(captured.len(), 1);
    let image_tool_schema = captured[0]
        .tools
        .iter()
        .find(|tool| tool["function"]["name"] == json!("image_llm"))
        .map(|tool| &tool["function"]["parameters"])
        .expect("image_llm schema should be registered");
    assert_eq!(
        image_tool_schema,
        &json!({
            "type": "object",
            "properties": {
                "task": { "type": "string" }
            },
            "required": ["task"]
        })
    );
}
