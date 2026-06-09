use super::*;

const VISIBLE_INTERNAL_LLM_TOOL_TYPE: &str = "visible_internal_llm_tool";
const VISIBLE_INTERNAL_LLM_TOOL_VARIABLE: &str = "visible_internal_llm_tool";
const VISIBLE_INTERNAL_LLM_TOOL_SOURCE_HANDLE_PREFIX: &str = "visible_internal_llm_tool:";
const VISIBLE_INTERNAL_LLM_TOOL_CALLBACK_STATE_KEY: &str = "__visible_internal_llm_tool_callback";
const MAX_VISIBLE_INTERNAL_LLM_TOOL_ROUNDS: usize = 8;
const TOOL_RESULT_NODE_TYPE: &str = "tool_result";

#[derive(Debug, Clone, PartialEq)]
pub(super) struct VisibleInternalLlmTool {
    pub(super) name: String,
    pub(super) description: Option<String>,
    pub(super) target_node_id: String,
    pub(super) input_schema: Option<Value>,
}

#[derive(Debug, Clone, PartialEq)]
struct VisibleInternalLlmToolOutput {
    text: String,
    provider_events: Vec<ProviderStreamEvent>,
    route_events: Vec<Value>,
}

enum VisibleInternalLlmToolBranchExecution {
    Completed(VisibleInternalLlmToolOutput),
    Waiting {
        wait: LlmToolCallbackWait,
        branch_text: String,
        route_events: Vec<Value>,
    },
    Failed {
        error_payload: Value,
        route_events: Vec<Value>,
    },
}

enum VisibleInternalLlmToolNodeExecution {
    Completed(Value),
    Waiting(LlmToolCallbackWait),
    Failed(Value),
}

#[derive(Debug, Clone, PartialEq)]
struct VisibleInternalLlmToolPendingCall {
    tool_call: Value,
    tool: VisibleInternalLlmTool,
}

enum VisibleInternalLlmToolRemainingExecution {
    Completed {
        tool_results: Vec<Value>,
        visible_transcript: String,
        provider_events: Vec<ProviderStreamEvent>,
        route_events: Vec<Value>,
    },
    Waiting(LlmToolCallbackWait),
    Failed {
        error_payload: Value,
        provider_events: Vec<ProviderStreamEvent>,
        route_events: Vec<Value>,
    },
}

struct VisibleInternalLlmToolCallbackStateInput<'a> {
    main_node_id: &'a str,
    tool_call: &'a Value,
    tool_name: &'a str,
    target_node_id: &'a str,
    main_visible_transcript: &'a str,
    branch_text: &'a str,
    route_events: &'a [Value],
    completed_tool_results: &'a [Value],
    remaining_tool_calls: &'a [VisibleInternalLlmToolPendingCall],
}

pub(super) enum VisibleInternalLlmToolResume {
    Ready(Map<String, Value>),
    Waiting(LlmToolCallbackWait),
    Failed {
        node_id: String,
        node_alias: String,
        execution: LlmNodeExecution,
    },
}

pub(super) fn has_visible_internal_llm_tool_callback_state(
    variable_pool: &Map<String, Value>,
) -> bool {
    variable_pool
        .get(VISIBLE_INTERNAL_LLM_TOOL_CALLBACK_STATE_KEY)
        .and_then(Value::as_object)
        .is_some()
}

fn visible_internal_llm_tools_enabled(node: &CompiledNode) -> bool {
    node.config
        .get("visible_internal_llm_tools_enabled")
        .or_else(|| node.config.get("visibleInternalLlmToolsEnabled"))
        .and_then(Value::as_bool)
        == Some(true)
}

pub(super) fn is_visible_internal_llm_tool_source_handle(source_handle: Option<&str>) -> bool {
    source_handle
        .map(|handle| handle.starts_with(VISIBLE_INTERNAL_LLM_TOOL_SOURCE_HANDLE_PREFIX))
        .unwrap_or(false)
}

pub(super) fn visible_internal_llm_tools(node: &CompiledNode) -> Vec<VisibleInternalLlmTool> {
    if !visible_internal_llm_tools_enabled(node) {
        return Vec::new();
    }

    node.config
        .get("visible_internal_llm_tools")
        .or_else(|| node.config.get("visibleInternalLlmTools"))
        .and_then(Value::as_array)
        .map(|tools| {
            tools
                .iter()
                .filter_map(visible_internal_llm_tool_from_value)
                .collect()
        })
        .unwrap_or_default()
}

pub(super) fn visible_internal_llm_tool_target_node_ids(plan: &CompiledPlan) -> BTreeSet<String> {
    plan.nodes
        .values()
        .filter(|node| node.node_type == "llm")
        .flat_map(visible_internal_llm_tools)
        .map(|tool| tool.target_node_id)
        .collect()
}

pub(super) fn visible_internal_llm_provider_tools(node: &CompiledNode) -> Vec<Value> {
    visible_internal_llm_tools(node)
        .into_iter()
        .map(|tool| {
            let mut function = Map::new();
            function.insert("name".to_string(), Value::String(tool.name.clone()));
            if let Some(description) = tool.description.clone() {
                function.insert("description".to_string(), Value::String(description));
            }
            if let Some(input_schema) = visible_internal_llm_tool_provider_input_schema(&tool) {
                function.insert("parameters".to_string(), input_schema);
            }

            json!({
                "type": "function",
                "function": Value::Object(function)
            })
        })
        .collect()
}

pub(super) fn visible_internal_llm_node_has_media_tool(node: &CompiledNode) -> bool {
    visible_internal_llm_tools(node)
        .iter()
        .any(visible_internal_llm_tool_supports_media_contract)
}

pub(super) async fn execute_llm_node_with_visible_internal_tools<I>(
    plan: &CompiledPlan,
    node: &CompiledNode,
    resolved_inputs: &Map<String, Value>,
    rendered_templates: &Map<String, Value>,
    variable_pool: &Map<String, Value>,
    runtime_context: &ExecutionRuntimeContext,
    invoker: &I,
) -> Result<LlmNodeExecution>
where
    I: ProviderInvoker + CapabilityInvoker + CodeInvoker + ?Sized,
{
    let tools = visible_internal_llm_tools(node);
    if tools.is_empty() {
        return execute_llm_node_provider_round(
            node,
            resolved_inputs,
            rendered_templates,
            variable_pool,
            runtime_context,
            invoker,
        )
        .await;
    }

    let mut llm_variable_pool = variable_pool.clone();
    let mut visible_transcript =
        pending_llm_tool_callback_visible_internal_transcript(node, variable_pool)
            .unwrap_or_default();
    let mut route_events = pending_llm_tool_callback_visible_internal_events(node, variable_pool);
    let mut provider_events = Vec::new();

    for round_index in 0..MAX_VISIBLE_INTERNAL_LLM_TOOL_ROUNDS {
        let mut execution = execute_llm_node_provider_round(
            node,
            resolved_inputs,
            rendered_templates,
            &llm_variable_pool,
            runtime_context,
            invoker,
        )
        .await?;
        sanitize_visible_internal_llm_execution(&mut execution, &tools);
        provider_events.extend(execution.provider_events.clone());

        if execution.error_payload.is_some() {
            if !provider_events.is_empty() {
                execution.provider_events = provider_events;
            }
            return Ok(execution);
        }

        let Some(tool_calls) = output_tool_calls(&execution.output_payload) else {
            append_output_text(&mut visible_transcript, &execution.output_payload);
            return Ok(execution_with_visible_transcript(
                execution,
                visible_transcript,
                provider_events,
                route_events,
            ));
        };

        let internal_tool_calls = visible_internal_tool_calls(&tool_calls, &tools);
        if internal_tool_calls.is_empty() {
            if !provider_events.is_empty() {
                execution.provider_events = provider_events;
            }
            attach_visible_internal_llm_tool_events(&mut execution, &route_events);
            return Ok(execution);
        }
        if internal_tool_calls.len() != tool_calls.len() {
            return visible_internal_llm_tool_failure(
                node,
                provider_events,
                json!({
                    "error_code": "visible_internal_llm_tool_mixed_tool_calls",
                    "message": "visible internal LLM tools cannot be mixed with external client tool calls in the same provider round",
                }),
                route_events,
            );
        }

        append_output_text(&mut visible_transcript, &execution.output_payload);
        llm_variable_pool = variable_pool_with_pending_llm_tool_callback(
            node,
            resolved_inputs,
            &llm_variable_pool,
            &execution.output_payload,
        );
        set_pending_llm_tool_callback_visible_internal_transcript(
            &mut llm_variable_pool,
            &node.node_id,
            visible_transcript.clone(),
        )?;
        let mut tool_results = Vec::new();
        for (index, (tool_call, tool)) in internal_tool_calls.iter().enumerate() {
            let target_output = match execute_visible_internal_llm_tool_call(
                plan,
                &llm_variable_pool,
                runtime_context,
                invoker,
                &node.node_id,
                tool_call,
                tool,
            )
            .await?
            {
                VisibleInternalLlmToolBranchExecution::Completed(output) => output,
                VisibleInternalLlmToolBranchExecution::Waiting {
                    mut wait,
                    branch_text,
                    route_events: waiting_route_events,
                } => {
                    route_events.extend(waiting_route_events);
                    let remaining_tool_calls = internal_tool_calls
                        .iter()
                        .skip(index + 1)
                        .map(|(tool_call, tool)| VisibleInternalLlmToolPendingCall {
                            tool_call: tool_call.clone(),
                            tool: (*tool).clone(),
                        })
                        .collect::<Vec<_>>();
                    insert_visible_internal_llm_tool_callback_state(
                        &mut wait.checkpoint_variable_pool,
                        VisibleInternalLlmToolCallbackStateInput {
                            main_node_id: &node.node_id,
                            tool_call,
                            tool_name: &tool.name,
                            target_node_id: &tool.target_node_id,
                            main_visible_transcript: &visible_transcript,
                            branch_text: &branch_text,
                            route_events: &route_events,
                            completed_tool_results: &tool_results,
                            remaining_tool_calls: &remaining_tool_calls,
                        },
                    );
                    let pending_visible_transcript = format!("{visible_transcript}{branch_text}");
                    let mut pending_execution = execution_with_visible_transcript(
                        execution,
                        pending_visible_transcript,
                        provider_events,
                        route_events,
                    );
                    pending_execution.pending_callback = Some(wait);
                    return Ok(pending_execution);
                }
                VisibleInternalLlmToolBranchExecution::Failed {
                    error_payload,
                    route_events: failed_route_events,
                } => {
                    route_events.extend(failed_route_events);
                    if visible_internal_llm_tool_error_is_recoverable(&error_payload) {
                        tool_results.push(visible_internal_llm_tool_result(
                            tool_call,
                            &tool.name,
                            visible_internal_llm_tool_error_result_content(&error_payload),
                        ));
                        continue;
                    }
                    return visible_internal_llm_tool_failure(
                        node,
                        provider_events,
                        error_payload,
                        route_events,
                    );
                }
            };
            provider_events.extend(target_output.provider_events);
            route_events.extend(target_output.route_events);
            visible_transcript.push_str(&target_output.text);
            tool_results.push(visible_internal_llm_tool_result(
                tool_call,
                &tool.name,
                target_output.text,
            ));
        }

        append_llm_tool_result_messages(
            &mut llm_variable_pool,
            &node.node_id,
            &json!({ "tool_results": tool_results }),
        )?;
        set_pending_llm_tool_callback_visible_internal_transcript(
            &mut llm_variable_pool,
            &node.node_id,
            visible_transcript.clone(),
        )?;
        set_pending_llm_tool_callback_visible_internal_events(
            &mut llm_variable_pool,
            &node.node_id,
            route_events.clone(),
        )?;

        if round_index + 1 == MAX_VISIBLE_INTERNAL_LLM_TOOL_ROUNDS {
            return visible_internal_llm_tool_failure(
                node,
                provider_events,
                json!({
                    "error_code": "visible_internal_llm_tool_round_limit",
                    "message": "visible internal LLM tool execution exceeded the maximum callback rounds",
                }),
                route_events,
            );
        }
    }

    visible_internal_llm_tool_failure(
        node,
        provider_events,
        json!({
            "error_code": "visible_internal_llm_tool_round_limit",
            "message": "visible internal LLM tool execution exceeded the maximum callback rounds",
        }),
        route_events,
    )
}

async fn execute_visible_internal_llm_tool_call<I>(
    plan: &CompiledPlan,
    variable_pool: &Map<String, Value>,
    runtime_context: &ExecutionRuntimeContext,
    invoker: &I,
    main_node_id: &str,
    tool_call: &Value,
    tool: &VisibleInternalLlmTool,
) -> Result<VisibleInternalLlmToolBranchExecution>
where
    I: ProviderInvoker + CapabilityInvoker + CodeInvoker + ?Sized,
{
    let mut tool_variable_pool = variable_pool.clone();
    let inherited_context =
        visible_internal_llm_tool_inherited_context(variable_pool, main_node_id);
    tool_variable_pool.insert(
        VISIBLE_INTERNAL_LLM_TOOL_VARIABLE.to_string(),
        json!({
            "tool_call_id": tool_call_id(tool_call),
            "tool_name": tool.name,
            "arguments": visible_internal_llm_tool_arguments(tool_call, &inherited_context),
            "context": inherited_context,
        }),
    );

    execute_visible_internal_llm_tool_branch(
        plan,
        tool_variable_pool,
        runtime_context,
        invoker,
        main_node_id,
        tool_call,
        tool,
    )
    .await
}

async fn execute_visible_internal_llm_tool_branch<I>(
    plan: &CompiledPlan,
    variable_pool: Map<String, Value>,
    runtime_context: &ExecutionRuntimeContext,
    invoker: &I,
    main_node_id: &str,
    tool_call: &Value,
    tool: &VisibleInternalLlmTool,
) -> Result<VisibleInternalLlmToolBranchExecution>
where
    I: ProviderInvoker + CapabilityInvoker + CodeInvoker + ?Sized,
{
    let mut route_events = vec![visible_internal_llm_tool_route_event(
        "visible_internal_llm_tool_started",
        main_node_id,
        tool_call,
        tool,
        json!({}),
    )];

    if !plan.nodes.contains_key(&tool.target_node_id) {
        let error_payload = json!({
            "error_code": "visible_internal_llm_tool_failed",
            "message": "visible internal LLM tool branch entry node was not found",
            "tool_call_id": tool_call_id(tool_call),
            "tool_name": tool.name,
            "target_node_id": tool.target_node_id,
        });
        route_events.push(visible_internal_llm_tool_route_event(
            "visible_internal_llm_tool_failed",
            main_node_id,
            tool_call,
            tool,
            json!({ "error_payload": error_payload }),
        ));
        return Ok(VisibleInternalLlmToolBranchExecution::Failed {
            error_payload,
            route_events,
        });
    }

    let active_node_ids = BTreeSet::from([tool.target_node_id.clone()]);

    continue_visible_internal_llm_tool_branch(
        plan,
        variable_pool,
        active_node_ids,
        runtime_context,
        invoker,
        main_node_id,
        tool_call,
        tool,
        String::new(),
        Vec::new(),
        route_events,
    )
    .await
}

async fn continue_visible_internal_llm_tool_branch<I>(
    plan: &CompiledPlan,
    mut variable_pool: Map<String, Value>,
    mut active_node_ids: BTreeSet<String>,
    runtime_context: &ExecutionRuntimeContext,
    invoker: &I,
    main_node_id: &str,
    tool_call: &Value,
    tool: &VisibleInternalLlmTool,
    mut branch_text: String,
    mut provider_events: Vec<ProviderStreamEvent>,
    mut route_events: Vec<Value>,
) -> Result<VisibleInternalLlmToolBranchExecution>
where
    I: ProviderInvoker + CapabilityInvoker + CodeInvoker + ?Sized,
{
    for node_id in &plan.topological_order {
        if !active_node_ids.contains(node_id) {
            continue;
        }
        let Some(node) = plan.nodes.get(node_id) else {
            continue;
        };
        let resolved_inputs = match resolve_node_inputs(node, &variable_pool) {
            Ok(inputs) => inputs,
            Err(error) => {
                let error_payload = visible_internal_llm_tool_node_error(
                    tool_call,
                    tool,
                    node,
                    "visible internal LLM tool input resolution failed",
                    Some(error.to_string()),
                    None,
                );
                route_events.push(visible_internal_llm_tool_route_event(
                    "visible_internal_llm_tool_failed",
                    main_node_id,
                    tool_call,
                    tool,
                    json!({
                        "node_id": node.node_id,
                        "error_payload": error_payload,
                    }),
                ));
                return Ok(VisibleInternalLlmToolBranchExecution::Failed {
                    error_payload,
                    route_events,
                });
            }
        };
        let rendered_templates = render_templated_bindings(node, &resolved_inputs);
        let output_payload = match execute_visible_internal_llm_tool_node(
            node,
            &resolved_inputs,
            &rendered_templates,
            &mut variable_pool,
            runtime_context,
            invoker,
            &mut provider_events,
        )
        .await?
        {
            VisibleInternalLlmToolNodeExecution::Completed(output_payload) => output_payload,
            VisibleInternalLlmToolNodeExecution::Waiting(wait) => {
                let branch_text = format!(
                    "{}{}",
                    branch_text,
                    wait.node_trace
                        .as_ref()
                        .map(|trace| visible_internal_llm_tool_output_text(&trace.output_payload))
                        .unwrap_or_default()
                );
                route_events.push(visible_internal_llm_tool_route_event(
                    "visible_internal_llm_tool_waiting_callback",
                    main_node_id,
                    tool_call,
                    tool,
                    json!({
                        "waiting_node_id": wait.node_id,
                        "waiting_node_alias": wait.node_alias,
                        "request_payload": wait.request_payload,
                    }),
                ));
                return Ok(VisibleInternalLlmToolBranchExecution::Waiting {
                    wait,
                    branch_text,
                    route_events,
                });
            }
            VisibleInternalLlmToolNodeExecution::Failed(error_payload) => {
                let error_payload = visible_internal_llm_tool_node_error(
                    tool_call,
                    tool,
                    node,
                    "visible internal LLM tool branch node failed",
                    None,
                    Some(error_payload),
                );
                route_events.push(visible_internal_llm_tool_route_event(
                    "visible_internal_llm_tool_failed",
                    main_node_id,
                    tool_call,
                    tool,
                    json!({
                        "node_id": node.node_id,
                        "error_payload": error_payload,
                    }),
                ));
                return Ok(VisibleInternalLlmToolBranchExecution::Failed {
                    error_payload,
                    route_events,
                });
            }
        };

        branch_text = visible_internal_llm_tool_output_text(&output_payload);
        if node.node_type == "llm" {
            route_events.push(visible_internal_llm_tool_route_event(
                "visible_internal_llm_tool_completed",
                main_node_id,
                tool_call,
                tool,
                json!({
                    "node_id": node.node_id,
                    "provider_route": output_payload
                        .get("provider_route")
                        .cloned()
                        .unwrap_or(Value::Null),
                }),
            ));
        }
        variable_pool.insert(
            node.node_id.clone(),
            project_node_variable_payload(node, &output_payload)?,
        );
        if node.node_type == TOOL_RESULT_NODE_TYPE {
            return Ok(VisibleInternalLlmToolBranchExecution::Completed(
                VisibleInternalLlmToolOutput {
                    text: branch_text,
                    provider_events,
                    route_events,
                },
            ));
        }
        activate_downstream_nodes(plan, &mut active_node_ids, node, None);
    }

    Ok(VisibleInternalLlmToolBranchExecution::Completed(
        VisibleInternalLlmToolOutput {
            text: branch_text,
            provider_events,
            route_events,
        },
    ))
}

async fn execute_visible_internal_llm_tool_node<I>(
    node: &CompiledNode,
    resolved_inputs: &Map<String, Value>,
    rendered_templates: &Map<String, Value>,
    variable_pool: &mut Map<String, Value>,
    runtime_context: &ExecutionRuntimeContext,
    invoker: &I,
    provider_events: &mut Vec<ProviderStreamEvent>,
) -> Result<VisibleInternalLlmToolNodeExecution>
where
    I: ProviderInvoker + CapabilityInvoker + CodeInvoker + ?Sized,
{
    match node.node_type.as_str() {
        "llm" => {
            let resolved_inputs =
                visible_internal_llm_tool_llm_resolved_inputs(resolved_inputs, variable_pool);
            let execution = execute_llm_node_provider_round(
                node,
                &resolved_inputs,
                rendered_templates,
                variable_pool,
                runtime_context,
                invoker,
            )
            .await?;
            provider_events.extend(execution.provider_events.clone());
            if let Some(error_payload) = execution.error_payload {
                return Ok(VisibleInternalLlmToolNodeExecution::Failed(error_payload));
            }
            if let Some(mut wait) = build_llm_tool_callback_wait(
                node,
                &resolved_inputs,
                variable_pool,
                &execution.output_payload,
            ) {
                wait.node_trace = Some(NodeExecutionTrace {
                    node_id: node.node_id.clone(),
                    node_type: node.node_type.clone(),
                    node_alias: node.alias.clone(),
                    input_payload: Value::Object(resolved_inputs.clone()),
                    output_payload: execution.output_payload.clone(),
                    error_payload: None,
                    metrics_payload: execution.metrics_payload.clone(),
                    debug_payload: execution.debug_payload.clone(),
                    provider_events: execution.provider_events,
                });
                return Ok(VisibleInternalLlmToolNodeExecution::Waiting(wait));
            }

            Ok(VisibleInternalLlmToolNodeExecution::Completed(
                execution.output_payload,
            ))
        }
        "template_transform" | "answer" | TOOL_RESULT_NODE_TYPE => {
            let output_key = first_output_key(node);
            let output_value = rendered_templates
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
            Ok(VisibleInternalLlmToolNodeExecution::Completed(
                template_output_payload(node, output_key, output_value, variable_pool),
            ))
        }
        "code" => {
            let execution = execute_code_node(node, resolved_inputs, invoker).await?;
            if let Some(error_payload) = execution.error_payload {
                return Ok(VisibleInternalLlmToolNodeExecution::Failed(error_payload));
            }

            Ok(VisibleInternalLlmToolNodeExecution::Completed(
                execution.output_payload,
            ))
        }
        "http_request" => {
            let execution =
                execute_http_request_node(node, resolved_inputs, variable_pool, None).await?;
            if let Some(error_payload) = execution.error_payload {
                return Ok(VisibleInternalLlmToolNodeExecution::Failed(error_payload));
            }

            Ok(VisibleInternalLlmToolNodeExecution::Completed(
                execution.output_payload,
            ))
        }
        "plugin_node" => {
            let execution =
                execute_capability_plugin_node(node, resolved_inputs, rendered_templates, invoker)
                    .await?;
            if let Some(error_payload) = execution.error_payload {
                return Ok(VisibleInternalLlmToolNodeExecution::Failed(error_payload));
            }

            Ok(VisibleInternalLlmToolNodeExecution::Completed(
                execution.output_payload,
            ))
        }
        "variable_assigner" => Ok(VisibleInternalLlmToolNodeExecution::Completed(
            execute_variable_assignment_node(node, resolved_inputs, variable_pool)?,
        )),
        unsupported => Ok(VisibleInternalLlmToolNodeExecution::Failed(json!({
            "message": format!("visible internal LLM tool branch node type {unsupported} is not supported"),
        }))),
    }
}

async fn execute_remaining_visible_internal_llm_tool_calls<I>(
    plan: &CompiledPlan,
    variable_pool: &Map<String, Value>,
    runtime_context: &ExecutionRuntimeContext,
    invoker: &I,
    main_node_id: &str,
    mut completed_tool_results: Vec<Value>,
    mut visible_transcript: String,
    pending_calls: Vec<VisibleInternalLlmToolPendingCall>,
) -> Result<VisibleInternalLlmToolRemainingExecution>
where
    I: ProviderInvoker + CapabilityInvoker + CodeInvoker + ?Sized,
{
    let mut provider_events = Vec::new();
    let mut route_events = Vec::new();
    for (index, pending_call) in pending_calls.iter().enumerate() {
        match execute_visible_internal_llm_tool_call(
            plan,
            variable_pool,
            runtime_context,
            invoker,
            main_node_id,
            &pending_call.tool_call,
            &pending_call.tool,
        )
        .await?
        {
            VisibleInternalLlmToolBranchExecution::Completed(output) => {
                provider_events.extend(output.provider_events);
                route_events.extend(output.route_events);
                visible_transcript.push_str(&output.text);
                completed_tool_results.push(visible_internal_llm_tool_result(
                    &pending_call.tool_call,
                    &pending_call.tool.name,
                    output.text,
                ));
            }
            VisibleInternalLlmToolBranchExecution::Waiting {
                mut wait,
                branch_text,
                route_events: waiting_route_events,
            } => {
                route_events.extend(waiting_route_events);
                let remaining_tool_calls = pending_calls
                    .iter()
                    .skip(index + 1)
                    .cloned()
                    .collect::<Vec<_>>();
                insert_visible_internal_llm_tool_callback_state(
                    &mut wait.checkpoint_variable_pool,
                    VisibleInternalLlmToolCallbackStateInput {
                        main_node_id,
                        tool_call: &pending_call.tool_call,
                        tool_name: &pending_call.tool.name,
                        target_node_id: &pending_call.tool.target_node_id,
                        main_visible_transcript: &visible_transcript,
                        branch_text: &branch_text,
                        route_events: &route_events,
                        completed_tool_results: &completed_tool_results,
                        remaining_tool_calls: &remaining_tool_calls,
                    },
                );
                return Ok(VisibleInternalLlmToolRemainingExecution::Waiting(wait));
            }
            VisibleInternalLlmToolBranchExecution::Failed {
                error_payload,
                route_events: failed_route_events,
            } => {
                route_events.extend(failed_route_events);
                if visible_internal_llm_tool_error_is_recoverable(&error_payload) {
                    completed_tool_results.push(visible_internal_llm_tool_result(
                        &pending_call.tool_call,
                        &pending_call.tool.name,
                        visible_internal_llm_tool_error_result_content(&error_payload),
                    ));
                    continue;
                }
                return Ok(VisibleInternalLlmToolRemainingExecution::Failed {
                    error_payload,
                    provider_events,
                    route_events,
                });
            }
        }
    }

    Ok(VisibleInternalLlmToolRemainingExecution::Completed {
        tool_results: completed_tool_results,
        visible_transcript,
        provider_events,
        route_events,
    })
}

pub(super) async fn resume_visible_internal_llm_tool_callback<I>(
    plan: &CompiledPlan,
    waiting_node_id: &str,
    mut variable_pool: Map<String, Value>,
    runtime_context: &ExecutionRuntimeContext,
    invoker: &I,
) -> Result<VisibleInternalLlmToolResume>
where
    I: ProviderInvoker + CapabilityInvoker + CodeInvoker + ?Sized,
{
    let state = visible_internal_llm_tool_callback_state(&variable_pool)?;
    let node = plan.nodes.get(waiting_node_id).ok_or_else(|| {
        anyhow!("visible internal llm tool waiting node not found: {waiting_node_id}")
    })?;
    let resolved_inputs = resolve_node_inputs(node, &variable_pool).map_err(|error| {
        anyhow!("visible internal llm tool waiting node input resolution failed: {error}")
    })?;
    let rendered_templates = render_templated_bindings(node, &resolved_inputs);
    let mut provider_events = Vec::new();
    let mut route_events = state.route_events.clone();

    match execute_visible_internal_llm_tool_node(
        node,
        &resolved_inputs,
        &rendered_templates,
        &mut variable_pool,
        runtime_context,
        invoker,
        &mut provider_events,
    )
    .await?
    {
        VisibleInternalLlmToolNodeExecution::Waiting(mut wait) => {
            let output_text = wait
                .node_trace
                .as_ref()
                .map(|trace| visible_internal_llm_tool_output_text(&trace.output_payload))
                .unwrap_or_default();
            let branch_text = format!("{}{}", state.branch_text, output_text);
            route_events.push(visible_internal_llm_tool_route_event(
                "visible_internal_llm_tool_waiting_callback",
                &state.main_node_id,
                &state.tool_call,
                &VisibleInternalLlmTool {
                    name: state.tool_name.clone(),
                    description: None,
                    target_node_id: state.target_node_id.clone(),
                    input_schema: None,
                },
                json!({
                    "waiting_node_id": wait.node_id,
                    "waiting_node_alias": wait.node_alias,
                    "request_payload": wait.request_payload,
                }),
            ));
            insert_visible_internal_llm_tool_callback_state(
                &mut wait.checkpoint_variable_pool,
                VisibleInternalLlmToolCallbackStateInput {
                    main_node_id: &state.main_node_id,
                    tool_call: &state.tool_call,
                    tool_name: &state.tool_name,
                    target_node_id: &state.target_node_id,
                    main_visible_transcript: &state.main_visible_transcript,
                    branch_text: &branch_text,
                    route_events: &route_events,
                    completed_tool_results: &state.completed_tool_results,
                    remaining_tool_calls: &state.remaining_tool_calls,
                },
            );
            Ok(VisibleInternalLlmToolResume::Waiting(wait))
        }
        VisibleInternalLlmToolNodeExecution::Completed(output_payload) => {
            let branch_text = format!(
                "{}{}",
                state.branch_text,
                visible_internal_llm_tool_output_text(&output_payload)
            );
            let tool = VisibleInternalLlmTool {
                name: state.tool_name.clone(),
                description: None,
                target_node_id: state.target_node_id.clone(),
                input_schema: None,
            };
            variable_pool.insert(
                node.node_id.clone(),
                project_node_variable_payload(node, &output_payload)?,
            );
            route_events.push(visible_internal_llm_tool_route_event(
                "visible_internal_llm_tool_completed",
                &state.main_node_id,
                &state.tool_call,
                &tool,
                json!({
                    "node_id": node.node_id,
                    "provider_route": output_payload
                        .get("provider_route")
                        .cloned()
                        .unwrap_or(Value::Null),
                }),
            ));

            let mut active_node_ids = BTreeSet::new();
            activate_downstream_nodes(plan, &mut active_node_ids, node, None);
            let branch_execution = if active_node_ids.is_empty() {
                VisibleInternalLlmToolBranchExecution::Completed(VisibleInternalLlmToolOutput {
                    text: branch_text,
                    provider_events,
                    route_events,
                })
            } else {
                continue_visible_internal_llm_tool_branch(
                    plan,
                    variable_pool.clone(),
                    active_node_ids,
                    runtime_context,
                    invoker,
                    &state.main_node_id,
                    &state.tool_call,
                    &tool,
                    branch_text,
                    provider_events,
                    route_events,
                )
                .await?
            };

            let (branch_output, branch_variable_pool) = match branch_execution {
                VisibleInternalLlmToolBranchExecution::Completed(output) => (output, variable_pool),
                VisibleInternalLlmToolBranchExecution::Waiting {
                    mut wait,
                    branch_text,
                    route_events,
                } => {
                    insert_visible_internal_llm_tool_callback_state(
                        &mut wait.checkpoint_variable_pool,
                        VisibleInternalLlmToolCallbackStateInput {
                            main_node_id: &state.main_node_id,
                            tool_call: &state.tool_call,
                            tool_name: &state.tool_name,
                            target_node_id: &state.target_node_id,
                            main_visible_transcript: &state.main_visible_transcript,
                            branch_text: &branch_text,
                            route_events: &route_events,
                            completed_tool_results: &state.completed_tool_results,
                            remaining_tool_calls: &state.remaining_tool_calls,
                        },
                    );
                    return Ok(VisibleInternalLlmToolResume::Waiting(wait));
                }
                VisibleInternalLlmToolBranchExecution::Failed {
                    error_payload,
                    route_events,
                } => {
                    if visible_internal_llm_tool_error_is_recoverable(&error_payload) {
                        let mut completed_tool_results = state.completed_tool_results.clone();
                        completed_tool_results.push(visible_internal_llm_tool_result(
                            &state.tool_call,
                            &state.tool_name,
                            visible_internal_llm_tool_error_result_content(&error_payload),
                        ));
                        let visible_transcript =
                            format!("{}{}", state.main_visible_transcript, state.branch_text);
                        match execute_remaining_visible_internal_llm_tool_calls(
                            plan,
                            &variable_pool,
                            runtime_context,
                            invoker,
                            &state.main_node_id,
                            completed_tool_results,
                            visible_transcript,
                            state.remaining_tool_calls.clone(),
                        )
                        .await?
                        {
                            VisibleInternalLlmToolRemainingExecution::Completed {
                                tool_results,
                                visible_transcript,
                                provider_events: _remaining_provider_events,
                                route_events: remaining_route_events,
                            } => {
                                let mut route_events = route_events;
                                route_events.extend(remaining_route_events);
                                append_llm_tool_result_messages(
                                    &mut variable_pool,
                                    &state.main_node_id,
                                    &json!({ "tool_results": tool_results }),
                                )?;
                                set_pending_llm_tool_callback_visible_internal_transcript(
                                    &mut variable_pool,
                                    &state.main_node_id,
                                    visible_transcript,
                                )?;
                                set_pending_llm_tool_callback_visible_internal_events(
                                    &mut variable_pool,
                                    &state.main_node_id,
                                    route_events,
                                )?;
                                variable_pool.remove(VISIBLE_INTERNAL_LLM_TOOL_CALLBACK_STATE_KEY);
                                return Ok(VisibleInternalLlmToolResume::Ready(variable_pool));
                            }
                            VisibleInternalLlmToolRemainingExecution::Waiting(wait) => {
                                return Ok(VisibleInternalLlmToolResume::Waiting(wait));
                            }
                            VisibleInternalLlmToolRemainingExecution::Failed {
                                error_payload,
                                provider_events: remaining_provider_events,
                                route_events: remaining_route_events,
                            } => {
                                let mut route_events = route_events;
                                route_events.extend(remaining_route_events);
                                let main_node =
                                    plan.nodes.get(&state.main_node_id).ok_or_else(|| {
                                        anyhow!("visible internal llm tool main node not found")
                                    })?;
                                let execution = visible_internal_llm_tool_failure(
                                    main_node,
                                    remaining_provider_events,
                                    error_payload,
                                    route_events,
                                )?;
                                return Ok(VisibleInternalLlmToolResume::Failed {
                                    node_id: main_node.node_id.clone(),
                                    node_alias: main_node.alias.clone(),
                                    execution,
                                });
                            }
                        }
                    }

                    let main_node = plan
                        .nodes
                        .get(&state.main_node_id)
                        .ok_or_else(|| anyhow!("visible internal llm tool main node not found"))?;
                    let execution = visible_internal_llm_tool_failure(
                        main_node,
                        Vec::new(),
                        error_payload,
                        route_events,
                    )?;
                    return Ok(VisibleInternalLlmToolResume::Failed {
                        node_id: main_node.node_id.clone(),
                        node_alias: main_node.alias.clone(),
                        execution,
                    });
                }
            };

            variable_pool = branch_variable_pool;
            provider_events = branch_output.provider_events;
            route_events = branch_output.route_events;
            let mut completed_tool_results = state.completed_tool_results.clone();
            completed_tool_results.push(visible_internal_llm_tool_result(
                &state.tool_call,
                &state.tool_name,
                branch_output.text.clone(),
            ));
            let visible_transcript =
                format!("{}{}", state.main_visible_transcript, branch_output.text);
            match execute_remaining_visible_internal_llm_tool_calls(
                plan,
                &variable_pool,
                runtime_context,
                invoker,
                &state.main_node_id,
                completed_tool_results,
                visible_transcript,
                state.remaining_tool_calls.clone(),
            )
            .await?
            {
                VisibleInternalLlmToolRemainingExecution::Completed {
                    tool_results,
                    visible_transcript,
                    provider_events: remaining_provider_events,
                    route_events: remaining_route_events,
                } => {
                    provider_events.extend(remaining_provider_events);
                    route_events.extend(remaining_route_events);
                    append_llm_tool_result_messages(
                        &mut variable_pool,
                        &state.main_node_id,
                        &json!({ "tool_results": tool_results }),
                    )?;
                    set_pending_llm_tool_callback_visible_internal_transcript(
                        &mut variable_pool,
                        &state.main_node_id,
                        visible_transcript,
                    )?;
                    set_pending_llm_tool_callback_visible_internal_events(
                        &mut variable_pool,
                        &state.main_node_id,
                        route_events,
                    )?;
                    variable_pool.remove(VISIBLE_INTERNAL_LLM_TOOL_CALLBACK_STATE_KEY);
                    Ok(VisibleInternalLlmToolResume::Ready(variable_pool))
                }
                VisibleInternalLlmToolRemainingExecution::Waiting(wait) => {
                    Ok(VisibleInternalLlmToolResume::Waiting(wait))
                }
                VisibleInternalLlmToolRemainingExecution::Failed {
                    error_payload,
                    provider_events: remaining_provider_events,
                    route_events: remaining_route_events,
                } => {
                    provider_events.extend(remaining_provider_events);
                    route_events.extend(remaining_route_events);
                    let main_node = plan
                        .nodes
                        .get(&state.main_node_id)
                        .ok_or_else(|| anyhow!("visible internal llm tool main node not found"))?;
                    let execution = visible_internal_llm_tool_failure(
                        main_node,
                        provider_events,
                        error_payload,
                        route_events,
                    )?;
                    Ok(VisibleInternalLlmToolResume::Failed {
                        node_id: main_node.node_id.clone(),
                        node_alias: main_node.alias.clone(),
                        execution,
                    })
                }
            }
        }
        VisibleInternalLlmToolNodeExecution::Failed(error_payload) => {
            let main_node = plan
                .nodes
                .get(&state.main_node_id)
                .ok_or_else(|| anyhow!("visible internal llm tool main node not found"))?;
            let tool = VisibleInternalLlmTool {
                name: state.tool_name.clone(),
                description: None,
                target_node_id: state.target_node_id.clone(),
                input_schema: None,
            };
            let error_payload = visible_internal_llm_tool_node_error(
                &state.tool_call,
                &tool,
                node,
                "visible internal LLM tool branch node failed",
                None,
                Some(error_payload),
            );
            route_events.push(visible_internal_llm_tool_route_event(
                "visible_internal_llm_tool_failed",
                &state.main_node_id,
                &state.tool_call,
                &tool,
                json!({
                    "node_id": node.node_id,
                    "error_payload": error_payload,
                }),
            ));
            if visible_internal_llm_tool_error_is_recoverable(&error_payload) {
                let mut completed_tool_results = state.completed_tool_results.clone();
                completed_tool_results.push(visible_internal_llm_tool_result(
                    &state.tool_call,
                    &state.tool_name,
                    visible_internal_llm_tool_error_result_content(&error_payload),
                ));
                let visible_transcript =
                    format!("{}{}", state.main_visible_transcript, state.branch_text);
                match execute_remaining_visible_internal_llm_tool_calls(
                    plan,
                    &variable_pool,
                    runtime_context,
                    invoker,
                    &state.main_node_id,
                    completed_tool_results,
                    visible_transcript,
                    state.remaining_tool_calls.clone(),
                )
                .await?
                {
                    VisibleInternalLlmToolRemainingExecution::Completed {
                        tool_results,
                        visible_transcript,
                        provider_events: remaining_provider_events,
                        route_events: remaining_route_events,
                    } => {
                        provider_events.extend(remaining_provider_events);
                        route_events.extend(remaining_route_events);
                        append_llm_tool_result_messages(
                            &mut variable_pool,
                            &state.main_node_id,
                            &json!({ "tool_results": tool_results }),
                        )?;
                        set_pending_llm_tool_callback_visible_internal_transcript(
                            &mut variable_pool,
                            &state.main_node_id,
                            visible_transcript,
                        )?;
                        set_pending_llm_tool_callback_visible_internal_events(
                            &mut variable_pool,
                            &state.main_node_id,
                            route_events,
                        )?;
                        variable_pool.remove(VISIBLE_INTERNAL_LLM_TOOL_CALLBACK_STATE_KEY);
                        return Ok(VisibleInternalLlmToolResume::Ready(variable_pool));
                    }
                    VisibleInternalLlmToolRemainingExecution::Waiting(wait) => {
                        return Ok(VisibleInternalLlmToolResume::Waiting(wait));
                    }
                    VisibleInternalLlmToolRemainingExecution::Failed {
                        error_payload,
                        provider_events: remaining_provider_events,
                        route_events: remaining_route_events,
                    } => {
                        provider_events.extend(remaining_provider_events);
                        route_events.extend(remaining_route_events);
                        let execution = visible_internal_llm_tool_failure(
                            main_node,
                            provider_events,
                            error_payload,
                            route_events,
                        )?;
                        return Ok(VisibleInternalLlmToolResume::Failed {
                            node_id: main_node.node_id.clone(),
                            node_alias: main_node.alias.clone(),
                            execution,
                        });
                    }
                }
            }
            let execution = visible_internal_llm_tool_failure(
                main_node,
                provider_events,
                error_payload,
                route_events,
            )?;
            Ok(VisibleInternalLlmToolResume::Failed {
                node_id: main_node.node_id.clone(),
                node_alias: main_node.alias.clone(),
                execution,
            })
        }
    }
}

struct VisibleInternalLlmToolCallbackState {
    main_node_id: String,
    tool_call: Value,
    tool_name: String,
    target_node_id: String,
    main_visible_transcript: String,
    branch_text: String,
    route_events: Vec<Value>,
    completed_tool_results: Vec<Value>,
    remaining_tool_calls: Vec<VisibleInternalLlmToolPendingCall>,
}

fn visible_internal_llm_tool_callback_state(
    variable_pool: &Map<String, Value>,
) -> Result<VisibleInternalLlmToolCallbackState> {
    let state = variable_pool
        .get(VISIBLE_INTERNAL_LLM_TOOL_CALLBACK_STATE_KEY)
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("visible internal llm tool callback state not found"))?;
    Ok(VisibleInternalLlmToolCallbackState {
        main_node_id: state
            .get("main_node_id")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                anyhow!("visible internal llm tool callback state is missing main_node_id")
            })?
            .to_string(),
        tool_call: state.get("tool_call").cloned().ok_or_else(|| {
            anyhow!("visible internal llm tool callback state is missing tool_call")
        })?,
        tool_name: state
            .get("tool_name")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                anyhow!("visible internal llm tool callback state is missing tool_name")
            })?
            .to_string(),
        target_node_id: state
            .get("target_node_id")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        main_visible_transcript: state
            .get("main_visible_transcript")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        branch_text: state
            .get("branch_text")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        route_events: state
            .get("route_events")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default(),
        completed_tool_results: state
            .get("completed_tool_results")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default(),
        remaining_tool_calls: state
            .get("remaining_tool_calls")
            .and_then(Value::as_array)
            .map(|calls| {
                calls
                    .iter()
                    .map(visible_internal_llm_tool_pending_call_from_value)
                    .collect::<Result<Vec<_>>>()
            })
            .transpose()?
            .unwrap_or_default(),
    })
}

fn insert_visible_internal_llm_tool_callback_state(
    variable_pool: &mut Map<String, Value>,
    input: VisibleInternalLlmToolCallbackStateInput<'_>,
) {
    let remaining_tool_calls = input
        .remaining_tool_calls
        .iter()
        .map(visible_internal_llm_tool_pending_call_value)
        .collect::<Vec<_>>();
    let mut state = Map::new();
    state.insert(
        "main_node_id".to_string(),
        Value::String(input.main_node_id.to_string()),
    );
    state.insert("tool_call".to_string(), input.tool_call.clone());
    state.insert(
        "tool_name".to_string(),
        Value::String(input.tool_name.to_string()),
    );
    state.insert(
        "target_node_id".to_string(),
        Value::String(input.target_node_id.to_string()),
    );
    state.insert(
        "main_visible_transcript".to_string(),
        Value::String(input.main_visible_transcript.to_string()),
    );
    state.insert(
        "branch_text".to_string(),
        Value::String(input.branch_text.to_string()),
    );
    state.insert(
        "route_events".to_string(),
        Value::Array(input.route_events.to_vec()),
    );
    state.insert(
        "completed_tool_results".to_string(),
        Value::Array(input.completed_tool_results.to_vec()),
    );
    state.insert(
        "remaining_tool_calls".to_string(),
        Value::Array(remaining_tool_calls),
    );
    variable_pool.insert(
        VISIBLE_INTERNAL_LLM_TOOL_CALLBACK_STATE_KEY.to_string(),
        Value::Object(state),
    );
}

fn visible_internal_llm_tool_pending_call_value(
    pending_call: &VisibleInternalLlmToolPendingCall,
) -> Value {
    let mut tool = Map::new();
    tool.insert(
        "name".to_string(),
        Value::String(pending_call.tool.name.clone()),
    );
    tool.insert(
        "target_node_id".to_string(),
        Value::String(pending_call.tool.target_node_id.clone()),
    );
    if let Some(description) = pending_call.tool.description.clone() {
        tool.insert("description".to_string(), Value::String(description));
    }
    if let Some(input_schema) = pending_call.tool.input_schema.clone() {
        tool.insert("input_schema".to_string(), input_schema);
    }

    json!({
        "tool_call": pending_call.tool_call,
        "tool": Value::Object(tool)
    })
}

fn visible_internal_llm_tool_pending_call_from_value(
    value: &Value,
) -> Result<VisibleInternalLlmToolPendingCall> {
    let object = value
        .as_object()
        .ok_or_else(|| anyhow!("visible internal llm tool pending call must be an object"))?;
    let tool_call = object
        .get("tool_call")
        .cloned()
        .ok_or_else(|| anyhow!("visible internal llm tool pending call is missing tool_call"))?;
    let tool = object
        .get("tool")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("visible internal llm tool pending call is missing tool"))?;
    let name = tool
        .get("name")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .ok_or_else(|| anyhow!("visible internal llm tool pending call is missing tool name"))?
        .to_string();
    let target_node_id = tool
        .get("target_node_id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|node_id| !node_id.is_empty())
        .ok_or_else(|| anyhow!("visible internal llm tool pending call is missing target_node_id"))?
        .to_string();

    Ok(VisibleInternalLlmToolPendingCall {
        tool_call,
        tool: VisibleInternalLlmTool {
            name,
            description: tool
                .get("description")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|description| !description.is_empty())
                .map(str::to_string),
            target_node_id,
            input_schema: tool.get("input_schema").cloned(),
        },
    })
}

fn visible_internal_llm_tool_node_error(
    tool_call: &Value,
    tool: &VisibleInternalLlmTool,
    node: &CompiledNode,
    message: &str,
    runtime_message: Option<String>,
    details: Option<Value>,
) -> Value {
    json!({
        "error_code": "visible_internal_llm_tool_failed",
        "message": message,
        "runtime_message": runtime_message,
        "tool_call_id": tool_call_id(tool_call),
        "tool_name": tool.name,
        "target_node_id": tool.target_node_id,
        "node_id": node.node_id,
        "details": details,
    })
}

fn visible_internal_llm_tool_output_text(output_payload: &Value) -> String {
    output_payload
        .get("text")
        .or_else(|| output_payload.get("answer"))
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| {
            output_payload
                .as_object()
                .and_then(|object| object.values().find_map(Value::as_str))
                .map(str::to_string)
        })
        .unwrap_or_default()
}

fn visible_internal_llm_tool_from_value(value: &Value) -> Option<VisibleInternalLlmTool> {
    let object = value.as_object()?;
    if object.get("type").and_then(Value::as_str) != Some(VISIBLE_INTERNAL_LLM_TOOL_TYPE) {
        return None;
    }
    let name = object
        .get("tool_name")
        .or_else(|| object.get("name"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|name| !name.is_empty())?
        .to_string();
    let target_node_id = object
        .get("target_node_id")
        .or_else(|| object.get("targetNodeId"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|node_id| !node_id.is_empty())?
        .to_string();

    Some(VisibleInternalLlmTool {
        name,
        description: object
            .get("description")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|description| !description.is_empty())
            .map(str::to_string),
        target_node_id,
        input_schema: object
            .get("input_schema")
            .or_else(|| object.get("inputSchema"))
            .cloned(),
    })
}

fn output_tool_calls(output_payload: &Value) -> Option<Vec<Value>> {
    output_payload
        .get("tool_calls")
        .and_then(Value::as_array)
        .filter(|calls| !calls.is_empty())
        .cloned()
}

fn visible_internal_tool_calls<'a>(
    tool_calls: &[Value],
    tools: &'a [VisibleInternalLlmTool],
) -> Vec<(Value, &'a VisibleInternalLlmTool)> {
    tool_calls
        .iter()
        .filter_map(|tool_call| {
            let name = tool_call.get("name").and_then(Value::as_str)?;
            let tool = tools.iter().find(|tool| tool.name == name)?;
            Some((tool_call.clone(), tool))
        })
        .collect()
}

fn sanitize_visible_internal_llm_execution(
    execution: &mut LlmNodeExecution,
    tools: &[VisibleInternalLlmTool],
) {
    sanitize_visible_internal_tool_calls_in_value(&mut execution.output_payload, tools);
    sanitize_visible_internal_tool_calls_in_value(&mut execution.debug_payload, tools);
    sanitize_visible_internal_provider_events(&mut execution.provider_events, tools);
}

fn sanitize_visible_internal_tool_calls_in_value(
    value: &mut Value,
    tools: &[VisibleInternalLlmTool],
) {
    match value {
        Value::Array(items) => {
            for item in items {
                sanitize_visible_internal_tool_calls_in_value(item, tools);
            }
        }
        Value::Object(object) => {
            if object
                .get("name")
                .and_then(Value::as_str)
                .is_some_and(|name| visible_internal_llm_tool_name_matches(name, tools))
            {
                sanitize_visible_internal_tool_call_object(object);
            }
            if let Some(tool_calls) = object.get_mut("tool_calls") {
                sanitize_visible_internal_tool_calls_in_value(tool_calls, tools);
            }
            for value in object.values_mut() {
                sanitize_visible_internal_tool_calls_in_value(value, tools);
            }
        }
        _ => {}
    }
}

fn sanitize_visible_internal_provider_events(
    events: &mut [ProviderStreamEvent],
    tools: &[VisibleInternalLlmTool],
) {
    let mut internal_call_ids = BTreeSet::new();
    for event in events.iter() {
        match event {
            ProviderStreamEvent::ToolCallCommit { call }
                if visible_internal_llm_tool_name_matches(&call.name, tools) =>
            {
                internal_call_ids.insert(call.id.clone());
            }
            ProviderStreamEvent::ToolCallDelta { call_id, delta }
                if delta
                    .get("name")
                    .and_then(Value::as_str)
                    .is_some_and(|name| visible_internal_llm_tool_name_matches(name, tools)) =>
            {
                internal_call_ids.insert(call_id.clone());
            }
            _ => {}
        }
    }

    for event in events {
        match event {
            ProviderStreamEvent::ToolCallCommit { call }
                if visible_internal_llm_tool_name_matches(&call.name, tools) =>
            {
                call.arguments =
                    sanitized_visible_internal_llm_tool_arguments_value(&call.arguments);
            }
            ProviderStreamEvent::ToolCallDelta { call_id, delta }
                if internal_call_ids.contains(call_id)
                    || delta
                        .get("name")
                        .and_then(Value::as_str)
                        .is_some_and(|name| {
                            visible_internal_llm_tool_name_matches(name, tools)
                        }) =>
            {
                sanitize_visible_internal_tool_call_delta(delta);
            }
            _ => {}
        }
    }
}

fn sanitize_visible_internal_tool_call_delta(delta: &mut Value) {
    let Some(object) = delta.as_object_mut() else {
        return;
    };
    if let Some(arguments) = object.get("arguments").cloned() {
        object.insert(
            "arguments".to_string(),
            sanitized_visible_internal_llm_tool_arguments_value(&arguments),
        );
    }
}

fn sanitize_visible_internal_tool_call_object(object: &mut Map<String, Value>) {
    if let Some(arguments) = object.get("arguments").cloned() {
        object.insert(
            "arguments".to_string(),
            sanitized_visible_internal_llm_tool_arguments_value(&arguments),
        );
    }
}

fn sanitized_visible_internal_llm_tool_arguments_value(arguments: &Value) -> Value {
    let mut arguments = arguments.as_object().cloned().unwrap_or_default();
    if media_argument_has_items(arguments.get("media")) {
        let media = sanitized_visible_internal_llm_tool_media(arguments.get("media"));
        if media.is_empty() {
            arguments.remove("media");
        } else {
            arguments.insert("media".to_string(), Value::Array(media));
        }
    }
    Value::Object(arguments)
}

fn visible_internal_llm_tool_name_matches(name: &str, tools: &[VisibleInternalLlmTool]) -> bool {
    tools.iter().any(|tool| tool.name == name)
}

fn append_output_text(target: &mut String, output_payload: &Value) {
    if let Some(text) = output_payload.get("text").and_then(Value::as_str) {
        target.push_str(text);
    }
}

fn attach_visible_internal_llm_tool_events(
    execution: &mut LlmNodeExecution,
    route_events: &[Value],
) {
    if route_events.is_empty() {
        return;
    }
    if !execution.debug_payload.is_object() {
        execution.debug_payload = json!({});
    }
    if let Some(debug) = execution.debug_payload.as_object_mut() {
        debug.insert(
            "visible_internal_llm_tool_events".to_string(),
            Value::Array(route_events.to_vec()),
        );
    }
}

fn execution_with_visible_transcript(
    mut execution: LlmNodeExecution,
    visible_transcript: String,
    provider_events: Vec<ProviderStreamEvent>,
    route_events: Vec<Value>,
) -> LlmNodeExecution {
    if !visible_transcript.is_empty() {
        if let Some(output) = execution.output_payload.as_object_mut() {
            output.insert("text".to_string(), Value::String(visible_transcript));
        }
    }
    if !provider_events.is_empty() {
        execution.provider_events = provider_events;
    }
    attach_visible_internal_llm_tool_events(&mut execution, &route_events);
    execution.pending_callback = None;
    execution
}

fn visible_internal_llm_tool_llm_resolved_inputs(
    resolved_inputs: &Map<String, Value>,
    variable_pool: &Map<String, Value>,
) -> Map<String, Value> {
    let mut inputs = resolved_inputs.clone();
    let Some(context) = variable_pool
        .get(VISIBLE_INTERNAL_LLM_TOOL_VARIABLE)
        .and_then(|value| value.get("context"))
        .and_then(Value::as_object)
    else {
        return inputs;
    };

    if !inputs.contains_key("history") {
        if let Some(history) = context
            .get("history")
            .and_then(Value::as_array)
            .filter(|history| !history.is_empty())
        {
            inputs.insert("history".to_string(), Value::Array(history.clone()));
        }
    }
    if !inputs.contains_key("files") {
        if let Some(files) = context
            .get("files")
            .and_then(Value::as_array)
            .filter(|files| !files.is_empty())
        {
            inputs.insert("files".to_string(), Value::Array(files.clone()));
        }
    }
    if !inputs.contains_key("tools") {
        if !visible_internal_llm_tool_has_media_argument(variable_pool) {
            if let Some(tools) = context
                .get("tools")
                .and_then(Value::as_array)
                .filter(|tools| !tools.is_empty())
            {
                inputs.insert("tools".to_string(), Value::Array(tools.clone()));
            }
        }
    }

    inputs
}

pub(super) async fn inject_visible_internal_llm_tool_media_content_blocks(
    input: &mut ProviderInvocationInput,
    variable_pool: &Map<String, Value>,
) {
    let media_items = visible_internal_llm_tool_media_argument(variable_pool);
    if media_items.is_empty() {
        return;
    }

    let mut injected_blocks = Vec::new();
    for media in media_items {
        if let Some(block) = image_content_block_from_workspace_media(&media).await {
            injected_blocks.push(block);
        }
    }
    if injected_blocks.is_empty() {
        return;
    }

    let Some(message) = input
        .messages
        .iter_mut()
        .rev()
        .find(|message| message.role == ProviderMessageRole::User)
    else {
        return;
    };

    let mut content_blocks = message
        .content_blocks
        .take()
        .and_then(|value| value.as_array().cloned())
        .unwrap_or_else(|| {
            let content = message.content.trim();
            if content.is_empty() {
                Vec::new()
            } else {
                vec![json!({ "type": "text", "text": content })]
            }
        });
    content_blocks.extend(injected_blocks);
    message.content_blocks = Some(Value::Array(content_blocks));
}

pub(super) fn visible_internal_llm_tool_has_media_argument(
    variable_pool: &Map<String, Value>,
) -> bool {
    !visible_internal_llm_tool_media_argument(variable_pool).is_empty()
}

fn visible_internal_llm_tool_provider_input_schema(tool: &VisibleInternalLlmTool) -> Option<Value> {
    if !visible_internal_llm_tool_supports_media_contract(tool) {
        return tool.input_schema.clone();
    }

    let had_schema = tool.input_schema.is_some();
    let mut schema = tool.input_schema.clone().unwrap_or_else(|| json!({}));
    if !schema.is_object() {
        schema = json!({});
    }
    let schema_object = schema
        .as_object_mut()
        .expect("schema was normalized to object");
    schema_object
        .entry("type".to_string())
        .or_insert_with(|| Value::String("object".to_string()));
    let properties = schema_object
        .entry("properties".to_string())
        .or_insert_with(|| json!({}));
    if !properties.is_object() {
        *properties = json!({});
    }
    let properties_object = properties
        .as_object_mut()
        .expect("properties was normalized to object");
    properties_object
        .entry("task".to_string())
        .or_insert_with(|| json!({ "type": "string" }));
    properties_object
        .entry("media".to_string())
        .or_insert_with(visible_internal_llm_tool_media_schema);
    if !had_schema {
        schema_object.insert("required".to_string(), json!(["task"]));
    }

    Some(schema)
}

fn visible_internal_llm_tool_supports_media_contract(tool: &VisibleInternalLlmTool) -> bool {
    tool.name.to_ascii_lowercase().contains("image")
        || tool
            .input_schema
            .as_ref()
            .and_then(|schema| schema.get("properties"))
            .and_then(|properties| properties.get("media"))
            .is_some()
}

fn visible_internal_llm_tool_media_schema() -> Value {
    json!({
        "type": "array",
        "description": "Workspace image references only. Do not include file bytes, data URLs, remote URLs, or absolute paths.",
        "items": {
            "type": "object",
            "properties": {
                "kind": { "type": "string", "enum": ["image"] },
                "source": { "type": "string", "enum": ["workspace_path"] },
                "path": {
                    "type": "string",
                    "description": "Workspace-relative image path, for example uploads/example.png."
                }
            },
            "required": ["kind", "source", "path"]
        }
    })
}

fn visible_internal_llm_tool_arguments(tool_call: &Value, context: &Value) -> Value {
    let mut arguments = tool_call
        .get("arguments")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    if media_argument_has_items(arguments.get("media")) {
        let media = sanitized_visible_internal_llm_tool_media(arguments.get("media"));
        if media.is_empty() {
            arguments.remove("media");
        } else {
            arguments.insert("media".to_string(), Value::Array(media));
        }
    }
    if !media_argument_has_items(arguments.get("media")) {
        let media = inferred_visible_internal_llm_tool_media(&arguments, context);
        if !media.is_empty() {
            arguments.insert("media".to_string(), Value::Array(media));
        }
    }

    Value::Object(arguments)
}

fn inferred_visible_internal_llm_tool_media(
    arguments: &Map<String, Value>,
    context: &Value,
) -> Vec<Value> {
    let mut paths = BTreeSet::new();
    let mut media = Vec::new();

    collect_media_refs_from_files(context.get("files"), &mut paths, &mut media);
    collect_media_refs_from_text(
        arguments
            .get("task")
            .or_else(|| arguments.get("query"))
            .and_then(Value::as_str),
        &mut paths,
        &mut media,
    );
    collect_media_refs_from_text(
        context.get("query").and_then(Value::as_str),
        &mut paths,
        &mut media,
    );
    if let Some(history) = context.get("history").and_then(Value::as_array) {
        for message in history {
            collect_media_refs_from_text(
                message.get("content").and_then(Value::as_str),
                &mut paths,
                &mut media,
            );
        }
    }

    media
}

fn collect_media_refs_from_files(
    files: Option<&Value>,
    paths: &mut BTreeSet<String>,
    media: &mut Vec<Value>,
) {
    let Some(files) = files.and_then(Value::as_array) else {
        return;
    };
    for file in files {
        let Some(path) = file
            .get("path")
            .or_else(|| file.get("file_path"))
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|path| !path.is_empty())
        else {
            continue;
        };
        if !looks_like_image_path(path) || !paths.insert(path.to_string()) {
            continue;
        }
        media.push(workspace_path_media_ref(path));
    }
}

fn collect_media_refs_from_text(
    text: Option<&str>,
    paths: &mut BTreeSet<String>,
    media: &mut Vec<Value>,
) {
    let Some(text) = text else {
        return;
    };
    let Ok(pattern) = regex::Regex::new(r"(?i)[A-Za-z0-9_./\\:-]+\.(png|jpe?g|gif|webp|bmp)")
    else {
        return;
    };
    for capture in pattern.find_iter(text) {
        let path = capture
            .as_str()
            .trim_matches(|character: char| {
                matches!(
                    character,
                    '"' | '\'' | '`' | ',' | '，' | '.' | '。' | ')' | '）' | ']' | '】'
                )
            })
            .replace('\\', "/");
        if !path.is_empty() && paths.insert(path.clone()) {
            media.push(workspace_path_media_ref(&path));
        }
    }
}

fn workspace_path_media_ref(path: &str) -> Value {
    json!({
        "kind": "image",
        "source": "workspace_path",
        "path": visible_workspace_media_path(path),
    })
}

fn visible_workspace_media_path(path: &str) -> String {
    let normalized = path.trim().replace('\\', "/");
    let normalized = normalized.trim_start_matches("./");
    let requested_path = std::path::Path::new(normalized);
    if !requested_path.is_absolute() {
        return normalized.to_string();
    }

    let Ok(canonical) = std::fs::canonicalize(requested_path) else {
        return normalized.to_string();
    };
    let Some(current_dir) = std::env::current_dir().ok() else {
        return normalized.to_string();
    };
    let roots = [
        Some(current_dir.clone()),
        current_dir.parent().map(Into::into),
    ];
    for root in roots.into_iter().flatten() {
        let Ok(canonical_root) = std::fs::canonicalize(root) else {
            continue;
        };
        let Ok(relative) = canonical.strip_prefix(&canonical_root) else {
            continue;
        };
        let relative = relative.to_string_lossy().replace('\\', "/");
        if !relative.is_empty() {
            return relative;
        }
    }

    normalized.to_string()
}

fn visible_internal_llm_tool_media_argument(variable_pool: &Map<String, Value>) -> Vec<Value> {
    variable_pool
        .get(VISIBLE_INTERNAL_LLM_TOOL_VARIABLE)
        .and_then(|value| value.get("arguments"))
        .and_then(|arguments| arguments.get("media"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

fn media_argument_has_items(media: Option<&Value>) -> bool {
    media
        .and_then(Value::as_array)
        .is_some_and(|media| !media.is_empty())
}

fn sanitized_visible_internal_llm_tool_media(media: Option<&Value>) -> Vec<Value> {
    let Some(items) = media.and_then(Value::as_array) else {
        return Vec::new();
    };
    let mut paths = BTreeSet::new();
    items
        .iter()
        .filter_map(|item| {
            let path = sanitized_workspace_path_media_ref(item)?;
            let path_key = path
                .get("path")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            paths.insert(path_key).then_some(path)
        })
        .collect()
}

fn sanitized_workspace_path_media_ref(media: &Value) -> Option<Value> {
    if media.get("kind").and_then(Value::as_str) != Some("image")
        || media.get("source").and_then(Value::as_str) != Some("workspace_path")
    {
        return None;
    }
    let path = media.get("path").and_then(Value::as_str)?.trim();
    if path.is_empty() || !looks_like_image_path(path) {
        return None;
    }
    Some(workspace_path_media_ref(path))
}

async fn image_content_block_from_workspace_media(media: &Value) -> Option<Value> {
    if media.get("kind").and_then(Value::as_str) != Some("image")
        || media.get("source").and_then(Value::as_str) != Some("workspace_path")
    {
        return None;
    }
    let raw_path = media.get("path").and_then(Value::as_str)?.trim();
    if raw_path.is_empty() || !looks_like_image_path(raw_path) {
        return None;
    }
    let resolved_path = resolve_workspace_media_path(raw_path).await?;
    let bytes = tokio::fs::read(&resolved_path).await.ok()?;
    let media_type = image_media_type_from_path(&resolved_path)?;
    use base64::Engine as _;
    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    Some(json!({
        "type": "image_url",
        "image_url": {
            "url": format!("data:{media_type};base64,{encoded}")
        }
    }))
}

async fn resolve_workspace_media_path(raw_path: &str) -> Option<std::path::PathBuf> {
    let normalized = raw_path.replace('\\', "/");
    let requested_path = std::path::Path::new(&normalized);
    if !workspace_media_path_shape_allowed(requested_path) {
        return None;
    }

    let current_dir = std::env::current_dir().ok()?;
    let mut roots = vec![current_dir.clone()];
    if let Some(parent) = current_dir.parent() {
        roots.push(parent.to_path_buf());
    }
    let canonical_roots = roots
        .into_iter()
        .filter_map(|root| std::fs::canonicalize(root).ok())
        .collect::<Vec<_>>();

    if requested_path.is_absolute() {
        let canonical = tokio::fs::canonicalize(requested_path).await.ok()?;
        return canonical_roots
            .iter()
            .any(|root| canonical.starts_with(root))
            .then_some(canonical);
    }

    for root in canonical_roots {
        let candidate = root.join(requested_path);
        let Ok(canonical) = tokio::fs::canonicalize(candidate).await else {
            continue;
        };
        if canonical.starts_with(&root) {
            return Some(canonical);
        }
    }
    None
}

fn workspace_media_path_shape_allowed(path: &std::path::Path) -> bool {
    path.components().all(|component| {
        matches!(
            component,
            std::path::Component::Normal(_)
                | std::path::Component::CurDir
                | std::path::Component::RootDir
        )
    })
}

fn looks_like_image_path(path: &str) -> bool {
    let path = path.trim().to_ascii_lowercase();
    matches!(
        std::path::Path::new(&path)
            .extension()
            .and_then(|extension| extension.to_str()),
        Some("png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp")
    )
}

fn image_media_type_from_path(path: &std::path::Path) -> Option<String> {
    match path.extension().and_then(|extension| extension.to_str()) {
        Some("png") => Some("image/png".to_string()),
        Some("jpg" | "jpeg") => Some("image/jpeg".to_string()),
        Some("gif") => Some("image/gif".to_string()),
        Some("webp") => Some("image/webp".to_string()),
        Some("bmp") => Some("image/bmp".to_string()),
        _ => None,
    }
}

fn visible_internal_llm_tool_inherited_context(
    variable_pool: &Map<String, Value>,
    main_node_id: &str,
) -> Value {
    let history = inherited_main_llm_history(variable_pool, main_node_id)
        .or_else(|| synthesized_run_context_history(variable_pool))
        .unwrap_or_default();
    json!({
        "history": history,
        "query": find_run_context_value(variable_pool, "query").unwrap_or(Value::Null),
        "files": find_run_context_array(variable_pool, "files"),
        "tools": find_run_context_array(variable_pool, "tools"),
    })
}

fn inherited_main_llm_history(
    variable_pool: &Map<String, Value>,
    main_node_id: &str,
) -> Option<Vec<Value>> {
    let mut history = variable_pool
        .get(main_node_id)?
        .get(LLM_TOOL_CALLBACK_STATE_KEY)?
        .get("history")?
        .as_array()?
        .clone();
    if history
        .last()
        .and_then(Value::as_object)
        .is_some_and(|message| {
            message.get("role").and_then(Value::as_str) == Some("assistant")
                && message
                    .get("tool_calls")
                    .and_then(Value::as_array)
                    .is_some_and(|tool_calls| !tool_calls.is_empty())
        })
    {
        history.pop();
    }
    (!history.is_empty()).then_some(history)
}

fn synthesized_run_context_history(variable_pool: &Map<String, Value>) -> Option<Vec<Value>> {
    let mut content_parts = Vec::new();
    if let Some(query) =
        find_run_context_value(variable_pool, "query").and_then(|value| value_to_text(&value))
    {
        if !query.trim().is_empty() {
            content_parts.push(query);
        }
    }
    let files = find_run_context_array(variable_pool, "files");
    if !files.is_empty() {
        content_parts.push(format!("Files: {}", Value::Array(files)));
    }

    let content = content_parts.join("\n\n");
    (!content.trim().is_empty()).then(|| {
        vec![json!({
            "role": "user",
            "content": content,
        })]
    })
}

fn find_run_context_value(variable_pool: &Map<String, Value>, key: &str) -> Option<Value> {
    variable_pool
        .get("node-start")
        .and_then(|payload| payload.get(key))
        .cloned()
        .or_else(|| {
            variable_pool
                .values()
                .find_map(|payload| payload.as_object()?.get(key).cloned())
        })
}

fn find_run_context_array(variable_pool: &Map<String, Value>, key: &str) -> Vec<Value> {
    find_run_context_value(variable_pool, key)
        .and_then(|value| value.as_array().cloned())
        .unwrap_or_default()
}

fn visible_internal_llm_tool_error_is_recoverable(error_payload: &Value) -> bool {
    let message = error_payload
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let runtime_message = error_payload
        .get("runtime_message")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let details_message = error_payload
        .get("details")
        .and_then(|details| details.get("message"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    let details_code = error_payload
        .get("details")
        .and_then(|details| details.get("error_code"))
        .and_then(Value::as_str)
        .unwrap_or_default();

    details_code == "model_multimodal_unsupported"
        || message.contains("model_multimodal_unsupported")
        || runtime_message.contains("model_multimodal_unsupported")
        || details_message.contains("model_multimodal_unsupported")
}

fn visible_internal_llm_tool_error_result_content(error_payload: &Value) -> String {
    json!({
        "error_code": error_payload
            .get("details")
            .and_then(|details| details.get("error_code"))
            .or_else(|| error_payload.get("error_code"))
            .cloned()
            .unwrap_or(Value::String("visible_internal_llm_tool_failed".to_string())),
        "message": "visible internal LLM tool failed recoverably",
        "recoverable": true,
        "details": error_payload,
    })
    .to_string()
}

fn visible_internal_llm_tool_route_event(
    event_type: &str,
    main_node_id: &str,
    tool_call: &Value,
    tool: &VisibleInternalLlmTool,
    details: Value,
) -> Value {
    let mut payload = Map::new();
    payload.insert(
        "event_type".to_string(),
        Value::String(event_type.to_string()),
    );
    payload.insert(
        "main_node_id".to_string(),
        Value::String(main_node_id.to_string()),
    );
    payload.insert(
        "target_node_id".to_string(),
        Value::String(tool.target_node_id.clone()),
    );
    payload.insert("tool_name".to_string(), Value::String(tool.name.clone()));
    payload.insert(
        "tool_call_id".to_string(),
        Value::String(tool_call_id(tool_call)),
    );
    if let Some(arguments) = tool_call.get("arguments") {
        payload.insert("arguments".to_string(), arguments.clone());
    }
    if let Some(details) = details.as_object() {
        for (key, value) in details {
            payload.insert(key.clone(), value.clone());
        }
    }

    Value::Object(payload)
}

fn visible_internal_llm_tool_failure(
    node: &CompiledNode,
    provider_events: Vec<ProviderStreamEvent>,
    error_payload: Value,
    route_events: Vec<Value>,
) -> Result<LlmNodeExecution> {
    let runtime = node.llm_runtime.as_ref().ok_or_else(|| {
        anyhow!(
            "compiled llm node is missing runtime metadata: {}",
            node.node_id
        )
    })?;

    let mut execution = build_failed_llm_execution(
        node,
        runtime,
        error_payload,
        build_llm_metrics_payload(
            runtime,
            ProviderUsage::default(),
            Some(ProviderFinishReason::Error),
            provider_events.len(),
            Vec::new(),
            None,
            None,
        ),
        provider_events,
        true,
        LlmDebugInvocation {
            messages: &[],
            context: None,
        },
    )?;
    attach_visible_internal_llm_tool_events(&mut execution, &route_events);
    Ok(execution)
}

fn tool_call_id(tool_call: &Value) -> String {
    tool_call
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or("tool_call")
        .to_string()
}

fn visible_internal_llm_tool_result(tool_call: &Value, tool_name: &str, content: String) -> Value {
    json!({
        "tool_call_id": tool_call_id(tool_call),
        "name": tool_name,
        "content": content
    })
}
