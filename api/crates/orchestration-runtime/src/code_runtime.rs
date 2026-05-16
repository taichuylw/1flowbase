use anyhow::{Result, anyhow};
use async_trait::async_trait;
use serde_json::{Map, Value, json};

use crate::{
    compiled_plan::{CompiledCodeRuntime, CompiledNode},
    payload_builder::{
        BuiltNodePayloads, PublicOutputContract, RawNodeExecutionResult, is_reserved_payload_key,
    },
};

#[derive(Debug, Clone, PartialEq)]
pub struct CodeInvocationOutput {
    pub output_payload: Value,
}

#[async_trait]
pub trait CodeInvoker: Send + Sync {
    async fn invoke_code_node(
        &self,
        runtime: &CompiledCodeRuntime,
        config_payload: Value,
        input_payload: Value,
    ) -> Result<CodeInvocationOutput>;
}

#[derive(Debug, Clone, PartialEq)]
pub struct CodeNodeExecution {
    pub output_payload: Value,
    pub error_payload: Option<Value>,
    pub metrics_payload: Value,
    pub debug_payload: Value,
}

pub async fn execute_code_node<I>(
    node: &CompiledNode,
    resolved_inputs: &Map<String, Value>,
    invoker: &I,
) -> Result<CodeNodeExecution>
where
    I: CodeInvoker + ?Sized,
{
    let runtime = node.code_runtime.as_ref().ok_or_else(|| {
        anyhow!(
            "compiled code node is missing runtime metadata: {}",
            node.node_id
        )
    })?;
    let config_payload = node.config.clone();
    let input_payload = Value::Object(resolved_inputs.clone());

    match invoker
        .invoke_code_node(runtime, config_payload, input_payload)
        .await
    {
        Ok(output) => {
            let raw = RawNodeExecutionResult {
                executor_output: object_from_value(output.output_payload)?,
                metrics_facts: code_runtime_metrics(runtime, false)?,
                error_facts: Map::new(),
                debug_facts: Map::new(),
                provider_events: Vec::new(),
            };
            let built = build_code_node_payloads(node, raw)?;

            Ok(CodeNodeExecution {
                output_payload: built.output_payload,
                error_payload: None,
                metrics_payload: built.metrics_payload,
                debug_payload: built.debug_payload,
            })
        }
        Err(error) => {
            let raw = RawNodeExecutionResult {
                executor_output: Map::new(),
                metrics_facts: code_runtime_metrics(runtime, true)?,
                error_facts: object_from_value(json!({
                    "error_kind": "code_runtime_error",
                    "message": "code execution failed",
                    "runtime_message": error.to_string(),
                }))?,
                debug_facts: Map::new(),
                provider_events: Vec::new(),
            };
            let built = build_code_node_payloads(node, raw)?;

            Ok(CodeNodeExecution {
                output_payload: built.output_payload,
                error_payload: Some(built.error_payload),
                metrics_payload: built.metrics_payload,
                debug_payload: built.debug_payload,
            })
        }
    }
}

fn code_runtime_metrics(runtime: &CompiledCodeRuntime, error: bool) -> Result<Map<String, Value>> {
    object_from_value(json!({
        "language": runtime.language,
        "entrypoint": runtime.entrypoint,
        "imports": runtime.imports,
        "dependency_count": runtime.dependencies.len(),
        "error": error,
    }))
}

fn build_code_node_payloads(
    node: &CompiledNode,
    raw: RawNodeExecutionResult,
) -> Result<BuiltNodePayloads> {
    for key in raw.executor_output.keys() {
        if is_reserved_payload_key(key) {
            return Err(anyhow!(
                "reserved code output key `{key}` cannot be returned by code node executor"
            ));
        }
    }

    PublicOutputContract::from_compiled_outputs(&node.outputs)?.build_node_payloads(raw)
}

fn object_from_value(value: Value) -> Result<Map<String, Value>> {
    value
        .as_object()
        .cloned()
        .ok_or_else(|| anyhow!("payload bucket facts must be an object"))
}
