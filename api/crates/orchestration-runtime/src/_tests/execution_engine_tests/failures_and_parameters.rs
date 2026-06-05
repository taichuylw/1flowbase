use super::*;

#[tokio::test]
async fn provider_error_marks_flow_failed_and_redacts_summary() {
    let outcome = start_flow_debug_run(
        &base_plan(),
        &json!({ "node-start": { "query": "退款政策" } }),
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
            assert_eq!(failure.error_payload["error_code"], json!("auth_failed"));
            assert_eq!(
                outcome.node_traces[1].error_payload.as_ref().unwrap()["error_code"],
                json!("auth_failed")
            );
            assert_eq!(
                outcome.node_traces[1].output_payload["text"],
                failure.error_payload["message"]
            );
            assert_eq!(
                outcome.variable_pool["node-llm"]["text"],
                failure.error_payload["message"]
            );
            assert!(failure.error_payload["provider_summary"]
                .as_str()
                .unwrap()
                .contains("[REDACTED]"));
        }
        other => panic!("expected failed stop reason, got {other:?}"),
    }
}

#[tokio::test]
async fn provider_runtime_contract_error_is_renormalized_for_llm_output() {
    let outcome = start_flow_debug_run(
        &base_plan(),
        &json!({ "node-start": { "query": "退款政策" } }),
        &RuntimeContractErrorInvoker,
    )
    .await
    .unwrap();

    match outcome.stop_reason {
        ExecutionStopReason::Failed(ref failure) => {
            assert_eq!(failure.node_id, "node-llm");
            assert_eq!(failure.error_payload["error_code"], json!("auth_failed"));
            assert_eq!(
                failure.error_payload["message"],
                json!("401 401 Unauthorized: Incorrect API key provided")
            );
            assert_eq!(
                outcome.node_traces[1].error_payload.as_ref().unwrap()["message"],
                json!("401 401 Unauthorized: Incorrect API key provided")
            );
            assert_eq!(
                outcome.node_traces[1].output_payload["text"],
                failure.error_payload["message"]
            );
            assert_eq!(
                outcome.variable_pool["node-llm"]["text"],
                failure.error_payload["message"]
            );
        }
        other => panic!("expected failed stop reason, got {other:?}"),
    }
}

#[tokio::test]
async fn llm_failure_after_first_token_writes_error_text_to_public_output() {
    let outcome = start_flow_debug_run(
        &base_plan(),
        &json!({ "node-start": { "query": "退款政策" } }),
        &FailsAfterFirstTokenInvoker,
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

#[tokio::test]
async fn llm_runtime_sends_enabled_model_parameters_and_keeps_undeclared_structured_output_private()
{
    let mut plan = base_plan();
    let llm = plan
        .nodes
        .get_mut("node-llm")
        .expect("llm node should exist");
    llm.config = json!({
        "model_provider": {
            "provider_instance_id": "provider-ready",
            "model_id": "gpt-5.4-mini"
        },
        "llm_parameters": {
            "schema_version": "1.0.0",
            "items": {
                "temperature": { "enabled": true, "value": 0.7 },
                "top_p": { "enabled": false, "value": 0.9 }
            }
        },
        "response_format": {
            "mode": "json_schema",
            "schema": { "type": "object" }
        }
    });

    let invoker = StubProviderInvoker {
        fail: false,
        captured_input: Arc::new(Mutex::new(None)),
        final_content: "{\"ok\":true}".to_string(),
    };

    let outcome = start_flow_debug_run(
        &plan,
        &json!({ "node-start": { "query": "输出 JSON" } }),
        &invoker,
    )
    .await
    .unwrap();

    let captured_input = invoker
        .captured_input
        .lock()
        .expect("captured input mutex poisoned")
        .clone()
        .expect("provider input should be captured");

    assert_eq!(
        captured_input.model_parameters.get("temperature"),
        Some(&json!(0.7))
    );
    assert!(!captured_input.model_parameters.contains_key("top_p"));
    assert_eq!(
        captured_input.response_format,
        Some(json!({ "mode": "json_schema", "schema": { "type": "object" } }))
    );
    assert_eq!(
        outcome.node_traces[1].output_payload["text"],
        json!("{\"ok\":true}")
    );
    assert!(outcome.node_traces[1]
        .output_payload
        .get("structured_output")
        .is_none());
}

#[tokio::test]
async fn llm_runtime_ignores_external_reasoning_parameters_without_node_opt_in() {
    let plan = base_plan();
    let invoker = StubProviderInvoker {
        fail: false,
        captured_input: Arc::new(Mutex::new(None)),
        final_content: "ok".to_string(),
    };

    start_flow_debug_run(
        &plan,
        &json!({
            "node-start": { "query": "hello" },
            "sys": {
                "model_parameters": {
                    "reasoning": {
                        "enabled": true,
                        "effort": "high",
                        "budget_tokens": 4096
                    }
                }
            }
        }),
        &invoker,
    )
    .await
    .unwrap();

    let captured_input = invoker
        .captured_input
        .lock()
        .expect("captured input mutex poisoned")
        .clone()
        .expect("provider input should be captured");

    assert!(!captured_input
        .model_parameters
        .contains_key("reasoning_effort"));
    assert!(!captured_input
        .model_parameters
        .contains_key("thinking_budget_tokens"));
}

#[tokio::test]
async fn llm_runtime_maps_external_reasoning_parameters_when_node_opts_in() {
    let mut plan = base_plan();
    let llm = plan
        .nodes
        .get_mut("node-llm")
        .expect("llm node should exist");
    llm.config = json!({
        "external_reasoning_policy": {
            "follow_external_reasoning": true
        }
    });
    let runtime = llm.llm_runtime.as_mut().expect("llm runtime should exist");
    runtime.provider_code = "openai".to_string();
    runtime.protocol = "openai_responses".to_string();
    let invoker = StubProviderInvoker {
        fail: false,
        captured_input: Arc::new(Mutex::new(None)),
        final_content: "ok".to_string(),
    };

    start_flow_debug_run(
        &plan,
        &json!({
            "node-start": { "query": "hello" },
            "sys": {
                "model_parameters": {
                    "reasoning": {
                        "enabled": true,
                        "effort": "high",
                        "budget_tokens": 4096
                    }
                }
            }
        }),
        &invoker,
    )
    .await
    .unwrap();

    let captured_input = invoker
        .captured_input
        .lock()
        .expect("captured input mutex poisoned")
        .clone()
        .expect("provider input should be captured");

    assert_eq!(
        captured_input.model_parameters.get("reasoning_effort"),
        Some(&json!("high"))
    );
}

#[tokio::test]
async fn llm_runtime_maps_external_reasoning_parameters_for_anthropic_runtime() {
    let mut plan = base_plan();
    let llm = plan
        .nodes
        .get_mut("node-llm")
        .expect("llm node should exist");
    llm.config = json!({
        "external_reasoning_policy": {
            "follow_external_reasoning": true
        }
    });
    let runtime = llm.llm_runtime.as_mut().expect("llm runtime should exist");
    runtime.provider_code = "anthropic".to_string();
    runtime.protocol = "anthropic_messages".to_string();
    let invoker = StubProviderInvoker {
        fail: false,
        captured_input: Arc::new(Mutex::new(None)),
        final_content: "ok".to_string(),
    };

    start_flow_debug_run(
        &plan,
        &json!({
            "node-start": { "query": "hello" },
            "sys": {
                "model_parameters": {
                    "reasoning": {
                        "enabled": true,
                        "effort": "high",
                        "budget_tokens": 4096
                    }
                }
            }
        }),
        &invoker,
    )
    .await
    .unwrap();

    let captured_input = invoker
        .captured_input
        .lock()
        .expect("captured input mutex poisoned")
        .clone()
        .expect("provider input should be captured");

    assert_eq!(
        captured_input.model_parameters.get("thinking_type"),
        Some(&json!("enabled"))
    );
    assert_eq!(
        captured_input
            .model_parameters
            .get("thinking_budget_tokens"),
        Some(&json!(4096))
    );
    assert!(!captured_input
        .model_parameters
        .contains_key("reasoning_effort"));
}

#[tokio::test]
async fn llm_runtime_maps_external_reasoning_parameters_for_bailian_runtime() {
    let mut plan = base_plan();
    let llm = plan
        .nodes
        .get_mut("node-llm")
        .expect("llm node should exist");
    llm.config = json!({
        "external_reasoning_policy": {
            "follow_external_reasoning": true
        }
    });
    let runtime = llm.llm_runtime.as_mut().expect("llm runtime should exist");
    runtime.provider_code = "aliyun_bailian".to_string();
    runtime.protocol = "openai_compatible".to_string();
    let invoker = StubProviderInvoker {
        fail: false,
        captured_input: Arc::new(Mutex::new(None)),
        final_content: "ok".to_string(),
    };

    start_flow_debug_run(
        &plan,
        &json!({
            "node-start": { "query": "hello" },
            "sys": {
                "model_parameters": {
                    "reasoning": {
                        "enabled": true,
                        "effort": "high",
                        "budget_tokens": 4096
                    }
                }
            }
        }),
        &invoker,
    )
    .await
    .unwrap();

    let captured_input = invoker
        .captured_input
        .lock()
        .expect("captured input mutex poisoned")
        .clone()
        .expect("provider input should be captured");

    assert_eq!(
        captured_input.model_parameters.get("enable_thinking"),
        Some(&json!(true))
    );
    assert_eq!(
        captured_input.model_parameters.get("reasoning_effort"),
        Some(&json!("high"))
    );
    assert!(!captured_input
        .model_parameters
        .contains_key("thinking_budget_tokens"));
}

#[tokio::test]
async fn llm_json_schema_response_exposes_structured_output_only_when_declared() {
    let mut plan = base_plan();
    let llm = plan
        .nodes
        .get_mut("node-llm")
        .expect("llm node should exist");
    llm.config = json!({
        "model_provider": {
            "provider_instance_id": "provider-ready",
            "model_id": "gpt-5.4-mini"
        },
        "response_format": {
            "mode": "json_schema",
            "schema": { "type": "object" }
        }
    });
    llm.outputs.push(CompiledOutput {
        key: "structured_output".to_string(),
        title: "结构化输出".to_string(),
        value_type: "json".to_string(),
        selector: Vec::new(),
        json_schema: None,
    });

    let outcome = start_flow_debug_run(
        &plan,
        &json!({ "node-start": { "query": "输出 JSON" } }),
        &StubProviderInvoker {
            fail: false,
            captured_input: Arc::new(Mutex::new(None)),
            final_content: "{\"ok\":true}".to_string(),
        },
    )
    .await
    .unwrap();

    assert_eq!(
        outcome.node_traces[1].output_payload["text"],
        json!("{\"ok\":true}")
    );
    assert_eq!(
        outcome.node_traces[1].output_payload["structured_output"],
        json!({ "ok": true })
    );
    assert_eq!(
        outcome.node_traces[1].metrics_payload["usage"]["total_tokens"],
        json!(12)
    );
}

#[tokio::test]
async fn llm_json_schema_response_rejects_invalid_structured_output() {
    let mut plan = base_plan();
    let llm = plan
        .nodes
        .get_mut("node-llm")
        .expect("llm node should exist");
    llm.config = json!({
        "model_provider": {
            "provider_instance_id": "provider-ready",
            "model_id": "gpt-5.4-mini"
        },
        "response_format": {
            "mode": "json_schema",
            "schema": { "type": "object" }
        }
    });
    llm.outputs.push(CompiledOutput {
        key: "structured_output".to_string(),
        title: "结构化输出".to_string(),
        value_type: "json".to_string(),
        selector: Vec::new(),
        json_schema: None,
    });

    let error = start_flow_debug_run(
        &plan,
        &json!({ "node-start": { "query": "输出 JSON" } }),
        &StubProviderInvoker {
            fail: false,
            captured_input: Arc::new(Mutex::new(None)),
            final_content: "not json".to_string(),
        },
    )
    .await
    .expect_err("invalid structured LLM output should fail the node");

    assert!(error.to_string().contains("invalid structured LLM output"));
}
