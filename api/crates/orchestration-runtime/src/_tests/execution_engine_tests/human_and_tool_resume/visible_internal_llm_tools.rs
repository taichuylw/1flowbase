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
    let mut plan = visible_internal_llm_tool_plan();
    plan.nodes
        .get_mut("node-llm")
        .expect("main llm node should exist")
        .config["visible_internal_llm_tools"][0]["external_tool_policy"] = json!("inherited");

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
async fn fusion_visible_internal_llm_tool_blocks_external_callback_wait() {
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
        tool_call_response(vec![ProviderToolCall {
            id: "call_bash".to_string(),
            name: "Bash".to_string(),
            arguments: json!({ "command": "pwd" }),
            provider_metadata: json!({}),
        }]),
        final_llm_response("main-after"),
    ]);
    let mut plan = visible_internal_llm_tool_plan();
    let main_llm = plan
        .nodes
        .get_mut("node-llm")
        .expect("main llm node should exist");
    main_llm.config["visible_internal_llm_tools"][0]["tool_mode"] = json!("fusion");
    main_llm.config["visible_internal_llm_tools"][0]["external_tool_policy"] = json!("forbidden");
    main_llm.config["visible_internal_llm_tools"][0]["external_callback_policy"] =
        json!("forbidden");
    main_llm.config["visible_internal_llm_tools"][0]["execution_mode"] =
        json!("bounded_parallel_panel");

    let outcome = start_flow_debug_run(
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
        &invoker,
    )
    .await
    .unwrap();

    assert!(
        matches!(outcome.stop_reason, ExecutionStopReason::Completed),
        "fusion mode should return an internal tool error to the main LLM instead of waiting, got {:?}",
        outcome.stop_reason
    );
    assert!(outcome.checkpoint_snapshot.is_none());
    assert_eq!(
        outcome.variable_pool["node-answer"]["answer"],
        json!("main-before main-after")
    );

    let captured = captured_inputs
        .lock()
        .expect("captured inputs mutex poisoned")
        .clone();
    assert_eq!(captured.len(), 3);
    assert!(
        captured[1].tools.is_empty(),
        "fusion mounted LLM must not inherit run-context tools, got {:?}",
        captured[1].tools
    );
    let main_tool_result = captured[2]
        .messages
        .iter()
        .find(|message| {
            message.role == ProviderMessageRole::Tool
                && message.tool_call_id.as_deref() == Some("call_visible")
        })
        .expect("main LLM recall should include the fusion callback rejection");
    assert!(main_tool_result
        .content
        .contains("external callback is forbidden"));
}

#[tokio::test]
async fn fusion_visible_internal_llm_tool_executes_panel_llms_in_bounded_parallel() {
    let invoker = FusionPanelTimingInvoker::default();
    let max_panel_inflight = invoker.max_panel_inflight.clone();
    let plan = fusion_panel_plan();

    let outcome = start_flow_debug_run(
        &plan,
        &json!({
            "node-start": {
                "query": "compare panel answers",
                "history": []
            }
        }),
        &invoker,
    )
    .await
    .unwrap();

    assert!(
        matches!(outcome.stop_reason, ExecutionStopReason::Completed),
        "expected completed fusion panel run, got {:?}",
        outcome.stop_reason
    );
    assert_eq!(
        max_panel_inflight.load(std::sync::atomic::Ordering::SeqCst),
        2,
        "fusion panel LLMs should overlap within the bounded parallel executor"
    );
    assert_eq!(
        outcome.variable_pool["node-answer"]["answer"],
        json!("main-before judge-result main-after")
    );

    let captured = invoker
        .captured_inputs
        .lock()
        .expect("captured inputs mutex poisoned")
        .clone();
    assert_eq!(captured.len(), 5);
    let main_recall = captured
        .last()
        .expect("main recall should be the final provider call");
    let tool_result = main_recall
        .messages
        .iter()
        .find(|message| {
            message.role == ProviderMessageRole::Tool
                && message.tool_call_id.as_deref() == Some("call_visible")
        })
        .expect("main LLM recall should include the fusion judge result");
    assert_eq!(tool_result.content, "judge-result ");
}

#[tokio::test]
async fn fusion_visible_internal_llm_tool_executes_direct_panel_roots_in_bounded_parallel() {
    let invoker = FusionPanelTimingInvoker::default();
    let max_panel_inflight = invoker.max_panel_inflight.clone();
    let plan = direct_fusion_panel_plan();

    let outcome = start_flow_debug_run(
        &plan,
        &json!({
            "node-start": {
                "query": "compare panel answers",
                "history": []
            }
        }),
        &invoker,
    )
    .await
    .unwrap();

    assert!(
        matches!(outcome.stop_reason, ExecutionStopReason::Completed),
        "expected completed direct fusion panel run, got {:?}",
        outcome.stop_reason
    );
    assert_eq!(
        max_panel_inflight.load(std::sync::atomic::Ordering::SeqCst),
        2,
        "direct fusion panel roots should overlap within the bounded parallel executor"
    );
    assert_eq!(
        outcome.variable_pool["node-answer"]["answer"],
        json!("main-before judge-result main-after")
    );
}

#[tokio::test]
async fn visible_internal_llm_tool_branch_canonicalizes_inherited_tool_name_case() {
    let (waiting_invoker, _waiting_inputs) = sequential_tool_invoker(vec![
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
            name: "bash".to_string(),
            arguments: json!({ "command": "pwd" }),
            provider_metadata: json!({}),
        }]),
    ]);
    let mut plan = visible_internal_llm_tool_plan();
    plan.nodes
        .get_mut("node-llm")
        .expect("main llm node should exist")
        .config["visible_internal_llm_tools"][0]["external_tool_policy"] = json!("inherited");

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
            assert_eq!(
                pending.request_payload["tool_calls"][0]["name"],
                json!("Bash")
            );
        }
        other => panic!("expected mounted llm external tool callback wait, got {other:?}"),
    }
    let main_wait_trace = waiting
        .node_traces
        .iter()
        .find(|trace| trace.node_id == "node-llm")
        .expect("main llm waiting trace should exist");
    assert_eq!(
        main_wait_trace.debug_payload["visible_internal_llm_tool_events"][1]["request_payload"]
            ["tool_calls"][0]["name"],
        json!("Bash")
    );
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
    assert_eq!(
        main_tool_names,
        vec!["Bash", "Read", "inspect_visible_context"]
    );
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
    let resumed_main_tool_names = captured[2]
        .tools
        .iter()
        .filter_map(|tool| tool["function"]["name"].as_str())
        .collect::<Vec<_>>();
    assert_eq!(resumed_main_tool_names, vec!["Bash", "Read"]);
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
async fn visible_internal_image_llm_tool_ignores_repeated_media_call_when_external_tool_waits() {
    let media_dir = std::env::current_dir()
        .expect("test current dir should be available")
        .join("target")
        .join("visible-internal-media");
    tokio::fs::create_dir_all(&media_dir)
        .await
        .expect("test media dir should be created");
    let image_path = media_dir.join("mixed-repeat.png");
    tokio::fs::write(&image_path, b"image")
        .await
        .expect("test image should be written");
    let relative_image_path = "target/visible-internal-media/mixed-repeat.png";

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
        tool_call_response(vec![
            ProviderToolCall {
                id: "call_bash".to_string(),
                name: "Bash".to_string(),
                arguments: json!({ "command": "rg Navigation web/app/src" }),
                provider_metadata: json!({}),
            },
            ProviderToolCall {
                id: "call_visible_again".to_string(),
                name: "image_llm".to_string(),
                arguments: json!({
                    "task": "再描述一次图片中的导航栏",
                    "media": [
                        {
                            "kind": "image",
                            "source": "workspace_path",
                            "path": relative_image_path
                        }
                    ]
                }),
                provider_metadata: json!({}),
            },
        ]),
    ]);
    let mut plan = visible_internal_llm_tool_plan();
    let main_llm = plan
        .nodes
        .get_mut("node-llm")
        .expect("main llm node should exist");
    main_llm.config["visible_internal_llm_tools"][0]["tool_name"] = json!("image_llm");
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

    let outcome = start_flow_debug_run(
        &plan,
        &json!({
            "node-start": {
                "query": format!("看{} 然后继续查代码", relative_image_path),
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
        &invoker,
    )
    .await
    .unwrap();

    match outcome.stop_reason {
        ExecutionStopReason::WaitingCallback(ref pending) => {
            assert_eq!(pending.node_id, "node-llm");
            assert_eq!(pending.callback_kind, "llm_tool_calls");
            let pending_tool_names = pending.request_payload["tool_calls"]
                .as_array()
                .expect("pending request should include tool calls")
                .iter()
                .filter_map(|tool_call| tool_call["name"].as_str())
                .collect::<Vec<_>>();
            assert_eq!(pending_tool_names, vec!["Bash"]);
        }
        other => panic!("expected main llm external tool callback wait, got {other:?}"),
    }

    let captured = captured_inputs
        .lock()
        .expect("captured inputs mutex poisoned")
        .clone();
    assert_eq!(captured.len(), 3);
    let resumed_main_tool_names = captured[2]
        .tools
        .iter()
        .filter_map(|tool| tool["function"]["name"].as_str())
        .collect::<Vec<_>>();
    assert_eq!(resumed_main_tool_names, vec!["Bash"]);
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

#[tokio::test]
async fn visible_internal_llm_tool_default_external_tool_policy_blocks_run_context_tools() {
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

    start_flow_debug_run(
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
        &invoker,
    )
    .await
    .unwrap();

    let captured = captured_inputs
        .lock()
        .expect("captured inputs mutex poisoned")
        .clone();
    assert_eq!(captured.len(), 3);
    assert!(
        captured[1].tools.is_empty(),
        "mounted LLM without external_tool_policy must default to forbidden and receive no run-context tools, got {:?}",
        captured[1].tools
    );
    let resumed_main_tool_names = captured[2]
        .tools
        .iter()
        .filter_map(|tool| tool["function"]["name"].as_str())
        .collect::<Vec<_>>();
    assert!(resumed_main_tool_names.contains(&"Bash"));
}

#[tokio::test]
async fn visible_internal_llm_tool_inherited_policy_keeps_run_context_tools_with_media() {
    let media_dir = std::env::current_dir()
        .expect("test current dir should be available")
        .join("target")
        .join("visible-internal-media");
    tokio::fs::create_dir_all(&media_dir)
        .await
        .expect("test media dir should be created");
    let image_path = media_dir.join("inherited-policy.png");
    tokio::fs::write(&image_path, b"image")
        .await
        .expect("test image should be written");
    let relative_image_path = "target/visible-internal-media/inherited-policy.png";

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
    main_llm.config["visible_internal_llm_tools"][0]["external_tool_policy"] = json!("inherited");
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
    let mounted_tool_names = mounted_input
        .tools
        .iter()
        .filter_map(|tool| tool["function"]["name"].as_str())
        .collect::<Vec<_>>();
    assert!(
        mounted_tool_names.contains(&"Bash"),
        "mounted LLM with external_tool_policy inherited must receive run-context tools even with media, got {mounted_tool_names:?}"
    );
    let media_blocks = mounted_input.messages[0]
        .content_blocks
        .as_ref()
        .expect("mounted LLM should still receive media content blocks")
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
async fn visible_internal_llm_tool_mixed_round_runs_internal_inline_and_waits_for_external() {
    let (waiting_invoker, waiting_inputs) = sequential_tool_invoker(vec![
        ProviderInvocationResult {
            final_content: Some("main-before ".to_string()),
            tool_calls: vec![
                ProviderToolCall {
                    id: "call_visible".to_string(),
                    name: "inspect_visible_context".to_string(),
                    arguments: json!({ "query": "image?" }),
                    provider_metadata: json!({}),
                },
                ProviderToolCall {
                    id: "call_bash".to_string(),
                    name: "Bash".to_string(),
                    arguments: json!({ "command": "ls uploads" }),
                    provider_metadata: json!({}),
                },
            ],
            finish_reason: Some(ProviderFinishReason::ToolCall),
            ..ProviderInvocationResult::default()
        },
        final_llm_response("mounted-visible "),
    ]);
    let plan = visible_internal_llm_tool_plan();

    let waiting = start_flow_debug_run(
        &plan,
        &json!({
            "node-start": {
                "query": "describe the picture then inspect the repo",
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
            assert_eq!(pending.node_id, "node-llm");
            assert_eq!(pending.callback_kind, "llm_tool_calls");
            let pending_tool_names = pending.request_payload["tool_calls"]
                .as_array()
                .expect("mixed round waiting payload should carry tool calls")
                .iter()
                .filter_map(|tool_call| tool_call["name"].as_str())
                .collect::<Vec<_>>();
            assert_eq!(
                pending_tool_names,
                vec!["Bash"],
                "internal tool call must not leak into the client-facing waiting payload"
            );
        }
        other => panic!("expected main llm external tool callback wait, got {other:?}"),
    }

    let captured_waiting = waiting_inputs
        .lock()
        .expect("captured inputs mutex poisoned")
        .clone();
    assert_eq!(
        captured_waiting.len(),
        2,
        "mixed round should execute the mounted branch inline before waiting"
    );

    let checkpoint = waiting
        .checkpoint_snapshot
        .clone()
        .expect("mixed round should checkpoint while waiting for the client tool");

    let (resume_invoker, resumed_inputs) =
        sequential_tool_invoker(vec![final_llm_response("main-after")]);
    let resumed = resume_flow_debug_run(
        &plan,
        &checkpoint,
        "node-llm",
        &json!({
            "tool_results": [
                {
                    "tool_call_id": "call_bash",
                    "content": "image-1.png"
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
        json!("main-before mounted-visible main-after")
    );

    let captured_resumed = resumed_inputs
        .lock()
        .expect("captured inputs mutex poisoned")
        .clone();
    assert_eq!(captured_resumed.len(), 1);
    let recall_messages = &captured_resumed[0].messages;
    let internal_result = recall_messages
        .iter()
        .find(|message| {
            message.role == ProviderMessageRole::Tool
                && message.tool_call_id.as_deref() == Some("call_visible")
        })
        .expect("main llm recall should include the hidden internal tool result");
    assert_eq!(internal_result.content, "mounted-visible ");
    let external_result = recall_messages
        .iter()
        .find(|message| {
            message.role == ProviderMessageRole::Tool
                && message.tool_call_id.as_deref() == Some("call_bash")
        })
        .expect("main llm recall should include the client tool result");
    assert_eq!(external_result.content, "image-1.png");
}

#[derive(Clone, Default)]
struct FusionPanelTimingInvoker {
    captured_inputs: Arc<Mutex<Vec<ProviderInvocationInput>>>,
    current_panel_inflight: Arc<std::sync::atomic::AtomicUsize>,
    max_panel_inflight: Arc<std::sync::atomic::AtomicUsize>,
}

#[async_trait]
impl ProviderInvoker for FusionPanelTimingInvoker {
    async fn invoke_llm(
        &self,
        _runtime: &CompiledLlmRuntime,
        input: ProviderInvocationInput,
    ) -> Result<ProviderInvocationOutput> {
        self.captured_inputs
            .lock()
            .expect("captured inputs mutex poisoned")
            .push(input.clone());

        let prompt_text = input
            .messages
            .iter()
            .map(|message| message.content.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        let has_internal_tool = input
            .tools
            .iter()
            .any(|tool| tool["function"]["name"] == json!("inspect_visible_context"));
        let has_internal_tool_result = input.messages.iter().any(|message| {
            message.role == ProviderMessageRole::Tool
                && message.tool_call_id.as_deref() == Some("call_visible")
        });

        if has_internal_tool && !has_internal_tool_result {
            return Ok(provider_output(ProviderInvocationResult {
                final_content: Some("main-before ".to_string()),
                tool_calls: vec![ProviderToolCall {
                    id: "call_visible".to_string(),
                    name: "inspect_visible_context".to_string(),
                    arguments: json!({ "query": "compare panel answers" }),
                    provider_metadata: json!({}),
                }],
                finish_reason: Some(ProviderFinishReason::ToolCall),
                ..ProviderInvocationResult::default()
            }));
        }

        if prompt_text.contains("Panel A") || prompt_text.contains("Panel B") {
            let current = self
                .current_panel_inflight
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
                + 1;
            self.max_panel_inflight
                .fetch_max(current, std::sync::atomic::Ordering::SeqCst);
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            self.current_panel_inflight
                .fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
            let content = if prompt_text.contains("Panel A") {
                "panel-a "
            } else {
                "panel-b "
            };
            return Ok(provider_output(final_llm_response(content)));
        }

        if prompt_text.contains("Judge") {
            return Ok(provider_output(final_llm_response("judge-result ")));
        }

        Ok(provider_output(final_llm_response("main-after")))
    }
}

#[async_trait]
impl CapabilityInvoker for FusionPanelTimingInvoker {
    async fn invoke_capability_node(
        &self,
        _runtime: &CompiledPluginRuntime,
        _config_payload: Value,
        _input_payload: Value,
    ) -> Result<CapabilityInvocationOutput> {
        unreachable!("fusion panel timing test does not execute capability nodes")
    }
}

#[async_trait]
impl CodeInvoker for FusionPanelTimingInvoker {
    async fn invoke_code_node(
        &self,
        _runtime: &CompiledCodeRuntime,
        _config_payload: Value,
        _input_payload: Value,
    ) -> Result<CodeInvocationOutput> {
        unreachable!("fusion panel timing test does not execute code nodes")
    }
}

fn fusion_panel_plan() -> CompiledPlan {
    let mut plan = llm_answer_plan();
    plan.topological_order = vec![
        "node-start".to_string(),
        "node-llm".to_string(),
        "node-panel-seed".to_string(),
        "node-panel-a".to_string(),
        "node-panel-b".to_string(),
        "node-judge".to_string(),
        "node-tool-result".to_string(),
        "node-answer".to_string(),
    ];
    plan.edges = vec![
        CompiledEdge {
            edge_id: "edge-start-llm".to_string(),
            source: "node-start".to_string(),
            target: "node-llm".to_string(),
            source_handle: None,
            target_handle: None,
        },
        CompiledEdge {
            edge_id: "edge-llm-answer".to_string(),
            source: "node-llm".to_string(),
            target: "node-answer".to_string(),
            source_handle: None,
            target_handle: None,
        },
        CompiledEdge {
            edge_id: "edge-llm-fusion-tool".to_string(),
            source: "node-llm".to_string(),
            target: "node-panel-seed".to_string(),
            source_handle: Some("visible_internal_llm_tool:inspect_visible_context".to_string()),
            target_handle: None,
        },
        CompiledEdge {
            edge_id: "edge-seed-panel-a".to_string(),
            source: "node-panel-seed".to_string(),
            target: "node-panel-a".to_string(),
            source_handle: None,
            target_handle: None,
        },
        CompiledEdge {
            edge_id: "edge-seed-panel-b".to_string(),
            source: "node-panel-seed".to_string(),
            target: "node-panel-b".to_string(),
            source_handle: None,
            target_handle: None,
        },
        CompiledEdge {
            edge_id: "edge-panel-a-judge".to_string(),
            source: "node-panel-a".to_string(),
            target: "node-judge".to_string(),
            source_handle: None,
            target_handle: None,
        },
        CompiledEdge {
            edge_id: "edge-panel-b-judge".to_string(),
            source: "node-panel-b".to_string(),
            target: "node-judge".to_string(),
            source_handle: None,
            target_handle: None,
        },
        CompiledEdge {
            edge_id: "edge-judge-tool-result".to_string(),
            source: "node-judge".to_string(),
            target: "node-tool-result".to_string(),
            source_handle: None,
            target_handle: None,
        },
    ];

    let main_llm = plan
        .nodes
        .get_mut("node-llm")
        .expect("main llm node should exist");
    main_llm.downstream_node_ids = vec!["node-answer".to_string(), "node-panel-seed".to_string()];
    main_llm.config["visible_internal_llm_tools_enabled"] = json!(true);
    main_llm.config["visible_internal_llm_tools"] = json!([
        {
            "type": "visible_internal_llm_tool",
            "tool_name": "inspect_visible_context",
            "connector_id": "inspect_visible_context",
            "tool_mode": "fusion",
            "internal_llm_node_policy": "allowed",
            "external_tool_policy": "forbidden",
            "external_callback_policy": "forbidden",
            "execution_mode": "bounded_parallel_panel",
            "description": "Compare panel model answers",
            "target_node_id": "node-panel-seed",
            "input_schema": {
                "type": "object",
                "properties": {
                    "query": { "type": "string" }
                }
            }
        }
    ]);

    plan.nodes.insert(
        "node-panel-seed".to_string(),
        CompiledNode {
            node_id: "node-panel-seed".to_string(),
            node_type: "template_transform".to_string(),
            alias: "Panel Seed".to_string(),
            container_id: None,
            dependency_node_ids: vec!["node-llm".to_string()],
            downstream_node_ids: vec!["node-panel-a".to_string(), "node-panel-b".to_string()],
            bindings: BTreeMap::from([(
                "template".to_string(),
                CompiledBinding {
                    kind: "templated_text".to_string(),
                    selector_paths: vec![vec![
                        "visible_internal_llm_tool".to_string(),
                        "arguments".to_string(),
                        "query".to_string(),
                    ]],
                    raw_value: json!("{{ visible_internal_llm_tool.arguments.query }}"),
                },
            )]),
            outputs: vec![CompiledOutput {
                key: "text".to_string(),
                title: "Panel Seed".to_string(),
                value_type: "string".to_string(),
                selector: Vec::new(),
                json_schema: None,
            }],
            config: json!({}),
            plugin_runtime: None,
            llm_runtime: None,
            code_runtime: None,
        },
    );
    plan.nodes.insert(
        "node-panel-a".to_string(),
        fusion_panel_llm_node("node-panel-a", "Panel A"),
    );
    plan.nodes.insert(
        "node-panel-b".to_string(),
        fusion_panel_llm_node("node-panel-b", "Panel B"),
    );
    plan.nodes.insert(
        "node-judge".to_string(),
        CompiledNode {
            node_id: "node-judge".to_string(),
            node_type: "llm".to_string(),
            alias: "Judge LLM".to_string(),
            container_id: None,
            dependency_node_ids: vec!["node-panel-a".to_string(), "node-panel-b".to_string()],
            downstream_node_ids: vec!["node-tool-result".to_string()],
            bindings: BTreeMap::from([(
                "prompt_messages".to_string(),
                CompiledBinding {
                    kind: "prompt_messages".to_string(),
                    selector_paths: vec![
                        vec!["node-panel-a".to_string(), "text".to_string()],
                        vec!["node-panel-b".to_string(), "text".to_string()],
                    ],
                    raw_value: json!([
                        {
                            "id": "judge-user",
                            "role": "user",
                            "content": {
                                "kind": "templated_text",
                                "value": "Judge {{ node-panel-a.text }} / {{ node-panel-b.text }}"
                            }
                        }
                    ]),
                },
            )]),
            outputs: vec![CompiledOutput {
                key: "text".to_string(),
                title: "Judge Output".to_string(),
                value_type: "string".to_string(),
                selector: Vec::new(),
                json_schema: None,
            }],
            config: json!({
                "model_provider": {
                    "provider_code": "fixture_provider",
                    "model_id": "gpt-5.4-mini"
                }
            }),
            plugin_runtime: None,
            llm_runtime: Some(CompiledLlmRuntime {
                provider_instance_id: "provider-ready".to_string(),
                provider_code: "fixture_provider".to_string(),
                protocol: "openai_compatible".to_string(),
                model: "gpt-5.4-mini".to_string(),
                routing: None,
            }),
            code_runtime: None,
        },
    );
    plan.nodes.insert(
        "node-tool-result".to_string(),
        CompiledNode {
            node_id: "node-tool-result".to_string(),
            node_type: "tool_result".to_string(),
            alias: "Tool Result".to_string(),
            container_id: None,
            dependency_node_ids: vec!["node-judge".to_string()],
            downstream_node_ids: Vec::new(),
            bindings: BTreeMap::from([(
                "result_template".to_string(),
                CompiledBinding {
                    kind: "templated_text".to_string(),
                    selector_paths: vec![vec!["node-judge".to_string(), "text".to_string()]],
                    raw_value: json!("{{ node-judge.text }}"),
                },
            )]),
            outputs: vec![CompiledOutput {
                key: "result".to_string(),
                title: "Tool Result".to_string(),
                value_type: "string".to_string(),
                selector: Vec::new(),
                json_schema: None,
            }],
            config: json!({}),
            plugin_runtime: None,
            llm_runtime: None,
            code_runtime: None,
        },
    );
    plan
}

fn direct_fusion_panel_plan() -> CompiledPlan {
    let mut plan = fusion_panel_plan();
    plan.topological_order = vec![
        "node-start".to_string(),
        "node-llm".to_string(),
        "node-panel-a".to_string(),
        "node-panel-b".to_string(),
        "node-judge".to_string(),
        "node-tool-result".to_string(),
        "node-answer".to_string(),
    ];
    plan.edges = vec![
        CompiledEdge {
            edge_id: "edge-start-llm".to_string(),
            source: "node-start".to_string(),
            target: "node-llm".to_string(),
            source_handle: None,
            target_handle: None,
        },
        CompiledEdge {
            edge_id: "edge-llm-answer".to_string(),
            source: "node-llm".to_string(),
            target: "node-answer".to_string(),
            source_handle: None,
            target_handle: None,
        },
        CompiledEdge {
            edge_id: "edge-llm-panel-a".to_string(),
            source: "node-llm".to_string(),
            target: "node-panel-a".to_string(),
            source_handle: Some("visible_internal_llm_tool:inspect_visible_context".to_string()),
            target_handle: None,
        },
        CompiledEdge {
            edge_id: "edge-llm-panel-b".to_string(),
            source: "node-llm".to_string(),
            target: "node-panel-b".to_string(),
            source_handle: Some("visible_internal_llm_tool:inspect_visible_context".to_string()),
            target_handle: None,
        },
        CompiledEdge {
            edge_id: "edge-panel-a-judge".to_string(),
            source: "node-panel-a".to_string(),
            target: "node-judge".to_string(),
            source_handle: None,
            target_handle: None,
        },
        CompiledEdge {
            edge_id: "edge-panel-b-judge".to_string(),
            source: "node-panel-b".to_string(),
            target: "node-judge".to_string(),
            source_handle: None,
            target_handle: None,
        },
        CompiledEdge {
            edge_id: "edge-judge-tool-result".to_string(),
            source: "node-judge".to_string(),
            target: "node-tool-result".to_string(),
            source_handle: None,
            target_handle: None,
        },
    ];
    plan.nodes.remove("node-panel-seed");
    let main_llm = plan
        .nodes
        .get_mut("node-llm")
        .expect("main llm node should exist");
    main_llm.downstream_node_ids = vec![
        "node-answer".to_string(),
        "node-panel-a".to_string(),
        "node-panel-b".to_string(),
    ];
    main_llm.config["visible_internal_llm_tools"][0]["target_node_id"] = json!("node-panel-a");
    main_llm.config["visible_internal_llm_tools"][0]["target_node_ids"] =
        json!(["node-panel-a", "node-panel-b"]);
    for node_id in ["node-panel-a", "node-panel-b"] {
        let panel_node = plan
            .nodes
            .get_mut(node_id)
            .expect("panel node should exist");
        panel_node.dependency_node_ids = vec!["node-llm".to_string()];
    }
    plan
}

fn fusion_panel_llm_node(node_id: &str, prompt_prefix: &str) -> CompiledNode {
    CompiledNode {
        node_id: node_id.to_string(),
        node_type: "llm".to_string(),
        alias: prompt_prefix.to_string(),
        container_id: None,
        dependency_node_ids: vec!["node-panel-seed".to_string()],
        downstream_node_ids: vec!["node-judge".to_string()],
        bindings: BTreeMap::from([(
            "prompt_messages".to_string(),
            CompiledBinding {
                kind: "prompt_messages".to_string(),
                selector_paths: vec![vec![
                    "visible_internal_llm_tool".to_string(),
                    "arguments".to_string(),
                    "query".to_string(),
                ]],
                raw_value: json!([
                    {
                        "id": format!("{node_id}-user"),
                        "role": "user",
                        "content": {
                            "kind": "templated_text",
                            "value": format!("{prompt_prefix}: {{{{ visible_internal_llm_tool.arguments.query }}}}")
                        }
                    }
                ]),
            },
        )]),
        outputs: vec![CompiledOutput {
            key: "text".to_string(),
            title: "Panel Output".to_string(),
            value_type: "string".to_string(),
            selector: Vec::new(),
            json_schema: None,
        }],
        config: json!({
            "model_provider": {
                "provider_code": "fixture_provider",
                "model_id": "gpt-5.4-mini"
            }
        }),
        plugin_runtime: None,
        llm_runtime: Some(CompiledLlmRuntime {
            provider_instance_id: "provider-ready".to_string(),
            provider_code: "fixture_provider".to_string(),
            protocol: "openai_compatible".to_string(),
            model: "gpt-5.4-mini".to_string(),
            routing: None,
        }),
        code_runtime: None,
    }
}
