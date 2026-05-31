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
        render_templated_bindings, resolve_answer_node_inputs, resolve_node_inputs,
        BindingResolutionIssue,
    },
    compiled_plan::{
        CompiledLlmRuntime, CompiledNode, CompiledPlan, CompiledPluginRuntime, LlmRoutingMode,
    },
    execution_state::{
        CheckpointSnapshot, ExecutionStopReason, FlowDebugExecutionOutcome, NodeExecutionFailure,
        NodeExecutionTrace, PendingCallbackTask, PendingHumanInput,
    },
    node_errors::build_node_type_not_implemented_error_payload,
    payload_builder::{
        is_reserved_payload_key, BuiltNodePayloads, PublicOutputContract, RawNodeExecutionResult,
    },
};

pub use crate::code_runtime::{
    execute_code_node, CodeInvocationOutput, CodeInvoker, QuickJsCodeInvoker,
};

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

    execute_from(plan, 0, variable_pool, &runtime_context, invoker).await
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
        &runtime_context,
        invoker,
    )
    .await
}

async fn execute_from<I>(
    plan: &CompiledPlan,
    next_node_index: usize,
    mut variable_pool: Map<String, Value>,
    runtime_context: &ExecutionRuntimeContext,
    invoker: &I,
) -> Result<FlowDebugExecutionOutcome>
where
    I: ProviderInvoker + CapabilityInvoker + CodeInvoker + ?Sized,
{
    let mut node_traces = Vec::new();
    let mut pending_failure: Option<NodeExecutionFailure> = None;

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
        let invocation = build_provider_invocation(
            node,
            attempt_runtime,
            resolved_inputs,
            rendered_templates,
            variable_pool,
            runtime_context,
        );
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
                &invocation_messages,
                Some(&invocation.debug_context),
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
                    &invocation_messages,
                    Some(&invocation.debug_context),
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
                &invocation_messages,
                Some(&invocation.debug_context),
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
            &invocation_messages,
            &invocation.debug_context,
        );
    }

    let error_payload = json!({
        "error_kind": "provider_unavailable",
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
        &[],
        None,
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

fn llm_attempt_runtimes(runtime: &CompiledLlmRuntime) -> Vec<CompiledLlmRuntime> {
    let Some(routing) = runtime.routing.as_ref() else {
        return vec![runtime.clone()];
    };
    if routing.routing_mode != LlmRoutingMode::FailoverQueue || routing.queue_targets.is_empty() {
        return vec![runtime.clone()];
    }

    routing
        .queue_targets
        .iter()
        .map(|target| {
            let mut attempt = runtime.clone();
            attempt.provider_instance_id = target.provider_instance_id.clone();
            attempt.provider_code = target.provider_code.clone();
            attempt.protocol = target.protocol.clone();
            attempt.model = target.upstream_model_id.clone();
            attempt
        })
        .collect()
}

struct AttemptMetricInput<'a> {
    attempt_index: usize,
    runtime: &'a CompiledLlmRuntime,
    status: &'a str,
    failed_after_first_token: bool,
    error_payload: Option<&'a Value>,
    usage: &'a ProviderUsage,
    event_count: usize,
    first_token_at: Option<OffsetDateTime>,
    time_to_first_token_ms: Option<u64>,
}

fn build_attempt_metric(input: AttemptMetricInput<'_>) -> Value {
    json!({
        "attempt_index": input.attempt_index,
        "provider_instance_id": input.runtime.provider_instance_id,
        "provider_code": input.runtime.provider_code,
        "protocol": input.runtime.protocol,
        "upstream_model_id": input.runtime.model,
        "model": input.runtime.model,
        "status": input.status,
        "failed_after_first_token": input.failed_after_first_token,
        "event_count": input.event_count,
        "first_token_at": offset_datetime_json(input.first_token_at),
        "time_to_first_token_ms": input.time_to_first_token_ms,
        "usage": serde_json::to_value(input.usage).unwrap_or(Value::Null),
        "error_code": input.error_payload
            .and_then(|payload| payload.get("error_kind"))
            .cloned()
            .unwrap_or(Value::Null),
        "error_message_ref": input.error_payload
            .and_then(|payload| payload.get("message"))
            .and_then(Value::as_str)
            .map(|message| format!("runtime_artifact:inline:error:{message}"))
            .map(Value::String)
            .unwrap_or(Value::Null),
    })
}

fn build_llm_metrics_payload(
    runtime: &CompiledLlmRuntime,
    usage: ProviderUsage,
    finish_reason: Option<ProviderFinishReason>,
    event_count: usize,
    attempts: Vec<Value>,
    first_token_at: Option<OffsetDateTime>,
    time_to_first_token_ms: Option<u64>,
) -> Value {
    json!({
        "provider_instance_id": runtime.provider_instance_id,
        "provider_code": runtime.provider_code,
        "protocol": runtime.protocol,
        "model": runtime.model,
        "event_count": event_count,
        "first_token_at": offset_datetime_json(first_token_at),
        "time_to_first_token_ms": time_to_first_token_ms,
        "route": build_llm_route_payload(runtime),
        "usage": serde_json::to_value(&usage).unwrap_or(Value::Null),
        "finish_reason": finish_reason
            .as_ref()
            .map(|reason| serde_json::to_value(reason).unwrap_or(Value::Null))
            .unwrap_or(Value::Null),
        "queue_snapshot_id": runtime
            .routing
            .as_ref()
            .and_then(|routing| routing.queue_snapshot_id.clone())
            .map(Value::String)
            .unwrap_or(Value::Null),
        "attempts": attempts,
    })
}

fn offset_datetime_json(value: Option<OffsetDateTime>) -> Value {
    value
        .and_then(|datetime| datetime.format(&Rfc3339).ok())
        .map(Value::String)
        .unwrap_or(Value::Null)
}

fn build_llm_route_payload(runtime: &CompiledLlmRuntime) -> Value {
    match runtime.routing.as_ref() {
        Some(routing) => json!({
            "routing_mode": routing.routing_mode,
            "fixed_model_target": routing.fixed_model_target,
            "queue_template_id": routing.queue_template_id,
            "provider_instance_id": runtime.provider_instance_id,
            "provider_code": runtime.provider_code,
            "upstream_model_id": runtime.model,
            "protocol": runtime.protocol,
        }),
        None => json!({
            "routing_mode": "fixed_model",
            "provider_instance_id": runtime.provider_instance_id,
            "provider_code": runtime.provider_code,
            "upstream_model_id": runtime.model,
            "protocol": runtime.protocol,
        }),
    }
}

#[derive(Debug, Clone)]
struct BuiltProviderInvocation {
    input: ProviderInvocationInput,
    debug_context: LlmInvocationDebugContext,
}

#[derive(Debug, Clone)]
struct LlmInvocationDebugContext {
    context_policy: Value,
    effective_system: Option<String>,
    provider_messages: Vec<Value>,
    compatibility_promotions: Vec<Value>,
    system_sources: Vec<Value>,
    previous_response_id: Option<String>,
}

impl LlmInvocationDebugContext {
    fn from_provider_context(
        context_policy: Value,
        previous_response_id: Option<String>,
        context: &ProviderPromptContext,
    ) -> Self {
        Self {
            context_policy,
            effective_system: context.system.clone(),
            provider_messages: prompt_messages_from_provider_messages(&context.messages),
            compatibility_promotions: context.compatibility_promotions.clone(),
            system_sources: context.system_sources.clone(),
            previous_response_id,
        }
    }

    fn to_payload(&self) -> Value {
        let mut payload = Map::new();
        payload.insert("context_policy".to_string(), self.context_policy.clone());
        payload.insert(
            "effective_system".to_string(),
            self.effective_system
                .clone()
                .map(Value::String)
                .unwrap_or(Value::Null),
        );
        payload.insert(
            "provider_messages".to_string(),
            Value::Array(self.provider_messages.clone()),
        );
        payload.insert(
            "compatibility_promotions".to_string(),
            Value::Array(self.compatibility_promotions.clone()),
        );
        payload.insert(
            "system_sources".to_string(),
            Value::Array(self.system_sources.clone()),
        );
        if let Some(previous_response_id) = &self.previous_response_id {
            payload.insert(
                "previous_response_id".to_string(),
                Value::String(previous_response_id.clone()),
            );
        }
        Value::Object(payload)
    }
}

#[derive(Debug, Clone)]
struct ProviderPromptContext {
    system: Option<String>,
    messages: Vec<ProviderMessage>,
    compatibility_promotions: Vec<Value>,
    system_sources: Vec<Value>,
}

fn build_provider_invocation(
    node: &CompiledNode,
    runtime: &CompiledLlmRuntime,
    resolved_inputs: &Map<String, Value>,
    rendered_templates: &Map<String, Value>,
    variable_pool: &Map<String, Value>,
    runtime_context: &ExecutionRuntimeContext,
) -> BuiltProviderInvocation {
    let previous_response_id =
        pending_llm_tool_callback_previous_response_id(node, runtime, variable_pool);
    let context_policy = llm_context_policy(node, runtime);
    let provider_context = if previous_response_id.is_some() {
        let prompt_messages = pending_llm_tool_callback_delta_messages(node, variable_pool)
            .unwrap_or_else(|| {
                binding_prompt_messages_with_context_sources(
                    node,
                    rendered_templates,
                    resolved_inputs,
                    variable_pool,
                    &context_policy,
                )
            });
        let mut context = provider_context_from_prompt_messages(prompt_messages);
        if context.system.is_none() {
            if let Some(system) = pending_llm_tool_callback_system(node, variable_pool) {
                context.system = Some(system);
                context.system_sources.push(json!({
                    "source": format!("{}.{}", node.node_id, LLM_TOOL_CALLBACK_STATE_KEY),
                    "source_kind": "pending_tool_callback_history",
                    "target": "effective_system"
                }));
            }
        }
        context
    } else {
        provider_context_from_prompt_messages(binding_prompt_messages_with_context_sources(
            node,
            rendered_templates,
            resolved_inputs,
            variable_pool,
            &context_policy,
        ))
    };

    let trace_context = BTreeMap::from([
        ("node_id".to_string(), node.node_id.clone()),
        ("node_alias".to_string(), node.alias.clone()),
    ]);
    let debug_context = LlmInvocationDebugContext::from_provider_context(
        context_policy,
        previous_response_id.clone(),
        &provider_context,
    );

    let input = ProviderInvocationInput {
        provider_instance_id: runtime.provider_instance_id.clone(),
        provider_code: runtime.provider_code.clone(),
        protocol: runtime.protocol.clone(),
        model: runtime.model.clone(),
        previous_response_id,
        provider_config: Value::Null,
        messages: provider_context.messages,
        system: provider_context.system,
        tools: provider_tools(
            node,
            resolved_inputs,
            rendered_templates,
            variable_pool,
            runtime_context,
        ),
        mcp_bindings: Vec::new(),
        response_format: build_response_format(&node.config),
        model_parameters: build_model_parameters(node, runtime, variable_pool),
        trace_context,
        run_context: BTreeMap::from([(
            "resolved_inputs".to_string(),
            Value::Object(resolved_inputs.clone()),
        )]),
    };

    BuiltProviderInvocation {
        input,
        debug_context,
    }
}

fn prompt_messages_from_provider_messages(messages: &[ProviderMessage]) -> Vec<Value> {
    messages
        .iter()
        .map(|message| {
            let mut payload = Map::new();
            payload.insert(
                "role".to_string(),
                serde_json::to_value(&message.role).unwrap_or(Value::Null),
            );
            payload.insert(
                "content".to_string(),
                Value::String(message.content.clone()),
            );
            if let Some(name) = &message.name {
                payload.insert("name".to_string(), Value::String(name.clone()));
            }
            if let Some(tool_call_id) = &message.tool_call_id {
                payload.insert(
                    "tool_call_id".to_string(),
                    Value::String(tool_call_id.clone()),
                );
            }
            if let Some(tool_calls) = &message.tool_calls {
                payload.insert("tool_calls".to_string(), tool_calls.clone());
            }
            if let Some(content_blocks) = &message.content_blocks {
                payload.insert("content_blocks".to_string(), content_blocks.clone());
            }

            Value::Object(payload)
        })
        .collect()
}

fn build_llm_debug_invocation_messages(
    node: &CompiledNode,
    resolved_inputs: &Map<String, Value>,
    rendered_templates: &Map<String, Value>,
    variable_pool: &Map<String, Value>,
    invocation_input: &ProviderInvocationInput,
) -> Vec<Value> {
    if invocation_input.previous_response_id.is_some()
        || pending_llm_tool_callback_state(variable_pool, &node.node_id).is_some()
    {
        return binding_prompt_messages(node, rendered_templates, resolved_inputs, variable_pool);
    }

    prompt_messages_from_provider_messages(&invocation_input.messages)
}

fn has_pending_tool_calls(output_payload: &Value) -> bool {
    output_payload
        .get("tool_calls")
        .and_then(Value::as_array)
        .is_some_and(|tool_calls| !tool_calls.is_empty())
}

pub fn build_llm_tool_callback_wait(
    node: &CompiledNode,
    resolved_inputs: &Map<String, Value>,
    variable_pool: &Map<String, Value>,
    output_payload: &Value,
) -> Option<LlmToolCallbackWait> {
    has_pending_tool_calls(output_payload).then(|| LlmToolCallbackWait {
        request_payload: build_llm_tool_callback_request_payload(
            node,
            resolved_inputs,
            variable_pool,
            output_payload,
        ),
        checkpoint_variable_pool: variable_pool_with_pending_llm_tool_callback(
            node,
            resolved_inputs,
            variable_pool,
            output_payload,
        ),
    })
}

fn build_llm_tool_callback_request_payload(
    node: &CompiledNode,
    resolved_inputs: &Map<String, Value>,
    variable_pool: &Map<String, Value>,
    output_payload: &Value,
) -> Value {
    let history = llm_callback_history_after_assistant_tool_call(
        node,
        resolved_inputs,
        variable_pool,
        output_payload,
    );
    let mut payload = Map::new();

    for key in [
        "text",
        "tool_calls",
        "finish_reason",
        "usage",
        "provider_route",
        "response_id",
        "provider_metadata",
    ] {
        if let Some(value) = output_payload.get(key) {
            payload.insert(key.to_string(), value.clone());
        }
    }
    payload.insert(
        "callback_kind".to_string(),
        Value::String(LLM_TOOL_CALLBACK_KIND.to_string()),
    );
    payload.insert("history".to_string(), Value::Array(history));

    Value::Object(payload)
}

fn variable_pool_with_pending_llm_tool_callback(
    node: &CompiledNode,
    resolved_inputs: &Map<String, Value>,
    variable_pool: &Map<String, Value>,
    output_payload: &Value,
) -> Map<String, Value> {
    let mut checkpoint_variable_pool = variable_pool.clone();
    let history = llm_callback_history_after_assistant_tool_call(
        node,
        resolved_inputs,
        variable_pool,
        output_payload,
    );
    let mut callback_state = Map::new();
    callback_state.insert(
        "callback_kind".to_string(),
        Value::String(LLM_TOOL_CALLBACK_KIND.to_string()),
    );
    callback_state.insert(
        "pending_tool_calls".to_string(),
        output_payload
            .get("tool_calls")
            .cloned()
            .unwrap_or_else(|| Value::Array(Vec::new())),
    );
    callback_state.insert("history".to_string(), Value::Array(history));
    if let Some(response_id) = output_payload
        .get("response_id")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
    {
        callback_state.insert(
            "response_id".to_string(),
            Value::String(response_id.to_string()),
        );
    }
    if let Some(provider_route) = output_payload.get("provider_route") {
        callback_state.insert("provider_route".to_string(), provider_route.clone());
    }
    if let Some(provider_metadata) = output_payload.get("provider_metadata") {
        callback_state.insert("provider_metadata".to_string(), provider_metadata.clone());
    }
    let mut node_state = Map::new();
    node_state.insert(
        LLM_TOOL_CALLBACK_STATE_KEY.to_string(),
        Value::Object(callback_state),
    );
    checkpoint_variable_pool.insert(node.node_id.clone(), Value::Object(node_state));
    checkpoint_variable_pool
}

fn llm_callback_history_after_assistant_tool_call(
    node: &CompiledNode,
    resolved_inputs: &Map<String, Value>,
    variable_pool: &Map<String, Value>,
    output_payload: &Value,
) -> Vec<Value> {
    let mut history = if let Some(history) = pending_llm_tool_callback_history(node, variable_pool)
    {
        history
    } else {
        let mut history = compatible_history_messages(node, resolved_inputs, variable_pool);
        history.extend(prompt_messages_from_bindings(None, resolved_inputs));
        history
    };
    let mut assistant_message = Map::new();
    assistant_message.insert("role".to_string(), Value::String("assistant".to_string()));
    assistant_message.insert(
        "content".to_string(),
        Value::String(
            output_payload
                .get("text")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
        ),
    );
    assistant_message.insert(
        "tool_calls".to_string(),
        output_payload
            .get("tool_calls")
            .cloned()
            .unwrap_or_else(|| Value::Array(Vec::new())),
    );
    if let Some(usage) = output_payload.get("usage") {
        assistant_message.insert("usage".to_string(), usage.clone());
    }
    history.push(Value::Object(assistant_message));
    history
}

fn append_llm_tool_result_messages(
    variable_pool: &mut Map<String, Value>,
    waiting_node_id: &str,
    resume_payload: &Value,
) -> Result<()> {
    let state = pending_llm_tool_callback_state(variable_pool, waiting_node_id)
        .ok_or_else(|| anyhow!("llm tool callback state not found for {waiting_node_id}"))?;
    let response_id = state
        .get("response_id")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(ToOwned::to_owned);
    let provider_route = state.get("provider_route").cloned();
    let provider_metadata = state.get("provider_metadata").cloned();
    let pending_tool_calls = state
        .get("pending_tool_calls")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("llm tool callback state is missing pending_tool_calls"))?;
    let tool_results = resume_payload
        .get("tool_results")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("llm tool callback resume payload requires tool_results"))?;
    let mut history = state
        .get("history")
        .and_then(Value::as_array)
        .cloned()
        .ok_or_else(|| anyhow!("llm tool callback state is missing history"))?;
    let mut expected_ids = BTreeSet::new();
    let mut ordered_ids = Vec::new();
    let mut pending_tool_names_by_id = BTreeMap::new();
    let mut delta_messages = Vec::new();

    for tool_call in pending_tool_calls {
        let id = tool_call
            .get("id")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("pending tool call is missing id"))?;
        expected_ids.insert(id.to_string());
        ordered_ids.push(id.to_string());
        if let Some(name) = tool_call
            .get("name")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
        {
            pending_tool_names_by_id.insert(id.to_string(), name.to_string());
        }
    }

    let mut results_by_id = BTreeMap::new();
    for tool_result in tool_results {
        let object = tool_result
            .as_object()
            .ok_or_else(|| anyhow!("tool result must be an object"))?;
        let tool_call_id = object
            .get("tool_call_id")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("tool result is missing tool_call_id"))?;
        if !expected_ids.contains(tool_call_id) {
            return Err(anyhow!("unexpected tool result for {tool_call_id}"));
        }
        if results_by_id
            .insert(tool_call_id.to_string(), object.clone())
            .is_some()
        {
            return Err(anyhow!("duplicate tool result for {tool_call_id}"));
        }
    }
    for expected_id in &ordered_ids {
        if !results_by_id.contains_key(expected_id) {
            return Err(anyhow!("missing tool result for {expected_id}"));
        }
    }

    for tool_call_id in ordered_ids {
        let result = results_by_id
            .remove(&tool_call_id)
            .ok_or_else(|| anyhow!("missing tool result for {tool_call_id}"))?;
        let mut message = Map::new();
        message.insert("role".to_string(), Value::String("tool".to_string()));
        message.insert(
            "tool_call_id".to_string(),
            Value::String(tool_call_id.clone()),
        );
        message.insert(
            "content".to_string(),
            result
                .get("content")
                .cloned()
                .map(tool_result_content_value)
                .unwrap_or_else(|| Value::String(String::new())),
        );
        let name = result
            .get("name")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .map(ToOwned::to_owned)
            .or_else(|| pending_tool_names_by_id.get(&tool_call_id).cloned());
        if let Some(name) = name {
            message.insert("name".to_string(), Value::String(name.to_string()));
        }
        let message = Value::Object(message);
        history.push(message.clone());
        delta_messages.push(message);
    }

    let mut callback_state = Map::new();
    callback_state.insert(
        "callback_kind".to_string(),
        Value::String(LLM_TOOL_CALLBACK_KIND.to_string()),
    );
    callback_state.insert("history".to_string(), Value::Array(history));
    if let Some(response_id) = response_id {
        callback_state.insert("response_id".to_string(), Value::String(response_id));
    }
    if let Some(provider_route) = provider_route {
        callback_state.insert("provider_route".to_string(), provider_route);
    }
    if let Some(provider_metadata) = provider_metadata {
        callback_state.insert("provider_metadata".to_string(), provider_metadata);
    }
    if !delta_messages.is_empty() {
        callback_state.insert("delta_messages".to_string(), Value::Array(delta_messages));
    }
    let mut node_state = Map::new();
    node_state.insert(
        LLM_TOOL_CALLBACK_STATE_KEY.to_string(),
        Value::Object(callback_state),
    );
    variable_pool.insert(waiting_node_id.to_string(), Value::Object(node_state));

    Ok(())
}

fn tool_result_content_value(value: Value) -> Value {
    match value {
        Value::String(_) => value,
        other => Value::String(other.to_string()),
    }
}

fn pending_llm_tool_callback_state<'a>(
    variable_pool: &'a Map<String, Value>,
    node_id: &str,
) -> Option<&'a Map<String, Value>> {
    variable_pool
        .get(node_id)?
        .get(LLM_TOOL_CALLBACK_STATE_KEY)?
        .as_object()
}

fn pending_llm_tool_callback_history(
    node: &CompiledNode,
    variable_pool: &Map<String, Value>,
) -> Option<Vec<Value>> {
    pending_llm_tool_callback_state(variable_pool, &node.node_id)?
        .get("history")?
        .as_array()
        .cloned()
}

fn pending_llm_tool_callback_delta_messages(
    node: &CompiledNode,
    variable_pool: &Map<String, Value>,
) -> Option<Vec<Value>> {
    pending_llm_tool_callback_state(variable_pool, &node.node_id)?
        .get("delta_messages")?
        .as_array()
        .cloned()
}

fn pending_llm_tool_callback_system(
    node: &CompiledNode,
    variable_pool: &Map<String, Value>,
) -> Option<String> {
    let history = pending_llm_tool_callback_history(node, variable_pool)?;
    provider_messages_from_prompt_messages(history).0
}

fn pending_llm_tool_callback_previous_response_id(
    node: &CompiledNode,
    runtime: &CompiledLlmRuntime,
    variable_pool: &Map<String, Value>,
) -> Option<String> {
    let state = pending_llm_tool_callback_state(variable_pool, &node.node_id)?;
    if !pending_llm_tool_callback_route_matches(runtime, state.get("provider_route")?) {
        return None;
    }
    if !pending_llm_tool_callback_uses_responses_websocket_cursor(state) {
        return None;
    }
    state
        .get("response_id")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(ToOwned::to_owned)
}

fn pending_llm_tool_callback_uses_responses_websocket_cursor(state: &Map<String, Value>) -> bool {
    state
        .get("provider_metadata")
        .and_then(|metadata| metadata.get("transport"))
        .and_then(Value::as_str)
        == Some(RESPONSES_WEBSOCKET_TRANSPORT)
}

fn pending_llm_tool_callback_route_matches(
    runtime: &CompiledLlmRuntime,
    provider_route: &Value,
) -> bool {
    let Some(provider_route) = provider_route.as_object() else {
        return false;
    };

    provider_route
        .get("provider_instance_id")
        .and_then(Value::as_str)
        == Some(runtime.provider_instance_id.as_str())
        && provider_route.get("provider_code").and_then(Value::as_str)
            == Some(runtime.provider_code.as_str())
        && provider_route.get("protocol").and_then(Value::as_str) == Some(runtime.protocol.as_str())
        && provider_route.get("model").and_then(Value::as_str) == Some(runtime.model.as_str())
}

fn build_empty_prompt_messages_error_payload(runtime: &CompiledLlmRuntime) -> Value {
    json!({
        "provider_instance_id": runtime.provider_instance_id,
        "provider_code": runtime.provider_code,
        "protocol": runtime.protocol,
        "error_kind": "prompt_messages_empty",
        "message": "LLM node requires at least one non-empty user or assistant prompt message",
    })
}

fn build_binding_resolution_error_payload(error: &anyhow::Error) -> Value {
    let message = error.to_string();
    let error_kind = if message.contains("unresolved template selector") {
        "prompt_template_unresolved"
    } else {
        "binding_resolution_failed"
    };

    json!({
        "error_kind": error_kind,
        "message": message,
    })
}

fn build_answer_binding_resolution_error_payload(
    node: &CompiledNode,
    issues: &[BindingResolutionIssue],
) -> Value {
    let error_kind = if issues
        .iter()
        .any(|issue| issue.selector.is_some() || issue.message.contains("selector"))
    {
        "prompt_template_unresolved"
    } else {
        "binding_resolution_failed"
    };
    let message = if issues.len() == 1 {
        let issue = &issues[0];
        format!(
            "failed to resolve binding {} for {}: {}",
            issue.binding_key, node.node_id, issue.message
        )
    } else {
        format!(
            "failed to resolve {} bindings for {}",
            issues.len(),
            node.node_id
        )
    };
    let details = issues
        .iter()
        .map(|issue| {
            json!({
                "binding_key": issue.binding_key,
                "selector": issue.selector.as_ref().map(|selector| selector.join(".")),
                "selector_path": issue.selector,
                "message": issue.message,
            })
        })
        .collect::<Vec<_>>();

    json!({
        "error_kind": error_kind,
        "message": message,
        "details": details,
    })
}

fn build_response_format(config: &Value) -> Option<Value> {
    let response_format = config.get("response_format")?;

    if response_format
        .get("mode")
        .and_then(Value::as_str)
        .is_some_and(|mode| mode == "text")
    {
        return None;
    }

    Some(response_format.clone())
}

const LLM_CONTEXT_SOURCE_KEY: &str = "__context_source";

fn llm_context_policy(node: &CompiledNode, runtime: &CompiledLlmRuntime) -> Value {
    runtime
        .routing
        .as_ref()
        .map(|routing| routing.context_policy.clone())
        .filter(|value| value.is_object())
        .or_else(|| node.config.get("context_policy").cloned())
        .filter(|value| value.is_object())
        .unwrap_or_else(|| json!({ "integration_context": "enabled" }))
}

fn integration_context_enabled(context_policy: &Value) -> bool {
    context_policy
        .get("integration_context")
        .and_then(Value::as_str)
        != Some("disabled")
}

fn binding_prompt_messages<'a>(
    node: &'a CompiledNode,
    rendered_templates: &'a Map<String, Value>,
    resolved_inputs: &'a Map<String, Value>,
    variable_pool: &'a Map<String, Value>,
) -> Vec<Value> {
    if let Some(history) = pending_llm_tool_callback_history(node, variable_pool) {
        return history;
    }

    let mut messages = compatible_history_messages(node, resolved_inputs, variable_pool);
    messages.extend(prompt_messages_from_bindings(
        Some(rendered_templates),
        resolved_inputs,
    ));
    messages
}

fn binding_prompt_messages_with_context_sources(
    node: &CompiledNode,
    rendered_templates: &Map<String, Value>,
    resolved_inputs: &Map<String, Value>,
    variable_pool: &Map<String, Value>,
    context_policy: &Value,
) -> Vec<Value> {
    if let Some(history) = pending_llm_tool_callback_history(node, variable_pool) {
        return annotate_prompt_messages(
            history,
            "pending_tool_callback_history",
            format!("{}.{}", node.node_id, LLM_TOOL_CALLBACK_STATE_KEY),
        );
    }

    let mut messages = Vec::new();
    if integration_context_enabled(context_policy) {
        messages.extend(run_level_system_prompt_messages(
            node,
            resolved_inputs,
            variable_pool,
        ));
    }
    messages.extend(compatible_history_messages_with_context_sources(
        node,
        resolved_inputs,
        variable_pool,
    ));
    messages.extend(annotate_prompt_messages(
        prompt_messages_from_bindings(Some(rendered_templates), resolved_inputs),
        "node_prompt",
        "bindings.prompt_messages".to_string(),
    ));
    messages
}

fn run_level_system_prompt_messages(
    node: &CompiledNode,
    resolved_inputs: &Map<String, Value>,
    variable_pool: &Map<String, Value>,
) -> Vec<Value> {
    let mut messages = Vec::new();
    if let Some(system) = resolved_inputs
        .get("system")
        .and_then(value_to_text)
        .and_then(|value| {
            let trimmed = value.trim();
            (!trimmed.is_empty()).then(|| trimmed.to_string())
        })
    {
        messages.push(system_prompt_message_with_source(
            &system,
            "run_level_system",
            "resolved_inputs.system",
        ));
    }

    for node_id in &node.dependency_node_ids {
        if let Some(system) = variable_pool
            .get(node_id)
            .and_then(|payload| payload.get("system"))
            .and_then(value_to_text)
            .and_then(|value| {
                let trimmed = value.trim();
                (!trimmed.is_empty()).then(|| trimmed.to_string())
            })
        {
            messages.push(system_prompt_message_with_source(
                &system,
                "run_level_system",
                format!("{node_id}.system"),
            ));
        }
    }

    messages
}

fn system_prompt_message_with_source(
    content: &str,
    source_kind: &str,
    source: impl Into<String>,
) -> Value {
    let mut message = Map::new();
    message.insert("role".to_string(), Value::String("system".to_string()));
    message.insert("content".to_string(), Value::String(content.to_string()));
    message.insert(
        LLM_CONTEXT_SOURCE_KEY.to_string(),
        json!({
            "source_kind": source_kind,
            "source": source.into(),
            "target": "effective_system",
        }),
    );
    Value::Object(message)
}

fn compatible_history_messages_with_context_sources(
    node: &CompiledNode,
    resolved_inputs: &Map<String, Value>,
    variable_pool: &Map<String, Value>,
) -> Vec<Value> {
    let direct_history = resolved_inputs
        .get("history")
        .and_then(Value::as_array)
        .cloned();
    if let Some(history) = direct_history {
        return annotate_prompt_messages(history, "history", "resolved_inputs.history".to_string());
    }

    node.dependency_node_ids
        .iter()
        .filter_map(|node_id| {
            variable_pool
                .get(node_id)?
                .get("history")
                .and_then(Value::as_array)
                .cloned()
                .filter(|history| !history.is_empty())
                .map(|history| {
                    annotate_prompt_messages(history, "history", format!("{node_id}.history"))
                })
        })
        .next()
        .unwrap_or_default()
}

fn annotate_prompt_messages(messages: Vec<Value>, source_kind: &str, source: String) -> Vec<Value> {
    messages
        .into_iter()
        .enumerate()
        .map(|(index, message)| annotate_prompt_message(message, source_kind, &source, index))
        .collect()
}

fn annotate_prompt_message(message: Value, source_kind: &str, source: &str, index: usize) -> Value {
    match message {
        Value::Object(mut object) => {
            object.insert(
                LLM_CONTEXT_SOURCE_KEY.to_string(),
                json!({
                    "source": source,
                    "source_kind": source_kind,
                    "message_index": index,
                    "target": "effective_system",
                }),
            );
            Value::Object(object)
        }
        other => other,
    }
}

fn prompt_messages_from_bindings(
    rendered_templates: Option<&Map<String, Value>>,
    resolved_inputs: &Map<String, Value>,
) -> Vec<Value> {
    rendered_templates
        .and_then(|templates| templates.get("prompt_messages"))
        .or_else(|| resolved_inputs.get("prompt_messages"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

fn provider_messages_from_prompt_messages(
    prompt_messages: Vec<Value>,
) -> (Option<String>, Vec<ProviderMessage>) {
    let context = provider_context_from_prompt_messages(prompt_messages);

    (context.system, context.messages)
}

fn provider_context_from_prompt_messages(prompt_messages: Vec<Value>) -> ProviderPromptContext {
    let mut system_parts = Vec::new();
    let mut messages = Vec::new();
    let mut compatibility_promotions = Vec::new();
    let mut system_sources = Vec::new();

    for (index, message) in prompt_messages.iter().enumerate() {
        let content = message
            .get("content")
            .and_then(value_to_text)
            .unwrap_or_default();

        let carries_tool_payload = message.get("tool_calls").is_some()
            || message.get("tool_call_id").is_some()
            || message.get("content_blocks").is_some();
        if content.trim().is_empty() && !carries_tool_payload {
            continue;
        }

        let role = message
            .get("role")
            .and_then(Value::as_str)
            .map(provider_message_role)
            .unwrap_or(ProviderMessageRole::User);

        if role == ProviderMessageRole::System {
            system_parts.push(content);
            let source = system_source_payload(message, index);
            if source.get("source_kind").and_then(Value::as_str) == Some("history") {
                compatibility_promotions.push(source.clone());
            }
            system_sources.push(source);
        } else {
            messages.push(ProviderMessage {
                role,
                content,
                name: message
                    .get("name")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned),
                tool_call_id: message
                    .get("tool_call_id")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned),
                tool_calls: message.get("tool_calls").map(provider_tool_calls_payload),
                content_blocks: message.get("content_blocks").cloned(),
            });
        }
    }

    let system = (!system_parts.is_empty()).then(|| system_parts.join("\n\n"));

    ProviderPromptContext {
        system,
        messages,
        compatibility_promotions,
        system_sources,
    }
}

fn system_source_payload(message: &Value, fallback_index: usize) -> Value {
    let source = message
        .get(LLM_CONTEXT_SOURCE_KEY)
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_else(Map::new);

    json!({
        "source": source
            .get("source")
            .and_then(Value::as_str)
            .unwrap_or("prompt_messages"),
        "source_kind": source
            .get("source_kind")
            .and_then(Value::as_str)
            .unwrap_or("prompt_messages"),
        "message_index": source
            .get("message_index")
            .and_then(Value::as_u64)
            .unwrap_or(fallback_index as u64),
        "target": "effective_system",
    })
}

fn provider_tool_calls_payload(tool_calls: &Value) -> Value {
    let Some(tool_calls) = tool_calls.as_array() else {
        return tool_calls.clone();
    };

    Value::Array(
        tool_calls
            .iter()
            .map(|tool_call| {
                let Some(object) = tool_call.as_object() else {
                    return tool_call.clone();
                };
                let mut provider_tool_call = object.clone();
                provider_tool_call.remove("call_usage");
                provider_tool_call.remove("call_input_tokens");
                provider_tool_call.remove("call_cached_input_tokens");
                provider_tool_call.remove("call_output_tokens");
                provider_tool_call.remove("result_input_tokens");
                provider_tool_call.remove("result_context_usage");
                provider_tool_call.remove("result_context_input_tokens");
                provider_tool_call.remove("result_context_cached_input_tokens");
                provider_tool_call.remove("token_delta");
                provider_tool_call.remove("token_count_method");
                Value::Object(provider_tool_call)
            })
            .collect(),
    )
}

fn provider_message_role(role: &str) -> ProviderMessageRole {
    match role {
        "system" => ProviderMessageRole::System,
        "assistant" => ProviderMessageRole::Assistant,
        "tool" => ProviderMessageRole::Tool,
        _ => ProviderMessageRole::User,
    }
}

fn compatible_history_messages(
    node: &CompiledNode,
    resolved_inputs: &Map<String, Value>,
    variable_pool: &Map<String, Value>,
) -> Vec<Value> {
    if let Some(history) = pending_llm_tool_callback_history(node, variable_pool) {
        return history;
    }

    let direct_history = resolved_inputs
        .get("history")
        .and_then(Value::as_array)
        .cloned();
    if let Some(history) = direct_history {
        return history;
    }

    node.dependency_node_ids
        .iter()
        .filter_map(|node_id| variable_pool.get(node_id))
        .find_map(|payload| {
            payload
                .get("history")
                .and_then(Value::as_array)
                .cloned()
                .filter(|history| !history.is_empty())
        })
        .unwrap_or_default()
}

fn provider_tools(
    node: &CompiledNode,
    resolved_inputs: &Map<String, Value>,
    rendered_templates: &Map<String, Value>,
    variable_pool: &Map<String, Value>,
    runtime_context: &ExecutionRuntimeContext,
) -> Vec<Value> {
    for candidate in [
        rendered_templates.get("tools"),
        resolved_inputs.get("tools"),
        resolved_inputs
            .get("compatibility")
            .and_then(|value| value.get("tools")),
        node.config.get("tools"),
        node.config
            .get("compatibility")
            .and_then(|value| value.get("tools")),
    ]
    .into_iter()
    .flatten()
    {
        if let Some(tools) = candidate.as_array() {
            if !tools.is_empty() {
                return provider_tool_payloads(tools);
            }
        }
    }

    node.dependency_node_ids
        .iter()
        .filter_map(|node_id| variable_pool.get(node_id))
        .find_map(|payload| {
            payload
                .get("compatibility")
                .and_then(|compatibility| compatibility.get("tools"))
                .and_then(Value::as_array)
                .map(|tools| provider_tool_payloads(tools))
                .filter(|tools| !tools.is_empty())
                .or_else(|| {
                    payload
                        .get("tools")
                        .and_then(Value::as_array)
                        .map(|tools| provider_tool_payloads(tools))
                        .filter(|tools| !tools.is_empty())
                })
        })
        .unwrap_or_else(|| runtime_context.tools.clone())
}

fn run_level_provider_tools(plan: &CompiledPlan, variable_pool: &Map<String, Value>) -> Vec<Value> {
    for candidate in [
        variable_pool.get("tools"),
        variable_pool
            .get("compatibility")
            .and_then(|value| value.get("tools")),
    ]
    .into_iter()
    .flatten()
    {
        if let Some(tools) = candidate.as_array() {
            let provider_tools = provider_tool_payloads(tools);
            if !provider_tools.is_empty() {
                return provider_tools;
            }
        }
    }

    for node_id in &plan.topological_order {
        let Some(start_node) = plan.nodes.get(node_id) else {
            continue;
        };
        if start_node.node_type != "start" {
            continue;
        }
        let Some(payload) = variable_pool.get(node_id) else {
            continue;
        };
        for candidate in [
            payload.get("tools"),
            payload
                .get("compatibility")
                .and_then(|value| value.get("tools")),
        ]
        .into_iter()
        .flatten()
        {
            if let Some(tools) = candidate.as_array() {
                let provider_tools = provider_tool_payloads(tools);
                if !provider_tools.is_empty() {
                    return provider_tools;
                }
            }
        }
    }

    Vec::new()
}

fn provider_tool_payloads(tools: &[Value]) -> Vec<Value> {
    tools.iter().map(provider_tool_payload).collect()
}

fn provider_tool_payload(tool: &Value) -> Value {
    if tool.get("function").is_some() {
        return tool.clone();
    }

    let Some(object) = tool.as_object() else {
        return tool.clone();
    };
    let Some(name) = object
        .get("name")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|name| !name.is_empty())
    else {
        return tool.clone();
    };

    let mut function = Map::new();
    function.insert("name".to_string(), Value::String(name.to_string()));
    if let Some(description) = object
        .get("description")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        function.insert(
            "description".to_string(),
            Value::String(description.to_string()),
        );
    }
    if let Some(input_schema) = object.get("input_schema") {
        function.insert("parameters".to_string(), input_schema.clone());
    }

    json!({
        "type": "function",
        "function": Value::Object(function),
    })
}

fn value_to_text(value: &Value) -> Option<String> {
    match value {
        Value::Null => None,
        Value::String(text) => Some(text.clone()),
        other => Some(other.to_string()),
    }
}

fn build_model_parameters(
    node: &CompiledNode,
    runtime: &CompiledLlmRuntime,
    variable_pool: &Map<String, Value>,
) -> BTreeMap<String, Value> {
    let mut parameters = build_configured_model_parameters(&node.config);
    if llm_follows_external_reasoning(&node.config) {
        apply_external_reasoning_parameters(&mut parameters, runtime, variable_pool);
    }
    parameters
}

fn build_configured_model_parameters(config: &Value) -> BTreeMap<String, Value> {
    if let Some(items) = config
        .get("llm_parameters")
        .and_then(Value::as_object)
        .and_then(|value| value.get("items"))
        .and_then(Value::as_object)
    {
        return items
            .iter()
            .filter_map(|(key, item)| {
                let enabled = item
                    .get("enabled")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                let value = item.get("value").cloned().unwrap_or(Value::Null);
                enabled.then_some((key.clone(), value))
            })
            .collect();
    }

    [
        "temperature",
        "top_p",
        "presence_penalty",
        "frequency_penalty",
        "max_tokens",
        "seed",
    ]
    .into_iter()
    .filter_map(|key| {
        config
            .get(key)
            .cloned()
            .map(|value| (key.to_string(), value))
    })
    .collect()
}

fn llm_follows_external_reasoning(config: &Value) -> bool {
    config
        .get("external_reasoning_policy")
        .and_then(|value| value.get("follow_external_reasoning"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn apply_external_reasoning_parameters(
    parameters: &mut BTreeMap<String, Value>,
    runtime: &CompiledLlmRuntime,
    variable_pool: &Map<String, Value>,
) {
    let Some(reasoning) = variable_pool
        .get("sys")
        .and_then(|value| value.get("model_parameters"))
        .and_then(|value| value.get("reasoning"))
        .and_then(Value::as_object)
    else {
        return;
    };
    let enabled = reasoning
        .get("enabled")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let effort = reasoning
        .get("effort")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let budget_tokens = reasoning.get("budget_tokens").and_then(Value::as_u64);

    if is_anthropic_reasoning_runtime(runtime) {
        insert_model_parameter_if_absent(
            parameters,
            "thinking_type",
            json!(if enabled { "enabled" } else { "disabled" }),
        );
        if enabled {
            if let Some(budget_tokens) = budget_tokens {
                insert_model_parameter_if_absent(
                    parameters,
                    "thinking_budget_tokens",
                    json!(budget_tokens),
                );
            }
        }
        return;
    }

    if is_bailian_reasoning_runtime(runtime) {
        insert_model_parameter_if_absent(parameters, "enable_thinking", json!(enabled));
        if enabled {
            if let Some(effort) = effort {
                insert_model_parameter_if_absent(parameters, "reasoning_effort", json!(effort));
            }
        }
        return;
    }

    if is_openai_reasoning_runtime(runtime) && enabled {
        if let Some(effort) = effort {
            insert_model_parameter_if_absent(parameters, "reasoning_effort", json!(effort));
        }
    }
}

fn insert_model_parameter_if_absent(
    parameters: &mut BTreeMap<String, Value>,
    key: &'static str,
    value: Value,
) {
    parameters.entry(key.to_string()).or_insert(value);
}

fn is_openai_reasoning_runtime(runtime: &CompiledLlmRuntime) -> bool {
    runtime.provider_code == "openai"
        || runtime.provider_code == "openai_compatible"
        || runtime.protocol == "openai_responses"
        || runtime.protocol == "openai_compatible"
}

fn is_anthropic_reasoning_runtime(runtime: &CompiledLlmRuntime) -> bool {
    runtime.provider_code == "anthropic" || runtime.protocol == "anthropic_messages"
}

fn is_bailian_reasoning_runtime(runtime: &CompiledLlmRuntime) -> bool {
    runtime.provider_code == "aliyun_bailian" || runtime.provider_code == "bailian"
}

fn first_output_key(node: &CompiledNode) -> String {
    node.outputs
        .first()
        .map(|output| output.key.clone())
        .unwrap_or_else(|| "result".to_string())
}

fn template_output_payload(
    node: &CompiledNode,
    output_key: String,
    output_value: Value,
    variable_pool: &Map<String, Value>,
) -> Value {
    let mut payload = Map::new();
    payload.insert(output_key, output_value);

    if node.node_type == "answer" {
        if let Some(sys) = variable_pool.get("sys") {
            payload.insert("sys".to_string(), sys.clone());
        }
        if let Some(env) = variable_pool.get("env") {
            payload.insert("env".to_string(), env.clone());
        }
    }

    Value::Object(payload)
}

fn answer_output_payload_with_error(
    mut output_payload: Value,
    error_payload: Option<&Value>,
) -> Value {
    if let (Value::Object(payload), Some(error_payload)) = (&mut output_payload, error_payload) {
        payload.insert("error".to_string(), error_payload.clone());
    }

    output_payload
}

fn can_continue_to_terminal_template_nodes(plan: &CompiledPlan, failed_node_index: usize) -> bool {
    let mut has_terminal_template_node = false;
    for node_id in plan.topological_order.iter().skip(failed_node_index + 1) {
        let Some(node) = plan.nodes.get(node_id) else {
            return false;
        };
        if !matches!(node.node_type.as_str(), "template_transform" | "answer") {
            return false;
        }
        has_terminal_template_node = true;
    }
    has_terminal_template_node
}

fn build_failed_llm_execution(
    node: &CompiledNode,
    runtime: &CompiledLlmRuntime,
    error_payload: Value,
    metrics_payload: Value,
    provider_events: Vec<ProviderStreamEvent>,
    include_output_payload: bool,
    invocation_messages: &[Value],
    invocation_debug_context: Option<&LlmInvocationDebugContext>,
) -> Result<LlmNodeExecution> {
    let mut executor_output = Map::new();
    executor_output.insert(
        first_output_key(node),
        Value::String(failed_llm_output_text(&error_payload)),
    );
    executor_output.insert(
        "provider_route".to_string(),
        build_llm_provider_route_payload(runtime),
    );
    executor_output.insert("finish_reason".to_string(), json!("error"));

    let raw = RawNodeExecutionResult {
        executor_output,
        metrics_facts: object_from_value(metrics_payload)?,
        error_facts: object_from_value(error_payload)?,
        debug_facts: build_llm_debug_facts(
            runtime,
            None,
            invocation_messages,
            None,
            invocation_debug_context,
        ),
        provider_events: provider_events.clone(),
    };
    let built = build_llm_node_payloads(node, raw)?;

    Ok(LlmNodeExecution {
        output_payload: if include_output_payload {
            built.output_payload
        } else {
            json!({})
        },
        error_payload: Some(built.error_payload),
        metrics_payload: built.metrics_payload,
        debug_payload: built.debug_payload,
        provider_events,
    })
}

fn failed_llm_output_text(error_payload: &Value) -> String {
    error_payload
        .get("message")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .or_else(|| error_payload.get("error_message").and_then(Value::as_str))
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("LLM node failed")
        .to_string()
}

fn build_successful_llm_execution(
    node: &CompiledNode,
    runtime: &CompiledLlmRuntime,
    result: &ProviderInvocationResult,
    final_content: Option<String>,
    metrics_payload: Value,
    provider_events: Vec<ProviderStreamEvent>,
    invocation_messages: &[Value],
    invocation_debug_context: &LlmInvocationDebugContext,
) -> Result<LlmNodeExecution> {
    let raw_text = final_content.unwrap_or_default();
    let answer_text = strip_llm_think_tags(&raw_text);
    let mut executor_output = Map::new();
    executor_output.insert("text".to_string(), Value::String(raw_text));
    executor_output.insert(
        "provider_route".to_string(),
        build_llm_provider_route_payload(runtime),
    );
    if let Some(finish_reason) = result.finish_reason.as_ref() {
        executor_output.insert(
            "finish_reason".to_string(),
            serde_json::to_value(finish_reason).unwrap_or(Value::Null),
        );
    }
    if let Some(usage) = metrics_payload.get("usage").cloned() {
        executor_output.insert("usage".to_string(), usage);
    }
    if !result.tool_calls.is_empty() {
        executor_output.insert(
            "tool_calls".to_string(),
            tool_calls_with_call_usage(&result.tool_calls, metrics_payload.get("usage")),
        );
    }
    if !result.mcp_calls.is_empty() {
        executor_output.insert(
            "mcp_calls".to_string(),
            serde_json::to_value(&result.mcp_calls).unwrap_or(Value::Null),
        );
    }
    if !result.provider_metadata.is_null() {
        executor_output.insert(
            "provider_metadata".to_string(),
            result.provider_metadata.clone(),
        );
    }
    if let Some(response_id) = result
        .response_id
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        executor_output.insert(
            "response_id".to_string(),
            Value::String(response_id.to_string()),
        );
    }
    if declares_public_output(node, "structured_output")
        && is_structured_response_format(&node.config)
    {
        executor_output.insert(
            "structured_output".to_string(),
            parse_structured_llm_output(&answer_text),
        );
    }

    let debug_facts = build_llm_debug_facts(
        runtime,
        Some(result),
        invocation_messages,
        metrics_payload.get("usage"),
        Some(invocation_debug_context),
    );
    let raw = RawNodeExecutionResult {
        executor_output,
        metrics_facts: object_from_value(metrics_payload)?,
        error_facts: Map::new(),
        debug_facts,
        provider_events: provider_events.clone(),
    };
    let built = build_llm_node_payloads(node, raw)?;

    Ok(LlmNodeExecution {
        output_payload: built.output_payload,
        error_payload: None,
        metrics_payload: built.metrics_payload,
        debug_payload: built.debug_payload,
        provider_events,
    })
}

fn build_llm_node_payloads(
    node: &CompiledNode,
    raw: RawNodeExecutionResult,
) -> Result<BuiltNodePayloads> {
    PublicOutputContract::from_compiled_outputs(&node.outputs)?.build_node_payloads(raw)
}

fn project_node_variable_payload(node: &CompiledNode, output_payload: &Value) -> Result<Value> {
    PublicOutputContract::from_compiled_outputs(&node.outputs)?
        .project_variable_payload(output_payload)
}

fn object_from_value(value: Value) -> Result<Map<String, Value>> {
    value
        .as_object()
        .cloned()
        .ok_or_else(|| anyhow!("payload bucket facts must be an object"))
}

fn build_llm_provider_route_payload(runtime: &CompiledLlmRuntime) -> Value {
    json!({
        "provider_instance_id": runtime.provider_instance_id,
        "provider_code": runtime.provider_code,
        "protocol": runtime.protocol,
        "model": runtime.model,
    })
}

fn build_llm_debug_facts(
    runtime: &CompiledLlmRuntime,
    result: Option<&ProviderInvocationResult>,
    invocation_messages: &[Value],
    result_usage: Option<&Value>,
    invocation_debug_context: Option<&LlmInvocationDebugContext>,
) -> Map<String, Value> {
    let mut debug = Map::new();
    let assistant_content = result
        .and_then(|result| result.final_content.as_deref())
        .unwrap_or_default();

    debug.insert(
        "assistant_message".to_string(),
        json!({
            "role": "assistant",
            "content": assistant_content,
        }),
    );
    let llm_rounds = build_llm_round_timeline(invocation_messages, result, result_usage);
    if !llm_rounds.is_empty() {
        debug.insert("llm_rounds".to_string(), Value::Array(llm_rounds));
    }
    if result.is_none() {
        debug.insert(
            "provider_route".to_string(),
            build_llm_provider_route_payload(runtime),
        );
    }
    if let Some(invocation_debug_context) = invocation_debug_context {
        debug.insert(
            "llm_context".to_string(),
            invocation_debug_context.to_payload(),
        );
    }

    debug
}

fn build_llm_round_timeline(
    invocation_messages: &[Value],
    result: Option<&ProviderInvocationResult>,
    result_usage: Option<&Value>,
) -> Vec<Value> {
    let mut rounds = Vec::new();

    for message in invocation_messages {
        match message.get("role").and_then(Value::as_str) {
            Some("assistant") => {
                let mut round = Map::new();
                round.insert("round_index".to_string(), json!(rounds.len()));
                round.insert("assistant".to_string(), message.clone());
                round.insert("tool_results".to_string(), Value::Array(Vec::new()));
                if let Some(usage) = message.get("usage") {
                    round.insert("usage".to_string(), usage.clone());
                }
                rounds.push(Value::Object(round));
            }
            Some("tool") => {
                if let Some(round) = rounds.last_mut().and_then(Value::as_object_mut) {
                    if let Some(tool_results) =
                        round.get_mut("tool_results").and_then(Value::as_array_mut)
                    {
                        tool_results.push(message.clone());
                    }
                }
            }
            _ => {}
        }
    }

    if let Some(result) = result {
        if let Some(result_usage) = result_usage {
            apply_result_context_usage_to_last_tool_results(&mut rounds, result_usage);
        }
        let mut round = Map::new();
        round.insert("round_index".to_string(), json!(rounds.len()));
        round.insert(
            "assistant".to_string(),
            provider_result_assistant_debug_payload(result, result_usage),
        );
        if let Some(result_usage) = result_usage {
            round.insert("usage".to_string(), result_usage.clone());
        }
        if let Some(finish_reason) = result.finish_reason.as_ref() {
            round.insert(
                "finish_reason".to_string(),
                serde_json::to_value(finish_reason).unwrap_or(Value::Null),
            );
        }
        rounds.push(Value::Object(round));
    }

    rounds
}

fn provider_result_assistant_debug_payload(
    result: &ProviderInvocationResult,
    usage: Option<&Value>,
) -> Value {
    let mut payload = Map::new();
    payload.insert("role".to_string(), Value::String("assistant".to_string()));
    payload.insert(
        "content".to_string(),
        Value::String(result.final_content.clone().unwrap_or_default()),
    );
    if !result.tool_calls.is_empty() {
        payload.insert(
            "tool_calls".to_string(),
            tool_calls_with_call_usage(&result.tool_calls, usage),
        );
    }

    Value::Object(payload)
}

fn tool_calls_with_call_usage(tool_calls: &[ProviderToolCall], usage: Option<&Value>) -> Value {
    Value::Array(
        tool_calls
            .iter()
            .map(|tool_call| {
                let value = serde_json::to_value(tool_call).unwrap_or(Value::Null);
                let Some(mut object) = value.as_object().cloned() else {
                    return value;
                };
                if let Some(usage) = usage {
                    object.insert("call_usage".to_string(), usage.clone());
                }
                Value::Object(object)
            })
            .collect(),
    )
}

fn apply_result_context_usage_to_last_tool_results(rounds: &mut [Value], usage: &Value) {
    let Some(tool_results) = rounds
        .last_mut()
        .and_then(Value::as_object_mut)
        .and_then(|round| round.get_mut("tool_results"))
        .and_then(Value::as_array_mut)
    else {
        return;
    };

    for tool_result in tool_results {
        let Some(tool_result) = tool_result.as_object_mut() else {
            continue;
        };
        tool_result.insert("result_context_usage".to_string(), usage.clone());
    }
}

fn declares_public_output(node: &CompiledNode, key: &str) -> bool {
    node.outputs.iter().any(|output| output.key == key)
}

fn is_structured_response_format(config: &Value) -> bool {
    config
        .get("response_format")
        .and_then(|format| format.get("mode"))
        .and_then(Value::as_str)
        .is_some_and(|mode| matches!(mode, "json_object" | "json_schema"))
}

fn parse_structured_llm_output(text: &str) -> Value {
    serde_json::from_str(text).unwrap_or(Value::Null)
}

fn strip_llm_think_tags(text: &str) -> String {
    let mut answer = String::new();
    let mut remaining = text;

    while let Some(start) = remaining.find("<think>") {
        answer.push_str(&remaining[..start]);
        let after_start = &remaining[start + "<think>".len()..];
        if let Some(end) = after_start.find("</think>") {
            remaining = &after_start[end + "</think>".len()..];
        } else {
            remaining = "";
            break;
        }
    }
    answer.push_str(remaining);

    answer
}

fn resolve_final_llm_content(
    result_content: Option<String>,
    stream_content: Option<String>,
) -> Option<String> {
    match (result_content, stream_content) {
        (Some(_), Some(stream)) if stream.contains("<think>") => Some(stream),
        (Some(result), _) => Some(result),
        (None, stream) => stream,
    }
}

fn collect_dify_style_deltas(events: &[ProviderStreamEvent]) -> Option<String> {
    let mut content = String::new();

    for event in events {
        match event {
            ProviderStreamEvent::ReasoningDelta { delta } => {
                append_reasoning_delta(&mut content, delta);
            }
            ProviderStreamEvent::TextDelta { delta } => {
                append_text_delta(&mut content, delta);
            }
            _ => {}
        }
    }

    close_open_think_block(&mut content);
    (!content.is_empty()).then_some(content)
}

fn append_reasoning_delta(content: &mut String, delta: &str) {
    if delta.is_empty() {
        return;
    }

    if !has_open_think_block(content) {
        content.push_str("<think>");
    }
    content.push_str(delta);
}

fn append_text_delta(content: &mut String, delta: &str) {
    close_open_think_block(content);
    content.push_str(delta);
}

fn close_open_think_block(content: &mut String) {
    if has_open_think_block(content) {
        content.push_str("</think>");
    }
}

fn has_open_think_block(content: &str) -> bool {
    content.rfind("<think>") > content.rfind("</think>")
}

fn collect_usage(events: &[ProviderStreamEvent], result_usage: &ProviderUsage) -> ProviderUsage {
    let mut usage = result_usage.clone();
    for event in events {
        match event {
            ProviderStreamEvent::UsageSnapshot { usage: snapshot } => {
                usage = snapshot.clone();
            }
            ProviderStreamEvent::UsageDelta { usage: delta } => {
                apply_usage_delta(&mut usage, delta)
            }
            _ => {}
        }
    }
    usage
}

fn apply_usage_delta(target: &mut ProviderUsage, delta: &ProviderUsage) {
    add_usage_value(&mut target.input_tokens, delta.input_tokens);
    add_usage_value(
        &mut target.input_cache_hit_tokens,
        delta.input_cache_hit_tokens,
    );
    add_usage_value(
        &mut target.input_cache_miss_tokens,
        delta.input_cache_miss_tokens,
    );
    add_usage_value(&mut target.output_tokens, delta.output_tokens);
    add_usage_value(&mut target.reasoning_tokens, delta.reasoning_tokens);
    add_usage_value(&mut target.cache_read_tokens, delta.cache_read_tokens);
    add_usage_value(&mut target.cache_write_tokens, delta.cache_write_tokens);
    add_usage_value(&mut target.total_tokens, delta.total_tokens);
}

fn add_usage_value(target: &mut Option<u64>, delta: Option<u64>) {
    if let Some(delta) = delta {
        *target = Some(target.unwrap_or_default() + delta);
    }
}

fn finish_reason_from_events(events: &[ProviderStreamEvent]) -> Option<ProviderFinishReason> {
    events.iter().rev().find_map(|event| match event {
        ProviderStreamEvent::Finish { reason } => Some(reason.clone()),
        _ => None,
    })
}

fn invalid_tool_call_finish_error(
    finish_reason: Option<&ProviderFinishReason>,
    result: &ProviderInvocationResult,
) -> Option<ProviderRuntimeError> {
    (matches!(finish_reason, Some(ProviderFinishReason::ToolCall)) && result.tool_calls.is_empty())
        .then(|| {
            ProviderRuntimeError::new(
                ProviderRuntimeErrorKind::ProviderInvalidResponse,
                "provider returned finish_reason=tool_call without tool_calls",
            )
        })
}

fn first_provider_error(events: &[ProviderStreamEvent]) -> Option<&ProviderRuntimeError> {
    events.iter().find_map(|event| match event {
        ProviderStreamEvent::Error { error } => Some(error),
        _ => None,
    })
}

fn content_delta_seen_before_terminal_failure(
    events: &[ProviderStreamEvent],
    finish_reason: Option<&ProviderFinishReason>,
) -> bool {
    let mut saw_content_delta = false;
    for event in events {
        match event {
            ProviderStreamEvent::TextDelta { .. } | ProviderStreamEvent::ReasoningDelta { .. } => {
                saw_content_delta = true
            }
            ProviderStreamEvent::Error { .. } => return saw_content_delta,
            ProviderStreamEvent::Finish {
                reason: ProviderFinishReason::Error,
            } => return saw_content_delta,
            _ => {}
        }
    }

    saw_content_delta && matches!(finish_reason, Some(ProviderFinishReason::Error))
}

fn build_provider_error_payload(
    runtime: &CompiledLlmRuntime,
    error: &ProviderRuntimeError,
) -> Value {
    json!({
        "provider_instance_id": runtime.provider_instance_id,
        "provider_code": runtime.provider_code,
        "protocol": runtime.protocol,
        "error_kind": serde_json::to_value(error.kind).unwrap_or(Value::Null),
        "message": sanitize_diagnostic_text(&error.message),
        "provider_summary": error
            .provider_summary
            .as_deref()
            .map(sanitize_diagnostic_text),
    })
}

fn provider_runtime_error_from_anyhow(error: &anyhow::Error) -> ProviderRuntimeError {
    if let Some(PluginFrameworkError::RuntimeContract { error }) =
        error.downcast_ref::<PluginFrameworkError>()
    {
        return normalize_runtime_contract_error(error);
    }

    ProviderRuntimeError::normalize("invoke", error.to_string(), None)
}

fn normalize_runtime_contract_error(error: &ProviderRuntimeError) -> ProviderRuntimeError {
    if error.kind != ProviderRuntimeErrorKind::ProviderInvalidResponse {
        return error.clone();
    }

    let normalized = ProviderRuntimeError::normalize(
        "invoke",
        &error.message,
        error.provider_summary.as_deref(),
    );
    if normalized.kind == ProviderRuntimeErrorKind::ProviderInvalidResponse {
        error.clone()
    } else {
        normalized
    }
}

fn sanitize_diagnostic_text(text: &str) -> String {
    let mut sanitized = text.to_string();
    for marker in [
        "bearer ",
        "authorization:",
        "\"authorization\":\"",
        "api_key=",
        "api_key:",
        "\"api_key\":\"",
        "token=",
        "secret=",
        "\"secret\":\"",
    ] {
        sanitized = redact_marker_value(&sanitized, marker);
    }
    sanitized = redact_prefixed_token(&sanitized, "sk-");
    let sanitized = sanitized.trim();
    if sanitized.chars().count() <= 240 {
        sanitized.to_string()
    } else {
        format!("{}...", sanitized.chars().take(240).collect::<String>())
    }
}

fn redact_marker_value(text: &str, marker: &str) -> String {
    let haystack = text.to_ascii_lowercase();
    let needle = marker.to_ascii_lowercase();
    let mut result = String::with_capacity(text.len());
    let mut cursor = 0;

    while let Some(offset) = haystack[cursor..].find(&needle) {
        let start = cursor + offset;
        let value_start = start + marker.len();
        result.push_str(&text[cursor..value_start]);
        let mut value_end = value_start;
        for ch in text[value_start..].chars() {
            if ch.is_whitespace() || matches!(ch, '"' | '\'' | ',' | '}' | ']' | '\n' | '\r') {
                break;
            }
            value_end += ch.len_utf8();
        }
        if value_end > value_start {
            result.push_str("[REDACTED]");
        }
        cursor = value_end;
    }

    result.push_str(&text[cursor..]);
    result
}

fn redact_prefixed_token(text: &str, prefix: &str) -> String {
    let haystack = text.to_ascii_lowercase();
    let needle = prefix.to_ascii_lowercase();
    let mut result = String::with_capacity(text.len());
    let mut cursor = 0;

    while let Some(offset) = haystack[cursor..].find(&needle) {
        let start = cursor + offset;
        result.push_str(&text[cursor..start]);
        result.push_str(prefix);
        result.push_str("[REDACTED]");
        let mut token_end = start + prefix.len();
        for ch in text[token_end..].chars() {
            if !(ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.')) {
                break;
            }
            token_end += ch.len_utf8();
        }
        cursor = token_end;
    }

    result.push_str(&text[cursor..]);
    result
}

#[cfg(test)]
mod input_cache_usage_tests {
    use super::*;

    #[test]
    fn usage_delta_accumulates_input_cache_hit_and_miss_tokens() {
        let mut usage = ProviderUsage {
            input_tokens: Some(100),
            input_cache_hit_tokens: Some(40),
            input_cache_miss_tokens: Some(60),
            output_tokens: Some(12),
            total_tokens: Some(112),
            ..ProviderUsage::default()
        };

        apply_usage_delta(
            &mut usage,
            &ProviderUsage {
                input_cache_hit_tokens: Some(5),
                input_cache_miss_tokens: Some(7),
                ..ProviderUsage::default()
            },
        );

        assert_eq!(usage.input_cache_hit_tokens, Some(45));
        assert_eq!(usage.input_cache_miss_tokens, Some(67));
        assert_eq!(usage.total_tokens(), Some(112));
    }
}

#[cfg(test)]
mod llm_round_timeline_tests {
    use super::*;

    #[test]
    fn llm_round_timeline_keeps_result_context_usage_without_token_delta() {
        let invocation_messages = vec![
            json!({
                "role": "assistant",
                "content": "need weather",
                "usage": {
                    "total_tokens": 8122
                },
                "tool_calls": [
                    {
                        "id": "call_weather",
                        "name": "lookup_weather"
                    }
                ]
            }),
            json!({
                "role": "tool",
                "tool_call_id": "call_weather",
                "content": "{\"temperature\":21}"
            }),
        ];
        let result = ProviderInvocationResult {
            final_content: Some("continue".into()),
            usage: ProviderUsage {
                total_tokens: Some(8224),
                ..ProviderUsage::default()
            },
            ..ProviderInvocationResult::default()
        };
        let result_usage = json!({
            "total_tokens": 8224
        });

        let rounds =
            build_llm_round_timeline(&invocation_messages, Some(&result), Some(&result_usage));

        assert_eq!(
            rounds[0]["tool_results"][0]["result_context_usage"]["total_tokens"],
            json!(8224)
        );
        assert!(rounds[0]["tool_results"][0].get("token_delta").is_none());
    }
}
