use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use control_plane::auth::{
    ApiKeyService, CreateUserApiKeyCommand, ListUserApiKeysCommand, RevokeUserApiKeyCommand,
    UserApiKeyExpirationPolicy,
};
use serde::{Deserialize, Serialize};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    app_state::ApiState,
    error_response::ApiError,
    middleware::{require_csrf::require_csrf, require_session::require_session},
    response::ApiSuccess,
};

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateUserApiKeyRequest {
    pub name: String,
    pub expiration_policy: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UserApiKeyResponse {
    pub id: Uuid,
    pub name: String,
    pub token: Option<String>,
    pub token_prefix: String,
    pub key_kind: String,
    pub creator_user_id: Uuid,
    pub tenant_id: Uuid,
    pub scope_kind: String,
    pub scope_id: Uuid,
    pub enabled: bool,
    pub revoked: bool,
    pub expires_at: Option<String>,
    pub last_used_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UserApiKeyListResponse {
    pub items: Vec<UserApiKeyResponse>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RevokeUserApiKeyResponse {
    pub id: Uuid,
}

pub fn router() -> Router<Arc<ApiState>> {
    Router::new()
        .route(
            "/user-api-keys",
            get(list_user_api_keys).post(create_user_api_key),
        )
        .route(
            "/user-api-keys/:api_key_id/revoke",
            post(revoke_user_api_key),
        )
}

fn parse_expiration_policy(raw: &str) -> Result<UserApiKeyExpirationPolicy, ApiError> {
    match raw {
        "30d" => Ok(UserApiKeyExpirationPolicy::ThirtyDays),
        "1y" => Ok(UserApiKeyExpirationPolicy::OneYear),
        "3y" => Ok(UserApiKeyExpirationPolicy::ThreeYears),
        "never" => Ok(UserApiKeyExpirationPolicy::Never),
        _ => {
            Err(control_plane::errors::ControlPlaneError::InvalidInput("expiration_policy").into())
        }
    }
}

fn format_time(value: OffsetDateTime) -> String {
    value
        .format(&Rfc3339)
        .expect("RFC3339 formatting should support OffsetDateTime")
}

fn format_optional_time(value: Option<OffsetDateTime>) -> Option<String> {
    value.map(format_time)
}

fn user_api_key_response(
    api_key: domain::ApiKeyRecord,
    token: Option<String>,
) -> UserApiKeyResponse {
    UserApiKeyResponse {
        id: api_key.id,
        name: api_key.name,
        token,
        token_prefix: api_key.token_prefix,
        key_kind: api_key.key_kind.as_str().to_string(),
        creator_user_id: api_key.creator_user_id,
        tenant_id: api_key.tenant_id,
        scope_kind: api_key.scope_kind.as_str().to_string(),
        scope_id: api_key.scope_id,
        enabled: api_key.enabled,
        revoked: !api_key.enabled,
        expires_at: format_optional_time(api_key.expires_at),
        last_used_at: format_optional_time(api_key.last_used_at),
        created_at: format_time(api_key.created_at),
        updated_at: format_time(api_key.updated_at),
    }
}

#[utoipa::path(
    get,
    path = "/api/console/user-api-keys",
    responses((status = 200, body = UserApiKeyListResponse), (status = 401, body = crate::error_response::ErrorBody))
)]
pub async fn list_user_api_keys(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
) -> Result<Json<ApiSuccess<UserApiKeyListResponse>>, ApiError> {
    let context = require_session(&state, &headers).await?;
    let items = ApiKeyService::new(state.store.clone())
        .list_user_api_keys(ListUserApiKeysCommand {
            actor_user_id: context.actor.user_id,
            tenant_id: context.actor.tenant_id,
            current_workspace_id: context.actor.current_workspace_id,
        })
        .await?
        .into_iter()
        .map(|api_key| user_api_key_response(api_key, None))
        .collect();

    Ok(Json(ApiSuccess::new(UserApiKeyListResponse { items })))
}

#[utoipa::path(
    post,
    path = "/api/console/user-api-keys",
    request_body = CreateUserApiKeyRequest,
    responses((status = 201, body = UserApiKeyResponse), (status = 400, body = crate::error_response::ErrorBody), (status = 401, body = crate::error_response::ErrorBody), (status = 403, body = crate::error_response::ErrorBody))
)]
pub async fn create_user_api_key(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Json(payload): Json<CreateUserApiKeyRequest>,
) -> Result<(StatusCode, Json<ApiSuccess<UserApiKeyResponse>>), ApiError> {
    let context = require_session(&state, &headers).await?;
    require_csrf(&headers, &context)?;
    let result = ApiKeyService::new(state.store.clone())
        .create_user_api_key(CreateUserApiKeyCommand {
            actor_user_id: context.actor.user_id,
            tenant_id: context.actor.tenant_id,
            current_workspace_id: context.actor.current_workspace_id,
            name: payload.name,
            expiration_policy: parse_expiration_policy(&payload.expiration_policy)?,
        })
        .await?;

    Ok((
        StatusCode::CREATED,
        Json(ApiSuccess::new(user_api_key_response(
            result.api_key,
            Some(result.token),
        ))),
    ))
}

#[utoipa::path(
    post,
    path = "/api/console/user-api-keys/{api_key_id}/revoke",
    params(("api_key_id" = Uuid, Path, description = "User API key id")),
    responses((status = 200, body = RevokeUserApiKeyResponse), (status = 401, body = crate::error_response::ErrorBody), (status = 403, body = crate::error_response::ErrorBody), (status = 404, body = crate::error_response::ErrorBody))
)]
pub async fn revoke_user_api_key(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path(api_key_id): Path<Uuid>,
) -> Result<Json<ApiSuccess<RevokeUserApiKeyResponse>>, ApiError> {
    let context = require_session(&state, &headers).await?;
    require_csrf(&headers, &context)?;
    ApiKeyService::new(state.store.clone())
        .revoke_user_api_key(RevokeUserApiKeyCommand {
            actor_user_id: context.actor.user_id,
            tenant_id: context.actor.tenant_id,
            current_workspace_id: context.actor.current_workspace_id,
            api_key_id,
        })
        .await?;

    Ok(Json(ApiSuccess::new(RevokeUserApiKeyResponse {
        id: api_key_id,
    })))
}
