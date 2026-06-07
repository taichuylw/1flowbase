use super::*;

pub(super) fn build_response_format(config: &Value) -> Option<Value> {
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

pub(super) const LLM_CONTEXT_SOURCE_KEY: &str = "__context_source";

pub(super) fn llm_context_policy(node: &CompiledNode, runtime: &CompiledLlmRuntime) -> Value {
    runtime
        .routing
        .as_ref()
        .map(|routing| routing.context_policy.clone())
        .filter(|value| value.is_object())
        .or_else(|| node.config.get("context_policy").cloned())
        .filter(|value| value.is_object())
        .unwrap_or_else(|| json!({ "integration_context": "enabled" }))
}

pub(super) fn integration_context_enabled(context_policy: &Value) -> bool {
    context_policy
        .get("integration_context")
        .and_then(Value::as_str)
        != Some("disabled")
}

pub(super) fn binding_prompt_messages<'a>(
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

pub(super) fn binding_prompt_messages_with_context_sources(
    node: &CompiledNode,
    rendered_templates: &Map<String, Value>,
    resolved_inputs: &Map<String, Value>,
    variable_pool: &Map<String, Value>,
    context_policy: &Value,
) -> Result<Vec<Value>, Value> {
    if let Some(history) = pending_llm_tool_callback_history(node, variable_pool) {
        return Ok(annotate_prompt_messages(
            history,
            "pending_tool_callback_history",
            format!("{}.{}", node.node_id, LLM_TOOL_CALLBACK_STATE_KEY),
        ));
    }

    let mut messages = Vec::new();
    if integration_context_enabled(context_policy) {
        messages.extend(run_level_system_prompt_messages(
            node,
            resolved_inputs,
            variable_pool,
        ));
        messages.extend(selected_context_messages_with_sources(
            node,
            variable_pool,
            context_policy,
        )?);
        if !context_policy_has_selector(context_policy) {
            messages.extend(compatible_history_messages_with_context_sources(
                node,
                resolved_inputs,
                variable_pool,
            ));
        }
    }
    messages.extend(annotate_prompt_messages(
        prompt_messages_from_bindings(Some(rendered_templates), resolved_inputs),
        "node_prompt",
        "bindings.prompt_messages".to_string(),
    ));
    Ok(messages)
}

pub(super) fn context_policy_has_selector(context_policy: &Value) -> bool {
    context_policy
        .get("context_selector")
        .and_then(Value::as_array)
        .is_some_and(|selector| selector.len() >= 2)
}

pub(super) fn selected_context_messages_with_sources(
    node: &CompiledNode,
    variable_pool: &Map<String, Value>,
    context_policy: &Value,
) -> Result<Vec<Value>, Value> {
    let Some(selector) = context_policy
        .get("context_selector")
        .and_then(Value::as_array)
        .map(|selector| {
            selector
                .iter()
                .filter_map(Value::as_str)
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>()
        })
        .filter(|selector| selector.len() >= 2)
    else {
        return Ok(Vec::new());
    };

    let Some(value) = read_variable_pool_selector(variable_pool, &selector) else {
        return Err(build_llm_context_selector_error_payload(
            node,
            &selector,
            "selector path not found",
        ));
    };

    if !value_is_llm_context_messages(value) {
        return Err(build_llm_context_selector_error_payload(
            node,
            &selector,
            "selector value must be an array of messages with role and content",
        ));
    }

    Ok(annotate_prompt_messages(
        value.as_array().cloned().unwrap_or_default(),
        "context_selector",
        selector.join("."),
    ))
}

pub(super) fn read_variable_pool_selector<'a>(
    variable_pool: &'a Map<String, Value>,
    selector: &[String],
) -> Option<&'a Value> {
    let (first, rest) = selector.split_first()?;
    let mut current = variable_pool.get(first)?;

    for segment in rest {
        current = current.as_object()?.get(segment)?;
    }

    Some(current)
}

pub(super) fn build_llm_context_selector_error_payload(
    node: &CompiledNode,
    selector: &[String],
    message: &str,
) -> Value {
    json!({
        "error_code": "llm_context_selector_error",
        "message": "LLM context selector validation failed",
        "runtime_message": format!(
            "node {} context_selector {}: {message}",
            node.node_id,
            selector.join(".")
        ),
    })
}

pub(super) fn run_level_system_prompt_messages(
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

pub(super) fn system_prompt_message_with_source(
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

pub(super) fn compatible_history_messages_with_context_sources(
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

pub(super) fn annotate_prompt_messages(
    messages: Vec<Value>,
    source_kind: &str,
    source: String,
) -> Vec<Value> {
    messages
        .into_iter()
        .enumerate()
        .map(|(index, message)| annotate_prompt_message(message, source_kind, &source, index))
        .collect()
}

pub(super) fn annotate_prompt_message(
    message: Value,
    source_kind: &str,
    source: &str,
    index: usize,
) -> Value {
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

pub(super) fn prompt_messages_from_bindings(
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

pub(super) fn provider_messages_from_prompt_messages(
    prompt_messages: Vec<Value>,
) -> (Option<String>, Vec<ProviderMessage>) {
    let context = provider_context_from_prompt_messages(prompt_messages);

    (context.system, context.messages)
}

pub(super) fn provider_context_from_prompt_messages(
    prompt_messages: Vec<Value>,
) -> ProviderPromptContext {
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
            let source = system_source_payload(message, index);
            system_parts.push(SystemPromptPart {
                content,
                source: source.clone(),
            });
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

    let system = if messages.is_empty() {
        seed_user_turn_from_system_only_node_prompt(
            &mut messages,
            &system_parts,
            &mut compatibility_promotions,
        )
    } else {
        system_prompt_text(&system_parts)
    };

    ProviderPromptContext {
        system,
        messages,
        compatibility_promotions,
        system_sources,
    }
}

pub(super) fn seed_user_turn_from_system_only_node_prompt(
    messages: &mut Vec<ProviderMessage>,
    system_parts: &[SystemPromptPart],
    compatibility_promotions: &mut Vec<Value>,
) -> Option<String> {
    let seeded_content = system_parts
        .iter()
        .filter(|part| system_prompt_part_can_seed_user_turn(&part.source))
        .map(|part| part.content.as_str())
        .collect::<Vec<_>>()
        .join("\n\n");
    if seeded_content.trim().is_empty() {
        return system_prompt_text(system_parts);
    }

    messages.push(ProviderMessage {
        role: ProviderMessageRole::User,
        content: seeded_content,
        name: None,
        tool_call_id: None,
        tool_calls: None,
        content_blocks: None,
    });
    compatibility_promotions.push(json!({
        "source_kind": "node_prompt_system_only",
        "source": "bindings.prompt_messages",
        "target": "provider_messages",
    }));

    system_prompt_text(system_parts)
}

pub(super) fn system_prompt_part_can_seed_user_turn(source: &Value) -> bool {
    matches!(
        source.get("source_kind").and_then(Value::as_str),
        Some("node_prompt" | "prompt_messages")
    )
}

pub(super) fn system_prompt_text(system_parts: &[SystemPromptPart]) -> Option<String> {
    (!system_parts.is_empty()).then(|| {
        system_parts
            .iter()
            .map(|part| part.content.as_str())
            .collect::<Vec<_>>()
            .join("\n\n")
    })
}

pub(super) fn system_source_payload(message: &Value, fallback_index: usize) -> Value {
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

pub(super) fn provider_tool_calls_payload(tool_calls: &Value) -> Value {
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

pub(super) fn provider_message_role(role: &str) -> ProviderMessageRole {
    match role {
        "system" => ProviderMessageRole::System,
        "assistant" => ProviderMessageRole::Assistant,
        "tool" => ProviderMessageRole::Tool,
        _ => ProviderMessageRole::User,
    }
}

pub(super) fn compatible_history_messages(
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

pub(super) fn provider_tools(
    node: &CompiledNode,
    resolved_inputs: &Map<String, Value>,
    rendered_templates: &Map<String, Value>,
    variable_pool: &Map<String, Value>,
    runtime_context: &ExecutionRuntimeContext,
) -> Vec<Value> {
    let mut tools = external_provider_tools(
        node,
        resolved_inputs,
        rendered_templates,
        variable_pool,
        runtime_context,
    );
    tools.extend(visible_internal_llm_provider_tools(node));
    tools
}

fn external_provider_tools(
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

pub(super) fn run_level_provider_tools(
    plan: &CompiledPlan,
    variable_pool: &Map<String, Value>,
) -> Vec<Value> {
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

pub(super) fn provider_tool_payloads(tools: &[Value]) -> Vec<Value> {
    tools.iter().map(provider_tool_payload).collect()
}

pub(super) fn provider_tool_payload(tool: &Value) -> Value {
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
