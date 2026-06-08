use super::*;

pub fn build_llm_tool_callback_wait(
    node: &CompiledNode,
    resolved_inputs: &Map<String, Value>,
    variable_pool: &Map<String, Value>,
    output_payload: &Value,
) -> Option<LlmToolCallbackWait> {
    has_pending_tool_calls(output_payload).then(|| LlmToolCallbackWait {
        node_id: node.node_id.clone(),
        node_alias: node.alias.clone(),
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
        node_trace: None,
    })
}

pub(super) fn build_llm_tool_callback_request_payload(
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

pub(super) fn variable_pool_with_pending_llm_tool_callback(
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

pub(super) fn llm_callback_history_after_assistant_tool_call(
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

pub(super) fn append_llm_tool_result_messages(
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
    let visible_internal_transcript = state.get("visible_internal_llm_tool_transcript").cloned();
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
        let (content, content_blocks) = tool_result_prompt_content(
            result
                .get("content")
                .cloned()
                .unwrap_or_else(|| Value::String(String::new())),
        );
        message.insert("content".to_string(), content);
        if let Some(content_blocks) = content_blocks {
            message.insert("content_blocks".to_string(), content_blocks);
        }
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
    if let Some(visible_internal_transcript) = visible_internal_transcript {
        callback_state.insert(
            "visible_internal_llm_tool_transcript".to_string(),
            visible_internal_transcript,
        );
    }
    let mut node_state = Map::new();
    node_state.insert(
        LLM_TOOL_CALLBACK_STATE_KEY.to_string(),
        Value::Object(callback_state),
    );
    variable_pool.insert(waiting_node_id.to_string(), Value::Object(node_state));

    Ok(())
}

pub(super) fn tool_result_prompt_content(value: Value) -> (Value, Option<Value>) {
    match value {
        Value::String(_) => (value, None),
        Value::Array(blocks) => {
            let text = blocks
                .iter()
                .filter_map(|entry| entry.get("text").and_then(Value::as_str))
                .collect::<Vec<_>>()
                .join("\n");
            (Value::String(text), Some(Value::Array(blocks)))
        }
        other => (Value::String(other.to_string()), None),
    }
}

pub(super) fn pending_llm_tool_callback_state<'a>(
    variable_pool: &'a Map<String, Value>,
    node_id: &str,
) -> Option<&'a Map<String, Value>> {
    variable_pool
        .get(node_id)?
        .get(LLM_TOOL_CALLBACK_STATE_KEY)?
        .as_object()
}

pub(super) fn pending_llm_tool_callback_history(
    node: &CompiledNode,
    variable_pool: &Map<String, Value>,
) -> Option<Vec<Value>> {
    pending_llm_tool_callback_state(variable_pool, &node.node_id)?
        .get("history")?
        .as_array()
        .cloned()
}

pub(super) fn pending_llm_tool_callback_visible_internal_transcript(
    node: &CompiledNode,
    variable_pool: &Map<String, Value>,
) -> Option<String> {
    pending_llm_tool_callback_state(variable_pool, &node.node_id)?
        .get("visible_internal_llm_tool_transcript")?
        .as_str()
        .map(str::to_string)
}

pub(super) fn set_pending_llm_tool_callback_visible_internal_transcript(
    variable_pool: &mut Map<String, Value>,
    node_id: &str,
    transcript: String,
) -> Result<()> {
    let state = variable_pool
        .get_mut(node_id)
        .and_then(Value::as_object_mut)
        .and_then(|node_state| node_state.get_mut(LLM_TOOL_CALLBACK_STATE_KEY))
        .and_then(Value::as_object_mut)
        .ok_or_else(|| anyhow!("llm tool callback state not found for {node_id}"))?;
    state.insert(
        "visible_internal_llm_tool_transcript".to_string(),
        Value::String(transcript),
    );
    Ok(())
}

pub(super) fn pending_llm_tool_callback_delta_messages(
    node: &CompiledNode,
    variable_pool: &Map<String, Value>,
) -> Option<Vec<Value>> {
    pending_llm_tool_callback_state(variable_pool, &node.node_id)?
        .get("delta_messages")?
        .as_array()
        .cloned()
}

pub(super) fn pending_llm_tool_callback_system(
    node: &CompiledNode,
    variable_pool: &Map<String, Value>,
) -> Option<String> {
    let history = pending_llm_tool_callback_history(node, variable_pool)?;
    provider_messages_from_prompt_messages(history).0
}

pub(super) fn pending_llm_tool_callback_previous_response_id(
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

pub(super) fn pending_llm_tool_callback_uses_responses_websocket_cursor(
    state: &Map<String, Value>,
) -> bool {
    state
        .get("provider_metadata")
        .and_then(|metadata| metadata.get("transport"))
        .and_then(Value::as_str)
        == Some(RESPONSES_WEBSOCKET_TRANSPORT)
}

pub(super) fn pending_llm_tool_callback_route_matches(
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
