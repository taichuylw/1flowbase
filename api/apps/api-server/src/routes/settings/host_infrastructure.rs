use std::sync::Arc;

use access_control::ensure_permission;
use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    routing::{get, post, put},
    Json, Router,
};
use control_plane::{
    audit::audit_log,
    errors::ControlPlaneError,
    host_infrastructure_config::{
        HostInfrastructureConfigService, HostInfrastructureProviderConfigView,
        SaveHostInfrastructureProviderConfigCommand,
    },
    ports::{
        AuthRepository, CacheDomainSnapshot, CacheEntrySnapshot, CacheInspectionCapabilities,
        CacheStore, DistributedLock, EphemeralEntrySnapshot, EphemeralEntryValueSnapshot,
        EphemeralInspectionCapabilities, EphemeralInspectionEntryPage,
        EphemeralInspectionPageRequest, EphemeralInspectionSummarySnapshot,
        EphemeralInspectionTreeNodeSnapshot, EphemeralInspectionTreePage, EphemeralValueRevealMode,
        EventBus, RateLimitStore, RuntimeEventStream, SessionStore, TaskQueue,
    },
};
use plugin_framework::provider_contract::{
    PluginFormCondition, PluginFormFieldSchema, PluginFormOption,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    app_state::ApiState,
    error_response::ApiError,
    middleware::{require_csrf::require_csrf, require_session::require_session},
    response::ApiSuccess,
};

#[derive(Debug, Serialize, ToSchema)]
pub struct PluginFormOptionResponse {
    pub label: String,
    #[schema(value_type = Object)]
    pub value: serde_json::Value,
    pub description: Option<String>,
    pub disabled: Option<bool>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PluginFormConditionResponse {
    pub field: String,
    pub operator: String,
    #[schema(value_type = Object)]
    pub value: Option<serde_json::Value>,
    #[schema(value_type = [Object])]
    pub values: Vec<serde_json::Value>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PluginFormFieldSchemaResponse {
    pub key: String,
    pub label: String,
    #[serde(rename = "type")]
    pub field_type: String,
    pub control: Option<String>,
    pub group: Option<String>,
    pub order: Option<i32>,
    pub advanced: Option<bool>,
    pub required: Option<bool>,
    pub send_mode: Option<String>,
    pub enabled_by_default: Option<bool>,
    pub description: Option<String>,
    pub placeholder: Option<String>,
    #[schema(value_type = Object)]
    pub default_value: Option<serde_json::Value>,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub step: Option<f64>,
    pub precision: Option<u32>,
    pub unit: Option<String>,
    pub options: Vec<PluginFormOptionResponse>,
    pub visible_when: Vec<PluginFormConditionResponse>,
    pub disabled_when: Vec<PluginFormConditionResponse>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct HostInfrastructureProviderConfigResponse {
    pub installation_id: String,
    pub extension_id: String,
    pub provider_code: String,
    pub display_name: String,
    pub description: Option<String>,
    pub runtime_status: String,
    pub desired_state: String,
    pub config_ref: String,
    pub contracts: Vec<String>,
    pub enabled_contracts: Vec<String>,
    pub config_schema: Vec<PluginFormFieldSchemaResponse>,
    #[schema(value_type = Object)]
    pub config_json: serde_json::Value,
    pub restart_required: bool,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct SaveHostInfrastructureProviderConfigBody {
    pub enabled_contracts: Vec<String>,
    #[schema(value_type = Object)]
    pub config_json: serde_json::Value,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SaveHostInfrastructureProviderConfigResponse {
    pub restart_required: bool,
    pub installation_desired_state: String,
    pub provider_config_status: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CacheInspectionCapabilitiesResponse {
    pub list_domains: bool,
    pub list_entries: bool,
    pub reveal_value: bool,
    pub clear_entry: bool,
    pub clear_domain: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CacheDomainResponse {
    pub domain_code: String,
    pub entry_count: u64,
    pub total_value_size_bytes: u64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CacheEntryMetadataResponse {
    pub domain_code: String,
    pub key: String,
    pub value_size_bytes: u64,
    pub ttl_seconds: Option<i64>,
    pub created_at_unix: Option<i64>,
    pub expires_at_unix: Option<i64>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CacheOverviewResponse {
    pub provider_code: Option<String>,
    pub can_manage: bool,
    pub capabilities: CacheInspectionCapabilitiesResponse,
    pub domains: Vec<CacheDomainResponse>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CacheEntriesResponse {
    pub domain_code: String,
    pub capabilities: CacheInspectionCapabilitiesResponse,
    pub entries: Vec<CacheEntryMetadataResponse>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CacheEntryKeyBody {
    pub key: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CacheEntryValueResponse {
    pub metadata: CacheEntryMetadataResponse,
    #[schema(value_type = Object)]
    pub value: serde_json::Value,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ClearCacheEntryResponse {
    pub cleared: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ClearCacheDomainResponse {
    pub cleared_count: u64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct MemoryInspectionCapabilitiesResponse {
    pub list_entries: bool,
    pub list_tree: bool,
    pub search_entries: bool,
    pub reveal_value: bool,
    pub default_page_size: u64,
    pub max_page_size: u64,
    pub default_byte_limit: u64,
    pub max_byte_limit: u64,
    pub default_preview_size_bytes: u64,
    pub max_full_value_size_bytes: u64,
    pub max_value_size_bytes: u64,
    pub max_payload_size_bytes: u64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct MemoryContractSummaryResponse {
    pub contract_code: String,
    pub label: String,
    pub provider_code: Option<String>,
    pub capabilities: MemoryInspectionCapabilitiesResponse,
    pub supported: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct MemoryOverviewResponse {
    pub can_manage: bool,
    pub contracts: Vec<MemoryContractSummaryResponse>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct MemoryStatsResponse {
    pub contract_code: String,
    pub label: String,
    pub provider_code: Option<String>,
    pub capabilities: MemoryInspectionCapabilitiesResponse,
    pub supported: bool,
    pub inspection_path: Vec<String>,
    pub entry_count: u64,
    pub sensitive_entry_count: u64,
    pub total_value_size_bytes: u64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct MemoryStatsOverviewResponse {
    pub inspection_path: Vec<String>,
    pub contracts: Vec<MemoryStatsResponse>,
    pub entry_count: u64,
    pub sensitive_entry_count: u64,
    pub total_value_size_bytes: u64,
}
#[derive(Debug, Serialize, ToSchema)]
pub struct MemoryEntryMetadataResponse {
    pub contract_code: String,
    pub group_code: Option<String>,
    pub entry_ref: String,
    pub key: String,
    pub inspection_path: Vec<String>,
    pub entry_kind: String,
    pub status: String,
    pub owner: Option<String>,
    pub value_size_bytes: u64,
    pub metadata_size_bytes: u64,
    pub ttl_seconds: Option<i64>,
    pub created_at_unix: Option<i64>,
    pub expires_at_unix: Option<i64>,
    pub sensitive: bool,
    #[schema(value_type = Object)]
    pub metadata: serde_json::Value,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct MemoryEntriesResponse {
    pub contract_code: String,
    pub label: String,
    pub provider_code: Option<String>,
    pub capabilities: MemoryInspectionCapabilitiesResponse,
    pub supported: bool,
    pub inspection_path: Vec<String>,
    pub entries: Vec<MemoryEntryMetadataResponse>,
    pub next_cursor: Option<String>,
    pub limit: u64,
    pub byte_limit: u64,
    pub emitted_bytes: u64,
    pub truncated_by_byte_limit: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct MemoryTreeNodeResponse {
    pub node_ref: String,
    pub label: String,
    pub inspection_path: Vec<String>,
    pub depth: u64,
    pub has_children: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct MemoryTreeResponse {
    pub contract_code: String,
    pub label: String,
    pub provider_code: Option<String>,
    pub capabilities: MemoryInspectionCapabilitiesResponse,
    pub supported: bool,
    pub inspection_path: Vec<String>,
    pub nodes: Vec<MemoryTreeNodeResponse>,
    pub next_cursor: Option<String>,
    pub limit: u64,
    pub byte_limit: u64,
    pub emitted_bytes: u64,
    pub truncated_by_byte_limit: bool,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct MemoryPageQuery {
    pub path: Option<String>,
    pub cursor: Option<String>,
    pub limit: Option<usize>,
    pub byte_limit: Option<usize>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct MemoryPathQuery {
    pub path: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct MemorySearchQuery {
    pub q: String,
    pub path: Option<String>,
    pub cursor: Option<String>,
    pub limit: Option<usize>,
    pub byte_limit: Option<usize>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct MemoryEntryRevealBody {
    pub entry_ref: String,
    pub reveal_mode: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct MemoryEntryValueResponse {
    pub metadata: MemoryEntryMetadataResponse,
    pub reveal_mode: String,
    pub value_state: String,
    #[schema(value_type = Object)]
    pub value: Option<serde_json::Value>,
    pub value_preview: Option<String>,
    pub preview_size_bytes: u64,
    pub full_value_size_bytes: u64,
}

pub fn router() -> Router<Arc<ApiState>> {
    Router::new()
        .route(
            "/settings/host-infrastructure/memory",
            get(get_host_infrastructure_memory_overview),
        )
        .route(
            "/settings/host-infrastructure/memory/stats",
            get(get_host_infrastructure_memory_stats_overview),
        )
        .route(
            "/settings/host-infrastructure/memory/contracts/:contract_code/entries",
            get(list_host_infrastructure_memory_entries),
        )
        .route(
            "/settings/host-infrastructure/memory/contracts/:contract_code/stats",
            get(get_host_infrastructure_memory_stats),
        )
        .route(
            "/settings/host-infrastructure/memory/contracts/:contract_code/entries/search",
            get(search_host_infrastructure_memory_entries),
        )
        .route(
            "/settings/host-infrastructure/memory/contracts/:contract_code/tree",
            get(list_host_infrastructure_memory_tree),
        )
        .route(
            "/settings/host-infrastructure/memory/contracts/:contract_code/entries/reveal",
            post(reveal_host_infrastructure_memory_entry),
        )
        .route(
            "/settings/host-infrastructure/cache",
            get(get_host_infrastructure_cache_overview),
        )
        .route(
            "/settings/host-infrastructure/cache/domains/:domain_code/entries",
            get(list_host_infrastructure_cache_entries),
        )
        .route(
            "/settings/host-infrastructure/cache/domains/:domain_code/entries/reveal",
            post(reveal_host_infrastructure_cache_entry),
        )
        .route(
            "/settings/host-infrastructure/cache/domains/:domain_code/entries/clear",
            post(clear_host_infrastructure_cache_entry),
        )
        .route(
            "/settings/host-infrastructure/cache/domains/:domain_code/clear",
            post(clear_host_infrastructure_cache_domain),
        )
        .route(
            "/settings/host-infrastructure/providers",
            get(list_host_infrastructure_providers),
        )
        .route(
            "/settings/host-infrastructure/providers/:installation_id/:provider_code/config",
            put(save_host_infrastructure_provider_config),
        )
}

#[utoipa::path(
    get,
    path = "/api/console/settings/host-infrastructure/memory",
    responses((status = 200, body = MemoryOverviewResponse), (status = 401, body = crate::error_response::ErrorBody), (status = 403, body = crate::error_response::ErrorBody))
)]
pub async fn get_host_infrastructure_memory_overview(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
) -> Result<Json<ApiSuccess<MemoryOverviewResponse>>, ApiError> {
    let context = require_session(&state, &headers).await?;
    ensure_memory_view(&context.actor)?;
    let mut contracts = Vec::new();
    for (contract_code, label) in memory_contract_definitions() {
        contracts.push(memory_contract_summary(&state, contract_code, label).await?);
    }

    Ok(Json(ApiSuccess::new(MemoryOverviewResponse {
        can_manage: can_manage_memory(&context.actor),
        contracts,
    })))
}

#[utoipa::path(
    get,
    path = "/api/console/settings/host-infrastructure/memory/stats",
    responses((status = 200, body = MemoryStatsOverviewResponse), (status = 401, body = crate::error_response::ErrorBody), (status = 403, body = crate::error_response::ErrorBody))
)]
pub async fn get_host_infrastructure_memory_stats_overview(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
) -> Result<Json<ApiSuccess<MemoryStatsOverviewResponse>>, ApiError> {
    let context = require_session(&state, &headers).await?;
    ensure_memory_view(&context.actor)?;
    let inspection_path = Vec::new();
    let mut contracts = Vec::new();
    let mut total = EphemeralInspectionSummarySnapshot::empty();
    for (contract_code, label) in memory_contract_definitions() {
        let stats =
            memory_contract_stats_response(&state, contract_code, label, &inspection_path).await?;
        total.entry_count += stats.entry_count;
        total.sensitive_entry_count += stats.sensitive_entry_count;
        total.total_value_size_bytes += stats.total_value_size_bytes;
        contracts.push(stats);
    }
    Ok(Json(ApiSuccess::new(MemoryStatsOverviewResponse {
        inspection_path,
        contracts,
        entry_count: total.entry_count,
        sensitive_entry_count: total.sensitive_entry_count,
        total_value_size_bytes: total.total_value_size_bytes,
    })))
}
#[utoipa::path(
    get,
    path = "/api/console/settings/host-infrastructure/memory/contracts/{contract_code}/entries",
    params(("contract_code" = String, Path)),
    responses((status = 200, body = MemoryEntriesResponse), (status = 401, body = crate::error_response::ErrorBody), (status = 403, body = crate::error_response::ErrorBody), (status = 404, body = crate::error_response::ErrorBody))
)]
pub async fn list_host_infrastructure_memory_entries(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path(contract_code): Path<String>,
    Query(query): Query<MemoryPageQuery>,
) -> Result<Json<ApiSuccess<MemoryEntriesResponse>>, ApiError> {
    let context = require_session(&state, &headers).await?;
    ensure_memory_view(&context.actor)?;
    let label = memory_contract_label(&contract_code)?;
    let target = memory_inspection_target(&state, &contract_code)?;
    let capabilities = target.capabilities();
    let page_request = memory_page_request(query.path, query.cursor, query.limit, query.byte_limit);
    let page = if capabilities.list_entries {
        target.list_entry_page(page_request).await?
    } else {
        empty_memory_entry_page(page_request)
    };

    Ok(Json(ApiSuccess::new(MemoryEntriesResponse {
        contract_code: contract_code.clone(),
        label: label.to_string(),
        provider_code: state
            .infrastructure
            .default_provider(&contract_code)
            .map(ToString::to_string),
        capabilities: capabilities.into(),
        supported: memory_contract_supported(capabilities),
        inspection_path: page.inspection_path,
        entries: page
            .entries
            .into_iter()
            .map(to_memory_entry_metadata_response)
            .collect(),
        next_cursor: page.next_cursor,
        limit: page.limit,
        byte_limit: page.byte_limit,
        emitted_bytes: page.emitted_bytes,
        truncated_by_byte_limit: page.truncated_by_byte_limit,
    })))
}

#[utoipa::path(
    get,
    path = "/api/console/settings/host-infrastructure/memory/contracts/{contract_code}/stats",
    params(("contract_code" = String, Path)),
    responses((status = 200, body = MemoryStatsResponse), (status = 401, body = crate::error_response::ErrorBody), (status = 403, body = crate::error_response::ErrorBody), (status = 404, body = crate::error_response::ErrorBody))
)]
pub async fn get_host_infrastructure_memory_stats(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path(contract_code): Path<String>,
    Query(query): Query<MemoryPathQuery>,
) -> Result<Json<ApiSuccess<MemoryStatsResponse>>, ApiError> {
    let context = require_session(&state, &headers).await?;
    ensure_memory_view(&context.actor)?;
    let label = memory_contract_label(&contract_code)?;
    let inspection_path = memory_query_path(query.path);
    let stats =
        memory_contract_stats_response(&state, &contract_code, label, &inspection_path).await?;

    Ok(Json(ApiSuccess::new(stats)))
}

#[utoipa::path(
    get,
    path = "/api/console/settings/host-infrastructure/memory/contracts/{contract_code}/tree",
    params(("contract_code" = String, Path)),
    responses((status = 200, body = MemoryTreeResponse), (status = 401, body = crate::error_response::ErrorBody), (status = 403, body = crate::error_response::ErrorBody), (status = 404, body = crate::error_response::ErrorBody))
)]
pub async fn list_host_infrastructure_memory_tree(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path(contract_code): Path<String>,
    Query(query): Query<MemoryPageQuery>,
) -> Result<Json<ApiSuccess<MemoryTreeResponse>>, ApiError> {
    let context = require_session(&state, &headers).await?;
    ensure_memory_view(&context.actor)?;
    let label = memory_contract_label(&contract_code)?;
    let target = memory_inspection_target(&state, &contract_code)?;
    let capabilities = target.capabilities();
    let page_request = memory_page_request(query.path, query.cursor, query.limit, query.byte_limit);
    let page = if capabilities.list_tree {
        target.list_tree(page_request).await?
    } else {
        empty_memory_tree_page(page_request)
    };

    Ok(Json(ApiSuccess::new(MemoryTreeResponse {
        contract_code: contract_code.clone(),
        label: label.to_string(),
        provider_code: state
            .infrastructure
            .default_provider(&contract_code)
            .map(ToString::to_string),
        capabilities: capabilities.into(),
        supported: memory_contract_supported(capabilities),
        inspection_path: page.inspection_path,
        nodes: page
            .nodes
            .into_iter()
            .map(to_memory_tree_node_response)
            .collect(),
        next_cursor: page.next_cursor,
        limit: page.limit,
        byte_limit: page.byte_limit,
        emitted_bytes: page.emitted_bytes,
        truncated_by_byte_limit: page.truncated_by_byte_limit,
    })))
}

#[utoipa::path(
    get,
    path = "/api/console/settings/host-infrastructure/memory/contracts/{contract_code}/entries/search",
    params(("contract_code" = String, Path)),
    responses((status = 200, body = MemoryEntriesResponse), (status = 401, body = crate::error_response::ErrorBody), (status = 403, body = crate::error_response::ErrorBody), (status = 404, body = crate::error_response::ErrorBody))
)]
pub async fn search_host_infrastructure_memory_entries(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path(contract_code): Path<String>,
    Query(query): Query<MemorySearchQuery>,
) -> Result<Json<ApiSuccess<MemoryEntriesResponse>>, ApiError> {
    let context = require_session(&state, &headers).await?;
    ensure_memory_view(&context.actor)?;
    let label = memory_contract_label(&contract_code)?;
    let target = memory_inspection_target(&state, &contract_code)?;
    let capabilities = target.capabilities();
    let page_request = memory_page_request(query.path, query.cursor, query.limit, query.byte_limit);
    let page = if capabilities.search_entries {
        target.search_entry_page(&query.q, page_request).await?
    } else {
        empty_memory_entry_page(page_request)
    };

    Ok(Json(ApiSuccess::new(MemoryEntriesResponse {
        contract_code: contract_code.clone(),
        label: label.to_string(),
        provider_code: state
            .infrastructure
            .default_provider(&contract_code)
            .map(ToString::to_string),
        capabilities: capabilities.into(),
        supported: memory_contract_supported(capabilities),
        inspection_path: page.inspection_path,
        entries: page
            .entries
            .into_iter()
            .map(to_memory_entry_metadata_response)
            .collect(),
        next_cursor: page.next_cursor,
        limit: page.limit,
        byte_limit: page.byte_limit,
        emitted_bytes: page.emitted_bytes,
        truncated_by_byte_limit: page.truncated_by_byte_limit,
    })))
}

#[utoipa::path(
    post,
    path = "/api/console/settings/host-infrastructure/memory/contracts/{contract_code}/entries/reveal",
    request_body = MemoryEntryRevealBody,
    params(("contract_code" = String, Path)),
    responses((status = 200, body = MemoryEntryValueResponse), (status = 401, body = crate::error_response::ErrorBody), (status = 403, body = crate::error_response::ErrorBody), (status = 404, body = crate::error_response::ErrorBody))
)]
pub async fn reveal_host_infrastructure_memory_entry(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path(contract_code): Path<String>,
    Json(body): Json<MemoryEntryRevealBody>,
) -> Result<Json<ApiSuccess<MemoryEntryValueResponse>>, ApiError> {
    let context = require_session(&state, &headers).await?;
    require_csrf(&headers, &context.session)?;
    ensure_memory_manage(&context.actor)?;
    let _label = memory_contract_label(&contract_code)?;
    let target = memory_inspection_target(&state, &contract_code)?;
    let capabilities = target.capabilities();
    if !capabilities.reveal_value {
        return Err(ControlPlaneError::InvalidInput("memory_inspection_unsupported").into());
    }

    let reveal_mode = parse_memory_reveal_mode(body.reveal_mode.as_deref())?;
    let value = target
        .reveal_entry(&body.entry_ref, reveal_mode)
        .await?
        .ok_or(ControlPlaneError::NotFound("memory_entry"))?;
    append_memory_audit(
        &state,
        &context.actor,
        "host_infrastructure.memory_value_revealed",
        serde_json::json!({
            "contract_code": value.metadata.contract_code.clone(),
            "group_code": value.metadata.group_code.clone(),
            "entry_ref": value.metadata.entry_ref.clone(),
            "key": value.metadata.key.clone(),
            "inspection_path": value.metadata.inspection_path.clone(),
            "entry_kind": value.metadata.entry_kind.clone(),
            "status": value.metadata.status.clone(),
            "owner": value.metadata.owner.clone(),
            "value_size_bytes": value.metadata.value_size_bytes,
            "reveal_mode": format_memory_reveal_mode(value.reveal_mode),
            "value_state": format_memory_value_state(value.value_state),
            "sensitive": value.metadata.sensitive,
        }),
    )
    .await?;

    Ok(Json(ApiSuccess::new(MemoryEntryValueResponse {
        metadata: to_memory_entry_metadata_response(value.metadata),
        reveal_mode: format_memory_reveal_mode(value.reveal_mode),
        value_state: format_memory_value_state(value.value_state),
        value: value.value,
        value_preview: value.value_preview,
        preview_size_bytes: value.preview_size_bytes,
        full_value_size_bytes: value.full_value_size_bytes,
    })))
}

#[utoipa::path(
    get,
    path = "/api/console/settings/host-infrastructure/cache",
    responses((status = 200, body = CacheOverviewResponse), (status = 401, body = crate::error_response::ErrorBody), (status = 403, body = crate::error_response::ErrorBody))
)]
pub async fn get_host_infrastructure_cache_overview(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
) -> Result<Json<ApiSuccess<CacheOverviewResponse>>, ApiError> {
    let context = require_session(&state, &headers).await?;
    ensure_cache_view(&context.actor)?;
    let cache = state.infrastructure.cache_store();
    let capabilities = cache.inspection_capabilities();
    let domains = if capabilities.list_domains {
        cache
            .list_cache_domains()
            .await?
            .into_iter()
            .map(to_cache_domain_response)
            .collect()
    } else {
        Vec::new()
    };

    Ok(Json(ApiSuccess::new(CacheOverviewResponse {
        provider_code: state
            .infrastructure
            .default_provider("cache-store")
            .map(ToString::to_string),
        can_manage: can_manage_cache(&context.actor),
        capabilities: capabilities.into(),
        domains,
    })))
}

#[utoipa::path(
    get,
    path = "/api/console/settings/host-infrastructure/cache/domains/{domain_code}/entries",
    params(("domain_code" = String, Path)),
    responses((status = 200, body = CacheEntriesResponse), (status = 401, body = crate::error_response::ErrorBody), (status = 403, body = crate::error_response::ErrorBody))
)]
pub async fn list_host_infrastructure_cache_entries(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path(domain_code): Path<String>,
) -> Result<Json<ApiSuccess<CacheEntriesResponse>>, ApiError> {
    let context = require_session(&state, &headers).await?;
    ensure_cache_view(&context.actor)?;
    let cache = state.infrastructure.cache_store();
    let capabilities = cache.inspection_capabilities();
    let entries = if capabilities.list_entries {
        cache
            .list_cache_entries(&domain_code)
            .await?
            .into_iter()
            .map(to_cache_entry_metadata_response)
            .collect()
    } else {
        Vec::new()
    };

    Ok(Json(ApiSuccess::new(CacheEntriesResponse {
        domain_code,
        capabilities: capabilities.into(),
        entries,
    })))
}

#[utoipa::path(
    post,
    path = "/api/console/settings/host-infrastructure/cache/domains/{domain_code}/entries/reveal",
    request_body = CacheEntryKeyBody,
    params(("domain_code" = String, Path)),
    responses((status = 200, body = CacheEntryValueResponse), (status = 401, body = crate::error_response::ErrorBody), (status = 403, body = crate::error_response::ErrorBody), (status = 404, body = crate::error_response::ErrorBody))
)]
pub async fn reveal_host_infrastructure_cache_entry(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path(domain_code): Path<String>,
    Json(body): Json<CacheEntryKeyBody>,
) -> Result<Json<ApiSuccess<CacheEntryValueResponse>>, ApiError> {
    let context = require_session(&state, &headers).await?;
    require_csrf(&headers, &context.session)?;
    ensure_cache_manage(&context.actor)?;
    let cache = state.infrastructure.cache_store();
    let capabilities = cache.inspection_capabilities();
    if !capabilities.reveal_value {
        return Err(ControlPlaneError::InvalidInput("cache_inspection_unsupported").into());
    }

    let value = cache
        .reveal_cache_entry(&domain_code, &body.key)
        .await?
        .ok_or(ControlPlaneError::NotFound("cache_entry"))?;
    append_cache_audit(
        &state,
        &context.actor,
        "host_infrastructure.cache_value_revealed",
        serde_json::json!({
            "domain_code": domain_code,
            "key": body.key,
            "value_size_bytes": value.metadata.value_size_bytes,
        }),
    )
    .await?;

    Ok(Json(ApiSuccess::new(CacheEntryValueResponse {
        metadata: to_cache_entry_metadata_response(value.metadata),
        value: value.value,
    })))
}

#[utoipa::path(
    post,
    path = "/api/console/settings/host-infrastructure/cache/domains/{domain_code}/entries/clear",
    request_body = CacheEntryKeyBody,
    params(("domain_code" = String, Path)),
    responses((status = 200, body = ClearCacheEntryResponse), (status = 401, body = crate::error_response::ErrorBody), (status = 403, body = crate::error_response::ErrorBody))
)]
pub async fn clear_host_infrastructure_cache_entry(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path(domain_code): Path<String>,
    Json(body): Json<CacheEntryKeyBody>,
) -> Result<Json<ApiSuccess<ClearCacheEntryResponse>>, ApiError> {
    let context = require_session(&state, &headers).await?;
    require_csrf(&headers, &context.session)?;
    ensure_cache_manage(&context.actor)?;
    let cache = state.infrastructure.cache_store();
    let capabilities = cache.inspection_capabilities();
    if !capabilities.clear_entry {
        return Err(ControlPlaneError::InvalidInput("cache_inspection_unsupported").into());
    }

    let cleared = cache.clear_cache_entry(&domain_code, &body.key).await?;
    append_cache_audit(
        &state,
        &context.actor,
        "host_infrastructure.cache_entry_cleared",
        serde_json::json!({
            "domain_code": domain_code,
            "key": body.key,
            "cleared": cleared,
        }),
    )
    .await?;

    Ok(Json(ApiSuccess::new(ClearCacheEntryResponse { cleared })))
}

#[utoipa::path(
    post,
    path = "/api/console/settings/host-infrastructure/cache/domains/{domain_code}/clear",
    params(("domain_code" = String, Path)),
    responses((status = 200, body = ClearCacheDomainResponse), (status = 401, body = crate::error_response::ErrorBody), (status = 403, body = crate::error_response::ErrorBody))
)]
pub async fn clear_host_infrastructure_cache_domain(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path(domain_code): Path<String>,
) -> Result<Json<ApiSuccess<ClearCacheDomainResponse>>, ApiError> {
    let context = require_session(&state, &headers).await?;
    require_csrf(&headers, &context.session)?;
    ensure_cache_manage(&context.actor)?;
    let cache = state.infrastructure.cache_store();
    let capabilities = cache.inspection_capabilities();
    if !capabilities.clear_domain {
        return Err(ControlPlaneError::InvalidInput("cache_inspection_unsupported").into());
    }

    let cleared_count = cache.clear_cache_domain(&domain_code).await?;
    append_cache_audit(
        &state,
        &context.actor,
        "host_infrastructure.cache_domain_cleared",
        serde_json::json!({
            "domain_code": domain_code,
            "cleared_count": cleared_count,
        }),
    )
    .await?;

    Ok(Json(ApiSuccess::new(ClearCacheDomainResponse {
        cleared_count,
    })))
}

#[utoipa::path(
    get,
    path = "/api/console/settings/host-infrastructure/providers",
    responses((status = 200, body = [HostInfrastructureProviderConfigResponse]), (status = 401, body = crate::error_response::ErrorBody), (status = 403, body = crate::error_response::ErrorBody))
)]
pub async fn list_host_infrastructure_providers(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
) -> Result<Json<ApiSuccess<Vec<HostInfrastructureProviderConfigResponse>>>, ApiError> {
    let context = require_session(&state, &headers).await?;
    let providers = HostInfrastructureConfigService::new(state.store.clone())
        .list_providers(context.actor)
        .await?
        .providers
        .into_iter()
        .map(to_provider_response)
        .collect();

    Ok(Json(ApiSuccess::new(providers)))
}

#[utoipa::path(
    put,
    path = "/api/console/settings/host-infrastructure/providers/{installation_id}/{provider_code}/config",
    request_body = SaveHostInfrastructureProviderConfigBody,
    params(("installation_id" = String, Path), ("provider_code" = String, Path)),
    responses((status = 200, body = SaveHostInfrastructureProviderConfigResponse), (status = 401, body = crate::error_response::ErrorBody), (status = 403, body = crate::error_response::ErrorBody))
)]
pub async fn save_host_infrastructure_provider_config(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path((installation_id, provider_code)): Path<(String, String)>,
    Json(body): Json<SaveHostInfrastructureProviderConfigBody>,
) -> Result<Json<ApiSuccess<SaveHostInfrastructureProviderConfigResponse>>, ApiError> {
    let context = require_session(&state, &headers).await?;
    require_csrf(&headers, &context.session)?;
    let installation_id = Uuid::parse_str(&installation_id)
        .map_err(|_| control_plane::errors::ControlPlaneError::InvalidInput("installation_id"))?;

    let result = HostInfrastructureConfigService::new(state.store.clone())
        .save_provider_config(SaveHostInfrastructureProviderConfigCommand {
            actor_user_id: context.user.id,
            installation_id,
            provider_code,
            enabled_contracts: body.enabled_contracts,
            config_json: body.config_json,
        })
        .await?;

    Ok(Json(ApiSuccess::new(
        SaveHostInfrastructureProviderConfigResponse {
            restart_required: result.restart_required,
            installation_desired_state: result.installation_desired_state,
            provider_config_status: result.provider_config_status,
        },
    )))
}

enum MemoryInspectionTarget {
    Session(Arc<dyn SessionStore>),
    Cache(Arc<dyn CacheStore>),
    RateLimit(Arc<dyn RateLimitStore>),
    Lock(Arc<dyn DistributedLock>),
    TaskQueue(Arc<dyn TaskQueue>),
    EventBus(Arc<dyn EventBus>),
    RuntimeEvents(Arc<dyn RuntimeEventStream>),
    Unsupported,
}

impl MemoryInspectionTarget {
    fn capabilities(&self) -> EphemeralInspectionCapabilities {
        match self {
            Self::Session(store) => store.ephemeral_inspection_capabilities(),
            Self::Cache(store) => store.ephemeral_inspection_capabilities(),
            Self::RateLimit(store) => store.ephemeral_inspection_capabilities(),
            Self::Lock(store) => store.ephemeral_inspection_capabilities(),
            Self::TaskQueue(store) => store.ephemeral_inspection_capabilities(),
            Self::EventBus(store) => store.ephemeral_inspection_capabilities(),
            Self::RuntimeEvents(stream) => stream.ephemeral_inspection_capabilities(),
            Self::Unsupported => EphemeralInspectionCapabilities::unsupported(),
        }
    }

    async fn summarize_entries_at_path(
        &self,
        inspection_path: &[String],
    ) -> anyhow::Result<EphemeralInspectionSummarySnapshot> {
        let entries = match self {
            Self::Session(store) => store.list_ephemeral_entries().await?,
            Self::Cache(store) => store.list_ephemeral_entries().await?,
            Self::RateLimit(store) => store.list_ephemeral_entries().await?,
            Self::Lock(store) => store.list_ephemeral_entries().await?,
            Self::TaskQueue(store) => store.list_ephemeral_entries().await?,
            Self::EventBus(store) => store.list_ephemeral_entries().await?,
            Self::RuntimeEvents(stream) => stream.list_ephemeral_entries().await?,
            Self::Unsupported => Vec::new(),
        };

        Ok(summarize_memory_entries_at_path(entries, inspection_path))
    }

    async fn list_tree(
        &self,
        request: EphemeralInspectionPageRequest,
    ) -> anyhow::Result<EphemeralInspectionTreePage> {
        match self {
            Self::Session(store) => store.list_ephemeral_tree(request).await,
            Self::Cache(store) => store.list_ephemeral_tree(request).await,
            Self::RateLimit(store) => store.list_ephemeral_tree(request).await,
            Self::Lock(store) => store.list_ephemeral_tree(request).await,
            Self::TaskQueue(store) => store.list_ephemeral_tree(request).await,
            Self::EventBus(store) => store.list_ephemeral_tree(request).await,
            Self::RuntimeEvents(stream) => stream.list_ephemeral_tree(request).await,
            Self::Unsupported => Ok(empty_memory_tree_page(request)),
        }
    }

    async fn list_entry_page(
        &self,
        request: EphemeralInspectionPageRequest,
    ) -> anyhow::Result<EphemeralInspectionEntryPage> {
        match self {
            Self::Session(store) => store.list_ephemeral_entry_page(request).await,
            Self::Cache(store) => store.list_ephemeral_entry_page(request).await,
            Self::RateLimit(store) => store.list_ephemeral_entry_page(request).await,
            Self::Lock(store) => store.list_ephemeral_entry_page(request).await,
            Self::TaskQueue(store) => store.list_ephemeral_entry_page(request).await,
            Self::EventBus(store) => store.list_ephemeral_entry_page(request).await,
            Self::RuntimeEvents(stream) => stream.list_ephemeral_entry_page(request).await,
            Self::Unsupported => Ok(empty_memory_entry_page(request)),
        }
    }

    async fn search_entry_page(
        &self,
        query: &str,
        request: EphemeralInspectionPageRequest,
    ) -> anyhow::Result<EphemeralInspectionEntryPage> {
        match self {
            Self::Session(store) => store.search_ephemeral_entry_page(query, request).await,
            Self::Cache(store) => store.search_ephemeral_entry_page(query, request).await,
            Self::RateLimit(store) => store.search_ephemeral_entry_page(query, request).await,
            Self::Lock(store) => store.search_ephemeral_entry_page(query, request).await,
            Self::TaskQueue(store) => store.search_ephemeral_entry_page(query, request).await,
            Self::EventBus(store) => store.search_ephemeral_entry_page(query, request).await,
            Self::RuntimeEvents(stream) => stream.search_ephemeral_entry_page(query, request).await,
            Self::Unsupported => Ok(empty_memory_entry_page(request)),
        }
    }

    async fn reveal_entry(
        &self,
        entry_ref: &str,
        reveal_mode: EphemeralValueRevealMode,
    ) -> anyhow::Result<Option<EphemeralEntryValueSnapshot>> {
        match self {
            Self::Session(store) => store.reveal_ephemeral_entry(entry_ref, reveal_mode).await,
            Self::Cache(store) => store.reveal_ephemeral_entry(entry_ref, reveal_mode).await,
            Self::RateLimit(store) => store.reveal_ephemeral_entry(entry_ref, reveal_mode).await,
            Self::Lock(store) => store.reveal_ephemeral_entry(entry_ref, reveal_mode).await,
            Self::TaskQueue(store) => store.reveal_ephemeral_entry(entry_ref, reveal_mode).await,
            Self::EventBus(store) => store.reveal_ephemeral_entry(entry_ref, reveal_mode).await,
            Self::RuntimeEvents(stream) => {
                stream.reveal_ephemeral_entry(entry_ref, reveal_mode).await
            }
            Self::Unsupported => Ok(None),
        }
    }
}

const MEMORY_CONTRACTS: &[(&str, &str)] = &[
    ("session-store", "Sessions"),
    ("cache-store", "Cache"),
    ("rate-limit-store", "Rate Limits"),
    ("distributed-lock", "Locks"),
    ("task-queue", "Task Queue"),
    ("event-bus", "Event Bus"),
    ("runtime-event-stream", "Runtime Events"),
];

fn memory_contract_definitions() -> &'static [(&'static str, &'static str)] {
    MEMORY_CONTRACTS
}

fn memory_contract_label(contract_code: &str) -> Result<&'static str, ApiError> {
    MEMORY_CONTRACTS
        .iter()
        .find_map(|(candidate, label)| (*candidate == contract_code).then_some(*label))
        .ok_or(ControlPlaneError::NotFound("memory_contract").into())
}

fn memory_inspection_target(
    state: &ApiState,
    contract_code: &str,
) -> Result<MemoryInspectionTarget, ApiError> {
    match contract_code {
        "session-store" => Ok(state
            .infrastructure
            .session_store()
            .map(MemoryInspectionTarget::Session)
            .unwrap_or(MemoryInspectionTarget::Unsupported)),
        "cache-store" => Ok(state
            .infrastructure
            .registered_cache_store()
            .map(MemoryInspectionTarget::Cache)
            .unwrap_or(MemoryInspectionTarget::Unsupported)),
        "rate-limit-store" => Ok(state
            .infrastructure
            .registered_rate_limit_store()
            .map(MemoryInspectionTarget::RateLimit)
            .unwrap_or(MemoryInspectionTarget::Unsupported)),
        "distributed-lock" => Ok(state
            .infrastructure
            .registered_distributed_lock()
            .map(MemoryInspectionTarget::Lock)
            .unwrap_or(MemoryInspectionTarget::Unsupported)),
        "task-queue" => Ok(state
            .infrastructure
            .registered_task_queue()
            .map(MemoryInspectionTarget::TaskQueue)
            .unwrap_or(MemoryInspectionTarget::Unsupported)),
        "event-bus" => Ok(state
            .infrastructure
            .registered_event_bus()
            .map(MemoryInspectionTarget::EventBus)
            .unwrap_or(MemoryInspectionTarget::Unsupported)),
        "runtime-event-stream" => Ok(state
            .infrastructure
            .runtime_event_stream()
            .map(MemoryInspectionTarget::RuntimeEvents)
            .unwrap_or(MemoryInspectionTarget::Unsupported)),
        _ => Err(ControlPlaneError::NotFound("memory_contract").into()),
    }
}

fn memory_contract_supported(capabilities: EphemeralInspectionCapabilities) -> bool {
    capabilities.list_entries
        || capabilities.list_tree
        || capabilities.search_entries
        || capabilities.reveal_value
}

fn memory_page_request(
    path: Option<String>,
    cursor: Option<String>,
    limit: Option<usize>,
    byte_limit: Option<usize>,
) -> EphemeralInspectionPageRequest {
    EphemeralInspectionPageRequest::new(
        memory_query_path(path),
        cursor.filter(|value| !value.is_empty()),
        limit,
        byte_limit,
    )
}

fn memory_query_path(path: Option<String>) -> Vec<String> {
    path.unwrap_or_default()
        .split('/')
        .filter(|segment| !segment.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn memory_path_has_prefix(path: &[String], prefix: &[String]) -> bool {
    path.len() >= prefix.len()
        && path
            .iter()
            .zip(prefix.iter())
            .all(|(left, right)| left == right)
}

fn summarize_memory_entries_at_path(
    entries: Vec<EphemeralEntrySnapshot>,
    inspection_path: &[String],
) -> EphemeralInspectionSummarySnapshot {
    let mut summary = EphemeralInspectionSummarySnapshot::empty();

    for entry in entries
        .into_iter()
        .filter(|entry| memory_path_has_prefix(&entry.inspection_path, inspection_path))
    {
        summary.entry_count += 1;
        summary.sensitive_entry_count += u64::from(entry.sensitive);
        summary.total_value_size_bytes += entry.value_size_bytes;
    }

    summary
}

fn empty_memory_entry_page(
    request: EphemeralInspectionPageRequest,
) -> EphemeralInspectionEntryPage {
    EphemeralInspectionEntryPage {
        inspection_path: request.inspection_path,
        entries: Vec::new(),
        next_cursor: None,
        limit: request.limit as u64,
        byte_limit: request.byte_limit as u64,
        emitted_bytes: 0,
        truncated_by_byte_limit: false,
    }
}

fn empty_memory_tree_page(request: EphemeralInspectionPageRequest) -> EphemeralInspectionTreePage {
    EphemeralInspectionTreePage {
        inspection_path: request.inspection_path,
        nodes: Vec::new(),
        next_cursor: None,
        limit: request.limit as u64,
        byte_limit: request.byte_limit as u64,
        emitted_bytes: 0,
        truncated_by_byte_limit: false,
    }
}

fn parse_memory_reveal_mode(
    reveal_mode: Option<&str>,
) -> Result<EphemeralValueRevealMode, ApiError> {
    match reveal_mode.unwrap_or("preview") {
        "metadata" => Ok(EphemeralValueRevealMode::Metadata),
        "preview" => Ok(EphemeralValueRevealMode::Preview),
        "full" => Ok(EphemeralValueRevealMode::Full),
        _ => Err(ControlPlaneError::InvalidInput("memory_reveal_mode").into()),
    }
}

fn format_memory_reveal_mode(reveal_mode: EphemeralValueRevealMode) -> String {
    match reveal_mode {
        EphemeralValueRevealMode::Metadata => "metadata",
        EphemeralValueRevealMode::Preview => "preview",
        EphemeralValueRevealMode::Full => "full",
    }
    .to_string()
}

fn format_memory_value_state(
    value_state: control_plane::ports::EphemeralEntryValueState,
) -> String {
    match value_state {
        control_plane::ports::EphemeralEntryValueState::Hidden => "hidden",
        control_plane::ports::EphemeralEntryValueState::Available => "available",
        control_plane::ports::EphemeralEntryValueState::Preview => "preview",
        control_plane::ports::EphemeralEntryValueState::ValueTooLarge => "value_too_large",
    }
    .to_string()
}

async fn memory_contract_summary(
    state: &ApiState,
    contract_code: &str,
    label: &str,
) -> Result<MemoryContractSummaryResponse, ApiError> {
    let target = memory_inspection_target(state, contract_code)?;
    let capabilities = target.capabilities();

    Ok(MemoryContractSummaryResponse {
        contract_code: contract_code.to_string(),
        label: label.to_string(),
        provider_code: state
            .infrastructure
            .default_provider(contract_code)
            .map(ToString::to_string),
        capabilities: capabilities.into(),
        supported: memory_contract_supported(capabilities),
    })
}

async fn memory_contract_stats_response(
    state: &ApiState,
    contract_code: &str,
    label: &str,
    inspection_path: &[String],
) -> Result<MemoryStatsResponse, ApiError> {
    let target = memory_inspection_target(state, contract_code)?;
    let capabilities = target.capabilities();
    let summary = if memory_contract_supported(capabilities) {
        target.summarize_entries_at_path(inspection_path).await?
    } else {
        EphemeralInspectionSummarySnapshot::empty()
    };
    Ok(MemoryStatsResponse {
        contract_code: contract_code.to_string(),
        label: label.to_string(),
        provider_code: state
            .infrastructure
            .default_provider(contract_code)
            .map(ToString::to_string),
        capabilities: capabilities.into(),
        supported: memory_contract_supported(capabilities),
        inspection_path: inspection_path.to_vec(),
        entry_count: summary.entry_count,
        sensitive_entry_count: summary.sensitive_entry_count,
        total_value_size_bytes: summary.total_value_size_bytes,
    })
}
fn to_provider_response(
    provider: HostInfrastructureProviderConfigView,
) -> HostInfrastructureProviderConfigResponse {
    HostInfrastructureProviderConfigResponse {
        installation_id: provider.installation_id.to_string(),
        extension_id: provider.extension_id,
        provider_code: provider.provider_code,
        display_name: provider.display_name,
        description: provider.description,
        runtime_status: provider.runtime_status,
        desired_state: provider.desired_state,
        config_ref: provider.config_ref,
        contracts: provider.contracts,
        enabled_contracts: provider.enabled_contracts,
        config_schema: provider
            .config_schema
            .into_iter()
            .map(to_plugin_form_field_schema_response)
            .collect(),
        config_json: provider.config_json,
        restart_required: provider.restart_required,
    }
}

fn ensure_memory_view(actor: &domain::ActorContext) -> Result<(), ApiError> {
    ensure_permission(actor, "plugin_config.view.all")
        .map_err(ControlPlaneError::PermissionDenied)?;
    Ok(())
}

fn ensure_memory_manage(actor: &domain::ActorContext) -> Result<(), ApiError> {
    ensure_permission(actor, "plugin_config.configure.all")
        .map_err(ControlPlaneError::PermissionDenied)?;
    Ok(())
}

fn can_manage_memory(actor: &domain::ActorContext) -> bool {
    actor.has_permission("plugin_config.configure.all")
}

async fn append_memory_audit(
    state: &ApiState,
    actor: &domain::ActorContext,
    event_code: &str,
    payload: serde_json::Value,
) -> Result<(), ApiError> {
    let workspace_id = if actor.current_workspace_id == domain::SYSTEM_SCOPE_ID {
        None
    } else {
        Some(actor.current_workspace_id)
    };
    AuthRepository::append_audit_log(
        &state.store,
        &audit_log(
            workspace_id,
            Some(actor.user_id),
            "host_infrastructure_memory",
            None,
            event_code,
            payload,
        ),
    )
    .await?;
    Ok(())
}

fn ensure_cache_view(actor: &domain::ActorContext) -> Result<(), ApiError> {
    ensure_permission(actor, "plugin_config.view.all")
        .map_err(ControlPlaneError::PermissionDenied)?;
    Ok(())
}

fn ensure_cache_manage(actor: &domain::ActorContext) -> Result<(), ApiError> {
    ensure_permission(actor, "plugin_config.configure.all")
        .map_err(ControlPlaneError::PermissionDenied)?;
    Ok(())
}

fn can_manage_cache(actor: &domain::ActorContext) -> bool {
    actor.has_permission("plugin_config.configure.all")
}

async fn append_cache_audit(
    state: &ApiState,
    actor: &domain::ActorContext,
    event_code: &str,
    payload: serde_json::Value,
) -> Result<(), ApiError> {
    let workspace_id = if actor.current_workspace_id == domain::SYSTEM_SCOPE_ID {
        None
    } else {
        Some(actor.current_workspace_id)
    };
    AuthRepository::append_audit_log(
        &state.store,
        &audit_log(
            workspace_id,
            Some(actor.user_id),
            "host_infrastructure_cache",
            None,
            event_code,
            payload,
        ),
    )
    .await?;
    Ok(())
}

impl From<EphemeralInspectionCapabilities> for MemoryInspectionCapabilitiesResponse {
    fn from(capabilities: EphemeralInspectionCapabilities) -> Self {
        Self {
            list_entries: capabilities.list_entries,
            list_tree: capabilities.list_tree,
            search_entries: capabilities.search_entries,
            reveal_value: capabilities.reveal_value,
            default_page_size: capabilities.default_page_size,
            max_page_size: capabilities.max_page_size,
            default_byte_limit: capabilities.default_byte_limit,
            max_byte_limit: capabilities.max_byte_limit,
            default_preview_size_bytes: capabilities.default_preview_size_bytes,
            max_full_value_size_bytes: capabilities.max_full_value_size_bytes,
            max_value_size_bytes: capabilities.max_value_size_bytes,
            max_payload_size_bytes: capabilities.max_payload_size_bytes,
        }
    }
}

fn to_memory_entry_metadata_response(entry: EphemeralEntrySnapshot) -> MemoryEntryMetadataResponse {
    MemoryEntryMetadataResponse {
        contract_code: entry.contract_code,
        group_code: entry.group_code,
        entry_ref: entry.entry_ref,
        key: entry.key,
        inspection_path: entry.inspection_path,
        entry_kind: entry.entry_kind,
        status: entry.status,
        owner: entry.owner,
        value_size_bytes: entry.value_size_bytes,
        metadata_size_bytes: entry.metadata_size_bytes,
        ttl_seconds: entry.ttl_seconds,
        created_at_unix: entry.created_at_unix,
        expires_at_unix: entry.expires_at_unix,
        sensitive: entry.sensitive,
        metadata: entry.metadata,
    }
}

fn to_memory_tree_node_response(
    node: EphemeralInspectionTreeNodeSnapshot,
) -> MemoryTreeNodeResponse {
    MemoryTreeNodeResponse {
        node_ref: node.node_ref,
        label: node.label,
        inspection_path: node.inspection_path,
        depth: node.depth,
        has_children: node.has_children,
    }
}

impl From<CacheInspectionCapabilities> for CacheInspectionCapabilitiesResponse {
    fn from(capabilities: CacheInspectionCapabilities) -> Self {
        Self {
            list_domains: capabilities.list_domains,
            list_entries: capabilities.list_entries,
            reveal_value: capabilities.reveal_value,
            clear_entry: capabilities.clear_entry,
            clear_domain: capabilities.clear_domain,
        }
    }
}

fn to_cache_domain_response(domain: CacheDomainSnapshot) -> CacheDomainResponse {
    CacheDomainResponse {
        domain_code: domain.domain_code,
        entry_count: domain.entry_count,
        total_value_size_bytes: domain.total_value_size_bytes,
    }
}

fn to_cache_entry_metadata_response(entry: CacheEntrySnapshot) -> CacheEntryMetadataResponse {
    CacheEntryMetadataResponse {
        domain_code: entry.domain_code,
        key: entry.key,
        value_size_bytes: entry.value_size_bytes,
        ttl_seconds: entry.ttl_seconds,
        created_at_unix: entry.created_at_unix,
        expires_at_unix: entry.expires_at_unix,
    }
}

fn to_plugin_form_option_response(option: PluginFormOption) -> PluginFormOptionResponse {
    PluginFormOptionResponse {
        label: option.label,
        value: option.value,
        description: option.description,
        disabled: option.disabled,
    }
}

fn to_plugin_form_condition_response(
    condition: PluginFormCondition,
) -> PluginFormConditionResponse {
    PluginFormConditionResponse {
        field: condition.field,
        operator: condition.operator,
        value: condition.value,
        values: condition.values,
    }
}

fn to_plugin_form_field_schema_response(
    field: PluginFormFieldSchema,
) -> PluginFormFieldSchemaResponse {
    PluginFormFieldSchemaResponse {
        key: field.key,
        label: field.label,
        field_type: field.field_type,
        control: field.control,
        group: field.group,
        order: field.order,
        advanced: field.advanced,
        required: field.required,
        send_mode: field.send_mode,
        enabled_by_default: field.enabled_by_default,
        description: field.description,
        placeholder: field.placeholder,
        default_value: field.default_value,
        min: field.min,
        max: field.max,
        step: field.step,
        precision: field.precision,
        unit: field.unit,
        options: field
            .options
            .into_iter()
            .map(to_plugin_form_option_response)
            .collect(),
        visible_when: field
            .visible_when
            .into_iter()
            .map(to_plugin_form_condition_response)
            .collect(),
        disabled_when: field
            .disabled_when
            .into_iter()
            .map(to_plugin_form_condition_response)
            .collect(),
    }
}
