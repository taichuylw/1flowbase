use super::*;

#[tokio::test]
async fn failed_llm_public_text_is_available_to_downstream_answer_contract() {
    let (invoker, _captured_inputs) = sequential_tool_invoker(vec![
        final_llm_response("first answer"),
        ProviderInvocationResult {
            finish_reason: Some(ProviderFinishReason::Error),
            ..ProviderInvocationResult::default()
        },
    ]);
    let mut plan = multi_llm_answer_plan();
    let answer = plan
        .nodes
        .get_mut("node-answer")
        .expect("answer node should exist");
    answer.dependency_node_ids = vec!["node-llm".to_string(), "node-llm-2".to_string()];
    answer.bindings = BTreeMap::from([(
        "answer_template".to_string(),
        CompiledBinding {
            kind: "templated_text".to_string(),
            selector_paths: vec![
                vec!["node-llm".to_string(), "text".to_string()],
                vec!["node-llm-2".to_string(), "text".to_string()],
            ],
            raw_value: json!("{{ node-llm.text }}\n----\n{{ node-llm-2.text }}"),
        },
    )]);

    let outcome = start_flow_debug_run(
        &plan,
        &json!({ "node-start": { "query": "hello" } }),
        &invoker,
    )
    .await
    .unwrap();

    match outcome.stop_reason {
        ExecutionStopReason::Failed(ref failure) => {
            assert_eq!(failure.node_id, "node-llm-2");
            assert_eq!(
                outcome.variable_pool["node-llm-2"]["text"],
                failure.error_payload["message"]
            );
            assert_eq!(
                outcome.variable_pool["node-answer"]["answer"],
                json!("first answer\n----\nprovider invocation finished with error")
            );
        }
        other => panic!("expected failed stop reason after answer, got {other:?}"),
    }
}

#[tokio::test]
async fn failed_llm_with_compiled_edges_activates_terminal_answer() {
    let mut plan = llm_answer_plan();
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
    ];

    let outcome = start_flow_debug_run(
        &plan,
        &json!({ "node-start": { "query": "hello" } }),
        &StubProviderInvoker {
            fail: true,
            captured_input: Arc::new(Mutex::new(None)),
            final_content: String::new(),
        },
    )
    .await
    .unwrap();

    match outcome.stop_reason {
        ExecutionStopReason::Failed(ref failure) => {
            assert_eq!(failure.node_id, "node-llm");
            assert_eq!(
                outcome.variable_pool["node-answer"]["answer"],
                failure.error_payload["message"]
            );
        }
        other => panic!("expected failed stop reason after terminal answer, got {other:?}"),
    }
}

#[tokio::test]
async fn answer_node_keeps_partial_output_when_template_selector_is_unresolved() {
    let mut plan = llm_answer_plan();
    let answer = plan
        .nodes
        .get_mut("node-answer")
        .expect("answer node should exist");
    answer.bindings = BTreeMap::from([(
        "answer_template".to_string(),
        CompiledBinding {
            kind: "templated_text".to_string(),
            selector_paths: vec![
                vec!["node-llm".to_string(), "text".to_string()],
                vec!["node-llm-1".to_string(), "text".to_string()],
            ],
            raw_value: json!("Answer: {{ node-llm.text }}\nMissing: {{ node-llm-1.text }}"),
        },
    )]);

    let outcome = start_flow_debug_run(
        &plan,
        &json!({ "node-start": { "query": "hello" } }),
        &StubProviderInvoker {
            fail: false,
            captured_input: Arc::new(Mutex::new(None)),
            final_content: "visible answer".to_string(),
        },
    )
    .await
    .unwrap();

    match outcome.stop_reason {
        ExecutionStopReason::Failed(ref failure) => {
            assert_eq!(failure.node_id, "node-answer");
            assert_eq!(
                failure.error_payload["error_code"],
                json!("prompt_template_unresolved")
            );
        }
        other => panic!("expected answer node failure, got {other:?}"),
    }

    let answer_trace = outcome
        .node_traces
        .iter()
        .find(|trace| trace.node_id == "node-answer")
        .expect("answer trace should exist");
    assert_eq!(
        answer_trace.output_payload["answer"],
        json!("Answer: visible answer\nMissing: ")
    );
    assert_eq!(
        answer_trace.output_payload["error"]["error_code"],
        json!("prompt_template_unresolved")
    );
    assert_eq!(
        answer_trace.output_payload["error"]["details"][0]["selector"],
        json!("node-llm-1.text")
    );
    assert_eq!(
        answer_trace
            .error_payload
            .as_ref()
            .expect("answer trace should keep structured error")["error_code"],
        json!("prompt_template_unresolved")
    );
    assert_eq!(
        outcome.variable_pool["node-answer"]["answer"],
        json!("Answer: visible answer\nMissing: ")
    );
}

#[tokio::test]
async fn failover_queue_retries_next_target_before_first_token() {
    let mut plan = base_plan();
    let llm = plan
        .nodes
        .get_mut("node-llm")
        .expect("llm node should exist");
    llm.llm_runtime = Some(CompiledLlmRuntime {
        provider_instance_id: "provider-primary".to_string(),
        provider_code: "fixture_provider".to_string(),
        protocol: "openai_compatible".to_string(),
        model: "primary-model".to_string(),
        routing: Some(CompiledLlmRouting {
            routing_mode: LlmRoutingMode::FailoverQueue,
            fixed_model_target: None,
            queue_template_id: Some("queue-template-1".to_string()),
            queue_snapshot_id: Some("queue-snapshot-1".to_string()),
            queue_targets: vec![
                CompiledLlmRouteTarget {
                    provider_instance_id: "provider-primary".to_string(),
                    provider_code: "fixture_provider".to_string(),
                    protocol: "openai_compatible".to_string(),
                    upstream_model_id: "primary-model".to_string(),
                },
                CompiledLlmRouteTarget {
                    provider_instance_id: "provider-backup".to_string(),
                    provider_code: "fixture_provider".to_string(),
                    protocol: "openai_compatible".to_string(),
                    upstream_model_id: "backup-model".to_string(),
                },
            ],
            context_policy: json!({}),
            stream_policy: json!({}),
        }),
    });
    let calls = Arc::new(Mutex::new(Vec::new()));
    let invoker = FailFirstFailoverInvoker {
        calls: calls.clone(),
    };

    let outcome = start_flow_debug_run(
        &plan,
        &json!({ "node-start": { "query": "hello" } }),
        &invoker,
    )
    .await
    .unwrap();
    let llm_trace = outcome
        .node_traces
        .iter()
        .find(|trace| trace.node_id == "node-llm")
        .expect("llm trace should exist");

    assert_eq!(
        calls.lock().expect("calls mutex poisoned").as_slice(),
        ["provider-primary", "provider-backup"]
    );
    assert_eq!(
        llm_trace.output_payload["text"],
        json!("winner:backup-model")
    );
    assert_eq!(
        llm_trace.metrics_payload["attempts"][0]["status"],
        json!("failed")
    );
    assert_eq!(
        llm_trace.metrics_payload["attempts"][1]["status"],
        json!("succeeded")
    );
    assert_eq!(
        llm_trace.metrics_payload["queue_snapshot_id"],
        json!("queue-snapshot-1")
    );
}

#[tokio::test]
async fn failover_queue_stops_when_primary_fails_after_finish_error_with_first_token() {
    let mut plan = base_plan();
    let llm = plan
        .nodes
        .get_mut("node-llm")
        .expect("llm node should exist");
    llm.llm_runtime = Some(CompiledLlmRuntime {
        provider_instance_id: "provider-primary".to_string(),
        provider_code: "fixture_provider".to_string(),
        protocol: "openai_compatible".to_string(),
        model: "primary-model".to_string(),
        routing: Some(CompiledLlmRouting {
            routing_mode: LlmRoutingMode::FailoverQueue,
            fixed_model_target: None,
            queue_template_id: Some("queue-template-1".to_string()),
            queue_snapshot_id: Some("queue-snapshot-1".to_string()),
            queue_targets: vec![
                CompiledLlmRouteTarget {
                    provider_instance_id: "provider-primary".to_string(),
                    provider_code: "fixture_provider".to_string(),
                    protocol: "openai_compatible".to_string(),
                    upstream_model_id: "primary-model".to_string(),
                },
                CompiledLlmRouteTarget {
                    provider_instance_id: "provider-backup".to_string(),
                    provider_code: "fixture_provider".to_string(),
                    protocol: "openai_compatible".to_string(),
                    upstream_model_id: "backup-model".to_string(),
                },
            ],
            context_policy: json!({}),
            stream_policy: json!({}),
        }),
    });
    let calls = Arc::new(Mutex::new(Vec::new()));
    let invoker = FailAfterTokenFinishErrorFailoverInvoker {
        calls: calls.clone(),
    };

    let outcome = start_flow_debug_run(
        &plan,
        &json!({ "node-start": { "query": "hello" } }),
        &invoker,
    )
    .await
    .unwrap();

    assert_eq!(
        calls.lock().expect("calls mutex poisoned").as_slice(),
        ["provider-primary"]
    );
    match outcome.stop_reason {
        ExecutionStopReason::Failed(ref failure) => {
            assert_eq!(failure.node_id, "node-llm");
            assert_eq!(
                outcome.node_traces[1].error_payload.as_ref().unwrap()["error_code"],
                json!("provider_invalid_response")
            );
            assert_eq!(
                outcome.node_traces[1].output_payload["text"],
                failure.error_payload["message"]
            );
            assert_eq!(
                outcome.variable_pool["node-llm"]["text"],
                failure.error_payload["message"]
            );
            assert_eq!(
                outcome.node_traces[1].metrics_payload["attempts"][0]["failed_after_first_token"],
                json!(true)
            );
        }
        other => panic!("expected failed stop reason, got {other:?}"),
    }
}
