use std::collections::BTreeMap;

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use plugin_framework::provider_contract::ProviderInvocationInput;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::{
    compiled_plan::{
        CompiledBinding, CompiledCodeRuntime, CompiledLlmRuntime, CompiledNode, CompiledOutput,
        CompiledPlan, CompiledPluginRuntime,
    },
    execution_engine::{
        CapabilityInvocationOutput, CapabilityInvoker, CodeInvocationOutput, CodeInvoker,
        ProviderInvocationOutput, ProviderInvoker, start_flow_debug_run,
    },
    execution_state::ExecutionStopReason,
};

struct CodeFixtureInvoker {
    output_payload: Value,
    fail_message: Option<String>,
}

#[async_trait]
impl ProviderInvoker for CodeFixtureInvoker {
    async fn invoke_llm(
        &self,
        _runtime: &CompiledLlmRuntime,
        _input: ProviderInvocationInput,
    ) -> Result<ProviderInvocationOutput> {
        unreachable!("code runtime tests do not execute llm nodes")
    }
}

#[async_trait]
impl CapabilityInvoker for CodeFixtureInvoker {
    async fn invoke_capability_node(
        &self,
        _runtime: &CompiledPluginRuntime,
        _config_payload: Value,
        _input_payload: Value,
    ) -> Result<CapabilityInvocationOutput> {
        unreachable!("code runtime tests do not execute capability nodes")
    }
}

#[async_trait]
impl CodeInvoker for CodeFixtureInvoker {
    async fn invoke_code_node(
        &self,
        runtime: &CompiledCodeRuntime,
        config_payload: Value,
        input_payload: Value,
    ) -> Result<CodeInvocationOutput> {
        assert_eq!(runtime.language, "javascript");
        assert_eq!(runtime.entrypoint, "main");
        assert_eq!(config_payload["entrypoint"], json!("main"));
        assert_eq!(input_payload["query"], json!("hello"));

        if let Some(message) = &self.fail_message {
            return Err(anyhow!(message.clone()));
        }

        Ok(CodeInvocationOutput {
            output_payload: self.output_payload.clone(),
        })
    }
}

fn code_runtime_plan() -> CompiledPlan {
    let mut nodes = BTreeMap::new();
    nodes.insert(
        "node-start".to_string(),
        CompiledNode {
            node_id: "node-start".to_string(),
            node_type: "start".to_string(),
            alias: "Start".to_string(),
            container_id: None,
            dependency_node_ids: vec![],
            downstream_node_ids: vec!["node-code".to_string()],
            bindings: BTreeMap::new(),
            outputs: vec![CompiledOutput {
                key: "query".to_string(),
                title: "Query".to_string(),
                value_type: "string".to_string(),
                selector: Vec::new(),
            }],
            config: json!({}),
            plugin_runtime: None,
            llm_runtime: None,
            code_runtime: None,
        },
    );
    nodes.insert(
        "node-code".to_string(),
        CompiledNode {
            node_id: "node-code".to_string(),
            node_type: "code".to_string(),
            alias: "Code".to_string(),
            container_id: None,
            dependency_node_ids: vec!["node-start".to_string()],
            downstream_node_ids: vec!["node-answer".to_string()],
            bindings: BTreeMap::from([(
                "query".to_string(),
                CompiledBinding {
                    kind: "selector".to_string(),
                    selector_paths: vec![vec!["node-start".to_string(), "query".to_string()]],
                    raw_value: json!(["node-start", "query"]),
                },
            )]),
            outputs: vec![CompiledOutput {
                key: "result".to_string(),
                title: "Result".to_string(),
                value_type: "string".to_string(),
                selector: Vec::new(),
            }],
            config: json!({
                "language": "javascript",
                "source": "export function main(input) { return { result: input.query }; }",
                "entrypoint": "main"
            }),
            plugin_runtime: None,
            llm_runtime: None,
            code_runtime: Some(CompiledCodeRuntime {
                language: "javascript".to_string(),
                source: Some(
                    "export function main(input) { return { result: input.query }; }".to_string(),
                ),
                source_ref: None,
                entrypoint: "main".to_string(),
                imports: Vec::new(),
                dependencies: Vec::new(),
            }),
        },
    );
    nodes.insert(
        "node-answer".to_string(),
        CompiledNode {
            node_id: "node-answer".to_string(),
            node_type: "answer".to_string(),
            alias: "Answer".to_string(),
            container_id: None,
            dependency_node_ids: vec!["node-code".to_string()],
            downstream_node_ids: vec![],
            bindings: BTreeMap::from([(
                "answer_template".to_string(),
                CompiledBinding {
                    kind: "templated_text".to_string(),
                    selector_paths: vec![vec!["node-code".to_string(), "result".to_string()]],
                    raw_value: json!("Code said: {{ node-code.result }}"),
                },
            )]),
            outputs: vec![CompiledOutput {
                key: "answer".to_string(),
                title: "Answer".to_string(),
                value_type: "string".to_string(),
                selector: Vec::new(),
            }],
            config: json!({}),
            plugin_runtime: None,
            llm_runtime: None,
            code_runtime: None,
        },
    );

    CompiledPlan {
        flow_id: Uuid::nil(),
        source_draft_id: "draft-1".to_string(),
        schema_version: "1flowbase.flow/v2".to_string(),
        topological_order: vec![
            "node-start".to_string(),
            "node-code".to_string(),
            "node-answer".to_string(),
        ],
        nodes,
        compile_issues: Vec::new(),
    }
}

#[tokio::test]
async fn code_runtime_invoker_success_projects_output_for_downstream_template_node() {
    let outcome = start_flow_debug_run(
        &code_runtime_plan(),
        &json!({ "node-start": { "query": "hello" } }),
        &CodeFixtureInvoker {
            output_payload: json!({ "result": "from-code" }),
            fail_message: None,
        },
    )
    .await
    .unwrap();

    assert_eq!(outcome.stop_reason, ExecutionStopReason::Completed);
    assert_eq!(
        outcome.variable_pool["node-code"],
        json!({ "result": "from-code" })
    );
    assert_eq!(
        outcome.variable_pool["node-answer"],
        json!({ "answer": "Code said: from-code" })
    );
    assert_eq!(
        outcome.node_traces[1].output_payload,
        json!({ "result": "from-code" })
    );
    assert_eq!(
        outcome.node_traces[1].metrics_payload["language"],
        json!("javascript")
    );
    assert!(outcome.node_traces[1].error_payload.is_none());
}

#[tokio::test]
async fn code_runtime_invoker_error_yields_stable_failed_stop_reason_and_trace_payload() {
    let outcome = start_flow_debug_run(
        &code_runtime_plan(),
        &json!({ "node-start": { "query": "hello" } }),
        &CodeFixtureInvoker {
            output_payload: json!({}),
            fail_message: Some("runtime failed: user code threw".to_string()),
        },
    )
    .await
    .unwrap();

    match outcome.stop_reason {
        ExecutionStopReason::Failed(failure) => {
            assert_eq!(failure.node_id, "node-code");
            assert_eq!(failure.node_alias, "Code");
            assert_eq!(
                failure.error_payload["error_kind"],
                json!("code_runtime_error")
            );
            assert_eq!(
                failure.error_payload["message"],
                json!("code execution failed")
            );
            assert_eq!(
                failure.error_payload["runtime_message"],
                json!("runtime failed: user code threw")
            );
            assert_eq!(outcome.node_traces[1].node_type, "code");
            assert!(
                outcome.node_traces[1]
                    .output_payload
                    .as_object()
                    .unwrap()
                    .is_empty()
            );
            assert_eq!(
                outcome.node_traces[1].error_payload.as_ref().unwrap()["error_kind"],
                json!("code_runtime_error")
            );
        }
        other => panic!("expected failed stop reason, got {other:?}"),
    }

    assert!(outcome.variable_pool.get("node-code").is_none());
    assert_eq!(outcome.node_traces.len(), 2);
}

#[tokio::test]
async fn code_runtime_missing_declared_output_projects_empty_variable_payload() {
    let outcome = start_flow_debug_run(
        &code_runtime_plan(),
        &json!({ "node-start": { "query": "hello" } }),
        &CodeFixtureInvoker {
            output_payload: json!({ "unexpected": true }),
            fail_message: None,
        },
    )
    .await
    .unwrap();

    match outcome.stop_reason {
        ExecutionStopReason::Failed(failure) => {
            assert_eq!(failure.node_id, "node-answer");
            assert!(
                failure.error_payload["message"]
                    .as_str()
                    .unwrap()
                    .contains("selector path not found: node-code.result")
            );
        }
        other => panic!("expected downstream binding failure, got {other:?}"),
    }
    assert_eq!(outcome.variable_pool["node-code"], json!({}));
    assert_eq!(
        outcome.node_traces[1].output_payload,
        json!({ "unexpected": true })
    );
}
