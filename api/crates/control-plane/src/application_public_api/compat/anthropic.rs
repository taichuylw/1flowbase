use serde_json::{Map, Value};

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
    let last_user_index = messages
        .iter()
        .rposition(|message| message.get("role").and_then(Value::as_str) == Some("user"))
        .ok_or_else(|| AnthropicCompatError::invalid("user message is required"))?;
    let latest_user_content = messages[last_user_index]
        .get("content")
        .ok_or_else(|| AnthropicCompatError::invalid("message content is required"))?;
    let query =
        sanitize_anthropic_compat_text("user", &anthropic_text_content(latest_user_content)?)
            .unwrap_or_default();
    let latest_user_media_blocks = query_media_content_blocks(latest_user_content);

    let mut history = Vec::new();
    for (index, message) in messages.iter().enumerate() {
        let role = message
            .get("role")
            .and_then(Value::as_str)
            .ok_or_else(|| AnthropicCompatError::invalid("message role is required"))?;
        let content_value = message
            .get("content")
            .ok_or_else(|| AnthropicCompatError::invalid("message content is required"))?;
        if index == last_user_index {
            continue;
        }
        let content = sanitize_anthropic_compat_text(role, &anthropic_text_content(content_value)?)
            .unwrap_or_default();
        let content_blocks = history_content_blocks(content_value);
        if content.trim().is_empty() && content_blocks.is_none() {
            continue;
        }
        let mut history_entry = serde_json::json!({ "role": role, "content": content });
        if let Some(content_blocks) = content_blocks {
            history_entry["content_blocks"] = content_blocks;
        }
        history.push(history_entry);
    }
    history = dedupe_anthropic_compat_history(history);
    history = drop_replayed_current_user_turn(history, &query);
    if let Some(content_blocks) = latest_user_media_blocks {
        history.push(serde_json::json!({
            "role": "user",
            "content": "",
            "content_blocks": content_blocks
        }));
    }

    let response_mode = object
        .get("stream")
        .and_then(Value::as_bool)
        .filter(|stream| *stream)
        .map(|_| "streaming".to_string());
    let conversation = metadata_conversation(object.get("metadata"));
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

fn metadata_conversation(metadata: Option<&Value>) -> Value {
    let mut conversation = Map::new();
    let Some(metadata) = metadata.and_then(Value::as_object) else {
        return Value::Object(conversation);
    };
    let user_id = metadata
        .get("user_id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let user_id_payload = user_id.and_then(|value| serde_json::from_str::<Value>(value).ok());
    if let Some(user) = metadata
        .get("expand_id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .or_else(|| metadata_user_from_user_id(user_id, user_id_payload.as_ref()))
    {
        conversation.insert("user".to_string(), Value::String(user));
    }
    if let Some(session_id) = user_id_payload
        .as_ref()
        .and_then(|payload| payload.get("session_id"))
        .and_then(Value::as_str)
        .or_else(|| metadata.get("session_id").and_then(Value::as_str))
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        conversation.insert("id".to_string(), Value::String(session_id.to_string()));
    }
    Value::Object(conversation)
}

fn metadata_user_from_user_id(user_id: Option<&str>, payload: Option<&Value>) -> Option<String> {
    payload
        .and_then(|payload| {
            payload
                .get("account_uuid")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .or_else(|| {
                    payload
                        .get("device_id")
                        .and_then(Value::as_str)
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                })
        })
        .or(user_id)
        .map(ToOwned::to_owned)
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
            "thinking" | "redacted_thinking" => {}
            "image" | "document" => {}
            _ => return Err(AnthropicCompatError::unsupported("messages")),
        }
    }
    Ok(text)
}

fn history_content_blocks(content: &Value) -> Option<Value> {
    let blocks = content.as_array()?;
    let blocks = blocks
        .iter()
        .filter(|block| {
            !matches!(
                block.get("type").and_then(Value::as_str),
                Some("thinking" | "redacted_thinking")
            )
        })
        .cloned()
        .collect::<Vec<_>>();
    (!blocks.is_empty()).then_some(Value::Array(blocks))
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

pub fn sanitize_anthropic_compat_assistant_text(content: &str) -> Option<String> {
    sanitize_anthropic_compat_text("assistant", content)
}

fn sanitize_anthropic_compat_text(role: &str, content: &str) -> Option<String> {
    let visible_content = match role {
        "assistant" => {
            let without_thinking = strip_xml_tag_blocks(content, "think");
            let without_tool_calls = strip_xml_tag_blocks(&without_thinking, "tool_call");
            content_after_beautified_marker(&without_tool_calls)
                .unwrap_or(without_tool_calls.as_str())
                .to_string()
        }
        "user" => strip_xml_tag_blocks(content, "system-reminder"),
        _ => content.to_string(),
    };
    trimmed_compat_text(&visible_content)
}

fn strip_xml_tag_blocks(content: &str, tag: &str) -> String {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let mut output = content.to_string();

    while let Some(start) = output.find(&open) {
        let search_start = start + open.len();
        let Some(end) = output[search_start..].find(&close) else {
            break;
        };
        let end = search_start + end + close.len();
        output.replace_range(start..end, "");
    }

    output
}

fn content_after_beautified_marker(content: &str) -> Option<&str> {
    let marker = "下面是美化后内容";
    let marker_start = content.find(marker)?;
    Some(
        content[marker_start + marker.len()..].trim_start_matches(|value: char| {
            value.is_whitespace() || value == '-' || value == '—'
        }),
    )
}

fn trimmed_compat_text(content: &str) -> Option<String> {
    let trimmed = content.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

fn dedupe_anthropic_compat_history(history: Vec<Value>) -> Vec<Value> {
    let mut deduped = Vec::new();
    let mut previous_user_turn: Option<(usize, String)> = None;

    for message in history {
        if message.get("role").and_then(Value::as_str) == Some("user") {
            if let Some(key) = dedupe_user_turn_key(&message) {
                if previous_user_turn
                    .as_ref()
                    .is_some_and(|(_, previous_key)| previous_key == &key)
                {
                    if let Some((start, _)) = previous_user_turn.take() {
                        deduped.truncate(start);
                    }
                }
                previous_user_turn = Some((deduped.len(), key));
            } else {
                previous_user_turn = None;
            }
        }
        deduped.push(message);
    }

    deduped
}

fn drop_replayed_current_user_turn(mut history: Vec<Value>, current_query: &str) -> Vec<Value> {
    let current_query = current_query.trim();
    if current_query.is_empty() {
        return history;
    }

    for (index, message) in history.iter().enumerate().rev() {
        if message.get("role").and_then(Value::as_str) != Some("user") {
            continue;
        }
        if dedupe_user_turn_key(message).is_some_and(|key| key == current_query) {
            history.truncate(index);
        }
        break;
    }

    history
}

fn dedupe_user_turn_key(message: &Value) -> Option<String> {
    if message
        .get("content_blocks")
        .and_then(Value::as_array)
        .is_some_and(|blocks| {
            blocks.iter().any(|block| {
                matches!(
                    block.get("type").and_then(Value::as_str),
                    Some("tool_result" | "tool_use" | "server_tool_use")
                )
            })
        })
    {
        return None;
    }

    message
        .get("content")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|content| !content.is_empty())
        .map(ToOwned::to_owned)
        .or_else(|| {
            message
                .get("content_blocks")
                .filter(|content_blocks| !content_blocks.is_null())
                .map(Value::to_string)
        })
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

    #[test]
    fn maps_latest_claude_code_query_without_system_reminder() {
        let request = map_messages_request(json!({
            "model": "1flowbase",
            "messages": [
                {
                    "role": "user",
                    "content": "<system-reminder>internal tools</system-reminder>\n\nhi？"
                }
            ]
        }))
        .unwrap();

        assert_eq!(request.query, "hi？");
        assert!(request.history.is_empty());
    }

    #[test]
    fn maps_claude_code_history_without_internal_transcript_payloads() {
        let request = map_messages_request(json!({
            "model": "1flowbase",
            "messages": [
                {
                    "role": "user",
                    "content": "<system-reminder>available tools</system-reminder>\n\nhi？"
                },
                {
                    "role": "assistant",
                    "content": "<think>private reasoning</think>嗨，有什么需要我帮忙的？"
                },
                {
                    "role": "user",
                    "content": "uploads/agent-flow-preview-debug.png 描述一下这幅图说什么？"
                }
            ],
            "tools": [
                {
                    "name": "Read",
                    "description": "Read a file",
                    "input_schema": { "type": "object" }
                }
            ]
        }))
        .unwrap();

        assert_eq!(
            request.query,
            "uploads/agent-flow-preview-debug.png 描述一下这幅图说什么？"
        );
        assert_eq!(
            request.history,
            vec![
                json!({"role": "user", "content": "hi？"}),
                json!({"role": "assistant", "content": "嗨，有什么需要我帮忙的？"}),
            ]
        );
        assert_eq!(request.inputs.as_value()["tools"][0]["name"], json!("Read"));
    }

    #[test]
    fn maps_claude_code_history_keeps_last_duplicate_user_turn() {
        let request = map_messages_request(json!({
            "model": "1flowbase",
            "messages": [
                {"role": "user", "content": "Describe image"},
                {"role": "assistant", "content": "old draft"},
                {"role": "user", "content": "Describe image"},
                {"role": "assistant", "content": "<think>retry</think>final draft"},
                {"role": "user", "content": "Continue"}
            ]
        }))
        .unwrap();

        assert_eq!(request.query, "Continue");
        assert_eq!(
            request.history,
            vec![
                json!({"role": "user", "content": "Describe image"}),
                json!({"role": "assistant", "content": "final draft"}),
            ]
        );
    }

    #[test]
    fn maps_claude_code_history_drops_replayed_current_user_turn() {
        let request = map_messages_request(json!({
            "model": "1flowbase",
            "messages": [
                {"role": "user", "content": "Describe image"},
                {"role": "assistant", "content": "old image answer"},
                {"role": "user", "content": "Describe image"}
            ]
        }))
        .unwrap();

        assert_eq!(request.query, "Describe image");
        assert!(request.history.is_empty());
    }

    #[test]
    fn maps_claude_code_history_preserves_latest_media_after_dropping_replay() {
        let request = map_messages_request(json!({
            "model": "1flowbase",
            "messages": [
                {"role": "user", "content": "Describe image"},
                {"role": "assistant", "content": "old image answer"},
                {
                    "role": "user",
                    "content": [
                        {"type": "text", "text": "Describe image"},
                        {
                            "type": "image",
                            "source": {
                                "type": "base64",
                                "media_type": "image/png",
                                "data": "aW1hZ2U="
                            }
                        }
                    ]
                }
            ]
        }))
        .unwrap();

        assert_eq!(request.query, "Describe image");
        assert_eq!(request.history.len(), 1);
        assert_eq!(request.history[0]["role"], json!("user"));
        assert_eq!(request.history[0]["content"], json!(""));
        assert_eq!(
            request.history[0]["content_blocks"][0]["type"],
            json!("image")
        );
    }

    #[test]
    fn maps_claude_code_history_keeps_visible_text_after_beautified_marker() {
        let request = map_messages_request(json!({
            "model": "1flowbase",
            "messages": [
                {"role": "user", "content": "hi？"},
                {
                    "role": "assistant",
                    "content": "<think>draft</think>嗨！\n\n---\n\n下面是美化后内容\n\n你好，有需要我随时帮你。"
                },
                {"role": "user", "content": "继续"}
            ]
        }))
        .unwrap();

        assert_eq!(
            request.history,
            vec![
                json!({"role": "user", "content": "hi？"}),
                json!({"role": "assistant", "content": "你好，有需要我随时帮你。"}),
            ]
        );
    }
}
