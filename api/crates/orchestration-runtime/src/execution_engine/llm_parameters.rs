use super::*;

pub(super) fn value_to_text(value: &Value) -> Option<String> {
    match value {
        Value::Null => None,
        Value::String(text) => Some(text.clone()),
        other => Some(other.to_string()),
    }
}

pub(super) fn build_model_parameters(
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

pub(super) fn build_configured_model_parameters(config: &Value) -> BTreeMap<String, Value> {
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

pub(super) fn llm_follows_external_reasoning(config: &Value) -> bool {
    config
        .get("external_reasoning_policy")
        .and_then(|value| value.get("follow_external_reasoning"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

pub(super) fn apply_external_reasoning_parameters(
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

pub(super) fn insert_model_parameter_if_absent(
    parameters: &mut BTreeMap<String, Value>,
    key: &'static str,
    value: Value,
) {
    parameters.entry(key.to_string()).or_insert(value);
}

pub(super) fn is_openai_reasoning_runtime(runtime: &CompiledLlmRuntime) -> bool {
    runtime.provider_code == "openai"
        || runtime.provider_code == "openai_compatible"
        || runtime.protocol == "openai_responses"
        || runtime.protocol == "openai_compatible"
}

pub(super) fn is_anthropic_reasoning_runtime(runtime: &CompiledLlmRuntime) -> bool {
    runtime.provider_code == "anthropic" || runtime.protocol == "anthropic_messages"
}

pub(super) fn is_bailian_reasoning_runtime(runtime: &CompiledLlmRuntime) -> bool {
    runtime.provider_code == "aliyun_bailian" || runtime.provider_code == "bailian"
}
