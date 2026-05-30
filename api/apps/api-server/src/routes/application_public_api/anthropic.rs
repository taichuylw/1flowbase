use std::sync::Arc;

use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use control_plane::application_public_api::{
    api_keys::ApplicationApiKeyService,
    compat::anthropic::{map_messages_request, AnthropicCompatError},
    native::{
        ApplicationNativeRunService, CreateNativeRunCommand, NativeRunRequest, NativeRunResult,
        NativeRunValidationError,
    },
    run_service::native_result_from_run_detail,
};
use control_plane::orchestration_runtime::{
    CompleteCallbackTaskCommand, OrchestrationRuntimeService,
};
use serde::Serialize;
use serde_json::{json, Value};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    app_state::ApiState,
    provider_runtime::ApiProviderRuntime,
    routes::application_public_api::{
        compat_sse, native,
        tool_callback_ids::{
            decode_anthropic_callback_tool_use_id, encode_anthropic_callback_tool_use_id,
        },
    },
};

#[derive(Debug, Serialize, ToSchema)]
pub struct AnthropicErrorBody {
    #[serde(rename = "type")]
    pub body_type: &'static str,
    pub error: AnthropicErrorObject,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AnthropicErrorObject {
    #[serde(rename = "type")]
    pub error_type: String,
    pub message: String,
}

#[derive(Debug)]
pub enum AnthropicRouteError {
    Compat(AnthropicCompatError),
    Native(native::NativeApiError),
    RequiredAction,
}

impl From<AnthropicCompatError> for AnthropicRouteError {
    fn from(error: AnthropicCompatError) -> Self {
        Self::Compat(error)
    }
}

impl From<native::NativeApiError> for AnthropicRouteError {
    fn from(error: native::NativeApiError) -> Self {
        Self::Native(error)
    }
}

impl IntoResponse for AnthropicRouteError {
    fn into_response(self) -> Response {
        let (status, error) = match self {
            AnthropicRouteError::Compat(error) => (
                StatusCode::BAD_REQUEST,
                AnthropicErrorObject {
                    error_type: error.error_type,
                    message: error.message,
                },
            ),
            AnthropicRouteError::Native(error) => (
                error.status,
                AnthropicErrorObject {
                    error_type: error.code.to_string(),
                    message: error.message,
                },
            ),
            AnthropicRouteError::RequiredAction => (
                StatusCode::CONFLICT,
                AnthropicErrorObject {
                    error_type: "required_action_not_supported".to_string(),
                    message: "waiting states are not supported by compatible endpoints; use the Native API to inspect and resume required_action runs".to_string(),
                },
            ),
        };
        (
            status,
            Json(AnthropicErrorBody {
                body_type: "error",
                error,
            }),
        )
            .into_response()
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AnthropicMessageResponse {
    pub id: String,
    #[serde(rename = "type")]
    pub response_type: &'static str,
    pub role: &'static str,
    pub model: String,
    pub content: Vec<Value>,
    pub stop_reason: &'static str,
    pub usage: AnthropicUsage,
}

#[derive(Debug, Default, Serialize, ToSchema)]
pub struct AnthropicUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
}

struct AnthropicToolResumeRequest {
    callback_task_id: Uuid,
    tool_results: Value,
}

#[utoipa::path(
    post,
    path = "/v1/messages",
    request_body = Value,
    responses(
        (status = 200, body = AnthropicMessageResponse),
        (status = 400, body = AnthropicErrorBody),
        (status = 401, body = AnthropicErrorBody),
        (status = 403, body = AnthropicErrorBody),
        (status = 409, body = AnthropicErrorBody)
    )
)]
pub async fn create_message(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response, AnthropicRouteError> {
    let bearer_token = anthropic_token(&headers)?;
    let value = parse_anthropic_json_body(body)?;
    let model = value
        .get("model")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let response_mode = value
        .get("stream")
        .and_then(Value::as_bool)
        .filter(|stream| *stream)
        .map(|_| "streaming".to_string());
    if let Some(resume) = anthropic_tool_resume_request(&value)? {
        let run = resume_anthropic_tool_call(
            state,
            &bearer_token,
            resume.callback_task_id,
            resume.tool_results,
        )
        .await?;
        if !anthropic_required_action_is_supported(&run) {
            return Err(AnthropicRouteError::RequiredAction);
        }
        if response_mode.as_deref() == Some("streaming") {
            return Ok(compat_sse::completed_anthropic_stream(run, model));
        }
        return Ok(Json(to_anthropic_response(run, model)).into_response());
    }
    let request = map_messages_request(value)?;
    let response_mode = request.response_mode.clone();
    let run = create_native_run(state.clone(), bearer_token.clone(), request).await?;

    if response_mode.as_deref() == Some("streaming") {
        return compat_sse::start_anthropic_run_stream(state, run, model)
            .await
            .map_err(Into::into);
    }

    let run = native::execute_blocking_native_run(state, bearer_token, run).await?;
    if !anthropic_required_action_is_supported(&run) {
        return Err(AnthropicRouteError::RequiredAction);
    }
    Ok(Json(to_anthropic_response(run, model)).into_response())
}

fn anthropic_token(headers: &HeaderMap) -> Result<String, native::NativeApiError> {
    if let Ok(token) = native::bearer_token(headers) {
        return Ok(token);
    }
    headers
        .get("x-api-key")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| {
            native::NativeApiError::new(
                StatusCode::UNAUTHORIZED,
                "not_authenticated",
                "missing Authorization bearer token or x-api-key",
            )
        })
}

fn parse_anthropic_json_body(body: Bytes) -> Result<Value, AnthropicRouteError> {
    serde_json::from_slice::<Value>(&body).map_err(|_| {
        AnthropicCompatError {
            message: "invalid JSON body".to_string(),
            error_type: "invalid_request".to_string(),
        }
        .into()
    })
}

async fn create_native_run(
    state: Arc<ApiState>,
    bearer_token: String,
    request: NativeRunRequest,
) -> Result<NativeRunResult, native::NativeApiError> {
    ApplicationNativeRunService::new(state.store.clone())
        .create_native_run(CreateNativeRunCommand {
            bearer_token,
            request,
        })
        .await
        .map_err(native::native_error)
}

async fn resume_anthropic_tool_call(
    state: Arc<ApiState>,
    bearer_token: &str,
    callback_task_id: Uuid,
    tool_results: Value,
) -> Result<NativeRunResult, AnthropicRouteError> {
    let api_actor = ApplicationApiKeyService::new(state.store.clone())
        .with_last_used_cache(state.infrastructure.cache_store())
        .authenticate_bearer_token(bearer_token)
        .await
        .map_err(|_| native::native_error(NativeRunValidationError::NotAuthenticated))?;
    let detail = OrchestrationRuntimeService::new(
        state.store.clone(),
        ApiProviderRuntime::new(state.provider_runtime.clone()),
        state.runtime_engine.clone(),
        state.provider_secret_master_key.clone(),
    )
    .complete_callback_task(CompleteCallbackTaskCommand {
        actor_user_id: api_actor.creator_user_id,
        application_id: api_actor.application_id,
        callback_task_id,
        response_payload: json!({ "tool_results": tool_results }),
    })
    .await
    .map_err(native::service_error)?;

    Ok(native_result_from_run_detail(
        &detail,
        json!({
            "external_user": detail.flow_run.external_user,
            "external_conversation_id": detail.flow_run.external_conversation_id,
            "external_trace_id": detail.flow_run.external_trace_id,
            "compatibility_mode": detail.flow_run.compatibility_mode,
            "idempotency_key": detail.flow_run.idempotency_key,
            "request": {
                "conversation": {
                    "id": detail.flow_run.external_conversation_id,
                    "user": detail.flow_run.external_user,
                }
            }
        }),
    ))
}

fn to_anthropic_response(run: NativeRunResult, model: String) -> AnthropicMessageResponse {
    let callback_task_id = callback_task_id_from_required_action(&run);
    let tool_blocks = anthropic_tool_use_blocks(run.tool_calls.as_ref(), callback_task_id);
    let has_tool_blocks = tool_blocks
        .as_ref()
        .is_some_and(|blocks| !blocks.is_empty());
    let mut content = Vec::new();
    if let Some(answer) = run.answer {
        if !answer.is_empty() {
            content.push(json!({"type": "text", "text": answer}));
        }
    }
    if let Some(blocks) = tool_blocks {
        content.extend(blocks);
    }
    if content.is_empty() {
        content.push(json!({"type": "text", "text": ""}));
    }
    AnthropicMessageResponse {
        id: format!("msg_{}", run.id),
        response_type: "message",
        role: "assistant",
        model,
        content,
        stop_reason: if has_tool_blocks {
            "tool_use"
        } else {
            "end_turn"
        },
        usage: anthropic_usage(run.usage),
    }
}

fn anthropic_tool_use_blocks(
    tool_calls: Option<&Value>,
    callback_task_id: Option<Uuid>,
) -> Option<Vec<Value>> {
    let calls = tool_calls?.as_array()?;
    let mapped = calls
        .iter()
        .filter_map(|call| {
            let name = call.get("name").and_then(Value::as_str)?;
            let original_id = call
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or("toolu_call")
                .to_string();
            let id = callback_task_id
                .map(|callback_task_id| {
                    encode_anthropic_callback_tool_use_id(callback_task_id, &original_id)
                })
                .unwrap_or(original_id);
            let input = call.get("arguments").cloned().unwrap_or_else(|| json!({}));
            Some(json!({
                "type": "tool_use",
                "id": id,
                "name": name,
                "input": input
            }))
        })
        .collect::<Vec<_>>();
    (!mapped.is_empty()).then_some(mapped)
}

fn anthropic_tool_resume_request(
    request: &Value,
) -> Result<Option<AnthropicToolResumeRequest>, AnthropicRouteError> {
    let Some(messages) = request.get("messages").and_then(Value::as_array) else {
        return Ok(None);
    };
    let mut callback_task_id = None;
    let mut tool_results = Vec::new();

    for message in messages {
        if message.get("role").and_then(Value::as_str) != Some("user") {
            continue;
        }
        let Some(blocks) = message.get("content").and_then(Value::as_array) else {
            continue;
        };
        for block in blocks {
            if block.get("type").and_then(Value::as_str) != Some("tool_result") {
                continue;
            }
            let Some(tool_use_id) = block.get("tool_use_id").and_then(Value::as_str) else {
                return Err(AnthropicCompatError {
                    message: "tool_result tool_use_id is required".to_string(),
                    error_type: "invalid_request".to_string(),
                }
                .into());
            };
            let Some((decoded_callback_task_id, original_tool_use_id)) =
                decode_anthropic_callback_tool_use_id(tool_use_id)
            else {
                continue;
            };
            if let Some(existing_callback_task_id) = callback_task_id {
                if existing_callback_task_id != decoded_callback_task_id {
                    return Err(AnthropicCompatError {
                        message: "tool_result blocks must belong to one callback task".to_string(),
                        error_type: "invalid_request".to_string(),
                    }
                    .into());
                }
            } else {
                callback_task_id = Some(decoded_callback_task_id);
            }
            tool_results.push(json!({
                "tool_call_id": original_tool_use_id,
                "content": anthropic_tool_result_content(block),
            }));
        }
    }

    Ok(
        callback_task_id.map(|callback_task_id| AnthropicToolResumeRequest {
            callback_task_id,
            tool_results: Value::Array(tool_results),
        }),
    )
}

fn anthropic_tool_result_content(block: &Value) -> String {
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

fn callback_task_id_from_required_action(run: &NativeRunResult) -> Option<Uuid> {
    run.required_action
        .as_ref()
        .and_then(|action| action.payload.get("callback_task_id"))
        .and_then(Value::as_str)
        .and_then(|value| Uuid::parse_str(value).ok())
}

fn anthropic_required_action_is_supported(run: &NativeRunResult) -> bool {
    run.required_action.as_ref().is_none_or(|action| {
        action.payload.get("callback_kind").and_then(Value::as_str) == Some("llm_tool_calls")
            && run
                .tool_calls
                .as_ref()
                .is_some_and(|value| value.as_array().is_some_and(|calls| !calls.is_empty()))
    })
}

fn anthropic_usage(
    usage: Option<control_plane::application_public_api::native::NativeUsage>,
) -> AnthropicUsage {
    let Some(usage) = usage else {
        return AnthropicUsage::default();
    };
    AnthropicUsage {
        input_tokens: usage.prompt_tokens.unwrap_or_default(),
        output_tokens: usage.completion_tokens.unwrap_or_default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use control_plane::application_public_api::native::{NativeRequiredAction, NativeRunStatus};
    use time::OffsetDateTime;
    use uuid::Uuid;

    #[test]
    fn anthropic_response_projects_native_tool_calls() {
        let run = NativeRunResult {
            id: Uuid::nil(),
            application_id: Uuid::nil(),
            api_key_id: Uuid::nil(),
            publication_version_id: Uuid::nil(),
            status: NativeRunStatus::Succeeded,
            node_input_payload: json!({}),
            metadata: json!({}),
            answer: None,
            required_action: None,
            tool_calls: Some(json!([
                {
                    "id": "toolu_123",
                    "name": "lookup_order",
                    "arguments": {"order_id": "order_123"}
                }
            ])),
            usage: None,
            error: None,
            created_at: OffsetDateTime::UNIX_EPOCH,
        };

        let payload = serde_json::to_value(to_anthropic_response(run, "provider/model".into()))
            .expect("anthropic response serializes");

        assert_eq!(payload["stop_reason"], json!("tool_use"));
        assert_eq!(payload["content"][0]["type"], json!("tool_use"));
        assert_eq!(payload["content"][0]["name"], json!("lookup_order"));
        assert_eq!(
            payload["content"][0]["input"]["order_id"],
            json!("order_123")
        );
    }

    #[test]
    fn anthropic_response_encodes_callback_task_id_into_tool_use_ids() {
        let callback_task_id = Uuid::from_u128(0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee);
        let run = NativeRunResult {
            id: Uuid::nil(),
            application_id: Uuid::nil(),
            api_key_id: Uuid::nil(),
            publication_version_id: Uuid::nil(),
            status: NativeRunStatus::Waiting,
            node_input_payload: json!({}),
            metadata: json!({}),
            answer: None,
            required_action: Some(NativeRequiredAction {
                action_type: "submit_tool_outputs".to_string(),
                payload: json!({ "callback_task_id": callback_task_id, "callback_kind": "llm_tool_calls" }),
            }),
            tool_calls: Some(json!([
                {
                    "id": "toolu_123",
                    "name": "lookup_order",
                    "arguments": {"order_id": "order_123"}
                }
            ])),
            usage: None,
            error: None,
            created_at: OffsetDateTime::UNIX_EPOCH,
        };

        let payload = serde_json::to_value(to_anthropic_response(run, "provider/model".into()))
            .expect("anthropic response serializes");

        let tool_use_id = payload["content"][0]["id"]
            .as_str()
            .expect("tool_use id should be encoded");
        assert_eq!(
            decode_anthropic_callback_tool_use_id(tool_use_id),
            Some((callback_task_id, "toolu_123".to_string()))
        );
    }

    #[test]
    fn anthropic_tool_resume_request_decodes_tool_result_blocks() {
        let callback_task_id = Uuid::from_u128(0xffffffffffffffffffffffffffffffff);
        let tool_use_id = encode_anthropic_callback_tool_use_id(callback_task_id, "toolu_123");

        let resume = anthropic_tool_resume_request(&json!({
            "model": "1flowbase",
            "messages": [
                {
                    "role": "user",
                    "content": [
                        {
                            "type": "tool_result",
                            "tool_use_id": tool_use_id,
                            "content": [{"type": "text", "text": "{\"order\":\"ready\"}"}]
                        }
                    ]
                }
            ]
        }))
        .expect("tool_result should parse")
        .expect("encoded tool_result should resume callback");

        assert_eq!(resume.callback_task_id, callback_task_id);
        assert_eq!(resume.tool_results[0]["tool_call_id"], json!("toolu_123"));
        assert_eq!(
            resume.tool_results[0]["content"],
            json!("{\"order\":\"ready\"}")
        );
    }

    #[test]
    fn anthropic_tool_resume_request_uses_latest_trailing_tool_result_message() {
        let previous_callback_task_id = Uuid::from_u128(0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa);
        let current_callback_task_id = Uuid::from_u128(0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb);
        let previous_tool_use_id =
            encode_anthropic_callback_tool_use_id(previous_callback_task_id, "toolu_previous");
        let current_tool_use_id =
            encode_anthropic_callback_tool_use_id(current_callback_task_id, "toolu_current");

        let resume = anthropic_tool_resume_request(&json!({
            "model": "1flowbase",
            "messages": [
                {"role": "user", "content": "first"},
                {
                    "role": "assistant",
                    "content": [{
                        "type": "tool_use",
                        "id": previous_tool_use_id,
                        "name": "lookup_previous",
                        "input": {}
                    }]
                },
                {
                    "role": "user",
                    "content": [{
                        "type": "tool_result",
                        "tool_use_id": previous_tool_use_id,
                        "content": "old result"
                    }]
                },
                {"role": "assistant", "content": "old answer"},
                {"role": "user", "content": "next"},
                {
                    "role": "assistant",
                    "content": [{
                        "type": "tool_use",
                        "id": current_tool_use_id,
                        "name": "lookup_current",
                        "input": {}
                    }]
                },
                {
                    "role": "user",
                    "content": [{
                        "type": "tool_result",
                        "tool_use_id": current_tool_use_id,
                        "content": "new result"
                    }]
                }
            ]
        }))
        .expect("resume request should parse")
        .expect("trailing tool_result should resume callback");

        assert_eq!(resume.callback_task_id, current_callback_task_id);
        assert_eq!(resume.tool_results.as_array().unwrap().len(), 1);
        assert_eq!(
            resume.tool_results[0]["tool_call_id"],
            json!("toolu_current")
        );
        assert_eq!(resume.tool_results[0]["content"], json!("new result"));
    }

    #[test]
    fn anthropic_tool_resume_request_ignores_historical_tool_results_before_latest_user_text() {
        let callback_task_id = Uuid::from_u128(0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa);
        let tool_use_id = encode_anthropic_callback_tool_use_id(callback_task_id, "toolu_previous");

        let resume = anthropic_tool_resume_request(&json!({
            "model": "1flowbase",
            "messages": [
                {"role": "user", "content": "first"},
                {
                    "role": "assistant",
                    "content": [{
                        "type": "tool_use",
                        "id": tool_use_id,
                        "name": "lookup_previous",
                        "input": {}
                    }]
                },
                {
                    "role": "user",
                    "content": [{
                        "type": "tool_result",
                        "tool_use_id": tool_use_id,
                        "content": "old result"
                    }]
                },
                {"role": "assistant", "content": "old answer"},
                {"role": "user", "content": "next question"}
            ]
        }))
        .expect("historical tool_result should parse");

        assert!(resume.is_none());
    }
}
