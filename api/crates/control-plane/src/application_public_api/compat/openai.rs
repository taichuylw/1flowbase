use serde_json::Value;

use crate::application_public_api::native::NativeRunRequest;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenAiCompatibleModel {
    pub id: String,
    pub name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenAiCompatError {
    pub message: String,
    pub error_type: String,
    pub param: Option<String>,
    pub code: String,
}

impl OpenAiCompatError {
    fn invalid(param: &'static str, message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            error_type: "invalid_request_error".to_string(),
            param: Some(param.to_string()),
            code: "invalid_request".to_string(),
        }
    }

    fn unsupported(param: &'static str) -> Self {
        Self {
            message: format!("{param} is not supported by this endpoint"),
            error_type: "invalid_request_error".to_string(),
            param: Some(param.to_string()),
            code: "unsupported_feature".to_string(),
        }
    }
}

pub fn map_chat_completion_request(request: Value) -> Result<NativeRunRequest, OpenAiCompatError> {
    reject_unsupported(&request)?;
    let object = request
        .as_object()
        .ok_or_else(|| OpenAiCompatError::invalid("body", "request body must be an object"))?;
    let model = object
        .get("model")
        .and_then(Value::as_str)
        .ok_or_else(|| OpenAiCompatError::invalid("model", "model is required"))?;
    let messages = object
        .get("messages")
        .and_then(Value::as_array)
        .ok_or_else(|| OpenAiCompatError::invalid("messages", "messages is required"))?;

    let last_user_index = messages
        .iter()
        .rposition(|message| message.get("role").and_then(Value::as_str) == Some("user"))
        .ok_or_else(|| OpenAiCompatError::invalid("messages", "user message is required"))?;
    let mut history = Vec::new();
    for (index, message) in messages.iter().enumerate() {
        let role = message
            .get("role")
            .and_then(Value::as_str)
            .ok_or_else(|| OpenAiCompatError::invalid("messages", "message role is required"))?;
        let content = openai_message_text(message)?;
        if index == last_user_index {
            continue;
        }
        let mut history_entry = serde_json::json!({ "role": role, "content": content });
        if let Some(tool_calls) = message.get("tool_calls").filter(|value| value.is_array()) {
            history_entry["tool_calls"] = tool_calls.clone();
        }
        if let Some(tool_call_id) = message.get("tool_call_id").and_then(Value::as_str) {
            history_entry["tool_call_id"] = Value::String(tool_call_id.to_string());
        }
        history.push(history_entry);
    }
    let query = openai_message_text(&messages[last_user_index])?;

    let response_mode = object
        .get("stream")
        .and_then(Value::as_bool)
        .filter(|stream| *stream)
        .map(|_| "streaming".to_string());
    let conversation = object
        .get("user")
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
        "compatibility_mode": "openai-chat-completions-v1"
    });
    if response_mode.is_none() {
        native
            .as_object_mut()
            .expect("native request object")
            .remove("response_mode");
    }

    serde_json::from_value(native)
        .map_err(|_| OpenAiCompatError::invalid("body", "failed to build Native request"))
}

pub fn extract_model_list_from_start_node(document: &Value) -> Vec<OpenAiCompatibleModel> {
    let Some(nodes) = document
        .get("graph")
        .and_then(|graph| graph.get("nodes"))
        .and_then(Value::as_array)
    else {
        return Vec::new();
    };
    let Some(start_node) = nodes
        .iter()
        .find(|node| node.get("type").and_then(Value::as_str) == Some("start"))
    else {
        return Vec::new();
    };
    let Some(model_list) = start_node
        .get("config")
        .and_then(|config| config.get("model_list"))
        .and_then(Value::as_array)
    else {
        return Vec::new();
    };

    let mut models = Vec::new();
    for value in model_list {
        if let Some(model) = normalize_model_descriptor(value) {
            if !models
                .iter()
                .any(|existing: &OpenAiCompatibleModel| existing.id == model.id)
            {
                models.push(model);
            }
        }
    }
    models
}

fn normalize_model_descriptor(value: &Value) -> Option<OpenAiCompatibleModel> {
    if let Some(id) = value.as_str().map(str::trim).filter(|id| !id.is_empty()) {
        return Some(OpenAiCompatibleModel {
            id: id.to_string(),
            name: None,
        });
    }

    let object = value.as_object()?;
    let id = ["id", "model", "value"]
        .iter()
        .find_map(|key| object.get(*key).and_then(Value::as_str))
        .map(str::trim)
        .filter(|id| !id.is_empty())?;
    let name = ["name", "label", "display_name"]
        .iter()
        .find_map(|key| object.get(*key).and_then(Value::as_str))
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(ToOwned::to_owned);

    Some(OpenAiCompatibleModel {
        id: id.to_string(),
        name,
    })
}

fn reject_unsupported(request: &Value) -> Result<(), OpenAiCompatError> {
    for field in ["audio", "modalities"] {
        if request.get(field).is_some() {
            return Err(OpenAiCompatError::unsupported(field));
        }
    }
    Ok(())
}

fn compatibility_payload(object: &serde_json::Map<String, Value>) -> Value {
    let mut compatibility = serde_json::Map::new();
    for key in ["tools", "tool_choice", "function_call"] {
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
    if compatibility.is_null() {
        return serde_json::json!({});
    }
    serde_json::json!({ "compatibility": compatibility })
}

fn openai_message_text(message: &Value) -> Result<String, OpenAiCompatError> {
    match message.get("content") {
        Some(Value::Null) | None if message.get("tool_calls").is_some() => Ok(String::new()),
        Some(content) => openai_text_content(content),
        None => Err(OpenAiCompatError::invalid(
            "messages",
            "message content is required",
        )),
    }
}

fn openai_text_content(content: &Value) -> Result<String, OpenAiCompatError> {
    if let Some(text) = content.as_str() {
        return Ok(text.to_string());
    }
    let parts = content
        .as_array()
        .ok_or_else(|| OpenAiCompatError::invalid("messages", "content must be text"))?;
    let mut text = String::new();
    for part in parts {
        let part_type = part.get("type").and_then(Value::as_str).unwrap_or_default();
        match part_type {
            "text" => {
                if let Some(value) = part.get("text").and_then(Value::as_str) {
                    if !text.is_empty() {
                        text.push('\n');
                    }
                    text.push_str(value);
                }
            }
            "input_text" => {
                if let Some(value) = part.get("text").and_then(Value::as_str) {
                    if !text.is_empty() {
                        text.push('\n');
                    }
                    text.push_str(value);
                }
            }
            "image_url" | "input_image" | "input_audio" | "file" | "input_file" => {
                return Err(OpenAiCompatError::unsupported("messages"));
            }
            _ => return Err(OpenAiCompatError::unsupported("messages")),
        }
    }
    Ok(text)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn extracts_start_node_model_list_from_strings_and_objects() {
        let document = json!({
            "graph": {
                "nodes": [
                    {
                        "id": "node-start",
                        "type": "start",
                        "config": {
                            "model_list": [
                                {"id": "qwen3.6-35b-a3b", "name": "Qwen 3.6 35B"},
                                "deepseek-v4-flash",
                                {"value": "deepseek-v4-flash", "label": "Duplicate"}
                            ]
                        }
                    }
                ]
            }
        });

        assert_eq!(
            extract_model_list_from_start_node(&document),
            vec![
                OpenAiCompatibleModel {
                    id: "qwen3.6-35b-a3b".into(),
                    name: Some("Qwen 3.6 35B".into()),
                },
                OpenAiCompatibleModel {
                    id: "deepseek-v4-flash".into(),
                    name: None,
                },
            ]
        );
    }
}
