use super::*;

pub(super) fn first_output_key(node: &CompiledNode) -> String {
    node.outputs
        .first()
        .map(|output| output.key.clone())
        .unwrap_or_else(|| "result".to_string())
}

pub(super) fn template_output_payload(
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

pub(super) fn answer_output_payload_with_error(
    mut output_payload: Value,
    error_payload: Option<&Value>,
) -> Value {
    if let (Value::Object(payload), Some(error_payload)) = (&mut output_payload, error_payload) {
        payload.insert("error".to_string(), error_payload.clone());
    }

    output_payload
}

pub(super) fn can_continue_to_terminal_template_nodes(
    plan: &CompiledPlan,
    failed_node_index: usize,
    active_node_ids: &BTreeSet<String>,
) -> bool {
    let mut has_terminal_template_node = false;
    for node_id in plan.topological_order.iter().skip(failed_node_index + 1) {
        if !active_node_ids.contains(node_id) {
            continue;
        }

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

pub(super) fn build_failed_llm_execution(
    node: &CompiledNode,
    runtime: &CompiledLlmRuntime,
    error_payload: Value,
    metrics_payload: Value,
    provider_events: Vec<ProviderStreamEvent>,
    include_output_payload: bool,
    debug_invocation: LlmDebugInvocation<'_>,
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
            debug_invocation.messages,
            None,
            debug_invocation.context,
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

pub(super) fn failed_llm_output_text(error_payload: &Value) -> String {
    error_payload
        .get("message")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .or_else(|| error_payload.get("error_message").and_then(Value::as_str))
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("LLM node failed")
        .to_string()
}

pub(super) fn build_successful_llm_execution(
    node: &CompiledNode,
    runtime: &CompiledLlmRuntime,
    result: &ProviderInvocationResult,
    final_content: Option<String>,
    metrics_payload: Value,
    provider_events: Vec<ProviderStreamEvent>,
    debug_invocation: LlmDebugInvocation<'_>,
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
        debug_invocation.messages,
        metrics_payload.get("usage"),
        debug_invocation.context,
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

pub(super) fn build_llm_node_payloads(
    node: &CompiledNode,
    raw: RawNodeExecutionResult,
) -> Result<BuiltNodePayloads> {
    PublicOutputContract::from_compiled_outputs(&node.outputs)?.build_node_payloads(raw)
}

pub(super) fn project_node_variable_payload(
    node: &CompiledNode,
    output_payload: &Value,
) -> Result<Value> {
    if node.node_type == "code" {
        return Ok(output_payload.clone());
    }

    PublicOutputContract::from_compiled_outputs(&node.outputs)?
        .project_variable_payload(output_payload)
}

pub(super) fn object_from_value(value: Value) -> Result<Map<String, Value>> {
    value
        .as_object()
        .cloned()
        .ok_or_else(|| anyhow!("payload bucket facts must be an object"))
}

pub(super) fn build_llm_provider_route_payload(runtime: &CompiledLlmRuntime) -> Value {
    json!({
        "provider_instance_id": runtime.provider_instance_id,
        "provider_code": runtime.provider_code,
        "protocol": runtime.protocol,
        "model": runtime.model,
    })
}

pub(super) fn build_llm_debug_facts(
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

pub(super) fn build_llm_round_timeline(
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

pub(super) fn provider_result_assistant_debug_payload(
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

pub(super) fn tool_calls_with_call_usage(
    tool_calls: &[ProviderToolCall],
    usage: Option<&Value>,
) -> Value {
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

pub(super) fn apply_result_context_usage_to_last_tool_results(rounds: &mut [Value], usage: &Value) {
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

pub(super) fn declares_public_output(node: &CompiledNode, key: &str) -> bool {
    node.outputs.iter().any(|output| output.key == key)
}

pub(super) fn is_structured_response_format(config: &Value) -> bool {
    config
        .get("response_format")
        .and_then(|format| format.get("mode"))
        .and_then(Value::as_str)
        .is_some_and(|mode| matches!(mode, "json_object" | "json_schema"))
}
