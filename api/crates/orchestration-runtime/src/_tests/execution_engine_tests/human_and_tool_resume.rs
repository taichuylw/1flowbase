use super::*;

#[tokio::test]
async fn start_flow_debug_run_waits_for_human_input() {
    let outcome = start_flow_debug_run(
        &base_plan(),
        &json!({
            "node-start": { "query": "请总结退款政策" }
        }),
        &successful_invoker(),
    )
    .await
    .unwrap();

    match outcome.stop_reason {
        ExecutionStopReason::WaitingHuman(ref wait) => {
            assert_eq!(wait.node_id, "node-human");
            assert!(wait.prompt.contains("请审核"));
        }
        other => panic!("expected waiting_human, got {other:?}"),
    }

    assert_eq!(outcome.node_traces.len(), 3);
    assert_eq!(outcome.node_traces[1].node_id, "node-llm");
    assert_eq!(
        outcome.node_traces[1].output_payload["text"],
        "echo:gpt-5.4-mini"
    );
    assert_eq!(outcome.node_traces[1].provider_events.len(), 3);
}

#[tokio::test]
async fn resume_flow_debug_run_completes_answer_after_human_input() {
    let waiting = start_flow_debug_run(
        &base_plan(),
        &json!({
            "node-start": { "query": "退款政策" },
            "sys": { "workflow_run_id": "run-1", "conversation_id": "conversation-1" },
            "env": { "ApiBaseUrl": "https://api.example.com" }
        }),
        &successful_invoker(),
    )
    .await
    .unwrap();

    let checkpoint = waiting.checkpoint_snapshot.clone().unwrap();
    let resumed = resume_flow_debug_run(
        &base_plan(),
        &checkpoint,
        "node-human",
        &json!({ "input": "已审核，可继续" }),
        &successful_invoker(),
    )
    .await
    .unwrap();

    assert!(matches!(
        resumed.stop_reason,
        ExecutionStopReason::Completed
    ));
    assert_eq!(
        resumed.variable_pool["node-answer"]["answer"],
        json!("已审核，可继续")
    );
    let answer_trace = resumed
        .node_traces
        .iter()
        .find(|trace| trace.node_id == "node-answer")
        .expect("answer trace should exist");
    assert_eq!(
        answer_trace.output_payload["sys"]["workflow_run_id"],
        json!("run-1")
    );
    assert_eq!(
        answer_trace.output_payload["env"]["ApiBaseUrl"],
        json!("https://api.example.com")
    );
}

#[tokio::test]
async fn resume_flow_debug_run_rejects_non_public_resume_output_keys() {
    let waiting = start_flow_debug_run(
        &base_plan(),
        &json!({ "node-start": { "query": "退款政策" } }),
        &successful_invoker(),
    )
    .await
    .unwrap();

    let checkpoint = waiting.checkpoint_snapshot.clone().unwrap();
    let error = resume_flow_debug_run(
        &base_plan(),
        &checkpoint,
        "node-human",
        &json!({
            "input": "已审核，可继续",
            "node-llm": { "text": "polluted" }
        }),
        &successful_invoker(),
    )
    .await
    .unwrap_err();

    assert!(error
        .to_string()
        .contains("resume payload key node-llm is not a public output for node-human"));
}

#[tokio::test]
async fn tool_node_emits_waiting_callback_stop_reason() {
    let mut plan = base_plan();
    plan.topological_order = vec!["node-start".to_string(), "node-tool".to_string()];
    plan.nodes.remove("node-llm");
    plan.nodes.remove("node-human");
    plan.nodes.remove("node-answer");
    plan.nodes.insert(
        "node-tool".to_string(),
        CompiledNode {
            node_id: "node-tool".to_string(),
            node_type: "tool".to_string(),
            alias: "Tool".to_string(),
            container_id: None,
            dependency_node_ids: vec!["node-start".to_string()],
            downstream_node_ids: vec![],
            bindings: BTreeMap::new(),
            outputs: vec![CompiledOutput {
                key: "result".to_string(),
                title: "工具输出".to_string(),
                value_type: "json".to_string(),
                selector: Vec::new(),
                json_schema: None,
            }],
            config: json!({ "tool_name": "lookup_order" }),
            plugin_runtime: None,
            llm_runtime: None,
            code_runtime: None,
        },
    );

    let outcome = start_flow_debug_run(
        &plan,
        &json!({ "node-start": { "query": "order_123" } }),
        &successful_invoker(),
    )
    .await
    .unwrap();

    match outcome.stop_reason {
        ExecutionStopReason::WaitingCallback(ref pending) => {
            assert_eq!(pending.node_id, "node-tool");
            assert_eq!(pending.callback_kind, "tool");
        }
        other => panic!("expected waiting_callback, got {other:?}"),
    }
}

#[tokio::test]
async fn llm_tool_calls_pause_current_llm_and_skip_downstream_answer() {
    let (invoker, _captured_inputs) = sequential_tool_invoker(vec![tool_call_response(vec![
        ProviderToolCall {
            id: "call_weather".to_string(),
            name: "lookup_weather".to_string(),
            arguments: json!({ "city": "Shanghai" }),
            provider_metadata: json!({
                "gemini": {
                    "thought_signature": "real-gemini-tool-signature"
                }
            }),
        },
        ProviderToolCall {
            id: "call_unit".to_string(),
            name: "lookup_unit".to_string(),
            arguments: json!({ "scale": "celsius" }),
            provider_metadata: json!({}),
        },
    ])]);

    let outcome = start_flow_debug_run(
        &llm_answer_plan(),
        &json!({ "node-start": { "query": "weather?" } }),
        &invoker,
    )
    .await
    .unwrap();

    match outcome.stop_reason {
        ExecutionStopReason::WaitingCallback(ref pending) => {
            assert_eq!(pending.node_id, "node-llm");
            assert_eq!(pending.callback_kind, "llm_tool_calls");
            assert_eq!(
                pending.request_payload["tool_calls"][0]["id"],
                json!("call_weather")
            );
            assert_eq!(
                pending.request_payload["tool_calls"][0]["call_usage"]["input_tokens"],
                json!(11)
            );
            assert_eq!(
                pending.request_payload["tool_calls"][0]["call_usage"]["input_cache_hit_tokens"],
                json!(5)
            );
            assert_eq!(
                pending.request_payload["tool_calls"][0]["call_usage"]["output_tokens"],
                json!(3)
            );
            assert_eq!(
                pending.request_payload["tool_calls"][0]["call_usage"]["total_tokens"],
                json!(14)
            );
            assert!(pending.request_payload["tool_calls"][0]
                .get("call_output_tokens")
                .is_none());
            assert!(pending.request_payload["tool_calls"][0]
                .get("result_input_tokens")
                .is_none());
            assert!(pending.request_payload["tool_calls"][0]
                .get("token_count_method")
                .is_none());
            assert_eq!(
                pending.request_payload["tool_calls"][1]["call_usage"]["total_tokens"],
                json!(14)
            );
            assert!(pending.request_payload["tool_calls"][0]
                .get("input_cache_hit_tokens")
                .is_none());
            assert_eq!(
                pending.request_payload["tool_calls"][0]["provider_metadata"]["gemini"]
                    ["thought_signature"],
                json!("real-gemini-tool-signature")
            );
            assert_eq!(pending.request_payload["finish_reason"], json!("tool_call"));
            assert_eq!(pending.request_payload["text"], json!("need tools"));
            assert_eq!(
                pending.request_payload["provider_route"]["provider_code"],
                json!("fixture_provider")
            );
            assert_eq!(pending.request_payload["usage"]["total_tokens"], json!(14));
            assert_eq!(pending.request_payload["history"][0]["role"], json!("user"));
            assert_eq!(
                pending.request_payload["history"][0]["content"],
                json!("weather?")
            );
            assert_eq!(
                pending.request_payload["history"][1]["tool_calls"][0]["id"],
                json!("call_weather")
            );
        }
        other => panic!("expected llm tool callback wait, got {other:?}"),
    }

    assert_eq!(outcome.node_traces.len(), 2);
    assert!(outcome
        .node_traces
        .iter()
        .all(|trace| trace.node_id != "node-answer"));
    assert!(outcome.variable_pool.get("node-answer").is_none());

    let llm_trace = outcome
        .node_traces
        .iter()
        .find(|trace| trace.node_id == "node-llm")
        .expect("llm trace should exist");
    assert_eq!(
        llm_trace.debug_payload["llm_rounds"][0]["assistant"]["content"],
        json!("need tools")
    );
    assert_eq!(
        llm_trace.debug_payload["llm_rounds"][0]["assistant"]["tool_calls"][0]["id"],
        json!("call_weather")
    );
    assert_eq!(
        llm_trace.debug_payload["llm_rounds"][0]["assistant"]["tool_calls"][0]["call_usage"]
            ["input_tokens"],
        json!(11)
    );
    assert_eq!(
        llm_trace.debug_payload["llm_rounds"][0]["assistant"]["tool_calls"][0]["call_usage"]
            ["input_cache_hit_tokens"],
        json!(5)
    );
    assert_eq!(
        llm_trace.debug_payload["llm_rounds"][0]["assistant"]["tool_calls"][0]["call_usage"]
            ["output_tokens"],
        json!(3)
    );
    assert_eq!(
        llm_trace.debug_payload["llm_rounds"][0]["usage"]["input_tokens"],
        json!(11)
    );
    assert_eq!(
        llm_trace.debug_payload["llm_rounds"][0]["usage"]["input_cache_hit_tokens"],
        json!(5)
    );
    assert!(
        llm_trace.debug_payload["llm_rounds"][0]["assistant"]["tool_calls"][0]
            .get("call_output_tokens")
            .is_none()
    );
    assert_eq!(
        llm_trace.debug_payload["llm_rounds"][0]["finish_reason"],
        json!("tool_call")
    );
}

#[tokio::test]
async fn llm_tool_call_finish_without_tool_calls_exposes_error_text_to_answer() {
    let (invoker, _captured_inputs) = sequential_tool_invoker(vec![tool_call_response(Vec::new())]);

    let outcome = start_flow_debug_run(
        &llm_answer_plan(),
        &json!({ "node-start": { "query": "weather?" } }),
        &invoker,
    )
    .await
    .unwrap();

    match outcome.stop_reason {
        ExecutionStopReason::Failed(ref failure) => {
            assert_eq!(failure.node_id, "node-llm");
            assert_eq!(
                failure.error_payload["error_code"],
                json!("provider_invalid_response")
            );
            assert!(failure.error_payload["message"]
                .as_str()
                .expect("failure message should be a string")
                .contains("finish_reason=tool_call"));
            assert_eq!(
                outcome.variable_pool["node-answer"]["answer"],
                failure.error_payload["message"]
            );
        }
        other => panic!("expected failed stop reason, got {other:?}"),
    }

    assert!(outcome
        .node_traces
        .iter()
        .any(|trace| trace.node_id == "node-answer"));
}

#[tokio::test]
async fn resume_llm_tool_results_recalls_same_llm_then_enters_downstream() {
    let (waiting_invoker, _waiting_inputs) =
        sequential_tool_invoker(vec![tool_call_response(vec![ProviderToolCall {
            id: "call_weather".to_string(),
            name: "lookup_weather".to_string(),
            arguments: json!({ "city": "Shanghai" }),
            provider_metadata: json!({}),
        }])]);
    let plan = llm_answer_plan();

    let waiting = start_flow_debug_run(
        &plan,
        &json!({ "node-start": { "query": "weather?" } }),
        &waiting_invoker,
    )
    .await
    .unwrap();
    let checkpoint = waiting
        .checkpoint_snapshot
        .clone()
        .expect("llm tool wait should have checkpoint");

    let (resume_invoker, resumed_inputs) =
        sequential_tool_invoker(vec![final_llm_response("weather is clear")]);
    let resumed = resume_flow_debug_run(
        &plan,
        &checkpoint,
        "node-llm",
        &json!({
            "tool_results": [
                {
                    "tool_call_id": "call_weather",
                    "content": "{\"temperature\":21}"
                }
            ]
        }),
        &resume_invoker,
    )
    .await
    .unwrap();

    assert!(matches!(
        resumed.stop_reason,
        ExecutionStopReason::Completed
    ));
    assert_eq!(
        resumed.variable_pool["node-answer"]["answer"],
        json!("weather is clear")
    );
    assert_eq!(resumed.node_traces[0].node_id, "node-llm");
    assert!(resumed
        .node_traces
        .iter()
        .any(|trace| trace.node_id == "node-answer"));

    let captured = resumed_inputs
        .lock()
        .expect("captured inputs mutex poisoned")
        .clone();
    assert_eq!(captured.len(), 1);
    let messages = serde_json::to_value(&captured[0].messages).expect("messages serialize");
    assert_eq!(messages[0]["role"], json!("user"));
    assert_eq!(messages[0]["content"], json!("weather?"));
    assert_eq!(messages[1]["role"], json!("assistant"));
    assert_eq!(messages[1]["tool_calls"][0]["id"], json!("call_weather"));
    assert!(messages[1]["tool_calls"][0]
        .get("call_input_tokens")
        .is_none());
    assert!(messages[1]["tool_calls"][0].get("call_usage").is_none());
    assert!(messages[1]["tool_calls"][0]
        .get("call_cached_input_tokens")
        .is_none());
    assert!(messages[1]["tool_calls"][0]
        .get("call_output_tokens")
        .is_none());
    assert_eq!(messages[2]["role"], json!("tool"));
    assert_eq!(messages[2]["tool_call_id"], json!("call_weather"));
    assert_eq!(messages[2]["name"], json!("lookup_weather"));

    let resumed_llm_trace = resumed
        .node_traces
        .iter()
        .find(|trace| trace.node_id == "node-llm")
        .expect("resumed llm trace should exist");
    assert_eq!(
        resumed_llm_trace.debug_payload["llm_rounds"][0]["assistant"]["tool_calls"][0]["id"],
        json!("call_weather")
    );
    assert_eq!(
        resumed_llm_trace.debug_payload["llm_rounds"][0]["tool_results"][0]["tool_call_id"],
        json!("call_weather")
    );
    assert!(
        resumed_llm_trace.debug_payload["llm_rounds"][0]["tool_results"][0]
            .get("result_input_tokens")
            .is_none()
    );
    assert_eq!(
        resumed_llm_trace.debug_payload["llm_rounds"][0]["tool_results"][0]["result_context_usage"]
            ["input_tokens"],
        json!(20)
    );
    assert_eq!(
        resumed_llm_trace.debug_payload["llm_rounds"][0]["tool_results"][0]["result_context_usage"]
            ["input_cache_hit_tokens"],
        json!(8)
    );
    assert_eq!(
        resumed_llm_trace.debug_payload["llm_rounds"][0]["tool_results"][0]["result_context_usage"]
            ["total_tokens"],
        json!(24)
    );
    assert!(
        resumed_llm_trace.debug_payload["llm_rounds"][0]["tool_results"][0]
            .get("call_output_tokens")
            .is_none()
    );
    assert!(
        resumed_llm_trace.debug_payload["llm_rounds"][0]["tool_results"][0]
            .get("token_count_method")
            .is_none()
    );
    assert!(
        resumed_llm_trace.debug_payload["llm_rounds"][0]["tool_results"][0]
            .get("input_cache_hit_tokens")
            .is_none()
    );
    assert_eq!(
        resumed_llm_trace.debug_payload["llm_rounds"][1]["assistant"]["content"],
        json!("weather is clear")
    );
    assert_eq!(
        resumed_llm_trace.debug_payload["llm_rounds"][1]["usage"]["input_tokens"],
        json!(20)
    );
    assert_eq!(
        resumed_llm_trace.debug_payload["llm_rounds"][1]["usage"]["input_cache_hit_tokens"],
        json!(8)
    );
    assert_eq!(
        resumed_llm_trace.debug_payload["llm_rounds"][1]["finish_reason"],
        json!("stop")
    );
}

#[tokio::test]
async fn resume_llm_tool_results_passes_native_response_cursor_system_and_delta_messages() {
    let mut waiting_response = tool_call_response(vec![ProviderToolCall {
        id: "call_weather".to_string(),
        name: "lookup_weather".to_string(),
        arguments: json!({ "city": "Shanghai" }),
        provider_metadata: json!({}),
    }]);
    waiting_response.response_id = Some("resp_previous".to_string());
    waiting_response.provider_metadata = json!({ "transport": "responses_websocket" });
    let (waiting_invoker, _waiting_inputs) = sequential_tool_invoker(vec![waiting_response]);
    let mut plan = llm_answer_plan();
    let llm = plan
        .nodes
        .get_mut("node-llm")
        .expect("llm node should exist");
    llm.bindings.insert(
        "prompt_messages".to_string(),
        CompiledBinding {
            kind: "prompt_messages".to_string(),
            selector_paths: vec![vec!["node-start".to_string(), "query".to_string()]],
            raw_value: json!([
                {
                    "id": "system-1",
                    "role": "system",
                    "content": {
                        "kind": "templated_text",
                        "value": "Always answer with current tool evidence."
                    }
                },
                {
                    "id": "user-1",
                    "role": "user",
                    "content": {
                        "kind": "templated_text",
                        "value": "{{node-start.query}}"
                    }
                }
            ]),
        },
    );

    let waiting = start_flow_debug_run(
        &plan,
        &json!({ "node-start": { "query": "weather?" } }),
        &waiting_invoker,
    )
    .await
    .unwrap();

    match waiting.stop_reason {
        ExecutionStopReason::WaitingCallback(ref pending) => {
            assert_eq!(
                pending.request_payload["response_id"],
                json!("resp_previous")
            );
        }
        other => panic!("expected llm tool callback wait, got {other:?}"),
    }

    let checkpoint = waiting
        .checkpoint_snapshot
        .clone()
        .expect("llm tool wait should have checkpoint");
    let (resume_invoker, resumed_inputs) =
        sequential_tool_invoker(vec![final_llm_response("weather is clear")]);

    resume_flow_debug_run(
        &plan,
        &checkpoint,
        "node-llm",
        &json!({
            "tool_results": [
                {
                    "tool_call_id": "call_weather",
                    "content": "{\"temperature\":21}"
                }
            ]
        }),
        &resume_invoker,
    )
    .await
    .unwrap();

    let captured = resumed_inputs
        .lock()
        .expect("captured inputs mutex poisoned")
        .clone();
    assert_eq!(captured.len(), 1);
    assert_eq!(
        captured[0].previous_response_id.as_deref(),
        Some("resp_previous")
    );
    assert_eq!(
        captured[0].system.as_deref(),
        Some("Always answer with current tool evidence.")
    );
    assert_eq!(captured[0].messages.len(), 1);
    assert_eq!(captured[0].messages[0].role, ProviderMessageRole::Tool);
    assert_eq!(
        captured[0].messages[0].tool_call_id.as_deref(),
        Some("call_weather")
    );
    assert_eq!(
        captured[0].messages[0].name.as_deref(),
        Some("lookup_weather")
    );
}

#[tokio::test]
async fn resume_llm_tool_results_replays_full_history_after_http_sse_response_cursor() {
    let mut waiting_response = tool_call_response(vec![ProviderToolCall {
        id: "call_weather".to_string(),
        name: "lookup_weather".to_string(),
        arguments: json!({ "city": "Shanghai" }),
        provider_metadata: json!({}),
    }]);
    waiting_response.response_id = Some("resp_from_http_sse".to_string());
    waiting_response.provider_metadata = json!({ "transport": "http_sse" });
    let (waiting_invoker, _waiting_inputs) = sequential_tool_invoker(vec![waiting_response]);
    let plan = llm_answer_plan();

    let waiting = start_flow_debug_run(
        &plan,
        &json!({ "node-start": { "query": "weather?" } }),
        &waiting_invoker,
    )
    .await
    .unwrap();

    match waiting.stop_reason {
        ExecutionStopReason::WaitingCallback(ref pending) => {
            assert_eq!(
                pending.request_payload["response_id"],
                json!("resp_from_http_sse")
            );
            assert_eq!(
                pending.request_payload["provider_metadata"]["transport"],
                json!("http_sse")
            );
        }
        other => panic!("expected llm tool callback wait, got {other:?}"),
    }

    let checkpoint = waiting
        .checkpoint_snapshot
        .clone()
        .expect("llm tool wait should have checkpoint");
    let (resume_invoker, resumed_inputs) =
        sequential_tool_invoker(vec![final_llm_response("weather is clear")]);

    resume_flow_debug_run(
        &plan,
        &checkpoint,
        "node-llm",
        &json!({
            "tool_results": [
                {
                    "tool_call_id": "call_weather",
                    "content": "{\"temperature\":21}"
                }
            ]
        }),
        &resume_invoker,
    )
    .await
    .unwrap();

    let captured = resumed_inputs
        .lock()
        .expect("captured inputs mutex poisoned")
        .clone();
    assert_eq!(captured.len(), 1);
    assert_eq!(captured[0].previous_response_id, None);
    assert_eq!(captured[0].messages.len(), 3);
    assert_eq!(captured[0].messages[0].role, ProviderMessageRole::User);
    assert_eq!(captured[0].messages[0].content, "weather?");
    assert_eq!(captured[0].messages[1].role, ProviderMessageRole::Assistant);
    assert_eq!(
        captured[0].messages[1].tool_calls.as_ref().unwrap()[0]["id"],
        json!("call_weather")
    );
    assert_eq!(captured[0].messages[2].role, ProviderMessageRole::Tool);
    assert_eq!(
        captured[0].messages[2].tool_call_id.as_deref(),
        Some("call_weather")
    );
}

#[tokio::test]
async fn resume_llm_tool_results_preserves_media_content_blocks() {
    let mut waiting_response = tool_call_response(vec![ProviderToolCall {
        id: "call_read_image".to_string(),
        name: "Read".to_string(),
        arguments: json!({ "file_path": "uploads/agent-flow-preview-debug.png" }),
        provider_metadata: json!({}),
    }]);
    waiting_response.response_id = Some("resp_from_http_sse".to_string());
    waiting_response.provider_metadata = json!({ "transport": "http_sse" });
    let (waiting_invoker, _waiting_inputs) = sequential_tool_invoker(vec![waiting_response]);
    let plan = llm_answer_plan();

    let waiting = start_flow_debug_run(
        &plan,
        &json!({ "node-start": { "query": "describe image" } }),
        &waiting_invoker,
    )
    .await
    .unwrap();
    let checkpoint = waiting
        .checkpoint_snapshot
        .clone()
        .expect("llm tool wait should have checkpoint");
    let (resume_invoker, resumed_inputs) =
        sequential_tool_invoker(vec![final_llm_response("image described")]);

    resume_flow_debug_run(
        &plan,
        &checkpoint,
        "node-llm",
        &json!({
            "tool_results": [
                {
                    "tool_call_id": "call_read_image",
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
        }),
        &resume_invoker,
    )
    .await
    .unwrap();

    let captured = resumed_inputs
        .lock()
        .expect("captured inputs mutex poisoned")
        .clone();
    assert_eq!(captured.len(), 1);
    assert_eq!(captured[0].messages.len(), 3);
    assert_eq!(captured[0].messages[2].role, ProviderMessageRole::Tool);
    assert_eq!(
        captured[0].messages[2].tool_call_id.as_deref(),
        Some("call_read_image")
    );
    assert_eq!(
        captured[0].messages[2].content_blocks.as_ref().unwrap()[0]["type"],
        json!("image")
    );
}

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

#[tokio::test]
async fn visible_internal_llm_tool_emits_structured_route_events_in_main_debug_payload() {
    let (invoker, _captured_inputs) = sequential_tool_invoker(vec![
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

    let outcome = start_flow_debug_run(
        &visible_internal_llm_tool_plan(),
        &json!({ "node-start": { "query": "describe the picture", "history": [] } }),
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
        .expect("main debug payload should include visible internal route events");
    assert_eq!(
        route_events[0]["event_type"],
        json!("visible_internal_llm_tool_started")
    );
    assert_eq!(route_events[0]["main_node_id"], json!("node-llm"));
    assert_eq!(route_events[0]["target_node_id"], json!("node-mounted-llm"));
    assert_eq!(
        route_events[0]["tool_name"],
        json!("inspect_visible_context")
    );
    assert_eq!(route_events[0]["tool_call_id"], json!("call_visible"));
    assert!(route_events.iter().any(|event| {
        event["event_type"] == json!("visible_internal_llm_tool_completed")
            && event["target_node_id"] == json!("node-mounted-llm")
            && event["provider_route"]["model"] == json!("gpt-5.4-mini")
    }));
}

#[tokio::test]
async fn visible_internal_llm_tool_recoverable_branch_model_error_returns_hidden_tool_result() {
    let captured_inputs = Arc::new(Mutex::new(Vec::new()));
    let invoker = MountedModelUnsupportedInvoker {
        captured_inputs: captured_inputs.clone(),
    };

    let outcome = start_flow_debug_run(
        &visible_internal_llm_tool_plan(),
        &json!({
            "node-start": {
                "query": "describe the picture",
                "history": [
                    { "role": "user", "content": "describe the picture" }
                ]
            }
        }),
        &invoker,
    )
    .await
    .unwrap();

    assert!(
        matches!(outcome.stop_reason, ExecutionStopReason::Completed),
        "recoverable branch model errors should return a hidden tool result, got {:?}",
        outcome.stop_reason
    );
    let captured = captured_inputs
        .lock()
        .expect("captured inputs mutex poisoned")
        .clone();
    assert_eq!(captured.len(), 3);
    let hidden_tool_result = captured[2]
        .messages
        .iter()
        .find(|message| {
            message.role == ProviderMessageRole::Tool
                && message.tool_call_id.as_deref() == Some("call_visible")
        })
        .expect("main recall should include hidden branch error as tool result");
    assert!(hidden_tool_result
        .content
        .contains("model_multimodal_unsupported"));
}

#[tokio::test]
async fn visible_internal_llm_tool_resume_recoverable_branch_model_error_returns_hidden_tool_result(
) {
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
    let checkpoint = waiting
        .checkpoint_snapshot
        .clone()
        .expect("mounted llm tool wait should have checkpoint");

    let captured_inputs = Arc::new(Mutex::new(Vec::new()));
    let resume_invoker = ResumeMountedModelUnsupportedInvoker {
        captured_inputs: captured_inputs.clone(),
    };
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
        "recoverable branch model errors after callback should return a hidden tool result, got {:?}",
        resumed.stop_reason
    );
    let captured = captured_inputs
        .lock()
        .expect("captured inputs mutex poisoned")
        .clone();
    assert_eq!(captured.len(), 2);
    let hidden_tool_result = captured[1]
        .messages
        .iter()
        .find(|message| {
            message.role == ProviderMessageRole::Tool
                && message.tool_call_id.as_deref() == Some("call_visible")
        })
        .expect("main recall should include hidden branch error as tool result");
    assert!(hidden_tool_result
        .content
        .contains("model_multimodal_unsupported"));
}

struct MountedModelUnsupportedInvoker {
    captured_inputs: Arc<Mutex<Vec<ProviderInvocationInput>>>,
}

#[async_trait]
impl ProviderInvoker for MountedModelUnsupportedInvoker {
    async fn invoke_llm(
        &self,
        _runtime: &CompiledLlmRuntime,
        input: ProviderInvocationInput,
    ) -> Result<ProviderInvocationOutput> {
        let mut captured = self
            .captured_inputs
            .lock()
            .expect("captured inputs mutex poisoned");
        let call_index = captured.len();
        captured.push(input);
        drop(captured);

        match call_index {
            0 => Ok(ProviderInvocationOutput {
                events: vec![ProviderStreamEvent::Finish {
                    reason: ProviderFinishReason::ToolCall,
                }],
                result: ProviderInvocationResult {
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
                first_token_at: None,
                time_to_first_token_ms: None,
            }),
            1 => Err(anyhow::anyhow!("conflict: model_multimodal_unsupported")),
            _ => Ok(ProviderInvocationOutput {
                events: vec![ProviderStreamEvent::Finish {
                    reason: ProviderFinishReason::Stop,
                }],
                result: final_llm_response("main-after"),
                first_token_at: None,
                time_to_first_token_ms: None,
            }),
        }
    }
}

#[async_trait]
impl CapabilityInvoker for MountedModelUnsupportedInvoker {
    async fn invoke_capability_node(
        &self,
        _runtime: &CompiledPluginRuntime,
        _config_payload: Value,
        _input_payload: Value,
    ) -> Result<CapabilityInvocationOutput> {
        unreachable!("visible internal model unsupported test does not execute capability nodes")
    }
}

#[async_trait]
impl CodeInvoker for MountedModelUnsupportedInvoker {
    async fn invoke_code_node(
        &self,
        _runtime: &CompiledCodeRuntime,
        _config_payload: Value,
        _input_payload: Value,
    ) -> Result<CodeInvocationOutput> {
        unreachable!("visible internal model unsupported test does not execute code nodes")
    }
}

struct ResumeMountedModelUnsupportedInvoker {
    captured_inputs: Arc<Mutex<Vec<ProviderInvocationInput>>>,
}

#[async_trait]
impl ProviderInvoker for ResumeMountedModelUnsupportedInvoker {
    async fn invoke_llm(
        &self,
        _runtime: &CompiledLlmRuntime,
        input: ProviderInvocationInput,
    ) -> Result<ProviderInvocationOutput> {
        let mut captured = self
            .captured_inputs
            .lock()
            .expect("captured inputs mutex poisoned");
        let call_index = captured.len();
        captured.push(input);
        drop(captured);

        match call_index {
            0 => Err(anyhow::anyhow!("conflict: model_multimodal_unsupported")),
            _ => Ok(ProviderInvocationOutput {
                events: vec![ProviderStreamEvent::Finish {
                    reason: ProviderFinishReason::Stop,
                }],
                result: final_llm_response("main-after"),
                first_token_at: None,
                time_to_first_token_ms: None,
            }),
        }
    }
}

#[async_trait]
impl CapabilityInvoker for ResumeMountedModelUnsupportedInvoker {
    async fn invoke_capability_node(
        &self,
        _runtime: &CompiledPluginRuntime,
        _config_payload: Value,
        _input_payload: Value,
    ) -> Result<CapabilityInvocationOutput> {
        unreachable!(
            "visible internal model unsupported resume test does not execute capability nodes"
        )
    }
}

#[async_trait]
impl CodeInvoker for ResumeMountedModelUnsupportedInvoker {
    async fn invoke_code_node(
        &self,
        _runtime: &CompiledCodeRuntime,
        _config_payload: Value,
        _input_payload: Value,
    ) -> Result<CodeInvocationOutput> {
        unreachable!("visible internal model unsupported resume test does not execute code nodes")
    }
}

#[tokio::test]
async fn visible_internal_llm_tool_callback_resume_keeps_completed_hidden_tool_results() {
    let mut plan = visible_internal_llm_tool_plan();
    plan.nodes
        .get_mut("node-llm")
        .expect("main llm node should exist")
        .config["visible_internal_llm_tools"]
        .as_array_mut()
        .expect("visible internal tools should be configured")
        .push(json!({
            "type": "visible_internal_llm_tool",
            "tool_name": "inspect_secondary_context",
            "connector_id": "inspect_secondary_context",
            "description": "Inspect secondary context with the mounted LLM",
            "target_node_id": "node-mounted-llm",
            "input_schema": {
                "type": "object",
                "properties": {
                    "query": { "type": "string" }
                }
            }
        }));

    let (waiting_invoker, _waiting_inputs) = sequential_tool_invoker(vec![
        ProviderInvocationResult {
            final_content: Some("main-before ".to_string()),
            tool_calls: vec![
                ProviderToolCall {
                    id: "call_visible".to_string(),
                    name: "inspect_visible_context".to_string(),
                    arguments: json!({ "query": "first image?" }),
                    provider_metadata: json!({}),
                },
                ProviderToolCall {
                    id: "call_secondary".to_string(),
                    name: "inspect_secondary_context".to_string(),
                    arguments: json!({ "query": "second image?" }),
                    provider_metadata: json!({}),
                },
            ],
            finish_reason: Some(ProviderFinishReason::ToolCall),
            ..ProviderInvocationResult::default()
        },
        final_llm_response("first-mounted "),
        tool_call_response(vec![ProviderToolCall {
            id: "call_bash".to_string(),
            name: "Bash".to_string(),
            arguments: json!({ "command": "file tmp/second-image.png" }),
            provider_metadata: json!({}),
        }]),
    ]);

    let waiting = start_flow_debug_run(
        &plan,
        &json!({
            "node-start": {
                "query": "describe the pictures",
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

    let checkpoint = waiting
        .checkpoint_snapshot
        .clone()
        .expect("mounted llm tool wait should have checkpoint");
    match waiting.stop_reason {
        ExecutionStopReason::WaitingCallback(ref pending) => {
            assert_eq!(pending.node_id, "node-mounted-llm");
            assert_eq!(
                pending.request_payload["tool_calls"][0]["id"],
                json!("call_bash")
            );
        }
        other => panic!("expected mounted llm external tool callback wait, got {other:?}"),
    }

    let (resume_invoker, resumed_inputs) = sequential_tool_invoker(vec![
        final_llm_response("second-mounted-after-tool "),
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
                    "content": "tmp/second-image.png: PNG image data"
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
        json!("main-before first-mounted second-mounted-after-tool main-after")
    );

    let captured_resumed = resumed_inputs
        .lock()
        .expect("captured inputs mutex poisoned")
        .clone();
    assert_eq!(captured_resumed.len(), 2);
    let main_recall_messages = &captured_resumed[1].messages;
    let first_hidden_tool_result = main_recall_messages
        .iter()
        .find(|message| {
            message.role == ProviderMessageRole::Tool
                && message.tool_call_id.as_deref() == Some("call_visible")
        })
        .expect("main llm recall should include first hidden tool result");
    assert_eq!(first_hidden_tool_result.content, "first-mounted ");
    let second_hidden_tool_result = main_recall_messages
        .iter()
        .find(|message| {
            message.role == ProviderMessageRole::Tool
                && message.tool_call_id.as_deref() == Some("call_secondary")
        })
        .expect("main llm recall should include second hidden tool result");
    assert_eq!(
        second_hidden_tool_result.content,
        "second-mounted-after-tool "
    );
}

#[tokio::test]
async fn visible_internal_llm_tool_failure_fails_main_llm_run() {
    let failing_internal_result = ProviderInvocationResult {
        final_content: Some("partial mounted output".to_string()),
        finish_reason: Some(ProviderFinishReason::Error),
        ..ProviderInvocationResult::default()
    };
    let (invoker, _captured_inputs) = sequential_tool_invoker(vec![
        ProviderInvocationResult {
            final_content: Some(String::new()),
            tool_calls: vec![ProviderToolCall {
                id: "call_visible".to_string(),
                name: "inspect_visible_context".to_string(),
                arguments: json!({}),
                provider_metadata: json!({}),
            }],
            finish_reason: Some(ProviderFinishReason::ToolCall),
            ..ProviderInvocationResult::default()
        },
        failing_internal_result,
    ]);

    let outcome = start_flow_debug_run(
        &visible_internal_llm_tool_plan(),
        &json!({ "node-start": { "query": "describe the picture" } }),
        &invoker,
    )
    .await
    .unwrap();

    match outcome.stop_reason {
        ExecutionStopReason::Failed(ref failure) => {
            assert_eq!(failure.node_id, "node-llm");
            assert_eq!(
                failure.error_payload["error_code"],
                json!("visible_internal_llm_tool_failed")
            );
            assert_eq!(
                failure.error_payload["target_node_id"],
                json!("node-mounted-llm")
            );
        }
        other => panic!("expected failed visible internal llm tool run, got {other:?}"),
    }
}

#[tokio::test]
async fn external_tool_calls_still_wait_for_client_when_internal_tools_are_configured() {
    let (invoker, captured_inputs) =
        sequential_tool_invoker(vec![tool_call_response(vec![ProviderToolCall {
            id: "call_external".to_string(),
            name: "lookup_weather".to_string(),
            arguments: json!({ "city": "Shanghai" }),
            provider_metadata: json!({}),
        }])]);

    let outcome = start_flow_debug_run(
        &visible_internal_llm_tool_plan(),
        &json!({
            "node-start": {
                "query": "weather?",
                "history": [
                    {
                        "role": "user",
                        "content": "上一轮看过 uploads/image_aionui_1781014667000.png"
                    }
                ],
                "tools": [
                    {
                        "name": "lookup_weather",
                        "description": "Lookup weather",
                        "input_schema": { "type": "object" }
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
            assert_eq!(pending.callback_kind, "llm_tool_calls");
            assert_eq!(
                pending.request_payload["tool_calls"][0]["name"],
                json!("lookup_weather")
            );
        }
        other => panic!("expected external llm tool callback wait, got {other:?}"),
    }

    let captured = captured_inputs
        .lock()
        .expect("captured inputs mutex poisoned")
        .clone();
    let tool_names = captured[0]
        .tools
        .iter()
        .filter_map(|tool| tool["function"]["name"].as_str())
        .collect::<Vec<_>>();
    assert!(tool_names.contains(&"inspect_visible_context"));
    assert!(tool_names.contains(&"lookup_weather"));
}

#[tokio::test]
async fn multi_round_llm_tool_callbacks_keep_previous_round_debug_evidence() {
    let first_call = ProviderToolCall {
        id: "call_weather".to_string(),
        name: "lookup_weather".to_string(),
        arguments: json!({ "city": "Shanghai" }),
        provider_metadata: json!({}),
    };
    let second_call = ProviderToolCall {
        id: "call_time".to_string(),
        name: "lookup_time".to_string(),
        arguments: json!({ "city": "Shanghai" }),
        provider_metadata: json!({}),
    };
    let plan = llm_answer_plan();
    let (waiting_invoker, _waiting_inputs) =
        sequential_tool_invoker(vec![tool_call_response(vec![first_call])]);

    let waiting = start_flow_debug_run(
        &plan,
        &json!({ "node-start": { "query": "weather and time?" } }),
        &waiting_invoker,
    )
    .await
    .unwrap();
    let first_checkpoint = waiting
        .checkpoint_snapshot
        .clone()
        .expect("first tool wait should have checkpoint");

    let (second_wait_invoker, _second_inputs) =
        sequential_tool_invoker(vec![tool_call_response(vec![second_call])]);
    let second_wait = resume_flow_debug_run(
        &plan,
        &first_checkpoint,
        "node-llm",
        &json!({
            "tool_results": [
                {
                    "tool_call_id": "call_weather",
                    "content": "{\"temperature\":21}"
                }
            ]
        }),
        &second_wait_invoker,
    )
    .await
    .unwrap();

    match second_wait.stop_reason {
        ExecutionStopReason::WaitingCallback(ref pending) => {
            assert_eq!(pending.node_id, "node-llm");
            assert_eq!(
                pending.request_payload["tool_calls"][0]["id"],
                json!("call_time")
            );
        }
        other => panic!("expected second llm tool callback wait, got {other:?}"),
    }

    let llm_trace = second_wait
        .node_traces
        .iter()
        .find(|trace| trace.node_id == "node-llm")
        .expect("second wait llm trace should exist");
    assert_eq!(
        llm_trace.debug_payload["llm_rounds"][0]["assistant"]["tool_calls"][0]["id"],
        json!("call_weather")
    );
    assert_eq!(
        llm_trace.debug_payload["llm_rounds"][0]["tool_results"][0]["tool_call_id"],
        json!("call_weather")
    );
    assert_eq!(
        llm_trace.debug_payload["llm_rounds"][1]["assistant"]["tool_calls"][0]["id"],
        json!("call_time")
    );
}

fn visible_internal_llm_tool_plan_with_result() -> CompiledPlan {
    let mut plan = visible_internal_llm_tool_plan();
    let tool_result = plan
        .nodes
        .get_mut("node-tool-result")
        .expect("tool result node should exist");
    tool_result.bindings = BTreeMap::from([(
        "result_template".to_string(),
        CompiledBinding {
            kind: "templated_text".to_string(),
            selector_paths: vec![vec!["node-mounted-llm".to_string(), "text".to_string()]],
            raw_value: json!("tool-result: {{ node-mounted-llm.text }}"),
        },
    )]);

    plan
}

fn visible_internal_llm_tool_plan() -> CompiledPlan {
    let mut plan = llm_answer_plan();
    plan.topological_order = vec![
        "node-start".to_string(),
        "node-llm".to_string(),
        "node-mounted-llm".to_string(),
        "node-tool-result".to_string(),
        "node-answer".to_string(),
    ];
    let main_llm = plan
        .nodes
        .get_mut("node-llm")
        .expect("main llm node should exist");
    main_llm.config["visible_internal_llm_tools_enabled"] = json!(true);
    main_llm.config["visible_internal_llm_tools"] = json!([
        {
            "type": "visible_internal_llm_tool",
            "tool_name": "inspect_visible_context",
            "connector_id": "inspect_visible_context",
            "internal_llm_node_policy": "allowed",
            "description": "Inspect the current user content with a mounted LLM",
            "target_node_id": "node-mounted-llm",
            "input_schema": {
                "type": "object",
                "properties": {
                    "query": { "type": "string" }
                }
            }
        }
    ]);
    main_llm
        .downstream_node_ids
        .push("node-mounted-llm".to_string());

    plan.nodes.insert(
        "node-mounted-llm".to_string(),
        CompiledNode {
            node_id: "node-mounted-llm".to_string(),
            node_type: "llm".to_string(),
            alias: "Mounted LLM".to_string(),
            container_id: None,
            dependency_node_ids: Vec::new(),
            downstream_node_ids: vec!["node-tool-result".to_string()],
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
                            "id": "mounted-user",
                            "role": "user",
                            "content": {
                                "kind": "templated_text",
                                "value": "Inspect {{ visible_internal_llm_tool.arguments.query }}"
                            }
                        }
                    ]),
                },
            )]),
            outputs: vec![CompiledOutput {
                key: "text".to_string(),
                title: "模型输出".to_string(),
                value_type: "string".to_string(),
                selector: Vec::new(),
                json_schema: None,
            }],
            config: json!({
                "model_provider": {
                    "provider_code": "fixture_provider",
                    "model_id": "gpt-5.4-mini"
                },
                "context_policy": {
                    "integration_context": "enabled",
                    "context_selector": ["node-start", "history"]
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
            dependency_node_ids: vec!["node-mounted-llm".to_string()],
            downstream_node_ids: Vec::new(),
            bindings: BTreeMap::from([(
                "result_template".to_string(),
                CompiledBinding {
                    kind: "templated_text".to_string(),
                    selector_paths: vec![vec!["node-mounted-llm".to_string(), "text".to_string()]],
                    raw_value: json!("{{ node-mounted-llm.text }}"),
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
    plan.edges.push(CompiledEdge {
        edge_id: "edge-start-llm".to_string(),
        source: "node-start".to_string(),
        target: "node-llm".to_string(),
        source_handle: None,
        target_handle: None,
    });
    plan.edges.push(CompiledEdge {
        edge_id: "edge-llm-answer".to_string(),
        source: "node-llm".to_string(),
        target: "node-answer".to_string(),
        source_handle: None,
        target_handle: None,
    });
    plan.edges.push(CompiledEdge {
        edge_id: "edge-llm-visible-tool-mounted".to_string(),
        source: "node-llm".to_string(),
        target: "node-mounted-llm".to_string(),
        source_handle: Some("visible_internal_llm_tool:inspect_visible_context".to_string()),
        target_handle: None,
    });
    plan.edges.push(CompiledEdge {
        edge_id: "edge-mounted-tool-result".to_string(),
        source: "node-mounted-llm".to_string(),
        target: "node-tool-result".to_string(),
        source_handle: None,
        target_handle: None,
    });

    plan
}

fn visible_internal_llm_tool_chain_plan() -> CompiledPlan {
    let mut plan = visible_internal_llm_tool_plan();
    plan.topological_order = vec![
        "node-start".to_string(),
        "node-llm".to_string(),
        "node-tool-transform".to_string(),
        "node-mounted-llm".to_string(),
        "node-tool-result".to_string(),
        "node-answer".to_string(),
    ];

    let main_llm = plan
        .nodes
        .get_mut("node-llm")
        .expect("main llm node should exist");
    main_llm.config["visible_internal_llm_tools"][0]["target_node_id"] =
        json!("node-tool-transform");
    main_llm.downstream_node_ids =
        vec!["node-answer".to_string(), "node-tool-transform".to_string()];

    plan.nodes.insert(
        "node-tool-transform".to_string(),
        CompiledNode {
            node_id: "node-tool-transform".to_string(),
            node_type: "template_transform".to_string(),
            alias: "Tool Transform".to_string(),
            container_id: None,
            dependency_node_ids: vec!["node-llm".to_string()],
            downstream_node_ids: vec!["node-mounted-llm".to_string()],
            bindings: BTreeMap::from([(
                "template".to_string(),
                CompiledBinding {
                    kind: "templated_text".to_string(),
                    selector_paths: vec![vec![
                        "visible_internal_llm_tool".to_string(),
                        "arguments".to_string(),
                        "query".to_string(),
                    ]],
                    raw_value: json!("transformed {{ visible_internal_llm_tool.arguments.query }}"),
                },
            )]),
            outputs: vec![CompiledOutput {
                key: "text".to_string(),
                title: "转换结果".to_string(),
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

    let mounted_llm = plan
        .nodes
        .get_mut("node-mounted-llm")
        .expect("mounted llm node should exist");
    mounted_llm.dependency_node_ids = vec!["node-tool-transform".to_string()];
    mounted_llm.bindings = BTreeMap::from([(
        "prompt_messages".to_string(),
        CompiledBinding {
            kind: "prompt_messages".to_string(),
            selector_paths: vec![vec!["node-tool-transform".to_string(), "text".to_string()]],
            raw_value: json!([
                {
                    "id": "mounted-user",
                    "role": "user",
                    "content": {
                        "kind": "templated_text",
                        "value": "Inspect {{ node-tool-transform.text }}"
                    }
                }
            ]),
        },
    )]);

    if let Some(edge) = plan
        .edges
        .iter_mut()
        .find(|edge| edge.edge_id == "edge-llm-visible-tool-mounted")
    {
        edge.edge_id = "edge-llm-visible-tool-transform".to_string();
        edge.target = "node-tool-transform".to_string();
    }
    plan.edges.push(CompiledEdge {
        edge_id: "edge-tool-transform-mounted".to_string(),
        source: "node-tool-transform".to_string(),
        target: "node-mounted-llm".to_string(),
        source_handle: None,
        target_handle: None,
    });

    plan
}

#[tokio::test]
async fn resume_llm_tool_results_rejects_missing_tool_results() {
    let (invoker, _captured_inputs) = sequential_tool_invoker(vec![tool_call_response(vec![
        ProviderToolCall {
            id: "call_weather".to_string(),
            name: "lookup_weather".to_string(),
            arguments: json!({ "city": "Shanghai" }),
            provider_metadata: json!({}),
        },
        ProviderToolCall {
            id: "call_time".to_string(),
            name: "lookup_time".to_string(),
            arguments: json!({ "city": "Shanghai" }),
            provider_metadata: json!({}),
        },
    ])]);
    let plan = llm_answer_plan();

    let waiting = start_flow_debug_run(
        &plan,
        &json!({ "node-start": { "query": "weather and time?" } }),
        &invoker,
    )
    .await
    .unwrap();
    let checkpoint = waiting
        .checkpoint_snapshot
        .clone()
        .expect("llm tool wait should have checkpoint");

    let (resume_invoker, _resume_inputs) =
        sequential_tool_invoker(vec![final_llm_response("should not be called")]);
    let error = resume_flow_debug_run(
        &plan,
        &checkpoint,
        "node-llm",
        &json!({
            "tool_results": [
                {
                    "tool_call_id": "call_weather",
                    "content": "{\"temperature\":21}"
                }
            ]
        }),
        &resume_invoker,
    )
    .await
    .unwrap_err();

    assert!(error
        .to_string()
        .contains("missing tool result for call_time"));
}
