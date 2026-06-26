use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post, put},
    Json, Router,
};
use control_plane::frontstage::{
    CreateFrontstageGroupCommand, CreateFrontstagePageCommand, DeleteFrontstagePageCommand,
    FrontstagePageService, GetFrontstageBlockCodeCommand, GetFrontstagePageDetailCommand,
    MoveFrontstagePageCommand, SaveFrontstageBlockCodeCommand, SaveFrontstagePageContentCommand,
    UpdateFrontstagePageMetadataCommand,
};
use serde::de::Deserializer;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    app_state::ApiState,
    error_response::ApiError,
    middleware::{require_csrf::require_csrf, require_session::require_session},
    response::ApiSuccess,
};

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum FrontstagePageTreeNodeKind {
    Group,
    Page,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct FrontstagePageTreeNodeResponse {
    pub id: String,
    pub title: Option<String>,
    pub icon: Option<String>,
    pub tooltip: Option<String>,
    pub is_hidden: bool,
    pub kind: FrontstagePageTreeNodeKind,
    #[serde(default)]
    #[schema(no_recursion)]
    pub children: Vec<FrontstagePageTreeNodeResponse>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct FrontstagePageResponse {
    pub id: String,
    title: Option<String>,
    pub icon: Option<String>,
    pub tooltip: Option<String>,
    pub is_hidden: bool,
    pub kind: FrontstagePageTreeNodeKind,
    pub parent_id: Option<String>,
    pub rank: String,
    pub schema_root_uid: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct FrontstagePageSchemaResponse {
    pub root_uid: String,
    pub payload: Value,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct FrontstagePageRootResponse {
    pub uid: String,
    pub payload: Value,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct FrontstagePageDetailResponse {
    pub page: FrontstagePageResponse,
    pub schema: FrontstagePageSchemaResponse,
    pub root: FrontstagePageRootResponse,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct FrontstageBlockCodeResponse {
    pub page_id: String,
    pub code_ref: String,
    pub code: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateFrontstageGroupBody {
    pub title: Option<String>,
    pub icon: Option<String>,
    pub tooltip: Option<String>,
    pub parent_id: Option<String>,
    pub rank: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateFrontstagePageBody {
    pub title: Option<String>,
    pub icon: Option<String>,
    pub tooltip: Option<String>,
    pub parent_id: Option<String>,
    pub rank: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateFrontstagePageMetadataBody {
    #[serde(default, deserialize_with = "deserialize_present_optional")]
    pub title: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_present_optional")]
    pub icon: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_present_optional")]
    pub tooltip: Option<Option<String>>,
    pub is_hidden: Option<bool>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct MoveFrontstagePageBody {
    pub parent_id: Option<String>,
    pub rank: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct SaveFrontstagePageContentPayloadBody {
    pub payload: Value,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct SaveFrontstagePageContentBody {
    pub schema: SaveFrontstagePageContentPayloadBody,
    pub root: SaveFrontstagePageContentPayloadBody,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct SaveFrontstageBlockCodeBody {
    pub code: String,
}

fn deserialize_present_optional<'de, D, T>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer).map(Some)
}

pub fn router() -> Router<Arc<ApiState>> {
    Router::new()
        .route(
            "/frontstage/:workspace_id/pages",
            get(list_frontstage_pages).post(create_frontstage_page),
        )
        .route(
            "/frontstage/:workspace_id/pages/groups",
            post(create_frontstage_group),
        )
        .route(
            "/frontstage/:workspace_id/pages/:page_id",
            get(get_frontstage_page_detail)
                .patch(update_frontstage_page_title)
                .delete(delete_frontstage_page),
        )
        .route(
            "/frontstage/:workspace_id/pages/:page_id/move",
            post(move_frontstage_page),
        )
        .route(
            "/frontstage/:workspace_id/pages/:page_id/content",
            put(save_frontstage_page_content),
        )
        .route(
            "/frontstage/:workspace_id/pages/:page_id/block-codes/:code_ref",
            get(get_frontstage_block_code).put(save_frontstage_block_code),
        )
}

#[utoipa::path(
    get,
    path = "/api/console/frontstage/{workspace_id}/pages",
    params(
        ("workspace_id" = String, Path, description = "Workspace id"),
    ),
    responses(
        (status = 200, body = [FrontstagePageTreeNodeResponse]),
        (status = 400, body = crate::error_response::ErrorBody),
        (status = 401, body = crate::error_response::ErrorBody),
        (status = 403, body = crate::error_response::ErrorBody)
    )
)]
pub async fn list_frontstage_pages(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path(workspace_id): Path<String>,
) -> Result<Json<ApiSuccess<Vec<FrontstagePageTreeNodeResponse>>>, ApiError> {
    let context = require_session(&state, &headers).await?;
    let workspace_id = parse_uuid(&workspace_id, "workspace_id")?;
    let tree = FrontstagePageService::new(state.store.clone())
        .list_page_tree(context.user.id, workspace_id)
        .await?;

    Ok(Json(ApiSuccess::new(
        tree.into_iter().map(to_tree_node_response).collect(),
    )))
}

#[utoipa::path(
    post,
    path = "/api/console/frontstage/{workspace_id}/pages/groups",
    request_body = CreateFrontstageGroupBody,
    params(("workspace_id" = String, Path, description = "Workspace id")),
    responses(
        (status = 201, body = FrontstagePageResponse),
        (status = 400, body = crate::error_response::ErrorBody),
        (status = 401, body = crate::error_response::ErrorBody),
        (status = 403, body = crate::error_response::ErrorBody)
    )
)]
pub async fn create_frontstage_group(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path(workspace_id): Path<String>,
    Json(body): Json<CreateFrontstageGroupBody>,
) -> Result<(StatusCode, Json<ApiSuccess<FrontstagePageResponse>>), ApiError> {
    let context = require_session(&state, &headers).await?;
    require_csrf(&headers, &context)?;
    let workspace_id = parse_uuid(&workspace_id, "workspace_id")?;
    let parent_id = parse_optional_uuid(body.parent_id.as_deref(), "parent_id")?;

    let page = FrontstagePageService::new(state.store.clone())
        .create_group(CreateFrontstageGroupCommand {
            actor_user_id: context.user.id,
            workspace_id,
            title: body.title,
            icon: body.icon,
            tooltip: body.tooltip,
            parent_id,
            rank: body.rank,
        })
        .await?;

    Ok((
        StatusCode::CREATED,
        Json(ApiSuccess::new(to_page_response(page))),
    ))
}

#[utoipa::path(
    post,
    path = "/api/console/frontstage/{workspace_id}/pages",
    request_body = CreateFrontstagePageBody,
    params(("workspace_id" = String, Path, description = "Workspace id")),
    responses(
        (status = 201, body = FrontstagePageResponse),
        (status = 400, body = crate::error_response::ErrorBody),
        (status = 401, body = crate::error_response::ErrorBody),
        (status = 403, body = crate::error_response::ErrorBody)
    )
)]
pub async fn create_frontstage_page(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path(workspace_id): Path<String>,
    Json(body): Json<CreateFrontstagePageBody>,
) -> Result<(StatusCode, Json<ApiSuccess<FrontstagePageResponse>>), ApiError> {
    let context = require_session(&state, &headers).await?;
    require_csrf(&headers, &context)?;
    let workspace_id = parse_uuid(&workspace_id, "workspace_id")?;
    let parent_id = parse_optional_uuid(body.parent_id.as_deref(), "parent_id")?;

    let page = FrontstagePageService::new(state.store.clone())
        .create_page(CreateFrontstagePageCommand {
            actor_user_id: context.user.id,
            workspace_id,
            title: body.title,
            icon: body.icon,
            tooltip: body.tooltip,
            parent_id,
            rank: body.rank,
        })
        .await?;

    Ok((
        StatusCode::CREATED,
        Json(ApiSuccess::new(to_page_response(page))),
    ))
}

#[utoipa::path(
    get,
    path = "/api/console/frontstage/{workspace_id}/pages/{page_id}",
    params(
        ("workspace_id" = String, Path, description = "Workspace id"),
        ("page_id" = String, Path, description = "Page id")
    ),
    responses(
        (status = 200, body = FrontstagePageDetailResponse),
        (status = 400, body = crate::error_response::ErrorBody),
        (status = 401, body = crate::error_response::ErrorBody),
        (status = 403, body = crate::error_response::ErrorBody),
        (status = 404, body = crate::error_response::ErrorBody)
    )
)]
pub async fn get_frontstage_page_detail(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path((workspace_id, page_id)): Path<(String, String)>,
) -> Result<Json<ApiSuccess<FrontstagePageDetailResponse>>, ApiError> {
    let context = require_session(&state, &headers).await?;
    let workspace_id = parse_uuid(&workspace_id, "workspace_id")?;
    let page_id = parse_uuid(&page_id, "page_id")?;

    let detail = FrontstagePageService::new(state.store.clone())
        .get_page_detail(GetFrontstagePageDetailCommand {
            actor_user_id: context.user.id,
            workspace_id,
            page_id,
        })
        .await?;

    Ok(Json(ApiSuccess::new(to_page_detail_response(detail))))
}

#[utoipa::path(
    patch,
    path = "/api/console/frontstage/{workspace_id}/pages/{page_id}",
    request_body = UpdateFrontstagePageMetadataBody,
    params(
        ("workspace_id" = String, Path, description = "Workspace id"),
        ("page_id" = String, Path, description = "Page or group id")
    ),
    responses(
        (status = 200, body = FrontstagePageResponse),
        (status = 400, body = crate::error_response::ErrorBody),
        (status = 401, body = crate::error_response::ErrorBody),
        (status = 403, body = crate::error_response::ErrorBody),
        (status = 404, body = crate::error_response::ErrorBody)
    )
)]
pub async fn update_frontstage_page_title(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path((workspace_id, page_id)): Path<(String, String)>,
    Json(body): Json<UpdateFrontstagePageMetadataBody>,
) -> Result<Json<ApiSuccess<FrontstagePageResponse>>, ApiError> {
    let context = require_session(&state, &headers).await?;
    require_csrf(&headers, &context)?;
    let workspace_id = parse_uuid(&workspace_id, "workspace_id")?;
    let page_id = parse_uuid(&page_id, "page_id")?;

    let page = FrontstagePageService::new(state.store.clone())
        .update_metadata(UpdateFrontstagePageMetadataCommand {
            actor_user_id: context.user.id,
            workspace_id,
            page_id,
            title: body.title,
            icon: body.icon,
            tooltip: body.tooltip,
            is_hidden: body.is_hidden,
        })
        .await?;

    Ok(Json(ApiSuccess::new(to_page_response(page))))
}

#[utoipa::path(
    post,
    path = "/api/console/frontstage/{workspace_id}/pages/{page_id}/move",
    request_body = MoveFrontstagePageBody,
    params(
        ("workspace_id" = String, Path, description = "Workspace id"),
        ("page_id" = String, Path, description = "Page or group id")
    ),
    responses(
        (status = 200, body = FrontstagePageResponse),
        (status = 400, body = crate::error_response::ErrorBody),
        (status = 401, body = crate::error_response::ErrorBody),
        (status = 403, body = crate::error_response::ErrorBody),
        (status = 404, body = crate::error_response::ErrorBody)
    )
)]
pub async fn move_frontstage_page(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path((workspace_id, page_id)): Path<(String, String)>,
    Json(body): Json<MoveFrontstagePageBody>,
) -> Result<Json<ApiSuccess<FrontstagePageResponse>>, ApiError> {
    let context = require_session(&state, &headers).await?;
    require_csrf(&headers, &context)?;
    let workspace_id = parse_uuid(&workspace_id, "workspace_id")?;
    let page_id = parse_uuid(&page_id, "page_id")?;
    let parent_id = parse_optional_uuid(body.parent_id.as_deref(), "parent_id")?;

    let page = FrontstagePageService::new(state.store.clone())
        .move_page(MoveFrontstagePageCommand {
            actor_user_id: context.user.id,
            workspace_id,
            page_id,
            parent_id,
            rank: body.rank,
        })
        .await?;

    Ok(Json(ApiSuccess::new(to_page_response(page))))
}

#[utoipa::path(
    delete,
    path = "/api/console/frontstage/{workspace_id}/pages/{page_id}",
    params(
        ("workspace_id" = String, Path, description = "Workspace id"),
        ("page_id" = String, Path, description = "Page or group id")
    ),
    responses(
        (status = 204),
        (status = 400, body = crate::error_response::ErrorBody),
        (status = 401, body = crate::error_response::ErrorBody),
        (status = 403, body = crate::error_response::ErrorBody),
        (status = 404, body = crate::error_response::ErrorBody)
    )
)]
pub async fn delete_frontstage_page(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path((workspace_id, page_id)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    let context = require_session(&state, &headers).await?;
    require_csrf(&headers, &context)?;
    let workspace_id = parse_uuid(&workspace_id, "workspace_id")?;
    let page_id = parse_uuid(&page_id, "page_id")?;

    FrontstagePageService::new(state.store.clone())
        .delete_page(DeleteFrontstagePageCommand {
            actor_user_id: context.user.id,
            workspace_id,
            page_id,
        })
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    put,
    path = "/api/console/frontstage/{workspace_id}/pages/{page_id}/content",
    request_body = SaveFrontstagePageContentBody,
    params(
        ("workspace_id" = String, Path, description = "Workspace id"),
        ("page_id" = String, Path, description = "Page id")
    ),
    responses(
        (status = 200, body = FrontstagePageDetailResponse),
        (status = 400, body = crate::error_response::ErrorBody),
        (status = 401, body = crate::error_response::ErrorBody),
        (status = 403, body = crate::error_response::ErrorBody),
        (status = 404, body = crate::error_response::ErrorBody)
    )
)]
pub async fn save_frontstage_page_content(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path((workspace_id, page_id)): Path<(String, String)>,
    Json(body): Json<SaveFrontstagePageContentBody>,
) -> Result<Json<ApiSuccess<FrontstagePageDetailResponse>>, ApiError> {
    let context = require_session(&state, &headers).await?;
    require_csrf(&headers, &context)?;
    let workspace_id = parse_uuid(&workspace_id, "workspace_id")?;
    let page_id = parse_uuid(&page_id, "page_id")?;

    let detail = FrontstagePageService::new(state.store.clone())
        .save_page_content(SaveFrontstagePageContentCommand {
            actor_user_id: context.user.id,
            workspace_id,
            page_id,
            schema_payload: body.schema.payload,
            root_payload: body.root.payload,
        })
        .await?;

    Ok(Json(ApiSuccess::new(to_page_detail_response(detail))))
}

#[utoipa::path(
    get,
    path = "/api/console/frontstage/{workspace_id}/pages/{page_id}/block-codes/{code_ref}",
    params(
        ("workspace_id" = String, Path, description = "Workspace id"),
        ("page_id" = String, Path, description = "Page id"),
        ("code_ref" = String, Path, description = "JS block code ref")
    ),
    responses(
        (status = 200, body = FrontstageBlockCodeResponse),
        (status = 400, body = crate::error_response::ErrorBody),
        (status = 401, body = crate::error_response::ErrorBody),
        (status = 403, body = crate::error_response::ErrorBody),
        (status = 404, body = crate::error_response::ErrorBody)
    )
)]
pub async fn get_frontstage_block_code(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path((workspace_id, page_id, code_ref)): Path<(String, String, String)>,
) -> Result<Json<ApiSuccess<FrontstageBlockCodeResponse>>, ApiError> {
    let context = require_session(&state, &headers).await?;
    let workspace_id = parse_uuid(&workspace_id, "workspace_id")?;
    let page_id = parse_uuid(&page_id, "page_id")?;

    let code = FrontstagePageService::new(state.store.clone())
        .get_block_code(GetFrontstageBlockCodeCommand {
            actor_user_id: context.user.id,
            workspace_id,
            page_id,
            code_ref,
        })
        .await?;

    Ok(Json(ApiSuccess::new(to_block_code_response(code))))
}

#[utoipa::path(
    put,
    path = "/api/console/frontstage/{workspace_id}/pages/{page_id}/block-codes/{code_ref}",
    request_body = SaveFrontstageBlockCodeBody,
    params(
        ("workspace_id" = String, Path, description = "Workspace id"),
        ("page_id" = String, Path, description = "Page id"),
        ("code_ref" = String, Path, description = "JS block code ref")
    ),
    responses(
        (status = 200, body = FrontstageBlockCodeResponse),
        (status = 400, body = crate::error_response::ErrorBody),
        (status = 401, body = crate::error_response::ErrorBody),
        (status = 403, body = crate::error_response::ErrorBody),
        (status = 404, body = crate::error_response::ErrorBody)
    )
)]
pub async fn save_frontstage_block_code(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path((workspace_id, page_id, code_ref)): Path<(String, String, String)>,
    Json(body): Json<SaveFrontstageBlockCodeBody>,
) -> Result<Json<ApiSuccess<FrontstageBlockCodeResponse>>, ApiError> {
    let context = require_session(&state, &headers).await?;
    require_csrf(&headers, &context)?;
    let workspace_id = parse_uuid(&workspace_id, "workspace_id")?;
    let page_id = parse_uuid(&page_id, "page_id")?;

    let code = FrontstagePageService::new(state.store.clone())
        .save_block_code(SaveFrontstageBlockCodeCommand {
            actor_user_id: context.user.id,
            workspace_id,
            page_id,
            code_ref,
            code: body.code,
        })
        .await?;

    Ok(Json(ApiSuccess::new(to_block_code_response(code))))
}

fn parse_uuid(raw: &str, field: &'static str) -> Result<Uuid, ApiError> {
    Uuid::parse_str(raw)
        .map_err(|_| control_plane::errors::ControlPlaneError::InvalidInput(field).into())
}

fn parse_optional_uuid(raw: Option<&str>, field: &'static str) -> Result<Option<Uuid>, ApiError> {
    raw.map(|value| parse_uuid(value, field)).transpose()
}

fn to_kind_response(kind: domain::FrontstagePageKind) -> FrontstagePageTreeNodeKind {
    match kind {
        domain::FrontstagePageKind::Group => FrontstagePageTreeNodeKind::Group,
        domain::FrontstagePageKind::Page => FrontstagePageTreeNodeKind::Page,
    }
}

fn to_page_response(page: domain::FrontstagePageRecord) -> FrontstagePageResponse {
    FrontstagePageResponse {
        id: page.id.to_string(),
        title: page.title,
        icon: page.icon,
        tooltip: page.tooltip,
        is_hidden: page.is_hidden,
        kind: to_kind_response(page.kind),
        parent_id: page.parent_id.map(|id| id.to_string()),
        rank: page.rank,
        schema_root_uid: page.schema_root_uid,
    }
}

fn to_page_detail_response(
    detail: domain::frontstage::FrontstagePageDetail,
) -> FrontstagePageDetailResponse {
    FrontstagePageDetailResponse {
        page: to_page_response(detail.page),
        schema: FrontstagePageSchemaResponse {
            root_uid: detail.schema.root_uid.clone(),
            payload: detail.schema.schema_payload,
        },
        root: FrontstagePageRootResponse {
            uid: detail.schema.root_uid,
            payload: detail.schema.root_payload,
        },
    }
}

fn to_block_code_response(
    code: domain::frontstage::FrontstageBlockCodeRecord,
) -> FrontstageBlockCodeResponse {
    FrontstageBlockCodeResponse {
        page_id: code.page_id.to_string(),
        code_ref: code.code_ref,
        code: code.code,
    }
}

fn to_tree_node_response(node: domain::FrontstagePageTreeNode) -> FrontstagePageTreeNodeResponse {
    FrontstagePageTreeNodeResponse {
        id: node.page.id.to_string(),
        title: node.page.title,
        icon: node.page.icon,
        tooltip: node.page.tooltip,
        is_hidden: node.page.is_hidden,
        kind: to_kind_response(node.page.kind),
        children: node
            .children
            .into_iter()
            .map(to_tree_node_response)
            .collect(),
    }
}
