use serde_json::Value;

use crate::application_public_api::native::NativeRunRequest;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnthropicCompatError {
    pub message: String,
    pub error_type: String,
}

impl AnthropicCompatError {
    fn invalid(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            error_type: "invalid_request".to_string(),
        }
    }

    fn unsupported(param: &'static str) -> Self {
        Self {
            message: format!("{param} is not supported by this endpoint"),
            error_type: "unsupported_feature".to_string(),
        }
    }
}

pub fn map_messages_request(request: Value) -> Result<NativeRunRequest, AnthropicCompatError> {
    let object = request
        .as_object()
        .ok_or_else(|| AnthropicCompatError::invalid("request body must be an object"))?;
    let model = object
        .get("model")
        .and_then(Value::as_str)
        .ok_or_else(|| AnthropicCompatError::invalid("model is required"))?;
    let messages = object
        .get("messages")
        .and_then(Value::as_array)
        .ok_or_else(|| AnthropicCompatError::invalid("messages is required"))?;

    let system = object
        .get("system")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);
    let mut history = Vec::new();

    let last_user_index = messages
        .iter()
        .rposition(|message| message.get("role").and_then(Value::as_str) == Some("user"))
        .ok_or_else(|| AnthropicCompatError::invalid("user message is required"))?;
    for (index, message) in messages.iter().enumerate() {
        let role = message
            .get("role")
            .and_then(Value::as_str)
            .ok_or_else(|| AnthropicCompatError::invalid("message role is required"))?;
        let content_value = message
            .get("content")
            .ok_or_else(|| AnthropicCompatError::invalid("message content is required"))?;
        if index == last_user_index {
            if let Some(content_blocks) = query_media_content_blocks(content_value) {
                history.push(serde_json::json!({
                    "role": role,
                    "content": "",
                    "content_blocks": content_blocks
                }));
            }
            continue;
        }
        let content = anthropic_text_content(content_value)?;
        let mut history_entry = serde_json::json!({ "role": role, "content": content });
        if content_value.is_array() {
            history_entry["content_blocks"] = content_value.clone();
        }
        history.push(history_entry);
    }
    let query = anthropic_text_content(
        messages[last_user_index]
            .get("content")
            .ok_or_else(|| AnthropicCompatError::invalid("message content is required"))?,
    )?;

    let response_mode = object
        .get("stream")
        .and_then(Value::as_bool)
        .filter(|stream| *stream)
        .map(|_| "streaming".to_string());
    let conversation = object
        .get("metadata")
        .and_then(|metadata| metadata.get("expand_id"))
        .and_then(Value::as_str)
        .map(|user| serde_json::json!({ "user": user }))
        .unwrap_or_else(|| serde_json::json!({}));
    let metadata = object
        .get("metadata")
        .filter(|value| value.is_object())
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    let compatibility = compatibility_payload(object);
    let mut metadata = metadata;
    if !compatibility.is_null() {
        metadata["compatibility"] = compatibility.clone();
    }

    let mut native = serde_json::json!({
        "query": query,
        "model": model,
        "inputs": compatibility_inputs(compatibility),
        "history": history,
        "conversation": conversation,
        "response_mode": response_mode,
        "metadata": metadata,
        "compatibility_mode": "anthropic-messages-v1"
    });
    if let Some(system) = system {
        native["system"] = Value::String(system);
    }
    if response_mode.is_none() {
        native
            .as_object_mut()
            .expect("native request object")
            .remove("response_mode");
    }

    serde_json::from_value(native)
        .map_err(|_| AnthropicCompatError::invalid("failed to build Native request"))
}

fn compatibility_payload(object: &serde_json::Map<String, Value>) -> Value {
    let mut compatibility = serde_json::Map::new();
    for key in ["tools", "tool_choice"] {
        if let Some(value) = object.get(key) {
            compatibility.insert(key.to_string(), value.clone());
        }
    }
    if compatibility.is_empty() {
        Value::Null
    } else {
        Value::Object(compatibility)
    }
}

fn compatibility_inputs(compatibility: Value) -> Value {
    let Some(object) = compatibility.as_object() else {
        return serde_json::json!({});
    };
    let mut inputs = serde_json::Map::new();
    if let Some(tools) = object
        .get("tools")
        .and_then(Value::as_array)
        .map(|tools| {
            tools
                .iter()
                .filter_map(normalize_anthropic_tool)
                .collect::<Vec<_>>()
        })
        .filter(|tools| !tools.is_empty())
    {
        inputs.insert("tools".to_string(), Value::Array(tools));
    }
    if let Some(tool_choice) = object.get("tool_choice") {
        inputs.insert("tool_choice".to_string(), tool_choice.clone());
    }
    Value::Object(inputs)
}

fn normalize_anthropic_tool(tool: &Value) -> Option<Value> {
    let object = tool.as_object()?;
    let name = object.get("name")?.as_str()?.trim();
    if name.is_empty() {
        return None;
    }
    let mut normalized = serde_json::Map::new();
    normalized.insert("name".to_string(), Value::String(name.to_string()));
    normalized.insert(
        "source".to_string(),
        Value::String("anthropic_compatible".to_string()),
    );
    if let Some(description) = object
        .get("description")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        normalized.insert(
            "description".to_string(),
            Value::String(description.to_string()),
        );
    }
    if let Some(input_schema) = object.get("input_schema") {
        normalized.insert("input_schema".to_string(), input_schema.clone());
    }
    Some(Value::Object(normalized))
}

fn anthropic_text_content(content: &Value) -> Result<String, AnthropicCompatError> {
    if let Some(text) = content.as_str() {
        return Ok(text.to_string());
    }
    let blocks = content
        .as_array()
        .ok_or_else(|| AnthropicCompatError::invalid("content must be text"))?;
    let mut text = String::new();
    for block in blocks {
        let block_type = block
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or_default();
        match block_type {
            "text" => {
                if let Some(value) = block.get("text").and_then(Value::as_str) {
                    if !text.is_empty() {
                        text.push('\n');
                    }
                    text.push_str(value);
                }
            }
            "tool_result" => {
                let value = anthropic_tool_result_text(block);
                if !value.is_empty() {
                    if !text.is_empty() {
                        text.push('\n');
                    }
                    text.push_str(&value);
                }
            }
            "tool_use" | "server_tool_use" => {
                if block
                    .get("name")
                    .and_then(Value::as_str)
                    .is_some_and(|name| name == "computer")
                {
                    return Err(AnthropicCompatError::unsupported("computer_use"));
                }
            }
            "computer_use" => {
                return Err(AnthropicCompatError::unsupported("computer_use"));
            }
            "image" | "document" => {}
            _ => return Err(AnthropicCompatError::unsupported("messages")),
        }
    }
    Ok(text)
}

fn query_media_content_blocks(content: &Value) -> Option<Value> {
    let blocks = content.as_array()?;
    let media_blocks = blocks
        .iter()
        .filter(|block| {
            matches!(
                block.get("type").and_then(Value::as_str),
                Some("image" | "document")
            )
        })
        .cloned()
        .collect::<Vec<_>>();
    (!media_blocks.is_empty()).then_some(Value::Array(media_blocks))
}

fn anthropic_tool_result_text(block: &Value) -> String {
    let Some(content) = block.get("content") else {
        return String::new();
    };
    if let Some(text) = content.as_str() {
        return text.to_string();
    }
    if let Some(blocks) = content.as_array() {
        return blocks
            .iter()
            .filter_map(|entry| entry.get("text").and_then(Value::as_str))
            .collect::<Vec<_>>()
            .join("\n");
    }
    content.to_string()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn maps_tools_into_start_tool_registry_variables() {
        let request = map_messages_request(json!({
            "model": "claude-compatible",
            "messages": [
                { "role": "user", "content": "say hello" }
            ],
            "tools": [
                {
                    "name": "read_file",
                    "description": "Read a file",
                    "input_schema": {
                        "type": "object",
                        "properties": {
                            "file_path": { "type": "string" }
                        }
                    }
                }
            ],
            "tool_choice": { "type": "auto" }
        }))
        .unwrap();

        let inputs = request.inputs.as_value();
        assert_eq!(inputs["tools"][0]["name"], json!("read_file"));
        assert_eq!(inputs["tools"][0]["source"], json!("anthropic_compatible"));
        assert_eq!(
            inputs["tools"][0]["input_schema"]["properties"]["file_path"]["type"],
            json!("string")
        );
        assert_eq!(inputs["tool_choice"], json!({ "type": "auto" }));
        assert!(inputs.get("compatibility").is_none());
    }
}
