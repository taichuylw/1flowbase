use super::types::*;
use super::*;

fn visible_internal_llm_tools_enabled(node: &CompiledNode) -> bool {
    node.config
        .get("visible_internal_llm_tools_enabled")
        .or_else(|| node.config.get("visibleInternalLlmToolsEnabled"))
        .and_then(Value::as_bool)
        == Some(true)
}

pub(in crate::execution_engine) fn is_visible_internal_llm_tool_source_handle(
    source_handle: Option<&str>,
) -> bool {
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

pub(in crate::execution_engine) fn visible_internal_llm_tool_target_node_ids(
    plan: &CompiledPlan,
) -> BTreeSet<String> {
    plan.nodes
        .values()
        .filter(|node| node.node_type == "llm")
        .flat_map(visible_internal_llm_tools)
        .map(|tool| tool.target_node_id)
        .collect()
}

pub(in crate::execution_engine) fn visible_internal_llm_provider_tools(
    node: &CompiledNode,
) -> Vec<Value> {
    visible_internal_llm_tools(node)
        .into_iter()
        .map(|tool| {
            let mut function = Map::new();
            function.insert("name".to_string(), Value::String(tool.name.clone()));
            if let Some(description) = tool.description.clone() {
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

pub(in crate::execution_engine) fn visible_internal_llm_media_tool_context(
    node: &CompiledNode,
) -> Option<Value> {
    let tools = visible_internal_llm_tools(node)
        .into_iter()
        .flat_map(|tool| {
            let tool_name = tool.name.clone();
            visible_internal_llm_tool_media_kinds(&tool)
                .into_iter()
                .map(move |media_kind| {
                    json!({
                        "name": tool_name.clone(),
                        "media_kind": media_kind,
                    })
                })
        })
        .collect::<Vec<_>>();

    (!tools.is_empty()).then(|| Value::Array(tools))
}

pub(in crate::execution_engine) fn visible_internal_llm_node_has_media_tool(
    node: &CompiledNode,
) -> bool {
    visible_internal_llm_tools(node)
        .iter()
        .any(visible_internal_llm_tool_has_configured_media_contract)
}

fn visible_internal_llm_tool_media_kinds(tool: &VisibleInternalLlmTool) -> Vec<String> {
    tool.preconditions
        .iter()
        .map(|precondition| match precondition {
            VisibleInternalLlmToolPrecondition::MediaContentAvailable(precondition) => precondition
                .media_kind
                .clone()
                .unwrap_or_else(|| "image".to_string()),
        })
        .collect()
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

    let input_schema = object
        .get("input_schema")
        .or_else(|| object.get("inputSchema"))
        .cloned();
    let preconditions =
        visible_internal_llm_tool_preconditions_from_config(object, input_schema.as_ref());

    Some(VisibleInternalLlmTool {
        name,
        description: object
            .get("description")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|description| !description.is_empty())
            .map(str::to_string),
        target_node_id,
        input_schema,
        external_tool_policy: visible_internal_llm_tool_external_tool_policy_from_object(object),
        preconditions,
    })
}

fn visible_internal_llm_tool_external_tool_policy_from_object(
    object: &Map<String, Value>,
) -> VisibleInternalLlmToolExternalToolPolicy {
    match object
        .get("external_tool_policy")
        .or_else(|| object.get("externalToolPolicy"))
        .and_then(Value::as_str)
        .map(str::trim)
    {
        Some(EXTERNAL_TOOL_POLICY_INHERITED) => VisibleInternalLlmToolExternalToolPolicy::Inherited,
        _ => VisibleInternalLlmToolExternalToolPolicy::Forbidden,
    }
}

fn visible_internal_llm_tool_has_configured_media_contract(tool: &VisibleInternalLlmTool) -> bool {
    tool.preconditions.iter().any(|precondition| {
        matches!(
            precondition,
            VisibleInternalLlmToolPrecondition::MediaContentAvailable(_)
        )
    })
}

fn visible_internal_llm_tool_preconditions_from_config(
    object: &Map<String, Value>,
    input_schema: Option<&Value>,
) -> Vec<VisibleInternalLlmToolPrecondition> {
    let explicit_preconditions = object
        .get("preconditions")
        .or_else(|| object.get("preConditions"));
    if explicit_preconditions.is_some() {
        let preconditions =
            visible_internal_llm_tool_preconditions_from_value(explicit_preconditions);
        if !preconditions.is_empty() {
            return preconditions;
        }
        if explicit_preconditions_are_empty_field_rows(explicit_preconditions) {
            return legacy_media_input_schema_preconditions(input_schema);
        }
        return Vec::new();
    }

    legacy_media_input_schema_preconditions(input_schema)
}

fn explicit_preconditions_are_empty_field_rows(value: Option<&Value>) -> bool {
    value
        .and_then(Value::as_array)
        .filter(|preconditions| !preconditions.is_empty())
        .is_some_and(|preconditions| {
            preconditions
                .iter()
                .all(|precondition| precondition.as_object().is_some_and(Map::is_empty))
        })
}

fn legacy_media_input_schema_preconditions(
    input_schema: Option<&Value>,
) -> Vec<VisibleInternalLlmToolPrecondition> {
    if input_schema
        .and_then(|schema| schema.get("properties"))
        .and_then(|properties| properties.get("media"))
        .is_none()
    {
        return Vec::new();
    }

    vec![VisibleInternalLlmToolPrecondition::MediaContentAvailable(
        VisibleInternalLlmToolMediaContentPrecondition {
            argument_path: vec!["media".to_string()],
            media_kind: Some("image".to_string()),
        },
    )]
}
