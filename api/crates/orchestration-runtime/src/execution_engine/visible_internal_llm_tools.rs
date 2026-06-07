use super::*;

const VISIBLE_INTERNAL_LLM_TOOL_TYPE: &str = "visible_internal_llm_tool";
const VISIBLE_INTERNAL_LLM_TOOL_ROLE: &str = "visible_internal_llm_tool";
const VISIBLE_INTERNAL_LLM_TOOL_VARIABLE: &str = "visible_internal_llm_tool";
const MAX_VISIBLE_INTERNAL_LLM_TOOL_ROUNDS: usize = 8;

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
}

pub(super) fn node_is_visible_internal_llm_tool(node: &CompiledNode) -> bool {
    node.config
        .get("execution_role")
        .or_else(|| node.config.get("executionRole"))
        .and_then(Value::as_str)
        == Some(VISIBLE_INTERNAL_LLM_TOOL_ROLE)
}

pub(super) fn visible_internal_llm_tools(node: &CompiledNode) -> Vec<VisibleInternalLlmTool> {
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

pub(super) fn visible_internal_llm_provider_tools(node: &CompiledNode) -> Vec<Value> {
    visible_internal_llm_tools(node)
        .into_iter()
        .map(|tool| {
            let mut function = Map::new();
            function.insert("name".to_string(), Value::String(tool.name));
            if let Some(description) = tool.description {
                function.insert("description".to_string(), Value::String(description));
            }
            if let Some(input_schema) = tool.input_schema {
                function.insert("parameters".to_string(), input_schema);
            }

            json!({
                "type": "function",
                "function": Value::Object(function)
            })
        })
        .collect()
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
    I: ProviderInvoker + ?Sized,
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
    let mut visible_transcript = String::new();
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
            ));
        };

        let internal_tool_calls = visible_internal_tool_calls(&tool_calls, &tools);
        if internal_tool_calls.is_empty() {
            if !provider_events.is_empty() {
                execution.provider_events = provider_events;
            }
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
            );
        }

        append_output_text(&mut visible_transcript, &execution.output_payload);
        let mut tool_results = Vec::new();
        for (tool_call, tool) in internal_tool_calls {
            let target_output = match execute_visible_internal_llm_tool_call(
                plan,
                &llm_variable_pool,
                runtime_context,
                invoker,
                &tool_call,
                tool,
            )
            .await?
            {
                Ok(output) => output,
                Err(error_payload) => {
                    return visible_internal_llm_tool_failure(node, provider_events, error_payload);
                }
            };
            provider_events.extend(target_output.provider_events);
            visible_transcript.push_str(&target_output.text);
            tool_results.push(json!({
                "tool_call_id": tool_call_id(&tool_call),
                "name": tool.name,
                "content": target_output.text
            }));
        }

        llm_variable_pool = variable_pool_with_pending_llm_tool_callback(
            node,
            resolved_inputs,
            &llm_variable_pool,
            &execution.output_payload,
        );
        append_llm_tool_result_messages(
            &mut llm_variable_pool,
            &node.node_id,
            &json!({ "tool_results": tool_results }),
        )?;

        if round_index + 1 == MAX_VISIBLE_INTERNAL_LLM_TOOL_ROUNDS {
            return visible_internal_llm_tool_failure(
                node,
                provider_events,
                json!({
                    "error_code": "visible_internal_llm_tool_round_limit",
                    "message": "visible internal LLM tool execution exceeded the maximum callback rounds",
                }),
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
    )
}

async fn execute_visible_internal_llm_tool_call<I>(
    plan: &CompiledPlan,
    variable_pool: &Map<String, Value>,
    runtime_context: &ExecutionRuntimeContext,
    invoker: &I,
    tool_call: &Value,
    tool: &VisibleInternalLlmTool,
) -> Result<Result<VisibleInternalLlmToolOutput, Value>>
where
    I: ProviderInvoker + ?Sized,
{
    let Some(target_node) = plan.nodes.get(&tool.target_node_id) else {
        return Ok(Err(json!({
            "error_code": "visible_internal_llm_tool_failed",
            "message": "visible internal LLM tool target node was not found",
            "tool_call_id": tool_call_id(tool_call),
            "tool_name": tool.name,
            "target_node_id": tool.target_node_id,
        })));
    };
    if target_node.node_type != "llm" || !node_is_visible_internal_llm_tool(target_node) {
        return Ok(Err(json!({
            "error_code": "visible_internal_llm_tool_failed",
            "message": "visible internal LLM tool target must be an LLM node with execution_role=visible_internal_llm_tool",
            "tool_call_id": tool_call_id(tool_call),
            "tool_name": tool.name,
            "target_node_id": tool.target_node_id,
        })));
    }

    let mut tool_variable_pool = variable_pool.clone();
    tool_variable_pool.insert(
        VISIBLE_INTERNAL_LLM_TOOL_VARIABLE.to_string(),
        json!({
            "tool_call_id": tool_call_id(tool_call),
            "tool_name": tool.name,
            "arguments": tool_call
                .get("arguments")
                .cloned()
                .unwrap_or_else(|| json!({})),
        }),
    );
    let resolved_inputs = match resolve_node_inputs(target_node, &tool_variable_pool) {
        Ok(inputs) => inputs,
        Err(error) => {
            return Ok(Err(json!({
                "error_code": "visible_internal_llm_tool_failed",
                "message": "visible internal LLM tool input resolution failed",
                "runtime_message": error.to_string(),
                "tool_call_id": tool_call_id(tool_call),
                "tool_name": tool.name,
                "target_node_id": tool.target_node_id,
            })));
        }
    };
    let rendered_templates = render_templated_bindings(target_node, &resolved_inputs);
    let execution = execute_llm_node_provider_round(
        target_node,
        &resolved_inputs,
        &rendered_templates,
        &tool_variable_pool,
        runtime_context,
        invoker,
    )
    .await?;

    if let Some(error_payload) = execution.error_payload {
        return Ok(Err(json!({
            "error_code": "visible_internal_llm_tool_failed",
            "message": "visible internal LLM tool target invocation failed",
            "tool_call_id": tool_call_id(tool_call),
            "tool_name": tool.name,
            "target_node_id": tool.target_node_id,
            "details": error_payload,
        })));
    }
    if output_tool_calls(&execution.output_payload).is_some() {
        return Ok(Err(json!({
            "error_code": "visible_internal_llm_tool_failed",
            "message": "visible internal LLM tool target cannot request external tool callbacks",
            "tool_call_id": tool_call_id(tool_call),
            "tool_name": tool.name,
            "target_node_id": tool.target_node_id,
        })));
    }

    Ok(Ok(VisibleInternalLlmToolOutput {
        text: execution
            .output_payload
            .get("text")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        provider_events: execution.provider_events,
    }))
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

fn append_output_text(target: &mut String, output_payload: &Value) {
    if let Some(text) = output_payload.get("text").and_then(Value::as_str) {
        target.push_str(text);
    }
}

fn execution_with_visible_transcript(
    mut execution: LlmNodeExecution,
    visible_transcript: String,
    provider_events: Vec<ProviderStreamEvent>,
) -> LlmNodeExecution {
    if !visible_transcript.is_empty() {
        if let Some(output) = execution.output_payload.as_object_mut() {
            output.insert("text".to_string(), Value::String(visible_transcript));
        }
    }
    if !provider_events.is_empty() {
        execution.provider_events = provider_events;
    }
    execution
}

fn visible_internal_llm_tool_failure(
    node: &CompiledNode,
    provider_events: Vec<ProviderStreamEvent>,
    error_payload: Value,
) -> Result<LlmNodeExecution> {
    let runtime = node.llm_runtime.as_ref().ok_or_else(|| {
        anyhow!(
            "compiled llm node is missing runtime metadata: {}",
            node.node_id
        )
    })?;

    build_failed_llm_execution(
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
    )
}

fn tool_call_id(tool_call: &Value) -> String {
    tool_call
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or("tool_call")
        .to_string()
}
