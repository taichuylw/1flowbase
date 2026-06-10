use std::{collections::HashSet, sync::Arc};

use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use control_plane::application_public_api::{
    api_keys::ApplicationApiKeyService,
    callback_resume::{
        ApplicationPublishedCallbackResumeService, PublishedCallbackResumeSource,
        PublishedCallbackResumeTarget, ResumePublishedCallbackCommand,
    },
    compat::anthropic::{
        map_messages_request, sanitize_anthropic_compat_assistant_text, AnthropicCompatError,
    },
    native::{
        ApplicationNativeRunService, CreateNativeRunCommand, GetNativeRunCommand, NativeRunRequest,
        NativeRunResult, NativeRunStatus, NativeRunValidationError,
    },
    run_service::ApplicationPublishedRunControlRepository,
};
use control_plane::orchestration_runtime::OrchestrationRuntimeService;
use serde::Serialize;
use serde_json::{json, Value};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    app_state::ApiState,
    provider_runtime::ApiProviderRuntime,
    routes::application_public_api::{
        compat_sse,
        llm_tool_visibility::external_llm_tool_calls,
        native,
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
    pub cache_creation_input_tokens: u64,
    pub cache_read_input_tokens: u64,
    pub output_tokens: u64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AnthropicCountTokensResponse {
    pub input_tokens: u64,
}

struct AnthropicToolResumeRequest {
    callback_task_id: Uuid,
    tool_results: Value,
}

struct AnthropicToolResumePlan {
    initial_run: NativeRunResult,
    command: ResumePublishedCallbackCommand,
}

struct AnthropicDecodedToolResult {
    callback_task_id: Uuid,
    original_tool_use_id: String,
    content: Value,
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
    let mut value = parse_anthropic_json_body(body)?;
    merge_claude_code_session_header(&mut value, &headers);
    let model = value
        .get("model")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    if let Some(response) = anthropic_probe_response(&value, &model) {
        authenticate_anthropic_token(state.as_ref(), &bearer_token).await?;
        return Ok(Json(response).into_response());
    }
    let response_mode = value
        .get("stream")
        .and_then(Value::as_bool)
        .filter(|stream| *stream)
        .map(|_| "streaming".to_string());
    if let Some(run) = anthropic_structured_output_run(&value)? {
        authenticate_anthropic_token(state.as_ref(), &bearer_token).await?;
        if response_mode.as_deref() == Some("streaming") {
            return Ok(compat_sse::completed_anthropic_stream(run, model));
        }
        return Ok(Json(to_anthropic_response(run, model)).into_response());
    }
    if let Some(resume) = anthropic_tool_resume_request(&value)? {
        if response_mode.as_deref() == Some("streaming") {
            let resume_plan = prepare_anthropic_tool_resume(
                state.clone(),
                &bearer_token,
                resume.callback_task_id,
                resume.tool_results,
            )
            .await?;
            return compat_sse::start_anthropic_resume_stream(
                state,
                resume_plan.initial_run,
                model,
                resume_plan.command,
            )
            .await
            .map_err(Into::into);
        }
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

#[utoipa::path(
    post,
    path = "/v1/messages/count_tokens",
    request_body = Value,
    responses(
        (status = 200, body = AnthropicCountTokensResponse),
        (status = 400, body = AnthropicErrorBody),
        (status = 401, body = AnthropicErrorBody)
    )
)]
pub async fn count_message_tokens(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<AnthropicCountTokensResponse>, AnthropicRouteError> {
    let bearer_token = anthropic_token(&headers)?;
    let mut value = parse_anthropic_json_body(body)?;
    merge_claude_code_session_header(&mut value, &headers);
    authenticate_anthropic_token(state.as_ref(), &bearer_token).await?;
    Ok(Json(to_anthropic_count_tokens_response(&value)))
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

fn merge_claude_code_session_header(value: &mut Value, headers: &HeaderMap) {
    let Some(session_id) = headers
        .get("x-claude-code-session-id")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return;
    };
    let Some(object) = value.as_object_mut() else {
        return;
    };
    let metadata = object.entry("metadata").or_insert_with(|| json!({}));
    let Some(metadata) = metadata.as_object_mut() else {
        return;
    };
    metadata
        .entry("session_id".to_string())
        .or_insert_with(|| Value::String(session_id.to_string()));
}

async fn authenticate_anthropic_token(
    state: &ApiState,
    bearer_token: &str,
) -> Result<(), native::NativeApiError> {
    ApplicationApiKeyService::new(state.store.clone())
        .with_last_used_cache(state.infrastructure.cache_store())
        .authenticate_bearer_token(bearer_token)
        .await
        .map(|_| ())
        .map_err(|_| native::native_error(NativeRunValidationError::NotAuthenticated))
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
    let response_payload = anthropic_tool_resume_response_payload(tool_results);
    let runtime_service = OrchestrationRuntimeService::new(
        state.store.clone(),
        ApiProviderRuntime::new(state.provider_runtime.clone()),
        state.runtime_engine.clone(),
        state.provider_secret_master_key.clone(),
    )
    .with_file_storage_registry(state.file_storage_registry.clone())
    .with_runtime_event_stream(state.runtime_event_stream.clone());
    let result =
        ApplicationPublishedCallbackResumeService::new(state.store.clone(), runtime_service)
            .with_last_used_cache(state.infrastructure.cache_store())
            .resume_callback(ResumePublishedCallbackCommand {
                bearer_token: bearer_token.to_string(),
                target: PublishedCallbackResumeTarget::CallbackTask { callback_task_id },
                source: PublishedCallbackResumeSource::AnthropicMessages,
                response_payload,
                response_mode: None,
            })
            .await
            .map_err(native::service_error)?;

    Ok(
        control_plane::application_public_api::run_service::native_result_from_run_detail(
            &result.detail,
            native::published_run_metadata(&result.detail.flow_run),
        ),
    )
}

fn anthropic_tool_resume_response_payload(tool_results: Value) -> Value {
    json!({ "tool_results": tool_results })
}

async fn prepare_anthropic_tool_resume(
    state: Arc<ApiState>,
    bearer_token: &str,
    callback_task_id: Uuid,
    tool_results: Value,
) -> Result<AnthropicToolResumePlan, AnthropicRouteError> {
    let callback_task = state
        .store
        .get_published_callback_task(callback_task_id)
        .await
        .map_err(native::service_error)?
        .ok_or_else(|| {
            native::NativeApiError::new(
                StatusCode::NOT_FOUND,
                "callback_task",
                "callback task was not found",
            )
        })?;
    let initial_run = ApplicationNativeRunService::new(state.store.clone())
        .with_last_used_cache(state.infrastructure.cache_store())
        .get_native_run(GetNativeRunCommand {
            bearer_token: bearer_token.to_string(),
            run_id: callback_task.flow_run_id,
        })
        .await
        .map_err(native::native_error)?;

    Ok(AnthropicToolResumePlan {
        initial_run,
        command: ResumePublishedCallbackCommand {
            bearer_token: bearer_token.to_string(),
            target: PublishedCallbackResumeTarget::CallbackTask { callback_task_id },
            source: PublishedCallbackResumeSource::AnthropicMessages,
            response_payload: anthropic_tool_resume_response_payload(tool_results),
            response_mode: Some("streaming".into()),
        },
    })
}

fn to_anthropic_response(run: NativeRunResult, model: String) -> AnthropicMessageResponse {
    let callback_task_id = callback_task_id_from_required_action(&run);
    let tool_blocks = anthropic_tool_use_blocks(run.tool_calls.as_ref(), callback_task_id);
    let has_tool_blocks = tool_blocks
        .as_ref()
        .is_some_and(|blocks| !blocks.is_empty());
    let mut content = Vec::new();
    if let Some(answer) = run.answer {
        if let Some(answer) = sanitize_anthropic_compat_assistant_text(&answer) {
            if !answer.is_empty() {
                content.push(json!({"type": "text", "text": answer}));
            }
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
    let calls = external_llm_tool_calls(tool_calls)?;
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
    let mut trailing_tool_result_messages = messages
        .iter()
        .rev()
        .take_while(|message| anthropic_message_has_only_tool_results(message))
        .collect::<Vec<_>>();
    if trailing_tool_result_messages.is_empty() {
        return Ok(None);
    }
    trailing_tool_result_messages.reverse();
    let trailing_start = messages.len() - trailing_tool_result_messages.len();
    let matching_tool_use_ids = anthropic_trailing_assistant_tool_use_ids(messages, trailing_start);
    if matching_tool_use_ids.is_empty() {
        return Ok(None);
    }

    let mut decoded_results = Vec::new();

    for message in trailing_tool_result_messages {
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
            if !matching_tool_use_ids.contains(tool_use_id) {
                continue;
            }
            let Some((decoded_callback_task_id, original_tool_use_id)) =
                decode_anthropic_callback_tool_use_id(tool_use_id)
            else {
                continue;
            };
            decoded_results.push(AnthropicDecodedToolResult {
                callback_task_id: decoded_callback_task_id,
                original_tool_use_id,
                content: anthropic_tool_result_content(block),
            });
        }
    }

    let Some(callback_task_id) = decoded_results.last().map(|result| result.callback_task_id)
    else {
        return Ok(None);
    };
    let mut tool_results = decoded_results
        .into_iter()
        .rev()
        .take_while(|result| result.callback_task_id == callback_task_id)
        .map(|result| {
            json!({
                "tool_call_id": result.original_tool_use_id,
                "content": result.content,
            })
        })
        .collect::<Vec<_>>();
    tool_results.reverse();

    Ok(Some(AnthropicToolResumeRequest {
        callback_task_id,
        tool_results: Value::Array(tool_results),
    }))
}

fn anthropic_trailing_assistant_tool_use_ids(
    messages: &[Value],
    trailing_start: usize,
) -> HashSet<String> {
    let mut tool_use_ids = HashSet::new();
    for message in messages[..trailing_start].iter().rev() {
        if message.get("role").and_then(Value::as_str) != Some("assistant") {
            break;
        }
        let Some(blocks) = message.get("content").and_then(Value::as_array) else {
            continue;
        };
        for block in blocks {
            if block.get("type").and_then(Value::as_str) != Some("tool_use") {
                continue;
            }
            if let Some(id) = block.get("id").and_then(Value::as_str) {
                tool_use_ids.insert(id.to_string());
            }
        }
    }
    tool_use_ids
}

fn anthropic_message_has_only_tool_results(message: &Value) -> bool {
    message.get("role").and_then(Value::as_str) == Some("user")
        && message
            .get("content")
            .and_then(Value::as_array)
            .is_some_and(|blocks| {
                !blocks.is_empty()
                    && blocks.iter().all(|block| {
                        block.get("type").and_then(Value::as_str) == Some("tool_result")
                    })
            })
}

fn anthropic_tool_result_content(block: &Value) -> Value {
    let Some(content) = block.get("content") else {
        return Value::String(String::new());
    };
    if let Some(text) = content.as_str() {
        return Value::String(text.to_string());
    }
    if let Some(blocks) = content.as_array() {
        let text = blocks
            .iter()
            .filter_map(|entry| entry.get("text").and_then(Value::as_str))
            .collect::<Vec<_>>()
            .join("\n");
        if blocks
            .iter()
            .all(|entry| entry.get("type").and_then(Value::as_str) == Some("text"))
        {
            return Value::String(text);
        }
        return Value::Array(blocks.clone());
    }
    Value::String(content.to_string())
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
        cache_creation_input_tokens: usage.cache_write_tokens.unwrap_or_default(),
        cache_read_input_tokens: usage
            .cache_read_tokens
            .or(usage.input_cache_hit_tokens)
            .unwrap_or_default(),
        output_tokens: usage.completion_tokens.unwrap_or_default(),
    }
}

fn to_anthropic_count_tokens_response(request: &Value) -> AnthropicCountTokensResponse {
    AnthropicCountTokensResponse {
        input_tokens: anthropic_count_input_tokens(request),
    }
}

fn anthropic_probe_response(request: &Value, model: &str) -> Option<AnthropicMessageResponse> {
    if request.get("stream").and_then(Value::as_bool) == Some(true)
        || request
            .get("max_tokens")
            .and_then(Value::as_u64)
            .is_none_or(|max_tokens| max_tokens > 1)
    {
        return None;
    }
    let probe_text = anthropic_single_user_text(request)?;
    if !matches!(probe_text.trim(), "test" | "foo" | "count") {
        return None;
    }

    Some(AnthropicMessageResponse {
        id: format!("msg_{}", Uuid::now_v7()),
        response_type: "message",
        role: "assistant",
        model: model.to_string(),
        content: vec![json!({"type": "text", "text": ""})],
        stop_reason: "end_turn",
        usage: AnthropicUsage {
            input_tokens: anthropic_count_input_tokens(request),
            cache_creation_input_tokens: 0,
            cache_read_input_tokens: 0,
            output_tokens: 0,
        },
    })
}

fn anthropic_structured_output_run(
    request: &Value,
) -> Result<Option<NativeRunResult>, AnthropicCompatError> {
    if !anthropic_title_output_requested(request) {
        return Ok(None);
    }
    let native = map_messages_request(request.clone())?;
    let content_text = json!({ "title": anthropic_title_from_query(&native.query) }).to_string();

    Ok(Some(NativeRunResult {
        id: Uuid::now_v7(),
        application_id: Uuid::nil(),
        api_key_id: Uuid::nil(),
        publication_version_id: Uuid::nil(),
        status: NativeRunStatus::Succeeded,
        node_input_payload: json!({}),
        metadata: json!({}),
        answer: Some(content_text),
        required_action: None,
        tool_calls: None,
        usage: None,
        error: None,
        created_at: time::OffsetDateTime::now_utc(),
    }))
}

fn anthropic_title_output_requested(request: &Value) -> bool {
    anthropic_title_json_schema_requested(request)
        || anthropic_title_system_prompt_requested(request)
}

fn anthropic_title_json_schema_requested(request: &Value) -> bool {
    let Some(format) = request
        .get("output_config")
        .and_then(|output_config| output_config.get("format"))
        .and_then(Value::as_object)
    else {
        return false;
    };
    format.get("type").and_then(Value::as_str) == Some("json_schema")
        && format
            .get("schema")
            .and_then(|schema| schema.get("properties"))
            .and_then(|properties| properties.get("title"))
            .and_then(|title| title.get("type"))
            .and_then(Value::as_str)
            == Some("string")
}

fn anthropic_title_system_prompt_requested(request: &Value) -> bool {
    let system_text = anthropic_system_text(request);
    system_text.contains("Generate a concise, sentence-case title")
        && system_text.contains("Return JSON with a single \"title\" field")
}

fn anthropic_system_text(request: &Value) -> String {
    match request.get("system") {
        Some(Value::String(text)) => text.to_string(),
        Some(Value::Array(blocks)) => blocks
            .iter()
            .filter_map(|block| match block {
                Value::String(text) => Some(text.as_str()),
                Value::Object(object) => object.get("text").and_then(Value::as_str),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n"),
        _ => String::new(),
    }
}

fn anthropic_title_from_query(query: &str) -> String {
    let collapsed = query
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("新会话")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let title = collapsed
        .trim_matches(|value: char| matches!(value, '"' | '\'' | '`' | ':' | '-' | '#'))
        .trim();
    let title = if title.is_empty() { "新会话" } else { title };
    let max_chars = 48;
    let mut shortened = title.chars().take(max_chars).collect::<String>();
    if title.chars().count() > max_chars {
        shortened.push_str("...");
    }
    shortened
}

fn anthropic_single_user_text(request: &Value) -> Option<&str> {
    let messages = request.get("messages").and_then(Value::as_array)?;
    if messages.len() != 1 {
        return None;
    }
    let message = messages.first()?;
    if message.get("role").and_then(Value::as_str) != Some("user") {
        return None;
    }
    match message.get("content")? {
        Value::String(text) => Some(text.as_str()),
        Value::Array(blocks) if blocks.len() == 1 => blocks
            .first()
            .filter(|block| block.get("type").and_then(Value::as_str) == Some("text"))
            .and_then(|block| block.get("text"))
            .and_then(Value::as_str),
        _ => None,
    }
}

fn anthropic_count_input_tokens(request: &Value) -> u64 {
    let mut tokens = 0_u64;
    for key in [
        "system",
        "messages",
        "tools",
        "tool_choice",
        "thinking",
        "container",
        "context_management",
    ] {
        tokens = tokens.saturating_add(anthropic_value_token_estimate(request.get(key)));
    }
    if request
        .get("tools")
        .and_then(Value::as_array)
        .is_some_and(|tools| !tools.is_empty())
    {
        tokens = tokens.saturating_add(500);
    }
    tokens.max(1)
}

fn anthropic_value_token_estimate(value: Option<&Value>) -> u64 {
    let Some(value) = value else {
        return 0;
    };
    let chars = anthropic_value_char_count(value) as u64;
    ((chars.saturating_add(3)) / 4).max(1)
}

fn anthropic_value_char_count(value: &Value) -> usize {
    match value {
        Value::Null => 0,
        Value::Bool(value) => value.to_string().chars().count(),
        Value::Number(value) => value.to_string().chars().count(),
        Value::String(value) => value.chars().count(),
        Value::Array(values) => values.iter().map(anthropic_value_char_count).sum(),
        Value::Object(map) => map
            .iter()
            .map(|(key, value)| key.chars().count() + anthropic_value_char_count(value))
            .sum(),
    }
}

#[cfg(test)]
#[cfg(test)]
mod tests;
