use serde_json::{Map, Value};
use uuid::Uuid;

use crate::application_public_api::callback_tool_ids::decode_openai_callback_tool_call_id;
pub use crate::application_public_api::model_catalog::{
    extract_agent_model_catalog_from_start_node as extract_model_list_from_start_node,
    AgentModelDescriptor as OpenAiCompatibleModel,
};
use crate::application_public_api::native::NativeRunRequest;

const OPENAI_CHAT_COMPLETIONS_COMPATIBILITY_MODE: &str = "openai-chat-completions-v1";
const OPENAI_RESPONSES_COMPATIBILITY_MODE: &str = "openai-responses-v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenAiCompatError {
    pub message: String,
    pub error_type: String,
    pub param: Option<String>,
    pub code: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenAiPreviousResponseContext {
    pub response_id: String,
    pub external_user: Option<String>,
    pub external_conversation_id: Option<String>,
    pub answer: Option<String>,
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
    let mut system_parts = Vec::new();
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
        if role == "system" {
            if !content.trim().is_empty() {
                system_parts.push(content);
            }
            continue;
        }
        let mut history_entry = serde_json::json!({ "role": role, "content": content });
        if let Some(tool_calls) = message.get("tool_calls").filter(|value| value.is_array()) {
            history_entry["tool_calls"] = openai_chat_history_tool_calls(tool_calls);
        }
        if let Some(tool_call_id) = message.get("tool_call_id").and_then(Value::as_str) {
            history_entry["tool_call_id"] =
                Value::String(openai_chat_history_tool_call_id(tool_call_id));
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
        "compatibility_mode": OPENAI_CHAT_COMPLETIONS_COMPATIBILITY_MODE
    });
    if let Some(system) = system_from_parts(system_parts) {
        native["system"] = Value::String(system);
    }
    if response_mode.is_none() {
        native
            .as_object_mut()
            .expect("native request object")
            .remove("response_mode");
    }

    serde_json::from_value(native)
        .map_err(|_| OpenAiCompatError::invalid("body", "failed to build Native request"))
}

pub fn map_response_request(
    request: Value,
    previous_response: Option<OpenAiPreviousResponseContext>,
) -> Result<NativeRunRequest, OpenAiCompatError> {
    reject_unsupported(&request)?;
    let object = request
        .as_object()
        .ok_or_else(|| OpenAiCompatError::invalid("body", "request body must be an object"))?;
    let model = object
        .get("model")
        .and_then(Value::as_str)
        .ok_or_else(|| OpenAiCompatError::invalid("model", "model is required"))?;
    let input = object
        .get("input")
        .ok_or_else(|| OpenAiCompatError::invalid("input", "input is required"))?;
    let (query, input_history) = responses_input_to_query_and_history(input)?;
    let mut history = responses_previous_history(previous_response.as_ref());
    history.extend(input_history);
    let system = object
        .get("instructions")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);

    let response_mode = object
        .get("stream")
        .and_then(Value::as_bool)
        .filter(|stream| *stream)
        .map(|_| "streaming".to_string());
    let conversation = responses_conversation(object, previous_response.as_ref());
    let metadata = object
        .get("metadata")
        .filter(|value| value.is_object())
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    let compatibility = responses_compatibility_payload(object, previous_response.as_ref());
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
        "compatibility_mode": OPENAI_RESPONSES_COMPATIBILITY_MODE
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
        .map_err(|_| OpenAiCompatError::invalid("body", "failed to build Native request"))
}

fn system_from_parts(parts: Vec<String>) -> Option<String> {
    (!parts.is_empty()).then(|| parts.join("\n\n"))
}

pub fn response_id_from_run_id(run_id: Uuid) -> String {
    format!("resp_{run_id}")
}

pub fn run_id_from_response_id(response_id: &str) -> Result<Uuid, OpenAiCompatError> {
    let run_id = response_id
        .strip_prefix("resp_")
        .ok_or_else(|| OpenAiCompatError::invalid("previous_response_id", "invalid response id"))?;
    Uuid::parse_str(run_id)
        .map_err(|_| OpenAiCompatError::invalid("previous_response_id", "invalid response id"))
}

fn openai_chat_history_tool_calls(tool_calls: &Value) -> Value {
    let Some(calls) = tool_calls.as_array() else {
        return tool_calls.clone();
    };
    Value::Array(
        calls
            .iter()
            .map(|call| {
                let Some(object) = call.as_object() else {
                    return call.clone();
                };
                let mut normalized = object.clone();
                if let Some(id) = object.get("id").and_then(Value::as_str) {
                    normalized.insert(
                        "id".to_string(),
                        Value::String(openai_chat_history_tool_call_id(id)),
                    );
                }
                Value::Object(normalized)
            })
            .collect(),
    )
}

fn openai_chat_history_tool_call_id(value: &str) -> String {
    decode_openai_callback_tool_call_id(value)
        .map(|(_, original_tool_call_id)| original_tool_call_id)
        .unwrap_or_else(|| value.to_string())
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

fn responses_compatibility_payload(
    object: &Map<String, Value>,
    previous_response: Option<&OpenAiPreviousResponseContext>,
) -> Value {
    let mut compatibility = serde_json::Map::new();
    for key in [
        "tools",
        "tool_choice",
        "parallel_tool_calls",
        "response_format",
        "text",
        "reasoning",
    ] {
        if let Some(value) = object.get(key) {
            compatibility.insert(key.to_string(), value.clone());
        }
    }
    if let Some(previous_response) = previous_response {
        compatibility.insert(
            "previous_response_id".to_string(),
            Value::String(previous_response.response_id.clone()),
        );
    }
    if compatibility.is_empty() {
        Value::Null
    } else {
        Value::Object(compatibility)
    }
}

fn responses_conversation(
    object: &Map<String, Value>,
    previous_response: Option<&OpenAiPreviousResponseContext>,
) -> Value {
    let mut conversation = serde_json::Map::new();
    if let Some(user) = object
        .get("user")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        conversation.insert("user".to_string(), Value::String(user.to_string()));
    }
    if let Some(previous_response) = previous_response {
        if !conversation.contains_key("user") {
            if let Some(user) = previous_response.external_user.as_ref() {
                conversation.insert("user".to_string(), Value::String(user.clone()));
            }
        }
        if let Some(conversation_id) = previous_response.external_conversation_id.as_ref() {
            conversation.insert("id".to_string(), Value::String(conversation_id.clone()));
        }
    }
    Value::Object(conversation)
}

fn responses_previous_history(
    previous_response: Option<&OpenAiPreviousResponseContext>,
) -> Vec<Value> {
    previous_response
        .and_then(|previous_response| {
            previous_response.answer.as_ref().map(|answer| {
                serde_json::json!({
                    "role": "assistant",
                    "content": answer,
                    "response_id": previous_response.response_id,
                })
            })
        })
        .into_iter()
        .collect()
}

fn responses_input_to_query_and_history(
    input: &Value,
) -> Result<(String, Vec<Value>), OpenAiCompatError> {
    if let Some(text) = input.as_str() {
        return Ok((text.to_string(), Vec::new()));
    }
    let items = input
        .as_array()
        .ok_or_else(|| OpenAiCompatError::invalid("input", "input must be text or messages"))?;
    let messages = items
        .iter()
        .map(responses_input_message)
        .collect::<Result<Vec<_>, _>>()?;
    let last_user_index = messages
        .iter()
        .rposition(|message| message.role == "user")
        .ok_or_else(|| OpenAiCompatError::invalid("input", "user input is required"))?;
    let mut history = Vec::new();
    for (index, message) in messages.into_iter().enumerate() {
        if index == last_user_index {
            return Ok((message.content, history));
        }
        history.push(serde_json::json!({
            "role": message.role,
            "content": message.content
        }));
    }
    Err(OpenAiCompatError::invalid(
        "input",
        "user input is required",
    ))
}

struct ResponsesInputMessage {
    role: String,
    content: String,
}

fn responses_input_message(item: &Value) -> Result<ResponsesInputMessage, OpenAiCompatError> {
    let object = item
        .as_object()
        .ok_or_else(|| OpenAiCompatError::invalid("input", "input message must be an object"))?;
    let role = object
        .get("role")
        .and_then(Value::as_str)
        .unwrap_or("user")
        .to_string();
    let content = match object.get("content") {
        Some(content) => openai_text_content(content)?,
        None => object
            .get("text")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
            .ok_or_else(|| OpenAiCompatError::invalid("input", "input content is required"))?,
    };
    Ok(ResponsesInputMessage { role, content })
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
                .filter_map(normalize_openai_tool)
                .collect::<Vec<_>>()
        })
        .filter(|tools| !tools.is_empty())
    {
        inputs.insert("tools".to_string(), Value::Array(tools));
    }
    if let Some(tool_choice) = object.get("tool_choice") {
        inputs.insert("tool_choice".to_string(), tool_choice.clone());
    } else if let Some(function_call) = object.get("function_call") {
        inputs.insert("tool_choice".to_string(), function_call.clone());
    }
    Value::Object(inputs)
}

fn normalize_openai_tool(tool: &Value) -> Option<Value> {
    let function = tool.get("function")?.as_object()?;
    let name = function.get("name")?.as_str()?.trim();
    if name.is_empty() {
        return None;
    }
    let mut normalized = serde_json::Map::new();
    normalized.insert("name".to_string(), Value::String(name.to_string()));
    normalized.insert(
        "source".to_string(),
        Value::String("openai_compatible".to_string()),
    );
    if let Some(description) = function
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
    if let Some(parameters) = function.get("parameters") {
        normalized.insert("input_schema".to_string(), parameters.clone());
    }
    Some(Value::Object(normalized))
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
    use crate::application_public_api::model_catalog::{
        AgentModelCapabilities, AgentModelReasoning,
    };

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
                                {
                                    "id": "qwen3.6-35b-a3b",
                                    "name": "Qwen 3.6 35B",
                                    "context_window": 128000,
                                    "auto_compact_token_limit": 110000
                                },
                                "deepseek-v4-flash",
                                {"id": "deepseek-v4-flash", "name": "Duplicate"}
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
                    context_window: Some(128000),
                    max_context_window: None,
                    max_output_tokens: None,
                    auto_compact_token_limit: Some(110000),
                    capabilities: AgentModelCapabilities::default(),
                    reasoning: None,
                },
                OpenAiCompatibleModel {
                    id: "deepseek-v4-flash".into(),
                    name: None,
                    context_window: None,
                    max_context_window: None,
                    max_output_tokens: None,
                    auto_compact_token_limit: None,
                    capabilities: AgentModelCapabilities::default(),
                    reasoning: None,
                },
            ]
        );
    }

    #[test]
    fn extracts_default_model_when_start_node_has_no_model_list() {
        let document = json!({
            "graph": {
                "nodes": [
                    {
                        "id": "node-start",
                        "type": "start",
                        "config": {
                            "input_fields": []
                        }
                    }
                ]
            }
        });

        assert_eq!(
            extract_model_list_from_start_node(&document),
            vec![OpenAiCompatibleModel {
                id: "1flowbase".into(),
                name: Some("1flowbase".into()),
                context_window: Some(257000),
                max_context_window: Some(128000),
                max_output_tokens: Some(32000),
                auto_compact_token_limit: Some(218450),
                capabilities: AgentModelCapabilities {
                    reasoning: true,
                    tool_call: true,
                    multimodal: true,
                    structured_output: true,
                },
                reasoning: Some(AgentModelReasoning {
                    default_effort: Some("medium".into()),
                    supported_efforts: vec![
                        "minimal".into(),
                        "low".into(),
                        "medium".into(),
                        "high".into(),
                        "xhigh".into(),
                    ],
                }),
            }]
        );
    }

    #[test]
    fn maps_tools_into_start_tool_registry_variables() {
        let request = map_chat_completion_request(json!({
            "model": "deepseek-v4-flash",
            "messages": [
                { "role": "user", "content": "say hello" }
            ],
            "tools": [
                {
                    "type": "function",
                    "function": {
                        "name": "read_file",
                        "description": "Read a file",
                        "parameters": {
                            "type": "object",
                            "properties": {
                                "file_path": { "type": "string" }
                            }
                        }
                    }
                }
            ],
            "tool_choice": "auto"
        }))
        .unwrap();

        let inputs = request.inputs.as_value();
        assert_eq!(inputs["tools"][0]["name"], json!("read_file"));
        assert_eq!(inputs["tools"][0]["source"], json!("openai_compatible"));
        assert_eq!(
            inputs["tools"][0]["input_schema"]["properties"]["file_path"]["type"],
            json!("string")
        );
        assert_eq!(inputs["tool_choice"], json!("auto"));
        assert!(inputs.get("function_call").is_none());
        assert!(inputs.get("compatibility").is_none());
    }

    #[test]
    fn chat_history_decodes_external_callback_tool_ids_before_native_history() {
        let external_tool_call_id = "calltask_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa_call_weather_lookup";

        let request = map_chat_completion_request(json!({
            "model": "deepseek-v4-flash",
            "messages": [
                { "role": "user", "content": "first question" },
                {
                    "role": "assistant",
                    "content": null,
                    "tool_calls": [
                        {
                            "id": external_tool_call_id,
                            "type": "function",
                            "function": {
                                "name": "lookup_weather",
                                "arguments": "{}"
                            }
                        }
                    ]
                },
                {
                    "role": "tool",
                    "tool_call_id": external_tool_call_id,
                    "content": "{\"temperature\":21}"
                },
                { "role": "assistant", "content": "old answer" },
                { "role": "user", "content": "next question" }
            ]
        }))
        .unwrap();

        assert_eq!(request.query, "next question");
        assert_eq!(
            request.history[1]["tool_calls"][0]["id"],
            json!("call_weather_lookup")
        );
        assert_eq!(
            request.history[2]["tool_call_id"],
            json!("call_weather_lookup")
        );
    }

    #[test]
    fn chat_history_preserves_unrecognized_tool_ids() {
        let request = map_chat_completion_request(json!({
            "model": "deepseek-v4-flash",
            "messages": [
                { "role": "user", "content": "first question" },
                {
                    "role": "assistant",
                    "content": null,
                    "tool_calls": [
                        {
                            "id": "calltask_not-a-valid-callback",
                            "type": "function",
                            "function": {
                                "name": "lookup_weather",
                                "arguments": "{}"
                            }
                        }
                    ]
                },
                {
                    "role": "tool",
                    "tool_call_id": "provider_native_call",
                    "content": "{\"temperature\":21}"
                },
                { "role": "user", "content": "next question" }
            ]
        }))
        .unwrap();

        assert_eq!(
            request.history[1]["tool_calls"][0]["id"],
            json!("calltask_not-a-valid-callback")
        );
        assert_eq!(
            request.history[2]["tool_call_id"],
            json!("provider_native_call")
        );
    }

    #[test]
    fn maps_legacy_function_call_into_tool_choice_variable() {
        let request = map_chat_completion_request(json!({
            "model": "deepseek-v4-flash",
            "messages": [
                { "role": "user", "content": "say hello" }
            ],
            "function_call": { "name": "read_file" }
        }))
        .unwrap();

        let inputs = request.inputs.as_value();
        assert_eq!(inputs["tool_choice"], json!({ "name": "read_file" }));
        assert!(inputs.get("function_call").is_none());
        assert!(inputs.get("compatibility").is_none());
    }

    #[test]
    fn maps_responses_text_input_into_native_run() {
        let request = map_response_request(
            json!({
                "model": "deepseek-v4-flash",
                "input": "Summarize the incident",
                "user": "external-user-1",
                "metadata": {"trace_id": "trace-responses"},
                "stream": true
            }),
            None,
        )
        .unwrap();

        assert_eq!(request.query, "Summarize the incident");
        assert_eq!(request.model.as_deref(), Some("deepseek-v4-flash"));
        assert_eq!(request.response_mode.as_deref(), Some("streaming"));
        assert_eq!(
            request.compatibility_mode.as_deref(),
            Some("openai-responses-v1")
        );
        assert_eq!(request.conversation["user"], json!("external-user-1"));
        assert_eq!(request.metadata["trace_id"], json!("trace-responses"));
    }

    #[test]
    fn maps_previous_response_context_into_native_conversation_and_history() {
        let request = map_response_request(
            json!({
                "model": "deepseek-v4-flash",
                "input": [{"role": "user", "content": [{"type": "input_text", "text": "Continue"}]}],
                "previous_response_id": "resp_11111111-1111-1111-1111-111111111111"
            }),
            Some(OpenAiPreviousResponseContext {
                response_id: "resp_11111111-1111-1111-1111-111111111111".to_string(),
                external_user: Some("external-user-1".to_string()),
                external_conversation_id: Some("conv_123".to_string()),
                answer: Some("Earlier answer".to_string()),
            }),
        )
        .unwrap();

        assert_eq!(request.query, "Continue");
        assert_eq!(request.conversation["user"], json!("external-user-1"));
        assert_eq!(request.conversation["id"], json!("conv_123"));
        assert_eq!(
            request.history,
            vec![json!({
                "role": "assistant",
                "content": "Earlier answer",
                "response_id": "resp_11111111-1111-1111-1111-111111111111"
            })]
        );
        assert_eq!(
            request.metadata["compatibility"]["previous_response_id"],
            json!("resp_11111111-1111-1111-1111-111111111111")
        );
    }
}
