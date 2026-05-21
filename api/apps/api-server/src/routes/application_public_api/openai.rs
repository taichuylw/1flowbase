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
};
use serde::Serialize;
use serde_json::{json, Value};
use tracing::{info, warn};
use utoipa::ToSchema;

use crate::{
    app_state::ApiState,
    routes::application_public_api::{compat_sse, native},
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
    pub output: Vec<OpenAiResponseOutputItem>,
    pub output_text: String,
    pub usage: OpenAiResponsesUsage,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_response_id: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct OpenAiResponseOutputItem {
    pub id: String,
    #[serde(rename = "type")]
    pub item_type: &'static str,
    pub status: &'static str,
    pub role: &'static str,
    pub content: Vec<OpenAiResponseContentPart>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct OpenAiResponseContentPart {
    #[serde(rename = "type")]
    pub part_type: &'static str,
    pub text: String,
    pub annotations: Vec<Value>,
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
    let request = match parse_openai_request(body) {
        Ok(request) => request,
        Err(error) => {
            warn_openai_route_error(
                "chat_completions",
                &error,
                "openai compatible request validation failed",
            );
            return Err(error);
        }
    };
    let model = request.model.clone().unwrap_or_default();
    let response_mode = request.response_mode.clone();
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
    if run.required_action.is_some() {
        warn!(
            route = "chat_completions",
            application_id = %run.application_id,
            flow_run_id = %run.id,
            "openai compatible blocking run reached unsupported required_action state"
        );
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
    if run.required_action.is_some() {
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

fn parse_openai_request(body: Bytes) -> Result<NativeRunRequest, OpenAiRouteError> {
    let value = parse_openai_json_body(body)?;
    map_chat_completion_request(value).map_err(Into::into)
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
    let tool_calls = openai_tool_calls(run.tool_calls.as_ref());
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
    let output_text = run.answer.unwrap_or_default();
    OpenAiResponsesObject {
        id: response_id_from_run_id(run.id),
        object: "response",
        created_at: run.created_at.unix_timestamp(),
        status: "completed",
        model,
        output: vec![OpenAiResponseOutputItem {
            id: format!("msg_{}", run.id),
            item_type: "message",
            status: "completed",
            role: "assistant",
            content: vec![OpenAiResponseContentPart {
                part_type: "output_text",
                text: output_text.clone(),
                annotations: Vec::new(),
            }],
        }],
        output_text,
        usage: openai_responses_usage(run.usage),
        previous_response_id,
    }
}

fn openai_tool_calls(tool_calls: Option<&Value>) -> Option<Vec<OpenAiToolCall>> {
    let calls = tool_calls?.as_array()?;
    let mapped = calls
        .iter()
        .filter_map(|call| {
            let name = call.get("name").and_then(Value::as_str)?;
            let id = call
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or("tool_call")
                .to_string();
            let arguments = call.get("arguments").cloned().unwrap_or_else(|| json!({}));
            Some(OpenAiToolCall {
                id,
                call_type: "function",
                function: OpenAiToolCallFunction {
                    name: name.to_string(),
                    arguments: match arguments {
                        Value::String(value) => value,
                        value => value.to_string(),
                    },
                },
            })
        })
        .collect::<Vec<_>>();
    (!mapped.is_empty()).then_some(mapped)
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
    use control_plane::application_public_api::native::NativeRunStatus;
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
}
