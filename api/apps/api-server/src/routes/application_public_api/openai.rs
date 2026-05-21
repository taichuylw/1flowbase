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
    compat::openai::{
        extract_model_list_from_start_node, map_chat_completion_request, map_response_request,
        response_id_from_run_id, run_id_from_response_id, OpenAiCompatError, OpenAiCompatibleModel,
        OpenAiPreviousResponseContext,
    },
    native::{
        ApplicationNativeRunService, CreateNativeRunCommand, GetNativeRunCommand, NativeRunRequest,
        NativeRunResult, NativeRunValidationError,
    },
    publications::{ApplicationPublicationService, LoadActiveApplicationPublicationCommand},
    run_service::{native_result_from_run_detail, ApplicationPublishedRunControlRepository},
};
use control_plane::orchestration_runtime::{
    CompleteCallbackTaskCommand, OrchestrationRuntimeService,
};
use serde::Serialize;
use serde_json::{json, Value};
use tracing::{info, warn};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    app_state::ApiState,
    provider_runtime::ApiProviderRuntime,
    routes::application_public_api::{
        compat_sse, native,
        tool_callback_ids::{
            decode_openai_callback_tool_call_id, encode_openai_callback_tool_call_id,
        },
    },
};

#[derive(Debug, Serialize, ToSchema)]
pub struct OpenAiErrorBody {
    pub error: OpenAiErrorObject,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct OpenAiErrorObject {
    pub message: String,
    #[serde(rename = "type")]
    pub error_type: String,
    pub param: Option<String>,
    pub code: String,
}

#[derive(Debug)]
pub enum OpenAiRouteError {
    Compat(OpenAiCompatError),
    Native(native::NativeApiError),
    RequiredAction,
}

impl From<OpenAiCompatError> for OpenAiRouteError {
    fn from(error: OpenAiCompatError) -> Self {
        Self::Compat(error)
    }
}

impl From<native::NativeApiError> for OpenAiRouteError {
    fn from(error: native::NativeApiError) -> Self {
        Self::Native(error)
    }
}

impl IntoResponse for OpenAiRouteError {
    fn into_response(self) -> Response {
        let (status, error) = match self {
            OpenAiRouteError::Compat(error) => (
                StatusCode::BAD_REQUEST,
                OpenAiErrorObject {
                    message: error.message,
                    error_type: error.error_type,
                    param: error.param,
                    code: error.code,
                },
            ),
            OpenAiRouteError::Native(error) => (
                error.status,
                OpenAiErrorObject {
                    message: error.message,
                    error_type: "invalid_request_error".to_string(),
                    param: None,
                    code: error.code.to_string(),
                },
            ),
            OpenAiRouteError::RequiredAction => (
                StatusCode::CONFLICT,
                OpenAiErrorObject {
                    message: "waiting states are not supported by compatible endpoints; use the Native API to inspect and resume required_action runs".to_string(),
                    error_type: "invalid_request_error".to_string(),
                    param: None,
                    code: "required_action_not_supported".to_string(),
                },
            ),
        };
        (status, Json(OpenAiErrorBody { error })).into_response()
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct OpenAiChatCompletionResponse {
    pub id: String,
    pub object: &'static str,
    pub created: i64,
    pub model: String,
    pub choices: Vec<OpenAiChatCompletionChoice>,
    pub usage: OpenAiUsage,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct OpenAiChatCompletionChoice {
    pub index: u32,
    pub message: OpenAiChatMessage,
    pub finish_reason: &'static str,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct OpenAiChatMessage {
    pub role: &'static str,
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<OpenAiToolCall>>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct OpenAiToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub call_type: &'static str,
    pub function: OpenAiToolCallFunction,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct OpenAiToolCallFunction {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Default, Serialize, ToSchema)]
pub struct OpenAiUsage {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_tokens: u64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct OpenAiModelListResponse {
    pub object: &'static str,
    pub data: Vec<OpenAiModelObject>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct OpenAiModelObject {
    pub id: String,
    pub object: &'static str,
    pub created: i64,
    pub owned_by: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct OpenAiResponsesObject {
    pub id: String,
    pub object: &'static str,
    pub created_at: i64,
    pub status: &'static str,
    pub model: String,
    pub output: Vec<Value>,
    pub output_text: String,
    pub usage: OpenAiResponsesUsage,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_response_id: Option<String>,
}

#[derive(Debug, Default, Serialize, ToSchema)]
pub struct OpenAiResponsesUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub total_tokens: u64,
}

struct OpenAiCredential {
    token: String,
    source: &'static str,
}

struct OpenAiToolResumeRequest {
    callback_task_id: Uuid,
    tool_results: Value,
}

#[utoipa::path(
    post,
    path = "/v1/chat/completions",
    request_body = Value,
    responses(
        (status = 200, body = OpenAiChatCompletionResponse),
        (status = 400, body = OpenAiErrorBody),
        (status = 401, body = OpenAiErrorBody),
        (status = 403, body = OpenAiErrorBody),
        (status = 409, body = OpenAiErrorBody)
    )
)]
pub async fn create_chat_completion(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response, OpenAiRouteError> {
    let credential = match openai_credential(&headers) {
        Ok(credential) => credential,
        Err(error) => {
            warn!(
                route = "chat_completions",
                status = error.status.as_u16(),
                code = error.code,
                "openai compatible authentication failed"
            );
            return Err(error.into());
        }
    };
    let value = match parse_openai_json_body(body) {
        Ok(value) => value,
        Err(error) => {
            warn_openai_route_error(
                "chat_completions",
                &error,
                "openai compatible JSON validation failed",
            );
            return Err(error);
        }
    };
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
    if let Some(resume) = openai_chat_tool_resume_request(&value)? {
        let run = resume_openai_tool_call(
            state,
            &credential.token,
            resume.callback_task_id,
            resume.tool_results,
        )
        .await?;
        if !openai_required_action_is_supported(&run) {
            return Err(OpenAiRouteError::RequiredAction);
        }
        if response_mode.as_deref() == Some("streaming") {
            return Ok(compat_sse::completed_openai_chat_stream(run, model));
        }
        return Ok(Json(to_openai_response(run, model)).into_response());
    }

    let request = match map_chat_completion_request(value) {
        Ok(request) => request,
        Err(error) => {
            let route_error = OpenAiRouteError::from(error);
            warn_openai_route_error(
                "chat_completions",
                &route_error,
                "openai compatible request validation failed",
            );
            return Err(route_error);
        }
    };
    let run = match create_native_run(state.clone(), credential.token.clone(), request).await {
        Ok(run) => run,
        Err(error) => {
            warn!(
                route = "chat_completions",
                auth_source = credential.source,
                status = error.status.as_u16(),
                code = error.code,
                "openai compatible native run validation failed"
            );
            return Err(error.into());
        }
    };

    info!(
        route = "chat_completions",
        auth_source = credential.source,
        application_id = %run.application_id,
        flow_run_id = %run.id,
        response_mode = response_mode.as_deref().unwrap_or("blocking"),
        model = %model,
        "openai compatible chat completion accepted"
    );

    if response_mode.as_deref() == Some("streaming") {
        return compat_sse::start_openai_run_stream(state, run, model)
            .await
            .map_err(Into::into);
    }

    let run = native::execute_blocking_native_run(state, credential.token, run).await?;
    if !openai_required_action_is_supported(&run) {
        return Err(OpenAiRouteError::RequiredAction);
    }
    Ok(Json(to_openai_response(run, model)).into_response())
}

#[utoipa::path(
    post,
    path = "/v1/responses",
    request_body = Value,
    responses(
        (status = 200, body = OpenAiResponsesObject),
        (status = 400, body = OpenAiErrorBody),
        (status = 401, body = OpenAiErrorBody),
        (status = 403, body = OpenAiErrorBody),
        (status = 409, body = OpenAiErrorBody)
    )
)]
pub async fn create_response(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response, OpenAiRouteError> {
    let credential = match openai_credential(&headers) {
        Ok(credential) => credential,
        Err(error) => {
            warn!(
                route = "responses",
                status = error.status.as_u16(),
                code = error.code,
                "openai responses compatible authentication failed"
            );
            return Err(error.into());
        }
    };
    let value = match parse_openai_json_body(body) {
        Ok(value) => value,
        Err(error) => {
            warn_openai_route_error(
                "responses",
                &error,
                "openai responses compatible JSON validation failed",
            );
            return Err(error);
        }
    };
    let previous_response_id = match optional_string_field(&value, "previous_response_id") {
        Ok(previous_response_id) => previous_response_id,
        Err(error) => {
            warn_openai_route_error(
                "responses",
                &error,
                "openai responses previous_response_id validation failed",
            );
            return Err(error);
        }
    };
    let previous_response = match load_previous_response_context(
        state.clone(),
        &credential.token,
        previous_response_id.as_deref(),
    )
    .await
    {
        Ok(previous_response) => previous_response,
        Err(error) => {
            warn_openai_route_error(
                "responses",
                &error,
                "openai responses previous_response_id lookup failed",
            );
            return Err(error);
        }
    };
    let response_mode = value
        .get("stream")
        .and_then(Value::as_bool)
        .filter(|stream| *stream)
        .map(|_| "streaming".to_string());
    let model = value
        .get("model")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    if let Some(resume) = openai_responses_tool_resume_request(&value)? {
        ensure_openai_responses_resume_matches_previous_response(
            state.as_ref(),
            previous_response_id.as_deref(),
            resume.callback_task_id,
        )
        .await?;
        let run = resume_openai_tool_call(
            state,
            &credential.token,
            resume.callback_task_id,
            resume.tool_results,
        )
        .await?;
        if !openai_required_action_is_supported(&run) {
            return Err(OpenAiRouteError::RequiredAction);
        }
        if response_mode.as_deref() == Some("streaming") {
            return Ok(compat_sse::completed_openai_response_stream(
                run,
                model,
                previous_response_id,
            ));
        }
        return Ok(Json(to_openai_responses_response(
            run,
            model,
            previous_response_id,
        ))
        .into_response());
    }
    let request = match map_response_request(value, previous_response) {
        Ok(request) => request,
        Err(error) => {
            let route_error = OpenAiRouteError::from(error);
            warn_openai_route_error(
                "responses",
                &route_error,
                "openai responses compatible request validation failed",
            );
            return Err(route_error);
        }
    };
    let model = request.model.clone().unwrap_or_default();
    let response_mode = request.response_mode.clone();
    let run = match create_native_run(state.clone(), credential.token.clone(), request).await {
        Ok(run) => run,
        Err(error) => {
            warn!(
                route = "responses",
                auth_source = credential.source,
                status = error.status.as_u16(),
                code = error.code,
                "openai responses compatible native run validation failed"
            );
            return Err(error.into());
        }
    };

    info!(
        route = "responses",
        auth_source = credential.source,
        application_id = %run.application_id,
        flow_run_id = %run.id,
        response_mode = response_mode.as_deref().unwrap_or("blocking"),
        model = %model,
        "openai responses compatible request accepted"
    );

    if response_mode.as_deref() == Some("streaming") {
        return compat_sse::start_openai_response_stream(state, run, model, previous_response_id)
            .await
            .map_err(Into::into);
    }

    let run = native::execute_blocking_native_run(state, credential.token, run).await?;
    if !openai_required_action_is_supported(&run) {
        warn!(
            route = "responses",
            application_id = %run.application_id,
            flow_run_id = %run.id,
            "openai responses compatible blocking run reached unsupported required_action state"
        );
        return Err(OpenAiRouteError::RequiredAction);
    }
    Ok(Json(to_openai_responses_response(
        run,
        model,
        previous_response_id,
    ))
    .into_response())
}

#[utoipa::path(
    get,
    path = "/v1/models",
    operation_id = "list_openai_compatible_models",
    responses(
        (status = 200, body = OpenAiModelListResponse),
        (status = 401, body = OpenAiErrorBody),
        (status = 403, body = OpenAiErrorBody),
        (status = 409, body = OpenAiErrorBody)
    )
)]
pub async fn list_models(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
) -> Result<Json<OpenAiModelListResponse>, OpenAiRouteError> {
    let credential = match openai_credential(&headers) {
        Ok(credential) => credential,
        Err(error) => {
            warn!(
                route = "models",
                status = error.status.as_u16(),
                code = error.code,
                "openai compatible model list authentication failed"
            );
            return Err(error.into());
        }
    };
    let actor = match ApplicationApiKeyService::new(state.store.clone())
        .authenticate_bearer_token(&credential.token)
        .await
    {
        Ok(actor) => actor,
        Err(_) => {
            warn!(
                route = "models",
                auth_source = credential.source,
                code = "not_authenticated",
                "openai compatible model list rejected invalid application API key"
            );
            return Err(native::native_error(NativeRunValidationError::NotAuthenticated).into());
        }
    };
    let publication = match ApplicationPublicationService::new(state.store.clone())
        .load_active_publication(LoadActiveApplicationPublicationCommand {
            application_id: actor.application_id,
        })
        .await
    {
        Ok(publication) => publication,
        Err(_) => {
            warn!(
                route = "models",
                auth_source = credential.source,
                application_id = %actor.application_id,
                api_key_id = %actor.api_key_id,
                code = "application_not_published",
                "openai compatible model list has no active publication"
            );
            return Err(
                native::native_error(NativeRunValidationError::ApplicationNotPublished).into(),
            );
        }
    };
    let models = extract_model_list_from_start_node(&publication.document_snapshot);
    let model_count = models.len();

    info!(
        route = "models",
        auth_source = credential.source,
        application_id = %actor.application_id,
        api_key_id = %actor.api_key_id,
        model_count,
        "openai compatible model list returned"
    );

    Ok(Json(to_openai_model_list_response(
        models,
        publication.created_at.unix_timestamp(),
    )))
}

fn openai_credential(headers: &HeaderMap) -> Result<OpenAiCredential, native::NativeApiError> {
    if let Ok(token) = native::bearer_token(headers) {
        return Ok(OpenAiCredential {
            token,
            source: "authorization_bearer",
        });
    }
    headers
        .get("x-api-key")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .map(|token| OpenAiCredential {
            token: token.to_owned(),
            source: "x_api_key",
        })
        .ok_or_else(|| {
            native::NativeApiError::new(
                StatusCode::UNAUTHORIZED,
                "not_authenticated",
                "missing Authorization bearer token or x-api-key",
            )
        })
}

fn parse_openai_json_body(body: Bytes) -> Result<Value, OpenAiRouteError> {
    serde_json::from_slice::<Value>(&body).map_err(|_| {
        OpenAiCompatError {
            message: "invalid JSON body".to_string(),
            error_type: "invalid_request_error".to_string(),
            param: Some("body".to_string()),
            code: "invalid_request".to_string(),
        }
        .into()
    })
}

fn optional_string_field(
    value: &Value,
    field: &'static str,
) -> Result<Option<String>, OpenAiRouteError> {
    match value.get(field) {
        Some(Value::String(value)) => Ok(Some(value.clone())),
        Some(_) => Err(OpenAiCompatError {
            message: format!("{field} must be a string"),
            error_type: "invalid_request_error".to_string(),
            param: Some(field.to_string()),
            code: "invalid_request".to_string(),
        }
        .into()),
        None => Ok(None),
    }
}

fn warn_openai_route_error(route: &'static str, error: &OpenAiRouteError, message: &'static str) {
    match error {
        OpenAiRouteError::Compat(error) => warn!(
            route,
            code = %error.code,
            param = error.param.as_deref().unwrap_or(""),
            error_type = %error.error_type,
            "{message}"
        ),
        OpenAiRouteError::Native(error) => warn!(
            route,
            status = error.status.as_u16(),
            code = error.code,
            "{message}"
        ),
        OpenAiRouteError::RequiredAction => {
            warn!(route, code = "required_action_not_supported", "{message}")
        }
    }
}

fn to_openai_model_list_response(
    models: Vec<OpenAiCompatibleModel>,
    created: i64,
) -> OpenAiModelListResponse {
    OpenAiModelListResponse {
        object: "list",
        data: models
            .into_iter()
            .map(|model| OpenAiModelObject {
                id: model.id,
                object: "model",
                created,
                owned_by: "1flowbase",
                name: model.name,
            })
            .collect(),
    }
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

async fn resume_openai_tool_call(
    state: Arc<ApiState>,
    bearer_token: &str,
    callback_task_id: Uuid,
    tool_results: Value,
) -> Result<NativeRunResult, OpenAiRouteError> {
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

fn openai_chat_tool_resume_request(
    request: &Value,
) -> Result<Option<OpenAiToolResumeRequest>, OpenAiRouteError> {
    let Some(messages) = request.get("messages").and_then(Value::as_array) else {
        return Ok(None);
    };
    let mut trailing_tool_messages = messages
        .iter()
        .rev()
        .take_while(|message| message.get("role").and_then(Value::as_str) == Some("tool"))
        .collect::<Vec<_>>();
    if trailing_tool_messages.is_empty() {
        return Ok(None);
    }
    trailing_tool_messages.reverse();

    let mut callback_task_id = None;
    let mut tool_results = Vec::new();

    for message in trailing_tool_messages {
        let Some(external_tool_call_id) = message.get("tool_call_id").and_then(Value::as_str)
        else {
            continue;
        };
        let Some((decoded_callback_task_id, original_tool_call_id)) =
            decode_openai_callback_tool_call_id(external_tool_call_id)
        else {
            continue;
        };
        if let Some(existing_callback_task_id) = callback_task_id {
            if existing_callback_task_id != decoded_callback_task_id {
                return Err(openai_invalid_request(
                    "messages",
                    "tool results must belong to one callback task",
                ));
            }
        } else {
            callback_task_id = Some(decoded_callback_task_id);
        }
        tool_results.push(json!({
            "tool_call_id": original_tool_call_id,
            "content": openai_tool_message_content(message),
        }));
    }

    Ok(
        callback_task_id.map(|callback_task_id| OpenAiToolResumeRequest {
            callback_task_id,
            tool_results: Value::Array(tool_results),
        }),
    )
}

fn openai_invalid_request(param: &'static str, message: impl Into<String>) -> OpenAiRouteError {
    OpenAiCompatError {
        message: message.into(),
        error_type: "invalid_request_error".to_string(),
        param: Some(param.to_string()),
        code: "invalid_request".to_string(),
    }
    .into()
}

fn openai_tool_message_content(message: &Value) -> String {
    match message.get("content") {
        Some(Value::String(content)) => content.clone(),
        Some(Value::Null) | None => String::new(),
        Some(content) => content.to_string(),
    }
}

fn openai_responses_tool_resume_request(
    request: &Value,
) -> Result<Option<OpenAiToolResumeRequest>, OpenAiRouteError> {
    let Some(items) = request.get("input").and_then(Value::as_array) else {
        return Ok(None);
    };
    let mut callback_task_id = None;
    let mut tool_results = Vec::new();

    for item in items {
        if item.get("type").and_then(Value::as_str) != Some("function_call_output") {
            continue;
        }
        let Some(call_id) = item.get("call_id").and_then(Value::as_str) else {
            return Err(openai_invalid_request(
                "input",
                "function_call_output call_id is required",
            ));
        };
        let Some((decoded_callback_task_id, original_tool_call_id)) =
            decode_openai_callback_tool_call_id(call_id)
        else {
            continue;
        };
        if let Some(existing_callback_task_id) = callback_task_id {
            if existing_callback_task_id != decoded_callback_task_id {
                return Err(openai_invalid_request(
                    "input",
                    "function_call_output items must belong to one callback task",
                ));
            }
        } else {
            callback_task_id = Some(decoded_callback_task_id);
        }
        tool_results.push(json!({
            "tool_call_id": original_tool_call_id,
            "content": openai_response_function_call_output(item),
        }));
    }

    Ok(
        callback_task_id.map(|callback_task_id| OpenAiToolResumeRequest {
            callback_task_id,
            tool_results: Value::Array(tool_results),
        }),
    )
}

async fn ensure_openai_responses_resume_matches_previous_response(
    state: &ApiState,
    previous_response_id: Option<&str>,
    callback_task_id: Uuid,
) -> Result<(), OpenAiRouteError> {
    let Some(response_id) = previous_response_id else {
        return Err(openai_invalid_request(
            "previous_response_id",
            "previous_response_id is required when submitting function_call_output",
        ));
    };
    let previous_run_id = run_id_from_response_id(response_id)?;
    let callback_task = state
        .store
        .get_published_callback_task(callback_task_id)
        .await
        .map_err(native::service_error)?
        .ok_or_else(|| {
            openai_invalid_request("input", "function_call_output callback task was not found")
        })?;

    if callback_task.flow_run_id != previous_run_id {
        return Err(openai_invalid_request(
            "previous_response_id",
            "previous_response_id does not match function_call_output callback",
        ));
    }

    Ok(())
}

fn openai_response_function_call_output(item: &Value) -> String {
    match item.get("output") {
        Some(Value::String(output)) => output.clone(),
        Some(Value::Null) | None => String::new(),
        Some(output) => output.to_string(),
    }
}

async fn load_previous_response_context(
    state: Arc<ApiState>,
    bearer_token: &str,
    previous_response_id: Option<&str>,
) -> Result<Option<OpenAiPreviousResponseContext>, OpenAiRouteError> {
    let Some(response_id) = previous_response_id else {
        return Ok(None);
    };
    let run_id = run_id_from_response_id(response_id)?;
    let run = ApplicationNativeRunService::new(state.store.clone())
        .get_native_run(GetNativeRunCommand {
            bearer_token: bearer_token.to_string(),
            run_id,
        })
        .await
        .map_err(native::native_error)?;
    Ok(Some(OpenAiPreviousResponseContext {
        response_id: response_id.to_string(),
        external_user: string_value(&run.metadata, "external_user"),
        external_conversation_id: string_value(&run.metadata, "external_conversation_id"),
        answer: run.answer,
    }))
}

fn string_value(value: &Value, field: &str) -> Option<String> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

fn to_openai_response(run: NativeRunResult, model: String) -> OpenAiChatCompletionResponse {
    let callback_task_id = callback_task_id_from_required_action(&run);
    let tool_calls = openai_tool_calls(run.tool_calls.as_ref(), callback_task_id);
    let finish_reason = if tool_calls.is_some() {
        "tool_calls"
    } else {
        "stop"
    };
    OpenAiChatCompletionResponse {
        id: format!("chatcmpl-{}", run.id),
        object: "chat.completion",
        created: run.created_at.unix_timestamp(),
        model,
        choices: vec![OpenAiChatCompletionChoice {
            index: 0,
            message: OpenAiChatMessage {
                role: "assistant",
                content: if tool_calls.is_some() {
                    run.answer
                } else {
                    Some(run.answer.unwrap_or_default())
                },
                tool_calls,
            },
            finish_reason,
        }],
        usage: openai_usage(run.usage),
    }
}

fn to_openai_responses_response(
    run: NativeRunResult,
    model: String,
    previous_response_id: Option<String>,
) -> OpenAiResponsesObject {
    let callback_task_id = callback_task_id_from_required_action(&run);
    let output_text = if run.tool_calls.is_some() {
        String::new()
    } else {
        run.answer.clone().unwrap_or_default()
    };
    let output = openai_response_function_call_items(run.tool_calls.as_ref(), callback_task_id)
        .unwrap_or_else(|| vec![openai_response_message_item(&run, &output_text)]);
    OpenAiResponsesObject {
        id: response_id_from_run_id(run.id),
        object: "response",
        created_at: run.created_at.unix_timestamp(),
        status: "completed",
        model,
        output,
        output_text,
        usage: openai_responses_usage(run.usage),
        previous_response_id,
    }
}

fn openai_response_message_item(run: &NativeRunResult, output_text: &str) -> Value {
    json!({
        "id": format!("msg_{}", run.id),
        "type": "message",
        "status": "completed",
        "role": "assistant",
        "content": [
            {
                "type": "output_text",
                "text": output_text,
                "annotations": []
            }
        ]
    })
}

fn openai_response_function_call_items(
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
                .unwrap_or("tool_call")
                .to_string();
            let call_id = callback_task_id
                .map(|callback_task_id| {
                    encode_openai_callback_tool_call_id(callback_task_id, &original_id)
                })
                .unwrap_or_else(|| original_id.clone());
            let arguments = call.get("arguments").cloned().unwrap_or_else(|| json!({}));
            Some(json!({
                "id": format!("fc_{}", original_id),
                "type": "function_call",
                "call_id": call_id,
                "name": name,
                "arguments": openai_arguments_string(arguments),
                "status": "completed"
            }))
        })
        .collect::<Vec<_>>();
    (!mapped.is_empty()).then_some(mapped)
}

fn openai_tool_calls(
    tool_calls: Option<&Value>,
    callback_task_id: Option<Uuid>,
) -> Option<Vec<OpenAiToolCall>> {
    let calls = tool_calls?.as_array()?;
    let mapped = calls
        .iter()
        .filter_map(|call| {
            let name = call.get("name").and_then(Value::as_str)?;
            let original_id = call
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or("tool_call")
                .to_string();
            let id = callback_task_id
                .map(|callback_task_id| {
                    encode_openai_callback_tool_call_id(callback_task_id, &original_id)
                })
                .unwrap_or(original_id);
            let arguments = call.get("arguments").cloned().unwrap_or_else(|| json!({}));
            Some(OpenAiToolCall {
                id,
                call_type: "function",
                function: OpenAiToolCallFunction {
                    name: name.to_string(),
                    arguments: openai_arguments_string(arguments),
                },
            })
        })
        .collect::<Vec<_>>();
    (!mapped.is_empty()).then_some(mapped)
}

fn openai_arguments_string(arguments: Value) -> String {
    match arguments {
        Value::String(value) => value,
        value => value.to_string(),
    }
}

fn callback_task_id_from_required_action(run: &NativeRunResult) -> Option<Uuid> {
    run.required_action
        .as_ref()
        .and_then(|action| action.payload.get("callback_task_id"))
        .and_then(Value::as_str)
        .and_then(|value| Uuid::parse_str(value).ok())
}

fn openai_required_action_is_supported(run: &NativeRunResult) -> bool {
    run.required_action.as_ref().is_none_or(|action| {
        action.payload.get("callback_kind").and_then(Value::as_str) == Some("llm_tool_calls")
            && run
                .tool_calls
                .as_ref()
                .is_some_and(|value| value.as_array().is_some_and(|calls| !calls.is_empty()))
    })
}

fn openai_usage(
    usage: Option<control_plane::application_public_api::native::NativeUsage>,
) -> OpenAiUsage {
    let Some(usage) = usage else {
        return OpenAiUsage::default();
    };
    OpenAiUsage {
        prompt_tokens: usage.prompt_tokens.unwrap_or_default(),
        completion_tokens: usage.completion_tokens.unwrap_or_default(),
        total_tokens: usage.total_tokens.unwrap_or_default(),
    }
}

fn openai_responses_usage(
    usage: Option<control_plane::application_public_api::native::NativeUsage>,
) -> OpenAiResponsesUsage {
    let Some(usage) = usage else {
        return OpenAiResponsesUsage::default();
    };
    OpenAiResponsesUsage {
        input_tokens: usage.prompt_tokens.unwrap_or_default(),
        output_tokens: usage.completion_tokens.unwrap_or_default(),
        total_tokens: usage.total_tokens.unwrap_or_default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use control_plane::application_public_api::native::{NativeRequiredAction, NativeRunStatus};
    use time::OffsetDateTime;
    use uuid::Uuid;

    #[test]
    fn openai_response_projects_native_tool_calls() {
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
                    "id": "call_123",
                    "name": "lookup_order",
                    "arguments": {"order_id": "order_123"}
                }
            ])),
            usage: None,
            error: None,
            created_at: OffsetDateTime::UNIX_EPOCH,
        };

        let payload = serde_json::to_value(to_openai_response(run, "provider/model".into()))
            .expect("openai response serializes");

        assert_eq!(payload["choices"][0]["finish_reason"], json!("tool_calls"));
        assert_eq!(
            payload["choices"][0]["message"]["tool_calls"][0]["function"]["name"],
            json!("lookup_order")
        );
        assert_eq!(
            payload["choices"][0]["message"]["tool_calls"][0]["function"]["arguments"],
            json!("{\"order_id\":\"order_123\"}")
        );
    }

    #[test]
    fn openai_response_encodes_callback_task_id_into_tool_call_ids() {
        let callback_task_id = Uuid::from_u128(0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa);
        let run = NativeRunResult {
            id: Uuid::nil(),
            application_id: Uuid::nil(),
            api_key_id: Uuid::nil(),
            publication_version_id: Uuid::nil(),
            status: NativeRunStatus::Waiting,
            node_input_payload: json!({}),
            metadata: json!({}),
            answer: Some("need tool".to_string()),
            required_action: Some(NativeRequiredAction {
                action_type: "submit_tool_outputs".to_string(),
                payload: json!({ "callback_task_id": callback_task_id }),
            }),
            tool_calls: Some(json!([
                {
                    "id": "call_123",
                    "name": "lookup_order",
                    "arguments": {"order_id": "order_123"}
                }
            ])),
            usage: None,
            error: None,
            created_at: OffsetDateTime::UNIX_EPOCH,
        };

        let payload = serde_json::to_value(to_openai_response(run, "provider/model".into()))
            .expect("openai response serializes");

        let tool_call_id = payload["choices"][0]["message"]["tool_calls"][0]["id"]
            .as_str()
            .expect("tool call id should be a string");
        assert!(tool_call_id.starts_with(
            crate::routes::application_public_api::tool_callback_ids::OPENAI_CALLBACK_TOOL_CALL_PREFIX
        ));
        assert_eq!(
            decode_openai_callback_tool_call_id(tool_call_id),
            Some((callback_task_id, "call_123".to_string()))
        );
    }

    #[test]
    fn openai_chat_tool_resume_request_decodes_tool_messages() {
        let callback_task_id = Uuid::from_u128(0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb);
        let external_tool_call_id =
            encode_openai_callback_tool_call_id(callback_task_id, "call_weather");

        let resume = openai_chat_tool_resume_request(&json!({
            "model": "1flowbase",
            "messages": [
                {"role": "assistant", "content": null, "tool_calls": [
                    {"id": external_tool_call_id, "type": "function", "function": {"name": "lookup_weather", "arguments": "{}"}}
                ]},
                {"role": "tool", "tool_call_id": external_tool_call_id, "content": "{\"temperature\":21}"}
            ]
        }))
        .expect("resume request should parse")
        .expect("tool message should resume callback");

        assert_eq!(resume.callback_task_id, callback_task_id);
        assert_eq!(
            resume.tool_results[0]["tool_call_id"],
            json!("call_weather")
        );
        assert_eq!(
            resume.tool_results[0]["content"],
            json!("{\"temperature\":21}")
        );
    }

    #[test]
    fn openai_chat_tool_resume_request_uses_latest_trailing_tool_messages() {
        let previous_callback_task_id = Uuid::from_u128(0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa);
        let current_callback_task_id = Uuid::from_u128(0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb);
        let previous_tool_call_id =
            encode_openai_callback_tool_call_id(previous_callback_task_id, "call_previous");
        let current_tool_call_id =
            encode_openai_callback_tool_call_id(current_callback_task_id, "call_current");

        let resume = openai_chat_tool_resume_request(&json!({
            "model": "1flowbase",
            "messages": [
                {"role": "user", "content": "first"},
                {"role": "assistant", "content": null, "tool_calls": [
                    {"id": previous_tool_call_id, "type": "function", "function": {"name": "lookup_previous", "arguments": "{}"}}
                ]},
                {"role": "tool", "tool_call_id": previous_tool_call_id, "content": "old result"},
                {"role": "assistant", "content": "old answer"},
                {"role": "user", "content": "next"},
                {"role": "assistant", "content": null, "tool_calls": [
                    {"id": current_tool_call_id, "type": "function", "function": {"name": "lookup_current", "arguments": "{}"}}
                ]},
                {"role": "tool", "tool_call_id": current_tool_call_id, "content": "new result"}
            ]
        }))
        .expect("resume request should parse")
        .expect("trailing tool messages should resume callback");

        assert_eq!(resume.callback_task_id, current_callback_task_id);
        assert_eq!(resume.tool_results.as_array().unwrap().len(), 1);
        assert_eq!(
            resume.tool_results[0]["tool_call_id"],
            json!("call_current")
        );
        assert_eq!(resume.tool_results[0]["content"], json!("new result"));
    }

    #[test]
    fn openai_chat_tool_resume_request_ignores_historical_tool_messages() {
        let callback_task_id = Uuid::from_u128(0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa);
        let external_tool_call_id =
            encode_openai_callback_tool_call_id(callback_task_id, "call_previous");

        let resume = openai_chat_tool_resume_request(&json!({
            "model": "1flowbase",
            "messages": [
                {"role": "user", "content": "first"},
                {"role": "assistant", "content": null, "tool_calls": [
                    {"id": external_tool_call_id, "type": "function", "function": {"name": "lookup_previous", "arguments": "{}"}}
                ]},
                {"role": "tool", "tool_call_id": external_tool_call_id, "content": "old result"},
                {"role": "assistant", "content": "old answer"},
                {"role": "user", "content": "next question"}
            ]
        }))
        .expect("historical tool messages should parse");

        assert!(resume.is_none());
    }

    #[test]
    fn openai_responses_response_projects_native_tool_calls_with_encoded_call_id() {
        let callback_task_id = Uuid::from_u128(0xcccccccccccccccccccccccccccccccc);
        let run = NativeRunResult {
            id: Uuid::nil(),
            application_id: Uuid::nil(),
            api_key_id: Uuid::nil(),
            publication_version_id: Uuid::nil(),
            status: NativeRunStatus::Waiting,
            node_input_payload: json!({}),
            metadata: json!({}),
            answer: Some("".to_string()),
            required_action: Some(NativeRequiredAction {
                action_type: "submit_tool_outputs".to_string(),
                payload: json!({ "callback_task_id": callback_task_id, "callback_kind": "llm_tool_calls" }),
            }),
            tool_calls: Some(json!([
                {
                    "id": "call_inventory",
                    "name": "lookup_inventory",
                    "arguments": {"sku": "sku_123"}
                }
            ])),
            usage: None,
            error: None,
            created_at: OffsetDateTime::UNIX_EPOCH,
        };

        let payload = serde_json::to_value(to_openai_responses_response(
            run,
            "provider/model".into(),
            Some("resp_previous".into()),
        ))
        .expect("responses object serializes");

        assert_eq!(payload["status"], json!("completed"));
        assert_eq!(payload["output_text"], json!(""));
        assert_eq!(payload["output"][0]["type"], json!("function_call"));
        assert_eq!(payload["output"][0]["name"], json!("lookup_inventory"));
        assert_eq!(
            payload["output"][0]["arguments"],
            json!("{\"sku\":\"sku_123\"}")
        );
        let call_id = payload["output"][0]["call_id"]
            .as_str()
            .expect("call_id should be encoded");
        assert_eq!(
            decode_openai_callback_tool_call_id(call_id),
            Some((callback_task_id, "call_inventory".to_string()))
        );
    }

    #[test]
    fn openai_responses_tool_resume_request_decodes_function_call_outputs() {
        let callback_task_id = Uuid::from_u128(0xdddddddddddddddddddddddddddddddd);
        let call_id = encode_openai_callback_tool_call_id(callback_task_id, "call_inventory");

        let resume = openai_responses_tool_resume_request(&json!({
            "model": "1flowbase",
            "previous_response_id": "resp_11111111-1111-1111-1111-111111111111",
            "input": [
                {
                    "type": "function_call_output",
                    "call_id": call_id,
                    "output": {"stock": 7}
                }
            ]
        }))
        .expect("resume request should parse")
        .expect("function_call_output should resume callback");

        assert_eq!(resume.callback_task_id, callback_task_id);
        assert_eq!(
            resume.tool_results[0]["tool_call_id"],
            json!("call_inventory")
        );
        assert_eq!(resume.tool_results[0]["content"], json!("{\"stock\":7}"));
    }
}
