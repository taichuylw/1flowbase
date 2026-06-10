use anyhow::{anyhow, Result};
use serde_json::{Map, Value};

use crate::{
    binding_runtime::{lookup_selector_value, render_template},
    compiled_plan::CompiledNode,
};

pub(crate) fn execute_variable_assignment_node(
    _node: &CompiledNode,
    resolved_inputs: &Map<String, Value>,
    variable_pool: &mut Map<String, Value>,
) -> Result<Value> {
    let operations = resolved_inputs
        .get("operations")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("variable assigner requires operations"))?;
    if operations.is_empty() {
        return Err(anyhow!("variable assigner requires at least one operation"));
    }

    let updates = operations
        .iter()
        .map(|operation| {
            let path = operation
                .get("path")
                .and_then(Value::as_array)
                .ok_or_else(|| anyhow!("variable assigner path is required"))?;
            let operator = operation
                .get("operator")
                .and_then(Value::as_str)
                .ok_or_else(|| anyhow!("variable assigner operator is required"))?;
            let target_namespace = path.first().and_then(Value::as_str);
            let target_name = path
                .get(1)
                .and_then(Value::as_str)
                .ok_or_else(|| anyhow!("variable assigner target name is required"))?;

            if operator != "set"
                || target_namespace != Some("conversation")
                || target_name.trim().is_empty()
            {
                return Err(anyhow!(
                    "variable assigner only supports setting conversation variables"
                ));
            }

            let value = resolve_variable_assignment_value(operation, variable_pool)?;

            Ok((target_name.to_string(), value))
        })
        .collect::<Result<Vec<_>>>()?;
    let mut updated = Map::new();
    let conversation_value = variable_pool
        .entry("conversation".to_string())
        .or_insert_with(|| Value::Object(Map::new()));

    if !conversation_value.is_object() {
        *conversation_value = Value::Object(Map::new());
    }

    let conversation = conversation_value
        .as_object_mut()
        .ok_or_else(|| anyhow!("conversation variable pool must be an object"))?;

    for (target_name, value) in updates {
        conversation.insert(target_name.clone(), value.clone());
        updated.insert(target_name, value);
    }

    Ok(Value::Object(updated))
}

fn resolve_variable_assignment_value(
    operation: &Value,
    variable_pool: &Map<String, Value>,
) -> Result<Value> {
    let value = operation
        .get("value")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("variable assigner value is required"))?;

    match value.get("kind").and_then(Value::as_str) {
        Some("constant") => Ok(value.get("value").cloned().unwrap_or(Value::Null)),
        Some("selector") => {
            let selector = value
                .get("selector")
                .and_then(Value::as_array)
                .ok_or_else(|| anyhow!("variable assigner selector value is required"))?
                .iter()
                .map(|segment| {
                    segment
                        .as_str()
                        .map(str::to_string)
                        .ok_or_else(|| anyhow!("variable assigner selector must be strings"))
                })
                .collect::<Result<Vec<_>>>()?;
            lookup_selector_value(variable_pool, &selector)
        }
        Some("templated_text") => {
            let template = value.get("value").and_then(Value::as_str).ok_or_else(|| {
                anyhow!("variable assigner templated_text value must be a string")
            })?;
            render_template(template, variable_pool).map(Value::String)
        }
        Some(kind) => Err(anyhow!(
            "variable assigner value kind is unsupported: {kind}"
        )),
        None => Err(anyhow!("variable assigner value kind is required")),
    }
}
