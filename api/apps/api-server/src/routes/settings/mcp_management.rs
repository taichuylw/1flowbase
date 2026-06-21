use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post, put},
    Json, Router,
};
use control_plane::mcp_management::{
    CreateMcpInstanceCommand, CreateMcpToolBindingCommand, CreateMcpToolCommand,
    McpManagementService, RefreshMcpToolDescriptionCommand, UpdateMcpMetaToolConfigCommand,
    UpdateMcpToolBindingCommand, UpdateMcpToolCommand, UpsertMcpGroupCommand,
};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::{
    app_state::ApiState,
    error_response::ApiError,
    middleware::{require_csrf::require_csrf, require_session::require_session},
    response::ApiSuccess,
};

#[derive(Debug, Serialize, ToSchema)]
pub struct McpInstanceResponse {
    pub id: String,
    pub workspace_id: String,
    pub instance_id: String,
    pub name: String,
    pub description_short: Option<String>,
    pub status: String,
    pub default_entry_path: String,
    pub is_default: bool,
    pub created_by: String,
    pub updated_by: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct McpGroupResponse {
    pub id: String,
    pub instance_record_id: String,
    pub path: String,
    pub display_name: String,
    pub description_short: Option<String>,
    pub enabled: bool,
    pub sort_order: i32,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct McpToolResponse {
    pub id: String,
    pub workspace_id: String,
    pub tool_id: String,
    pub name: String,
    pub short_description: String,
    pub usage_description: Option<String>,
    pub full_description: String,
    pub interface_id: String,
    #[schema(value_type = Object)]
    pub parameter_schema: serde_json::Value,
    #[schema(value_type = Object)]
    pub result_schema: serde_json::Value,
    #[schema(value_type = Object)]
    pub input_mapping: serde_json::Value,
    #[schema(value_type = Object)]
    pub output_mapping: serde_json::Value,
    pub permission_code: Option<String>,
    pub risk_level: String,
    #[schema(value_type = Object)]
    pub audit_policy: serde_json::Value,
    pub des_id: String,
    pub des_id_required: bool,
    pub status: String,
    pub revision: i32,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct McpToolBindingResponse {
    pub id: String,
    pub instance_record_id: String,
    pub tool_record_id: String,
    pub group_path: String,
    pub tool_id: String,
    pub display_alias: Option<String>,
    pub visible: bool,
    pub sort_order: i32,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct McpMetaToolConfigResponse {
    pub id: String,
    pub workspace_id: String,
    pub list_default_limit: i32,
    pub list_max_depth: i32,
    pub list_regex_enabled: bool,
    pub list_regex_max_length: i32,
    #[schema(value_type = Object)]
    pub list_return_fields: serde_json::Value,
    pub get_include_mapping_summary: bool,
    pub get_include_interface_summary: bool,
    pub call_default_des_id_policy: String,
    pub call_high_risk_requires_des_id: bool,
    pub call_validation_error_format: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct McpCatalogResponse {
    pub default_instance: Option<McpInstanceResponse>,
    pub instances: Vec<McpInstanceResponse>,
    pub groups: Vec<McpGroupResponse>,
    pub tools: Vec<McpToolResponse>,
    pub bindings: Vec<McpToolBindingResponse>,
    pub meta_tool_config: McpMetaToolConfigResponse,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct McpInterfaceCatalogEntryResponse {
    pub interface_id: String,
    pub name: String,
    pub short_description: String,
    #[schema(value_type = Object)]
    pub parameter_schema: serde_json::Value,
    #[schema(value_type = Object)]
    pub result_schema: serde_json::Value,
    pub permission_code: Option<String>,
    pub risk_level: String,
    pub bindable: bool,
    pub disabled_reason: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct McpDescriptionCheckResponse {
    pub accepted: bool,
    pub current_des_id: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct McpListItemSummaryResponse {
    pub id: String,
    pub item_kind: String,
    pub path: String,
    pub name: String,
    pub description_short: Option<String>,
    pub children_count: i64,
    pub risk_level: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct McpExportPackageResponse {
    pub instances: Vec<McpInstanceResponse>,
    pub groups: Vec<McpGroupResponse>,
    pub tools: Vec<McpToolResponse>,
    pub bindings: Vec<McpToolBindingResponse>,
    pub meta_tool_config: McpMetaToolConfigResponse,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct McpInstanceDirectoryExportPackageResponse {
    pub instances: Vec<McpInstanceResponse>,
    pub groups: Vec<McpGroupResponse>,
    pub bindings: Vec<McpToolBindingResponse>,
    pub meta_tool_config: McpMetaToolConfigResponse,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateMcpInstanceBody {
    pub instance_id: String,
    pub name: String,
    pub description_short: Option<String>,
    pub status: String,
    pub default_entry_path: String,
    pub is_default: bool,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpsertMcpGroupBody {
    pub path: String,
    pub display_name: String,
    pub description_short: Option<String>,
    pub enabled: bool,
    pub sort_order: i32,
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct DeleteMcpGroupQuery {
    pub path: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateMcpToolBody {
    pub tool_id: Option<String>,
    pub suggested_group_path: Option<String>,
    pub name: String,
    pub short_description: String,
    pub usage_description: Option<String>,
    pub full_description: String,
    pub interface_id: String,
    #[schema(value_type = Object)]
    pub parameter_schema: serde_json::Value,
    #[schema(value_type = Object)]
    pub result_schema: serde_json::Value,
    #[schema(value_type = Object)]
    pub input_mapping: serde_json::Value,
    #[schema(value_type = Object)]
    pub output_mapping: serde_json::Value,
    pub permission_code: Option<String>,
    pub risk_level: String,
    #[schema(value_type = Object)]
    pub audit_policy: serde_json::Value,
    pub des_id_required: bool,
    pub status: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateMcpToolBody {
    pub name: String,
    pub short_description: String,
    pub usage_description: Option<String>,
    pub full_description: String,
    pub interface_id: String,
    #[schema(value_type = Object)]
    pub parameter_schema: serde_json::Value,
    #[schema(value_type = Object)]
    pub result_schema: serde_json::Value,
    #[schema(value_type = Object)]
    pub input_mapping: serde_json::Value,
    #[schema(value_type = Object)]
    pub output_mapping: serde_json::Value,
    pub permission_code: Option<String>,
    pub risk_level: String,
    #[schema(value_type = Object)]
    pub audit_policy: serde_json::Value,
    pub des_id_required: bool,
    pub status: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateMcpToolBindingBody {
    pub group_path: String,
    pub tool_id: String,
    pub display_alias: Option<String>,
    pub visible: bool,
    pub sort_order: i32,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateMcpToolBindingBody {
    pub group_path: String,
    pub display_alias: Option<String>,
    pub visible: bool,
    pub sort_order: i32,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateMcpMetaToolConfigBody {
    pub list_default_limit: i32,
    pub list_max_depth: i32,
    pub list_regex_enabled: bool,
    pub list_regex_max_length: i32,
    #[schema(value_type = Object)]
    pub list_return_fields: serde_json::Value,
    pub get_include_mapping_summary: bool,
    pub get_include_interface_summary: bool,
    pub call_default_des_id_policy: String,
    pub call_high_risk_requires_des_id: bool,
    pub call_validation_error_format: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct McpDescriptionCheckBody {
    pub des_id: Option<String>,
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct McpInterfaceCatalogQuery {
    pub bindable_only: Option<bool>,
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct McpListQuery {
    pub instance_id: Option<String>,
    pub path: Option<String>,
    pub limit: Option<usize>,
}

pub fn router() -> Router<Arc<ApiState>> {
    Router::new()
        .route("/mcp/catalog", get(get_mcp_catalog))
        .route(
            "/mcp/interface-capabilities",
            get(list_mcp_interface_capabilities),
        )
        .route("/mcp/list", get(list_mcp_items))
        .route("/mcp/export", get(export_mcp_catalog))
        .route(
            "/mcp/instances",
            get(list_mcp_instances).post(create_mcp_instance),
        )
        .route("/mcp/instances/export", get(export_mcp_instance_directory))
        .route(
            "/mcp/instances/:instance_id",
            put(update_mcp_instance).delete(delete_mcp_instance),
        )
        .route(
            "/mcp/instances/:instance_id/groups",
            post(upsert_mcp_group).delete(delete_mcp_group),
        )
        .route(
            "/mcp/instances/:instance_id/tool-bindings",
            post(create_mcp_tool_binding),
        )
        .route(
            "/mcp/tool-bindings/:binding_id",
            put(update_mcp_tool_binding).delete(delete_mcp_tool_binding),
        )
        .route("/mcp/tools", get(list_mcp_tools).post(create_mcp_tool))
        .route(
            "/mcp/tools/:tool_id",
            get(get_mcp_tool)
                .put(update_mcp_tool)
                .delete(delete_mcp_tool),
        )
        .route(
            "/mcp/tools/:tool_id/description/refresh",
            post(refresh_mcp_tool_description),
        )
        .route(
            "/mcp/tools/:tool_id/description-check",
            post(check_mcp_tool_description),
        )
        .route(
            "/mcp/meta-tool-config",
            get(get_mcp_meta_tool_config).put(update_mcp_meta_tool_config),
        )
}

#[utoipa::path(get, path = "/api/console/mcp/catalog", responses((status = 200, body = McpCatalogResponse)))]
pub async fn get_mcp_catalog(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
) -> Result<Json<ApiSuccess<McpCatalogResponse>>, ApiError> {
    let context = require_session(&state, &headers).await?;
    let service = McpManagementService::new(state.store.clone());
    let snapshot = service.read_workspace_catalog(context.user.id).await?;
    let snapshot = if snapshot.instances.is_empty() {
        service
            .ensure_default_workspace_catalog(context.user.id)
            .await?
    } else {
        snapshot
    };
    Ok(Json(ApiSuccess::new(to_catalog_response(snapshot))))
}

#[utoipa::path(get, path = "/api/console/mcp/interface-capabilities", params(McpInterfaceCatalogQuery), responses((status = 200, body = [McpInterfaceCatalogEntryResponse])))]
pub async fn list_mcp_interface_capabilities(
    State(state): State<Arc<ApiState>>,
    Query(query): Query<McpInterfaceCatalogQuery>,
    headers: HeaderMap,
) -> Result<Json<ApiSuccess<Vec<McpInterfaceCatalogEntryResponse>>>, ApiError> {
    let context = require_session(&state, &headers).await?;
    let mut entries = McpManagementService::new(state.store.clone())
        .interface_catalog(context.user.id)
        .await?;
    if query.bindable_only.unwrap_or(false) {
        entries.retain(|entry| entry.bindable);
    }
    Ok(Json(ApiSuccess::new(
        entries.into_iter().map(to_interface_response).collect(),
    )))
}

#[utoipa::path(get, path = "/api/console/mcp/list", params(McpListQuery), responses((status = 200, body = [McpListItemSummaryResponse])))]
pub async fn list_mcp_items(
    State(state): State<Arc<ApiState>>,
    Query(query): Query<McpListQuery>,
    headers: HeaderMap,
) -> Result<Json<ApiSuccess<Vec<McpListItemSummaryResponse>>>, ApiError> {
    let context = require_session(&state, &headers).await?;
    let items = McpManagementService::new(state.store.clone())
        .list_items(
            context.user.id,
            query.instance_id.as_deref(),
            query.path.as_deref(),
            query.limit,
        )
        .await?;
    Ok(Json(ApiSuccess::new(
        items.into_iter().map(to_list_item_response).collect(),
    )))
}

#[utoipa::path(get, path = "/api/console/mcp/export", responses((status = 200, body = McpExportPackageResponse)))]
pub async fn export_mcp_catalog(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
) -> Result<Json<ApiSuccess<McpExportPackageResponse>>, ApiError> {
    let context = require_session(&state, &headers).await?;
    let export = McpManagementService::new(state.store.clone())
        .export_workspace_catalog(context.user.id)
        .await?;
    Ok(Json(ApiSuccess::new(to_export_response(export))))
}

#[utoipa::path(get, path = "/api/console/mcp/instances/export", responses((status = 200, body = McpInstanceDirectoryExportPackageResponse)))]
pub async fn export_mcp_instance_directory(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
) -> Result<Json<ApiSuccess<McpInstanceDirectoryExportPackageResponse>>, ApiError> {
    let context = require_session(&state, &headers).await?;
    let export = McpManagementService::new(state.store.clone())
        .export_instance_directory(context.user.id)
        .await?;
    Ok(Json(ApiSuccess::new(
        to_instance_directory_export_response(export),
    )))
}

#[utoipa::path(get, path = "/api/console/mcp/instances", responses((status = 200, body = [McpInstanceResponse])))]
pub async fn list_mcp_instances(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
) -> Result<Json<ApiSuccess<Vec<McpInstanceResponse>>>, ApiError> {
    let context = require_session(&state, &headers).await?;
    let snapshot = McpManagementService::new(state.store.clone())
        .read_workspace_catalog(context.user.id)
        .await?;
    Ok(Json(ApiSuccess::new(
        snapshot
            .instances
            .into_iter()
            .map(to_instance_response)
            .collect(),
    )))
}

#[utoipa::path(post, path = "/api/console/mcp/instances", request_body = CreateMcpInstanceBody, responses((status = 201, body = McpInstanceResponse)))]
pub async fn create_mcp_instance(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Json(body): Json<CreateMcpInstanceBody>,
) -> Result<(StatusCode, Json<ApiSuccess<McpInstanceResponse>>), ApiError> {
    let context = require_session(&state, &headers).await?;
    require_csrf(&headers, &context.session)?;
    let record = McpManagementService::new(state.store.clone())
        .create_instance(to_instance_command(context.user.id, body)?)
        .await?;
    Ok((
        StatusCode::CREATED,
        Json(ApiSuccess::new(to_instance_response(record))),
    ))
}

#[utoipa::path(put, path = "/api/console/mcp/instances/{instance_id}", request_body = CreateMcpInstanceBody, responses((status = 200, body = McpInstanceResponse)))]
pub async fn update_mcp_instance(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path(instance_id): Path<String>,
    Json(mut body): Json<CreateMcpInstanceBody>,
) -> Result<Json<ApiSuccess<McpInstanceResponse>>, ApiError> {
    let context = require_session(&state, &headers).await?;
    require_csrf(&headers, &context.session)?;
    body.instance_id = instance_id;
    let record = McpManagementService::new(state.store.clone())
        .update_instance(to_instance_command(context.user.id, body)?)
        .await?;
    Ok(Json(ApiSuccess::new(to_instance_response(record))))
}

#[utoipa::path(delete, path = "/api/console/mcp/instances/{instance_id}", responses((status = 204)))]
pub async fn delete_mcp_instance(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path(instance_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let context = require_session(&state, &headers).await?;
    require_csrf(&headers, &context.session)?;
    McpManagementService::new(state.store.clone())
        .delete_instance(context.user.id, &instance_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(post, path = "/api/console/mcp/instances/{instance_id}/groups", request_body = UpsertMcpGroupBody, responses((status = 200, body = McpGroupResponse)))]
pub async fn upsert_mcp_group(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path(instance_id): Path<String>,
    Json(body): Json<UpsertMcpGroupBody>,
) -> Result<Json<ApiSuccess<McpGroupResponse>>, ApiError> {
    let context = require_session(&state, &headers).await?;
    require_csrf(&headers, &context.session)?;
    let record = McpManagementService::new(state.store.clone())
        .upsert_group(UpsertMcpGroupCommand {
            actor_user_id: context.user.id,
            instance_id,
            path: body.path,
            display_name: body.display_name,
            description_short: body.description_short,
            enabled: body.enabled,
            sort_order: body.sort_order,
        })
        .await?;
    Ok(Json(ApiSuccess::new(to_group_response(record))))
}

#[utoipa::path(delete, path = "/api/console/mcp/instances/{instance_id}/groups", params(DeleteMcpGroupQuery), responses((status = 204)))]
pub async fn delete_mcp_group(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path(instance_id): Path<String>,
    Query(query): Query<DeleteMcpGroupQuery>,
) -> Result<StatusCode, ApiError> {
    let context = require_session(&state, &headers).await?;
    require_csrf(&headers, &context.session)?;
    McpManagementService::new(state.store.clone())
        .delete_group(context.user.id, &instance_id, &query.path)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(get, path = "/api/console/mcp/tools", responses((status = 200, body = [McpToolResponse])))]
pub async fn list_mcp_tools(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
) -> Result<Json<ApiSuccess<Vec<McpToolResponse>>>, ApiError> {
    let context = require_session(&state, &headers).await?;
    let snapshot = McpManagementService::new(state.store.clone())
        .read_workspace_catalog(context.user.id)
        .await?;
    Ok(Json(ApiSuccess::new(
        snapshot.tools.into_iter().map(to_tool_response).collect(),
    )))
}

#[utoipa::path(post, path = "/api/console/mcp/tools", request_body = CreateMcpToolBody, responses((status = 201, body = McpToolResponse)))]
pub async fn create_mcp_tool(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Json(body): Json<CreateMcpToolBody>,
) -> Result<(StatusCode, Json<ApiSuccess<McpToolResponse>>), ApiError> {
    let context = require_session(&state, &headers).await?;
    require_csrf(&headers, &context.session)?;
    let record = McpManagementService::new(state.store.clone())
        .create_tool(to_create_tool_command(context.user.id, body)?)
        .await?;
    Ok((
        StatusCode::CREATED,
        Json(ApiSuccess::new(to_tool_response(record))),
    ))
}

#[utoipa::path(get, path = "/api/console/mcp/tools/{tool_id}", responses((status = 200, body = McpToolResponse)))]
pub async fn get_mcp_tool(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path(tool_id): Path<String>,
) -> Result<Json<ApiSuccess<McpToolResponse>>, ApiError> {
    let context = require_session(&state, &headers).await?;
    let record = McpManagementService::new(state.store.clone())
        .get_tool(context.user.id, &tool_id)
        .await?;
    Ok(Json(ApiSuccess::new(to_tool_response(record))))
}

#[utoipa::path(put, path = "/api/console/mcp/tools/{tool_id}", request_body = UpdateMcpToolBody, responses((status = 200, body = McpToolResponse)))]
pub async fn update_mcp_tool(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path(tool_id): Path<String>,
    Json(body): Json<UpdateMcpToolBody>,
) -> Result<Json<ApiSuccess<McpToolResponse>>, ApiError> {
    let context = require_session(&state, &headers).await?;
    require_csrf(&headers, &context.session)?;
    let record = McpManagementService::new(state.store.clone())
        .update_tool(to_update_tool_command(context.user.id, tool_id, body)?)
        .await?;
    Ok(Json(ApiSuccess::new(to_tool_response(record))))
}

#[utoipa::path(delete, path = "/api/console/mcp/tools/{tool_id}", responses((status = 204)))]
pub async fn delete_mcp_tool(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path(tool_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let context = require_session(&state, &headers).await?;
    require_csrf(&headers, &context.session)?;
    McpManagementService::new(state.store.clone())
        .delete_tool(context.user.id, &tool_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(post, path = "/api/console/mcp/tools/{tool_id}/description/refresh", responses((status = 200, body = McpToolResponse)))]
pub async fn refresh_mcp_tool_description(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path(tool_id): Path<String>,
) -> Result<Json<ApiSuccess<McpToolResponse>>, ApiError> {
    let context = require_session(&state, &headers).await?;
    require_csrf(&headers, &context.session)?;
    let record = McpManagementService::new(state.store.clone())
        .refresh_tool_description(RefreshMcpToolDescriptionCommand {
            actor_user_id: context.user.id,
            tool_id,
        })
        .await?;
    Ok(Json(ApiSuccess::new(to_tool_response(record))))
}

#[utoipa::path(post, path = "/api/console/mcp/tools/{tool_id}/description-check", request_body = McpDescriptionCheckBody, responses((status = 200, body = McpDescriptionCheckResponse)))]
pub async fn check_mcp_tool_description(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path(tool_id): Path<String>,
    Json(body): Json<McpDescriptionCheckBody>,
) -> Result<Json<ApiSuccess<McpDescriptionCheckResponse>>, ApiError> {
    let context = require_session(&state, &headers).await?;
    let result = McpManagementService::new(state.store.clone())
        .description_check(context.user.id, &tool_id, body.des_id.as_deref())
        .await?;
    Ok(Json(ApiSuccess::new(McpDescriptionCheckResponse {
        accepted: result.accepted,
        current_des_id: result.current_des_id,
    })))
}

#[utoipa::path(post, path = "/api/console/mcp/instances/{instance_id}/tool-bindings", request_body = CreateMcpToolBindingBody, responses((status = 201, body = McpToolBindingResponse)))]
pub async fn create_mcp_tool_binding(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path(instance_id): Path<String>,
    Json(body): Json<CreateMcpToolBindingBody>,
) -> Result<(StatusCode, Json<ApiSuccess<McpToolBindingResponse>>), ApiError> {
    let context = require_session(&state, &headers).await?;
    require_csrf(&headers, &context.session)?;
    let record = McpManagementService::new(state.store.clone())
        .create_tool_binding(CreateMcpToolBindingCommand {
            actor_user_id: context.user.id,
            instance_id,
            group_path: body.group_path,
            tool_id: body.tool_id,
            display_alias: body.display_alias,
            visible: body.visible,
            sort_order: body.sort_order,
        })
        .await?;
    Ok((
        StatusCode::CREATED,
        Json(ApiSuccess::new(to_binding_response(record))),
    ))
}

#[utoipa::path(put, path = "/api/console/mcp/tool-bindings/{binding_id}", request_body = UpdateMcpToolBindingBody, responses((status = 200, body = McpToolBindingResponse)))]
pub async fn update_mcp_tool_binding(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path(binding_id): Path<String>,
    Json(body): Json<UpdateMcpToolBindingBody>,
) -> Result<Json<ApiSuccess<McpToolBindingResponse>>, ApiError> {
    let context = require_session(&state, &headers).await?;
    require_csrf(&headers, &context.session)?;
    let record = McpManagementService::new(state.store.clone())
        .update_tool_binding(UpdateMcpToolBindingCommand {
            actor_user_id: context.user.id,
            binding_id: parse_uuid(&binding_id, "binding_id")?,
            group_path: body.group_path,
            display_alias: body.display_alias,
            visible: body.visible,
            sort_order: body.sort_order,
        })
        .await?;
    Ok(Json(ApiSuccess::new(to_binding_response(record))))
}

#[utoipa::path(delete, path = "/api/console/mcp/tool-bindings/{binding_id}", responses((status = 204)))]
pub async fn delete_mcp_tool_binding(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path(binding_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let context = require_session(&state, &headers).await?;
    require_csrf(&headers, &context.session)?;
    McpManagementService::new(state.store.clone())
        .delete_tool_binding(context.user.id, parse_uuid(&binding_id, "binding_id")?)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(get, path = "/api/console/mcp/meta-tool-config", responses((status = 200, body = McpMetaToolConfigResponse)))]
pub async fn get_mcp_meta_tool_config(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
) -> Result<Json<ApiSuccess<McpMetaToolConfigResponse>>, ApiError> {
    let context = require_session(&state, &headers).await?;
    let snapshot = McpManagementService::new(state.store.clone())
        .read_workspace_catalog(context.user.id)
        .await?;
    Ok(Json(ApiSuccess::new(to_meta_config_response(
        snapshot.meta_tool_config,
    ))))
}

#[utoipa::path(put, path = "/api/console/mcp/meta-tool-config", request_body = UpdateMcpMetaToolConfigBody, responses((status = 200, body = McpMetaToolConfigResponse)))]
pub async fn update_mcp_meta_tool_config(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Json(body): Json<UpdateMcpMetaToolConfigBody>,
) -> Result<Json<ApiSuccess<McpMetaToolConfigResponse>>, ApiError> {
    let context = require_session(&state, &headers).await?;
    require_csrf(&headers, &context.session)?;
    let record = McpManagementService::new(state.store.clone())
        .update_meta_tool_config(UpdateMcpMetaToolConfigCommand {
            actor_user_id: context.user.id,
            list_default_limit: body.list_default_limit,
            list_max_depth: body.list_max_depth,
            list_regex_enabled: body.list_regex_enabled,
            list_regex_max_length: body.list_regex_max_length,
            list_return_fields: body.list_return_fields,
            get_include_mapping_summary: body.get_include_mapping_summary,
            get_include_interface_summary: body.get_include_interface_summary,
            call_default_des_id_policy: body.call_default_des_id_policy,
            call_high_risk_requires_des_id: body.call_high_risk_requires_des_id,
            call_validation_error_format: body.call_validation_error_format,
        })
        .await?;
    Ok(Json(ApiSuccess::new(to_meta_config_response(record))))
}

fn parse_uuid(raw: &str, field: &'static str) -> Result<Uuid, ApiError> {
    Uuid::parse_str(raw)
        .map_err(|_| control_plane::errors::ControlPlaneError::InvalidInput(field).into())
}

fn parse_instance_status(value: &str) -> Result<domain::McpInstanceStatus, ApiError> {
    match value {
        "draft" => Ok(domain::McpInstanceStatus::Draft),
        "enabled" => Ok(domain::McpInstanceStatus::Enabled),
        "disabled" => Ok(domain::McpInstanceStatus::Disabled),
        "archived" => Ok(domain::McpInstanceStatus::Archived),
        _ => Err(control_plane::errors::ControlPlaneError::InvalidInput("status").into()),
    }
}

fn parse_tool_status(value: &str) -> Result<domain::McpToolStatus, ApiError> {
    match value {
        "draft" => Ok(domain::McpToolStatus::Draft),
        "enabled" => Ok(domain::McpToolStatus::Enabled),
        "disabled" => Ok(domain::McpToolStatus::Disabled),
        "archived" => Ok(domain::McpToolStatus::Archived),
        _ => Err(control_plane::errors::ControlPlaneError::InvalidInput("status").into()),
    }
}

fn parse_risk_level(value: &str) -> Result<domain::McpRiskLevel, ApiError> {
    match value {
        "low" => Ok(domain::McpRiskLevel::Low),
        "medium" => Ok(domain::McpRiskLevel::Medium),
        "high" => Ok(domain::McpRiskLevel::High),
        "critical" => Ok(domain::McpRiskLevel::Critical),
        _ => Err(control_plane::errors::ControlPlaneError::InvalidInput("risk_level").into()),
    }
}

fn to_instance_command(
    actor_user_id: Uuid,
    body: CreateMcpInstanceBody,
) -> Result<CreateMcpInstanceCommand, ApiError> {
    Ok(CreateMcpInstanceCommand {
        actor_user_id,
        instance_id: body.instance_id,
        name: body.name,
        description_short: body.description_short,
        status: parse_instance_status(&body.status)?,
        default_entry_path: body.default_entry_path,
        is_default: body.is_default,
    })
}

fn to_create_tool_command(
    actor_user_id: Uuid,
    body: CreateMcpToolBody,
) -> Result<CreateMcpToolCommand, ApiError> {
    Ok(CreateMcpToolCommand {
        actor_user_id,
        tool_id: body.tool_id,
        suggested_group_path: body.suggested_group_path,
        name: body.name,
        short_description: body.short_description,
        usage_description: body.usage_description,
        full_description: body.full_description,
        interface_id: body.interface_id,
        parameter_schema: body.parameter_schema,
        result_schema: body.result_schema,
        input_mapping: body.input_mapping,
        output_mapping: body.output_mapping,
        permission_code: body.permission_code,
        risk_level: parse_risk_level(&body.risk_level)?,
        audit_policy: body.audit_policy,
        des_id_required: body.des_id_required,
        status: parse_tool_status(&body.status)?,
    })
}

fn to_update_tool_command(
    actor_user_id: Uuid,
    tool_id: String,
    body: UpdateMcpToolBody,
) -> Result<UpdateMcpToolCommand, ApiError> {
    Ok(UpdateMcpToolCommand {
        actor_user_id,
        tool_id,
        name: body.name,
        short_description: body.short_description,
        usage_description: body.usage_description,
        full_description: body.full_description,
        interface_id: body.interface_id,
        parameter_schema: body.parameter_schema,
        result_schema: body.result_schema,
        input_mapping: body.input_mapping,
        output_mapping: body.output_mapping,
        permission_code: body.permission_code,
        risk_level: parse_risk_level(&body.risk_level)?,
        audit_policy: body.audit_policy,
        des_id_required: body.des_id_required,
        status: parse_tool_status(&body.status)?,
    })
}

fn to_catalog_response(snapshot: domain::McpCatalogSnapshot) -> McpCatalogResponse {
    McpCatalogResponse {
        default_instance: snapshot.default_instance.map(to_instance_response),
        instances: snapshot
            .instances
            .into_iter()
            .map(to_instance_response)
            .collect(),
        groups: snapshot.groups.into_iter().map(to_group_response).collect(),
        tools: snapshot.tools.into_iter().map(to_tool_response).collect(),
        bindings: snapshot
            .bindings
            .into_iter()
            .map(to_binding_response)
            .collect(),
        meta_tool_config: to_meta_config_response(snapshot.meta_tool_config),
    }
}

fn to_export_response(export: domain::McpExportPackage) -> McpExportPackageResponse {
    McpExportPackageResponse {
        instances: export
            .instances
            .into_iter()
            .map(to_instance_response)
            .collect(),
        groups: export.groups.into_iter().map(to_group_response).collect(),
        tools: export.tools.into_iter().map(to_tool_response).collect(),
        bindings: export
            .bindings
            .into_iter()
            .map(to_binding_response)
            .collect(),
        meta_tool_config: to_meta_config_response(export.meta_tool_config),
    }
}

fn to_instance_directory_export_response(
    export: domain::McpInstanceDirectoryExportPackage,
) -> McpInstanceDirectoryExportPackageResponse {
    McpInstanceDirectoryExportPackageResponse {
        instances: export
            .instances
            .into_iter()
            .map(to_instance_response)
            .collect(),
        groups: export.groups.into_iter().map(to_group_response).collect(),
        bindings: export
            .bindings
            .into_iter()
            .map(to_binding_response)
            .collect(),
        meta_tool_config: to_meta_config_response(export.meta_tool_config),
    }
}

fn to_instance_response(record: domain::McpInstanceRecord) -> McpInstanceResponse {
    McpInstanceResponse {
        id: record.id.to_string(),
        workspace_id: record.workspace_id.to_string(),
        instance_id: record.instance_id,
        name: record.name,
        description_short: record.description_short,
        status: record.status.as_str().into(),
        default_entry_path: record.default_entry_path,
        is_default: record.is_default,
        created_by: record.created_by.to_string(),
        updated_by: record.updated_by.to_string(),
        created_at: record.created_at.to_string(),
        updated_at: record.updated_at.to_string(),
    }
}

fn to_group_response(record: domain::McpGroupRecord) -> McpGroupResponse {
    McpGroupResponse {
        id: record.id.to_string(),
        instance_record_id: record.instance_record_id.to_string(),
        path: record.path,
        display_name: record.display_name,
        description_short: record.description_short,
        enabled: record.enabled,
        sort_order: record.sort_order,
    }
}

fn to_tool_response(record: domain::McpToolRecord) -> McpToolResponse {
    McpToolResponse {
        id: record.id.to_string(),
        workspace_id: record.workspace_id.to_string(),
        tool_id: record.tool_id,
        name: record.name,
        short_description: record.short_description,
        usage_description: record.usage_description,
        full_description: record.full_description,
        interface_id: record.interface_id,
        parameter_schema: record.parameter_schema,
        result_schema: record.result_schema,
        input_mapping: record.input_mapping,
        output_mapping: record.output_mapping,
        permission_code: record.permission_code,
        risk_level: record.risk_level.as_str().into(),
        audit_policy: record.audit_policy,
        des_id: record.des_id,
        des_id_required: record.des_id_required,
        status: record.status.as_str().into(),
        revision: record.revision,
    }
}

fn to_binding_response(record: domain::McpToolBindingRecord) -> McpToolBindingResponse {
    McpToolBindingResponse {
        id: record.id.to_string(),
        instance_record_id: record.instance_record_id.to_string(),
        tool_record_id: record.tool_record_id.to_string(),
        group_path: record.group_path,
        tool_id: record.tool_id,
        display_alias: record.display_alias,
        visible: record.visible,
        sort_order: record.sort_order,
    }
}

fn to_meta_config_response(record: domain::McpMetaToolConfigRecord) -> McpMetaToolConfigResponse {
    McpMetaToolConfigResponse {
        id: record.id.to_string(),
        workspace_id: record.workspace_id.to_string(),
        list_default_limit: record.list_default_limit,
        list_max_depth: record.list_max_depth,
        list_regex_enabled: record.list_regex_enabled,
        list_regex_max_length: record.list_regex_max_length,
        list_return_fields: record.list_return_fields,
        get_include_mapping_summary: record.get_include_mapping_summary,
        get_include_interface_summary: record.get_include_interface_summary,
        call_default_des_id_policy: record.call_default_des_id_policy,
        call_high_risk_requires_des_id: record.call_high_risk_requires_des_id,
        call_validation_error_format: record.call_validation_error_format,
    }
}

fn to_interface_response(
    entry: domain::McpInterfaceCatalogEntry,
) -> McpInterfaceCatalogEntryResponse {
    McpInterfaceCatalogEntryResponse {
        interface_id: entry.interface_id,
        name: entry.name,
        short_description: entry.short_description,
        parameter_schema: entry.parameter_schema,
        result_schema: entry.result_schema,
        permission_code: entry.permission_code,
        risk_level: entry.risk_level.as_str().into(),
        bindable: entry.bindable,
        disabled_reason: entry.disabled_reason,
    }
}

fn to_list_item_response(item: domain::McpListItemSummary) -> McpListItemSummaryResponse {
    McpListItemSummaryResponse {
        id: item.id,
        item_kind: match item.item_kind {
            domain::McpListItemKind::Group => "group".into(),
            domain::McpListItemKind::Tool => "tool".into(),
        },
        path: item.path,
        name: item.name,
        description_short: item.description_short,
        children_count: item.children_count,
        risk_level: item.risk_level.map(|risk| risk.as_str().into()),
    }
}
