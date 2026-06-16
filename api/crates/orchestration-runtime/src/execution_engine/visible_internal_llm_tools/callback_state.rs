use super::types::*;
use super::*;

pub(super) struct VisibleInternalLlmToolCallbackStateInput<'a> {
    pub(super) main_node_id: &'a str,
    pub(super) tool_call: &'a Value,
    pub(super) tool_name: &'a str,
    pub(super) target_node_id: &'a str,
    pub(super) main_visible_transcript: &'a str,
    pub(super) branch_text: &'a str,
    pub(super) route_events: &'a [Value],
    pub(super) completed_tool_results: &'a [Value],
    pub(super) remaining_tool_calls: &'a [VisibleInternalLlmToolPendingCall],
}

pub(in crate::execution_engine) fn has_visible_internal_llm_tool_callback_state(
    variable_pool: &Map<String, Value>,
) -> bool {
    variable_pool
        .get(VISIBLE_INTERNAL_LLM_TOOL_CALLBACK_STATE_KEY)
        .and_then(Value::as_object)
        .is_some()
}

pub(super) struct VisibleInternalLlmToolCallbackState {
    pub(super) main_node_id: String,
    pub(super) tool_call: Value,
    pub(super) tool_name: String,
    pub(super) target_node_id: String,
    pub(super) main_visible_transcript: String,
    pub(super) branch_text: String,
    pub(super) route_events: Vec<Value>,
    pub(super) completed_tool_results: Vec<Value>,
    pub(super) remaining_tool_calls: Vec<VisibleInternalLlmToolPendingCall>,
}

pub(super) fn visible_internal_llm_tool_callback_state(
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

pub(super) fn insert_visible_internal_llm_tool_callback_state(
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
    tool.insert(
        "tool_mode".to_string(),
        Value::String(pending_call.tool.tool_mode.as_str().to_string()),
    );
    tool.insert(
        "external_tool_policy".to_string(),
        Value::String(pending_call.tool.external_tool_policy.as_str().to_string()),
    );
    tool.insert(
        "external_callback_policy".to_string(),
        Value::String(
            pending_call
                .tool
                .external_callback_policy
                .as_str()
                .to_string(),
        ),
    );
    tool.insert(
        "execution_mode".to_string(),
        Value::String(pending_call.tool.execution_mode.as_str().to_string()),
    );
    if let Some(description) = pending_call.tool.description.clone() {
        tool.insert("description".to_string(), Value::String(description));
    }
    if let Some(input_schema) = pending_call.tool.input_schema.clone() {
        tool.insert("input_schema".to_string(), input_schema);
    }
    if !pending_call.tool.preconditions.is_empty() {
        tool.insert(
            "preconditions".to_string(),
            visible_internal_llm_tool_preconditions_value(&pending_call.tool.preconditions),
        );
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

    let tool_mode = match tool.get("tool_mode").and_then(Value::as_str).map(str::trim) {
        Some(TOOL_MODE_FUSION) => VisibleInternalLlmToolMode::Fusion,
        _ => VisibleInternalLlmToolMode::Agent,
    };
    let external_callback_policy = if tool_mode == VisibleInternalLlmToolMode::Fusion {
        VisibleInternalLlmToolExternalCallbackPolicy::Forbidden
    } else {
        match tool
            .get("external_callback_policy")
            .and_then(Value::as_str)
            .map(str::trim)
        {
            Some(EXTERNAL_CALLBACK_POLICY_FORBIDDEN) => {
                VisibleInternalLlmToolExternalCallbackPolicy::Forbidden
            }
            _ => VisibleInternalLlmToolExternalCallbackPolicy::Inherited,
        }
    };
    let execution_mode = match tool
        .get("execution_mode")
        .and_then(Value::as_str)
        .map(str::trim)
    {
        Some(EXECUTION_MODE_BOUNDED_PARALLEL_PANEL) => {
            VisibleInternalLlmToolExecutionMode::BoundedParallelPanel
        }
        _ if tool_mode == VisibleInternalLlmToolMode::Fusion => {
            VisibleInternalLlmToolExecutionMode::BoundedParallelPanel
        }
        _ => VisibleInternalLlmToolExecutionMode::SequentialResume,
    };

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
            tool_mode,
            external_tool_policy: if tool_mode == VisibleInternalLlmToolMode::Fusion {
                VisibleInternalLlmToolExternalToolPolicy::Forbidden
            } else {
                match tool
                    .get("external_tool_policy")
                    .and_then(Value::as_str)
                    .map(str::trim)
                {
                    Some(EXTERNAL_TOOL_POLICY_INHERITED) => {
                        VisibleInternalLlmToolExternalToolPolicy::Inherited
                    }
                    _ => VisibleInternalLlmToolExternalToolPolicy::Forbidden,
                }
            },
            external_callback_policy,
            execution_mode,
            preconditions: visible_internal_llm_tool_preconditions_from_value(
                tool.get("preconditions"),
            ),
        },
    })
}
