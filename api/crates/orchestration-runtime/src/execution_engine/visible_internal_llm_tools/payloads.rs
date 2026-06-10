use super::types::*;
use super::*;

pub(super) fn visible_internal_llm_tool_node_error(
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

pub(super) fn visible_internal_llm_tool_output_text(output_payload: &Value) -> String {
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

pub(super) fn output_tool_calls(output_payload: &Value) -> Option<Vec<Value>> {
    output_payload
        .get("tool_calls")
        .and_then(Value::as_array)
        .filter(|calls| !calls.is_empty())
        .cloned()
}

pub(super) fn visible_internal_tool_calls<'a>(
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

pub(super) fn append_output_text(target: &mut String, output_payload: &Value) {
    if let Some(text) = output_payload.get("text").and_then(Value::as_str) {
        target.push_str(text);
    }
}

pub(super) fn attach_visible_internal_llm_tool_events(
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

pub(super) fn execution_with_visible_transcript(
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

pub(super) fn visible_internal_llm_tool_error_is_recoverable(error_payload: &Value) -> bool {
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

pub(super) fn visible_internal_llm_tool_error_result_content(error_payload: &Value) -> String {
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

pub(super) fn visible_internal_llm_tool_route_event(
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

pub(super) fn visible_internal_llm_tool_failure(
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

pub(super) fn tool_call_id(tool_call: &Value) -> String {
    tool_call
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or("tool_call")
        .to_string()
}

pub(super) fn visible_internal_llm_tool_result(
    tool_call: &Value,
    tool_name: &str,
    content: String,
) -> Value {
    json!({
        "tool_call_id": tool_call_id(tool_call),
        "name": tool_name,
        "content": content
    })
}
