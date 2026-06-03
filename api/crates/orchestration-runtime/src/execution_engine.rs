use std::collections::{BTreeMap, BTreeSet};

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use plugin_framework::{
    error::PluginFrameworkError,
    provider_contract::{
        ProviderFinishReason, ProviderInvocationInput, ProviderInvocationResult, ProviderMessage,
        ProviderMessageRole, ProviderRuntimeError, ProviderRuntimeErrorKind, ProviderStreamEvent,
        ProviderToolCall, ProviderUsage,
    },
};
use serde_json::{json, Map, Value};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

use crate::{
    binding_runtime::{
        lookup_selector_value, render_templated_bindings, resolve_answer_node_inputs,
        resolve_node_inputs, BindingResolutionIssue,
    },
    compiled_plan::{
        CompiledEdge, CompiledLlmRuntime, CompiledNode, CompiledPlan, CompiledPluginRuntime,
        LlmRoutingMode,
    },
    execution_state::{
        CheckpointSnapshot, ExecutionStopReason, FlowDebugExecutionOutcome, NodeExecutionFailure,
        NodeExecutionTrace, PendingCallbackTask, PendingHumanInput,
    },
    node_errors::build_node_type_not_implemented_error_payload,
    output_schema::value_is_llm_context_messages,
    payload_builder::{
        is_reserved_payload_key, BuiltNodePayloads, PublicOutputContract, RawNodeExecutionResult,
    },
};

pub use crate::code_runtime::{
    execute_code_node, CodeInvocationOutput, CodeInvoker, QuickJsCodeInvoker,
};

pub mod branching;
mod llm_callbacks;
mod llm_context;
mod llm_error_payloads;
mod llm_final_content;
mod llm_invocation;
mod llm_metrics;
mod llm_node_outputs;
mod llm_parameters;
#[cfg(test)]
mod tests;

use branching::*;
pub use llm_callbacks::build_llm_tool_callback_wait;
use llm_callbacks::*;
use llm_context::*;
use llm_error_payloads::*;
use llm_final_content::*;
use llm_invocation::*;
use llm_metrics::*;
use llm_node_outputs::*;
use llm_parameters::*;

const LLM_TOOL_CALLBACK_KIND: &str = "llm_tool_calls";
const LLM_TOOL_CALLBACK_STATE_KEY: &str = "__llm_tool_callback";
const RESPONSES_WEBSOCKET_TRANSPORT: &str = "responses_websocket";

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ExecutionRuntimeContext {
    tools: Vec<Value>,
}

impl ExecutionRuntimeContext {
    pub fn from_plan_input(plan: &CompiledPlan, variable_pool: &Map<String, Value>) -> Self {
        Self {
            tools: run_level_provider_tools(plan, variable_pool),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProviderInvocationOutput {
    pub events: Vec<ProviderStreamEvent>,
    pub result: ProviderInvocationResult,
    pub first_token_at: Option<OffsetDateTime>,
    pub time_to_first_token_ms: Option<u64>,
}

#[async_trait]
pub trait ProviderInvoker: Send + Sync {
    async fn invoke_llm(
        &self,
        runtime: &CompiledLlmRuntime,
        input: ProviderInvocationInput,
    ) -> Result<ProviderInvocationOutput>;
}

#[derive(Debug, Clone, PartialEq)]
pub struct CapabilityInvocationOutput {
    pub output_payload: Value,
}

#[async_trait]
pub trait CapabilityInvoker: Send + Sync {
    async fn invoke_capability_node(
        &self,
        runtime: &CompiledPluginRuntime,
        config_payload: Value,
        input_payload: Value,
    ) -> Result<CapabilityInvocationOutput>;
}

#[derive(Debug, Clone, PartialEq)]
pub struct LlmNodeExecution {
    pub output_payload: Value,
    pub error_payload: Option<Value>,
    pub metrics_payload: Value,
    pub debug_payload: Value,
    pub provider_events: Vec<ProviderStreamEvent>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CapabilityNodeExecution {
    pub output_payload: Value,
    pub error_payload: Option<Value>,
    pub metrics_payload: Value,
    pub debug_payload: Value,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LlmToolCallbackWait {
    pub request_payload: Value,
    pub checkpoint_variable_pool: Map<String, Value>,
}

pub async fn start_flow_debug_run<I>(
    plan: &CompiledPlan,
    input_payload: &Value,
    invoker: &I,
) -> Result<FlowDebugExecutionOutcome>
where
    I: ProviderInvoker + CapabilityInvoker + CodeInvoker + ?Sized,
{
    let variable_pool = input_payload
        .as_object()
        .cloned()
        .ok_or_else(|| anyhow!("input payload must be an object"))?;
    let runtime_context = ExecutionRuntimeContext::from_plan_input(plan, &variable_pool);

    execute_from(plan, 0, variable_pool, None, &runtime_context, invoker).await
}

pub async fn resume_flow_debug_run<I>(
    plan: &CompiledPlan,
    checkpoint: &CheckpointSnapshot,
    waiting_node_id: &str,
    resume_payload: &Value,
    invoker: &I,
) -> Result<FlowDebugExecutionOutcome>
where
    I: ProviderInvoker + CapabilityInvoker + CodeInvoker + ?Sized,
{
    let waiting_node = plan
        .nodes
        .get(waiting_node_id)
        .ok_or_else(|| anyhow!("waiting node not found: {waiting_node_id}"))?;
    let mut variable_pool = checkpoint.variable_pool.clone();
    let runtime_context = ExecutionRuntimeContext::from_plan_input(plan, &variable_pool);

    if pending_llm_tool_callback_state(&variable_pool, waiting_node_id).is_some() {
        append_llm_tool_result_messages(&mut variable_pool, waiting_node_id, resume_payload)?;
        return execute_from(
            plan,
            checkpoint.next_node_index,
            variable_pool,
            Some(checkpoint.active_node_ids.iter().cloned().collect()),
            &runtime_context,
            invoker,
        )
        .await;
    }

    let patch = resume_payload
        .as_object()
        .ok_or_else(|| anyhow!("resume payload must be an object"))?;
    let allowed_output_keys = waiting_node
        .outputs
        .iter()
        .map(|output| output.key.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    for key in patch.keys() {
        if !allowed_output_keys.contains(key.as_str()) {
            return Err(anyhow!(
                "resume payload key {key} is not a public output for {waiting_node_id}"
            ));
        }
    }
    variable_pool.insert(waiting_node_id.to_string(), Value::Object(patch.clone()));

    execute_from(
        plan,
        checkpoint.next_node_index,
        variable_pool,
        Some(checkpoint.active_node_ids.iter().cloned().collect()),
        &runtime_context,
        invoker,
    )
    .await
}

async fn execute_from<I>(
    plan: &CompiledPlan,
    next_node_index: usize,
    mut variable_pool: Map<String, Value>,
    active_node_ids: Option<BTreeSet<String>>,
    runtime_context: &ExecutionRuntimeContext,
    invoker: &I,
) -> Result<FlowDebugExecutionOutcome>
where
    I: ProviderInvoker + CapabilityInvoker + CodeInvoker + ?Sized,
{
    let mut node_traces = Vec::new();
    let mut pending_failure: Option<NodeExecutionFailure> = None;
    let mut active_node_ids = active_node_ids.unwrap_or_else(|| initial_active_node_ids(plan));

    for (index, node_id) in plan
        .topological_order
        .iter()
        .enumerate()
        .skip(next_node_index)
    {
        let node = plan
            .nodes
            .get(node_id)
            .ok_or_else(|| anyhow!("compiled node missing: {node_id}"))?;

        if !active_node_ids.contains(node_id) {
            continue;
        }

        let (resolved_inputs, answer_binding_error_payload) =
            match resolve_node_inputs(node, &variable_pool) {
                Ok(inputs) => (inputs, None),
                Err(_) if node.node_type == "answer" => {
                    let resolution = resolve_answer_node_inputs(node, &variable_pool);
                    let error_payload = (!resolution.issues.is_empty()).then(|| {
                        build_answer_binding_resolution_error_payload(node, &resolution.issues)
                    });
                    (resolution.resolved_inputs, error_payload)
                }
                Err(error) => {
                    let error_payload = build_binding_resolution_error_payload(&error);
                    node_traces.push(NodeExecutionTrace {
                        node_id: node.node_id.clone(),
                        node_type: node.node_type.clone(),
                        node_alias: node.alias.clone(),
                        input_payload: json!({}),
                        output_payload: json!({}),
                        error_payload: Some(error_payload.clone()),
                        metrics_payload: json!({ "preview_mode": true }),
                        debug_payload: json!({}),
                        provider_events: Vec::new(),
                    });
                    return Ok(FlowDebugExecutionOutcome {
                        stop_reason: ExecutionStopReason::Failed(NodeExecutionFailure {
                            node_id: node.node_id.clone(),
                            node_alias: node.alias.clone(),
                            error_payload,
                        }),
                        variable_pool,
                        checkpoint_snapshot: None,
                        node_traces,
                    });
                }
            };
        let rendered_templates = render_templated_bindings(node, &resolved_inputs);
        let mut selected_source_handle: Option<String> = None;

        match node.node_type.as_str() {
            "start" => {
                let payload = variable_pool
                    .get(node_id)
                    .cloned()
                    .unwrap_or_else(|| json!({}));
                node_traces.push(NodeExecutionTrace {
                    node_id: node.node_id.clone(),
                    node_type: node.node_type.clone(),
                    node_alias: node.alias.clone(),
                    input_payload: payload,
                    output_payload: json!({}),
                    error_payload: None,
                    metrics_payload: json!({ "preview_mode": true }),
                    debug_payload: json!({}),
                    provider_events: Vec::new(),
                });
            }
            "if_else" => {
                selected_source_handle = select_if_else_source_handle(node, &variable_pool)?;
                node_traces.push(NodeExecutionTrace {
                    node_id: node.node_id.clone(),
                    node_type: node.node_type.clone(),
                    node_alias: node.alias.clone(),
                    input_payload: Value::Object(resolved_inputs),
                    output_payload: json!({}),
                    error_payload: None,
                    metrics_payload: json!({ "preview_mode": true }),
                    debug_payload: json!({
                        "selected_source_handle": selected_source_handle.clone(),
                    }),
                    provider_events: Vec::new(),
                });
            }
            "llm" => {
                let execution = execute_llm_node(
                    node,
                    &resolved_inputs,
                    &rendered_templates,
                    &variable_pool,
                    runtime_context,
                    invoker,
                )
                .await?;
                let trace = NodeExecutionTrace {
                    node_id: node.node_id.clone(),
                    node_type: node.node_type.clone(),
                    node_alias: node.alias.clone(),
                    input_payload: Value::Object(resolved_inputs.clone()),
                    output_payload: execution.output_payload.clone(),
                    error_payload: execution.error_payload.clone(),
                    metrics_payload: execution.metrics_payload.clone(),
                    debug_payload: execution.debug_payload.clone(),
                    provider_events: execution.provider_events.clone(),
                };
                node_traces.push(trace);

                if let Some(error_payload) = execution.error_payload {
                    variable_pool.insert(
                        node.node_id.clone(),
                        project_node_variable_payload(node, &execution.output_payload)?,
                    );
                    let failure = NodeExecutionFailure {
                        node_id: node.node_id.clone(),
                        node_alias: node.alias.clone(),
                        error_payload,
                    };
                    if can_continue_to_terminal_template_nodes(plan, index) {
                        activate_downstream_nodes(
                            plan,
                            &mut active_node_ids,
                            node,
                            selected_source_handle.as_deref(),
                        );
                        pending_failure = Some(failure);
                        continue;
                    }
                    return Ok(FlowDebugExecutionOutcome {
                        stop_reason: ExecutionStopReason::Failed(failure),
                        variable_pool,
                        checkpoint_snapshot: None,
                        node_traces,
                    });
                }

                if let Some(wait) = build_llm_tool_callback_wait(
                    node,
                    &resolved_inputs,
                    &variable_pool,
                    &execution.output_payload,
                ) {
                    return Ok(FlowDebugExecutionOutcome {
                        stop_reason: ExecutionStopReason::WaitingCallback(PendingCallbackTask {
                            node_id: node.node_id.clone(),
                            node_alias: node.alias.clone(),
                            callback_kind: LLM_TOOL_CALLBACK_KIND.to_string(),
                            request_payload: wait.request_payload,
                        }),
                        variable_pool,
                        checkpoint_snapshot: Some(CheckpointSnapshot {
                            next_node_index: index,
                            variable_pool: wait.checkpoint_variable_pool,
                            active_node_ids: checkpoint_active_node_ids(&active_node_ids),
                        }),
                        node_traces,
                    });
                }

                variable_pool.insert(
                    node.node_id.clone(),
                    project_node_variable_payload(node, &execution.output_payload)?,
                );
            }
            "plugin_node" => {
                let execution = execute_capability_plugin_node(
                    node,
                    &resolved_inputs,
                    &rendered_templates,
                    invoker,
                )
                .await?;
                let trace = NodeExecutionTrace {
                    node_id: node.node_id.clone(),
                    node_type: node.node_type.clone(),
                    node_alias: node.alias.clone(),
                    input_payload: Value::Object(resolved_inputs),
                    output_payload: execution.output_payload.clone(),
                    error_payload: execution.error_payload.clone(),
                    metrics_payload: execution.metrics_payload.clone(),
                    debug_payload: execution.debug_payload.clone(),
                    provider_events: Vec::new(),
                };
                node_traces.push(trace);

                if let Some(error_payload) = execution.error_payload {
                    return Ok(FlowDebugExecutionOutcome {
                        stop_reason: ExecutionStopReason::Failed(NodeExecutionFailure {
                            node_id: node.node_id.clone(),
                            node_alias: node.alias.clone(),
                            error_payload,
                        }),
                        variable_pool,
                        checkpoint_snapshot: None,
                        node_traces,
                    });
                }

                variable_pool.insert(
                    node.node_id.clone(),
                    project_node_variable_payload(node, &execution.output_payload)?,
                );
            }
            "template_transform" | "answer" => {
                let output_key = first_output_key(node);
                let output_value =
                    rendered_templates
                        .values()
                        .next()
                        .cloned()
                        .unwrap_or_else(|| {
                            resolved_inputs
                                .values()
                                .next()
                                .cloned()
                                .unwrap_or(Value::Null)
                        });
                let output_payload =
                    template_output_payload(node, output_key, output_value, &variable_pool);
                let output_payload = answer_output_payload_with_error(
                    output_payload,
                    answer_binding_error_payload.as_ref(),
                );
                variable_pool.insert(
                    node.node_id.clone(),
                    project_node_variable_payload(node, &output_payload)?,
                );
                node_traces.push(NodeExecutionTrace {
                    node_id: node.node_id.clone(),
                    node_type: node.node_type.clone(),
                    node_alias: node.alias.clone(),
                    input_payload: Value::Object(resolved_inputs),
                    output_payload,
                    error_payload: answer_binding_error_payload.clone(),
                    metrics_payload: json!({ "preview_mode": true }),
                    debug_payload: json!({}),
                    provider_events: Vec::new(),
                });
                if pending_failure.is_none() {
                    if let Some(error_payload) = answer_binding_error_payload {
                        pending_failure = Some(NodeExecutionFailure {
                            node_id: node.node_id.clone(),
                            node_alias: node.alias.clone(),
                            error_payload,
                        });
                    }
                }
            }
            "human_input" => {
                let prompt = rendered_templates
                    .get("prompt")
                    .and_then(Value::as_str)
                    .unwrap_or("请提供人工输入")
                    .to_string();
                node_traces.push(NodeExecutionTrace {
                    node_id: node.node_id.clone(),
                    node_type: node.node_type.clone(),
                    node_alias: node.alias.clone(),
                    input_payload: Value::Object(resolved_inputs),
                    output_payload: json!({}),
                    error_payload: None,
                    metrics_payload: json!({ "preview_mode": true, "waiting": "human_input" }),
                    debug_payload: json!({}),
                    provider_events: Vec::new(),
                });
                activate_downstream_nodes(plan, &mut active_node_ids, node, None);
                return Ok(FlowDebugExecutionOutcome {
                    stop_reason: ExecutionStopReason::WaitingHuman(PendingHumanInput {
                        node_id: node.node_id.clone(),
                        node_alias: node.alias.clone(),
                        prompt,
                    }),
                    variable_pool: variable_pool.clone(),
                    checkpoint_snapshot: Some(CheckpointSnapshot {
                        next_node_index: index + 1,
                        variable_pool,
                        active_node_ids: checkpoint_active_node_ids(&active_node_ids),
                    }),
                    node_traces,
                });
            }
            "tool" | "http_request" => {
                node_traces.push(NodeExecutionTrace {
                    node_id: node.node_id.clone(),
                    node_type: node.node_type.clone(),
                    node_alias: node.alias.clone(),
                    input_payload: Value::Object(resolved_inputs.clone()),
                    output_payload: json!({}),
                    error_payload: None,
                    metrics_payload: json!({ "preview_mode": true, "waiting": node.node_type }),
                    debug_payload: json!({}),
                    provider_events: Vec::new(),
                });
                activate_downstream_nodes(plan, &mut active_node_ids, node, None);
                return Ok(FlowDebugExecutionOutcome {
                    stop_reason: ExecutionStopReason::WaitingCallback(PendingCallbackTask {
                        node_id: node.node_id.clone(),
                        node_alias: node.alias.clone(),
                        callback_kind: node.node_type.clone(),
                        request_payload: Value::Object(resolved_inputs),
                    }),
                    variable_pool: variable_pool.clone(),
                    checkpoint_snapshot: Some(CheckpointSnapshot {
                        next_node_index: index + 1,
                        variable_pool,
                        active_node_ids: checkpoint_active_node_ids(&active_node_ids),
                    }),
                    node_traces,
                });
            }
            "code" => {
                let execution = execute_code_node(node, &resolved_inputs, invoker).await?;
                node_traces.push(NodeExecutionTrace {
                    node_id: node.node_id.clone(),
                    node_type: node.node_type.clone(),
                    node_alias: node.alias.clone(),
                    input_payload: Value::Object(resolved_inputs),
                    output_payload: execution.output_payload.clone(),
                    error_payload: execution.error_payload.clone(),
                    metrics_payload: execution.metrics_payload.clone(),
                    debug_payload: execution.debug_payload.clone(),
                    provider_events: Vec::new(),
                });

                if let Some(error_payload) = execution.error_payload {
                    return Ok(FlowDebugExecutionOutcome {
                        stop_reason: ExecutionStopReason::Failed(NodeExecutionFailure {
                            node_id: node.node_id.clone(),
                            node_alias: node.alias.clone(),
                            error_payload,
                        }),
                        variable_pool,
                        checkpoint_snapshot: None,
                        node_traces,
                    });
                }

                variable_pool.insert(
                    node.node_id.clone(),
                    project_node_variable_payload(node, &execution.output_payload)?,
                );
            }
            other => {
                let error_payload = build_node_type_not_implemented_error_payload(other, "preview");
                node_traces.push(NodeExecutionTrace {
                    node_id: node.node_id.clone(),
                    node_type: node.node_type.clone(),
                    node_alias: node.alias.clone(),
                    input_payload: Value::Object(resolved_inputs),
                    output_payload: json!({}),
                    error_payload: Some(error_payload.clone()),
                    metrics_payload: json!({ "preview_mode": true }),
                    debug_payload: json!({}),
                    provider_events: Vec::new(),
                });
                return Ok(FlowDebugExecutionOutcome {
                    stop_reason: ExecutionStopReason::Failed(NodeExecutionFailure {
                        node_id: node.node_id.clone(),
                        node_alias: node.alias.clone(),
                        error_payload,
                    }),
                    variable_pool,
                    checkpoint_snapshot: None,
                    node_traces,
                });
            }
        }
        activate_downstream_nodes(
            plan,
            &mut active_node_ids,
            node,
            selected_source_handle.as_deref(),
        );
    }

    if let Some(failure) = pending_failure {
        return Ok(FlowDebugExecutionOutcome {
            stop_reason: ExecutionStopReason::Failed(failure),
            variable_pool,
            checkpoint_snapshot: None,
            node_traces,
        });
    }

    Ok(FlowDebugExecutionOutcome {
        stop_reason: ExecutionStopReason::Completed,
        variable_pool,
        checkpoint_snapshot: None,
        node_traces,
    })
}

pub async fn execute_llm_node<I>(
    node: &CompiledNode,
    resolved_inputs: &Map<String, Value>,
    rendered_templates: &Map<String, Value>,
    variable_pool: &Map<String, Value>,
    runtime_context: &ExecutionRuntimeContext,
    invoker: &I,
) -> Result<LlmNodeExecution>
where
    I: ProviderInvoker + ?Sized,
{
    let runtime = node.llm_runtime.as_ref().ok_or_else(|| {
        anyhow!(
            "compiled llm node is missing runtime metadata: {}",
            node.node_id
        )
    })?;
    let attempt_runtimes = llm_attempt_runtimes(runtime);
    let failover_enabled = runtime
        .routing
        .as_ref()
        .is_some_and(|routing| routing.routing_mode == LlmRoutingMode::FailoverQueue);
    let mut attempt_metrics = Vec::new();
    let mut failed_attempts = Vec::new();

    for (attempt_index, attempt_runtime) in attempt_runtimes.iter().enumerate() {
        let invocation = match build_provider_invocation(
            node,
            attempt_runtime,
            resolved_inputs,
            rendered_templates,
            variable_pool,
            runtime_context,
        ) {
            Ok(invocation) => invocation,
            Err(error_payload) => {
                return build_failed_llm_execution(
                    node,
                    attempt_runtime,
                    error_payload,
                    build_llm_metrics_payload(
                        attempt_runtime,
                        ProviderUsage::default(),
                        Some(ProviderFinishReason::Error),
                        0,
                        attempt_metrics,
                        None,
                        None,
                    ),
                    Vec::new(),
                    true,
                    LlmDebugInvocation {
                        messages: &[],
                        context: None,
                    },
                );
            }
        };
        let invocation_messages = build_llm_debug_invocation_messages(
            node,
            resolved_inputs,
            rendered_templates,
            variable_pool,
            &invocation.input,
        );
        if invocation.input.messages.is_empty() {
            let error_payload = build_empty_prompt_messages_error_payload(attempt_runtime);
            let attempt = build_attempt_metric(AttemptMetricInput {
                attempt_index,
                runtime: attempt_runtime,
                status: "failed",
                failed_after_first_token: false,
                error_payload: Some(&error_payload),
                usage: &ProviderUsage::default(),
                event_count: 0,
                first_token_at: None,
                time_to_first_token_ms: None,
            });
            attempt_metrics.push(attempt);

            return build_failed_llm_execution(
                node,
                attempt_runtime,
                error_payload,
                build_llm_metrics_payload(
                    attempt_runtime,
                    ProviderUsage::default(),
                    Some(ProviderFinishReason::Error),
                    0,
                    attempt_metrics,
                    None,
                    None,
                ),
                Vec::new(),
                true,
                LlmDebugInvocation {
                    messages: &invocation_messages,
                    context: Some(&invocation.debug_context),
                },
            );
        }
        let output = match invoker.invoke_llm(attempt_runtime, invocation.input).await {
            Ok(output) => output,
            Err(error) => {
                let provider_error = provider_runtime_error_from_anyhow(&error);
                let error_payload = build_provider_error_payload(attempt_runtime, &provider_error);
                let attempt = build_attempt_metric(AttemptMetricInput {
                    attempt_index,
                    runtime: attempt_runtime,
                    status: "failed",
                    failed_after_first_token: false,
                    error_payload: Some(&error_payload),
                    usage: &ProviderUsage::default(),
                    event_count: 0,
                    first_token_at: None,
                    time_to_first_token_ms: None,
                });
                attempt_metrics.push(attempt.clone());
                failed_attempts.push(attempt);
                if failover_enabled && attempt_index + 1 < attempt_runtimes.len() {
                    continue;
                }

                return build_failed_llm_execution(
                    node,
                    attempt_runtime,
                    error_payload,
                    build_llm_metrics_payload(
                        attempt_runtime,
                        ProviderUsage::default(),
                        Some(ProviderFinishReason::Error),
                        0,
                        attempt_metrics,
                        None,
                        None,
                    ),
                    Vec::new(),
                    true,
                    LlmDebugInvocation {
                        messages: &invocation_messages,
                        context: Some(&invocation.debug_context),
                    },
                );
            }
        };

        let usage = collect_usage(&output.events, &output.result.usage);
        let finish_reason = output
            .result
            .finish_reason
            .clone()
            .or_else(|| finish_reason_from_events(&output.events));
        let final_content = resolve_final_llm_content(
            output.result.final_content.clone(),
            collect_dify_style_deltas(&output.events),
        );
        let provider_error = first_provider_error(&output.events)
            .cloned()
            .or_else(|| invalid_tool_call_finish_error(finish_reason.as_ref(), &output.result))
            .or_else(|| {
                matches!(finish_reason, Some(ProviderFinishReason::Error)).then(|| {
                    ProviderRuntimeError::normalize(
                        "invoke",
                        "provider invocation finished with error",
                        None,
                    )
                })
            });
        let failed_after_first_token = provider_error.is_some()
            && content_delta_seen_before_terminal_failure(&output.events, finish_reason.as_ref());
        let error_payload = provider_error
            .as_ref()
            .map(|error| build_provider_error_payload(attempt_runtime, error));
        let attempt_status = if error_payload.is_some() {
            "failed"
        } else {
            "succeeded"
        };
        let attempt = build_attempt_metric(AttemptMetricInput {
            attempt_index,
            runtime: attempt_runtime,
            status: attempt_status,
            failed_after_first_token,
            error_payload: error_payload.as_ref(),
            usage: &usage,
            event_count: output.events.len(),
            first_token_at: output.first_token_at,
            time_to_first_token_ms: output.time_to_first_token_ms,
        });
        attempt_metrics.push(attempt.clone());

        if let Some(error_payload) = &error_payload {
            failed_attempts.push(attempt);
            if failover_enabled
                && !failed_after_first_token
                && attempt_index + 1 < attempt_runtimes.len()
            {
                continue;
            }
            return build_failed_llm_execution(
                node,
                attempt_runtime,
                error_payload.clone(),
                build_llm_metrics_payload(
                    attempt_runtime,
                    usage,
                    finish_reason,
                    output.events.len(),
                    attempt_metrics,
                    output.first_token_at,
                    output.time_to_first_token_ms,
                ),
                output.events,
                true,
                LlmDebugInvocation {
                    messages: &invocation_messages,
                    context: Some(&invocation.debug_context),
                },
            );
        }

        return build_successful_llm_execution(
            node,
            attempt_runtime,
            &output.result,
            final_content,
            build_llm_metrics_payload(
                attempt_runtime,
                usage,
                finish_reason.clone(),
                output.events.len(),
                attempt_metrics,
                output.first_token_at,
                output.time_to_first_token_ms,
            ),
            output.events,
            LlmDebugInvocation {
                messages: &invocation_messages,
                context: Some(&invocation.debug_context),
            },
        );
    }

    let error_payload = json!({
        "error_code": "provider_unavailable",
        "message": "all failover queue attempts failed",
        "attempts": failed_attempts,
    });
    build_failed_llm_execution(
        node,
        runtime,
        error_payload,
        build_llm_metrics_payload(
            runtime,
            ProviderUsage::default(),
            Some(ProviderFinishReason::Error),
            0,
            attempt_metrics,
            None,
            None,
        ),
        Vec::new(),
        true,
        LlmDebugInvocation {
            messages: &[],
            context: None,
        },
    )
}

pub async fn execute_capability_plugin_node<I>(
    node: &CompiledNode,
    resolved_inputs: &Map<String, Value>,
    _rendered_templates: &Map<String, Value>,
    invoker: &I,
) -> Result<CapabilityNodeExecution>
where
    I: CapabilityInvoker + ?Sized,
{
    let runtime = node.plugin_runtime.as_ref().ok_or_else(|| {
        anyhow!(
            "compiled plugin node is missing runtime metadata: {}",
            node.node_id
        )
    })?;
    let config_payload = node.config.clone();
    let input_payload = Value::Object(resolved_inputs.clone());

    match invoker
        .invoke_capability_node(runtime, config_payload, input_payload)
        .await
    {
        Ok(output) => {
            let raw = RawNodeExecutionResult {
                executor_output: object_from_value(output.output_payload)?,
                metrics_facts: object_from_value(json!({
                    "plugin_id": runtime.plugin_id,
                    "plugin_version": runtime.plugin_version,
                    "plugin_unique_identifier": runtime.plugin_unique_identifier,
                    "package_id": runtime.package_id,
                    "contribution_code": runtime.contribution_code,
                    "node_shell": runtime.node_shell,
                    "schema_version": runtime.schema_version,
                    "contribution_checksum": runtime.contribution_checksum,
                    "compiled_contribution_hash": runtime.compiled_contribution_hash,
                    "side_effect_policy": runtime.side_effect_policy,
                }))?,
                error_facts: Map::new(),
                debug_facts: Map::new(),
                provider_events: Vec::new(),
            };
            let built = build_plugin_node_payloads(node, raw)?;

            Ok(CapabilityNodeExecution {
                output_payload: built.output_payload,
                error_payload: None,
                metrics_payload: built.metrics_payload,
                debug_payload: built.debug_payload,
            })
        }
        Err(error) => {
            let raw = RawNodeExecutionResult {
                executor_output: object_from_value(json!({ first_output_key(node): Value::Null }))?,
                metrics_facts: object_from_value(json!({
                    "plugin_id": runtime.plugin_id,
                    "plugin_version": runtime.plugin_version,
                    "plugin_unique_identifier": runtime.plugin_unique_identifier,
                    "package_id": runtime.package_id,
                    "contribution_code": runtime.contribution_code,
                    "node_shell": runtime.node_shell,
                    "schema_version": runtime.schema_version,
                    "contribution_checksum": runtime.contribution_checksum,
                    "compiled_contribution_hash": runtime.compiled_contribution_hash,
                    "side_effect_policy": runtime.side_effect_policy,
                    "error": true,
                }))?,
                error_facts: object_from_value(json!({
                    "message": error.to_string(),
                }))?,
                debug_facts: Map::new(),
                provider_events: Vec::new(),
            };
            let built = build_plugin_node_payloads(node, raw)?;

            Ok(CapabilityNodeExecution {
                output_payload: built.output_payload,
                error_payload: Some(built.error_payload),
                metrics_payload: built.metrics_payload,
                debug_payload: built.debug_payload,
            })
        }
    }
}

fn build_plugin_node_payloads(
    node: &CompiledNode,
    raw: RawNodeExecutionResult,
) -> Result<BuiltNodePayloads> {
    for key in raw.executor_output.keys() {
        if is_reserved_payload_key(key) {
            return Err(anyhow!(
                "reserved plugin output key `{key}` cannot be returned by capability node executor"
            ));
        }
    }

    PublicOutputContract::from_compiled_outputs(&node.outputs)?.build_node_payloads(raw)
}
