use std::{collections::HashSet, convert::Infallible, future::Future, sync::Arc};

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::sse::{Event, KeepAlive, Sse},
    routing::{get, post, put},
    Json, Router,
};
use control_plane::{
    application::ApplicationService,
    errors::ControlPlaneError,
    orchestration_runtime::{
        debug_stream_events, fail_runtime_event_stream_if_missing_terminal,
        spawn_runtime_debug_event_persister, wait_for_runtime_debug_event_persister,
        CancelFlowRunCommand, CompleteCallbackTaskCommand, ContinueFlowDebugRunCommand,
        OrchestrationRuntimeService, PrepareFlowDebugRunCommand, ResumeFlowRunCommand,
        StartFlowDebugRunCommand, StartNodeDebugPreviewCommand,
    },
    ports::{
        ListApplicationConversationRunsPageInput, OrchestrationRuntimeRepository,
        RuntimeEventStreamPolicy,
    },
};
use serde::{Deserialize, Serialize};
use storage_durable::MainDurableStore;
use time::{format_description::well_known::Rfc3339, Duration, OffsetDateTime};
use tokio::sync::mpsc;
use tokio_stream::StreamExt;
use tracing::error;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    app_state::ApiState,
    error_response::ApiError,
    middleware::{require_csrf::require_csrf, require_session::require_session},
    provider_runtime::ApiProviderRuntime,
    response::ApiSuccess,
    runtime_activity::{scope_application_activity, ApplicationActivityKind},
};

use super::debug_run_stream;
mod application_log_cache;
mod application_logs;
pub(crate) mod application_monitoring;
pub(crate) mod debug_variable_cache;
pub(crate) mod debug_variable_snapshot;
mod runtime_debug_artifacts;

pub use debug_variable_cache::{
    delete_debug_variable_cache_entries, upsert_debug_variable_cache_entry,
};
pub use debug_variable_snapshot::{get_debug_variable_snapshot, DebugVariableSnapshotResponse};
use runtime_debug_artifacts::{
    application_run_answer, application_run_model, application_run_query,
    load_runtime_debug_artifact_json_value, load_runtime_debug_artifact_response,
    offload_application_run_detail_artifacts,
};

fn api_provider_runtime(state: &ApiState) -> ApiProviderRuntime {
    ApiProviderRuntime::new_with_activity(
        state.provider_runtime.clone(),
        state.runtime_activity.clone(),
    )
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct StartNodeDebugPreviewBody {
    pub input_payload: serde_json::Value,
    pub document: Option<serde_json::Value>,
    pub debug_session_id: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct StartFlowDebugRunBody {
    pub input_payload: serde_json::Value,
    pub document: Option<serde_json::Value>,
    pub debug_session_id: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct DebugRunStreamQuery {
    pub from_sequence: Option<i64>,
    pub last_event_id: Option<String>,
}

#[derive(Debug, Deserialize, Default, ToSchema)]
pub struct ApplicationRunsQuery {
    pub page: Option<i64>,
    pub page_size: Option<i64>,
    pub time_range_days: Option<i64>,
    pub sort_by: Option<String>,
    pub sort_order: Option<String>,
    pub cache_mode: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ResumeFlowRunBody {
    pub checkpoint_id: String,
    pub input_payload: serde_json::Value,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CompleteCallbackTaskBody {
    pub response_payload: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct FlowRunSummaryResponse {
    pub id: String,
    pub application_id: String,
    pub application_type: String,
    pub run_object_kind: String,
    pub run_kind: String,
    pub run_mode: String,
    pub status: String,
    pub target_node_id: Option<String>,
    pub title: String,
    pub expand_id: Option<String>,
    pub authorized_account: Option<String>,
    pub source: String,
    pub compatibility_mode: Option<String>,
    pub subject: application_logs::ApplicationRunSubjectResponse,
    pub actor: application_logs::ApplicationRunActorResponse,
    pub correlation: application_logs::ApplicationRunCorrelationResponse,
    pub statistics: application_logs::ApplicationRunStatisticsResponse,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct FlowRunSummaryPageResponse {
    pub items: Vec<FlowRunSummaryResponse>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct FlowRunResponse {
    pub id: String,
    pub application_id: String,
    pub flow_id: String,
    pub draft_id: String,
    pub compiled_plan_id: Option<String>,
    pub run_mode: String,
    pub status: String,
    pub target_node_id: Option<String>,
    pub title: String,
    pub expand_id: Option<String>,
    pub authorized_account: Option<String>,
    pub external_conversation_id: Option<String>,
    pub query: Option<String>,
    pub model: Option<String>,
    pub input_payload: serde_json::Value,
    pub output_payload: serde_json::Value,
    pub error_payload: Option<serde_json::Value>,
    pub created_by: String,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ApplicationConversationMessagesQuery {
    pub around_run_id: Option<Uuid>,
    pub before: Option<String>,
    pub after: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ApplicationConversationMessageResponse {
    pub run_id: String,
    pub detail_run_id: Option<String>,
    pub can_open_detail: bool,
    pub role: Option<String>,
    pub content: Option<String>,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub status: String,
    pub query: Option<String>,
    pub model: Option<String>,
    pub answer: Option<String>,
    pub is_current: bool,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ApplicationConversationMessagesPageInfoResponse {
    pub has_before: bool,
    pub has_after: bool,
    pub before_cursor: Option<String>,
    pub after_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ApplicationConversationMessagesPageResponse {
    pub items: Vec<ApplicationConversationMessageResponse>,
    pub page: ApplicationConversationMessagesPageInfoResponse,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct NodeRunResponse {
    pub id: String,
    pub flow_run_id: String,
    pub node_id: String,
    pub node_type: String,
    pub node_alias: String,
    pub status: String,
    pub input_payload: serde_json::Value,
    pub input_payload_view: serde_json::Value,
    pub output_payload: serde_json::Value,
    pub error_payload: Option<serde_json::Value>,
    pub metrics_payload: serde_json::Value,
    pub debug_payload: serde_json::Value,
    pub started_at: String,
    pub finished_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct CheckpointResponse {
    pub id: String,
    pub flow_run_id: String,
    pub node_run_id: Option<String>,
    pub status: String,
    pub reason: String,
    pub locator_payload: serde_json::Value,
    pub variable_snapshot: serde_json::Value,
    pub external_ref_payload: Option<serde_json::Value>,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct CallbackTaskResponse {
    pub id: String,
    pub flow_run_id: String,
    pub node_run_id: String,
    pub callback_kind: String,
    pub status: String,
    pub request_payload: serde_json::Value,
    pub response_payload: Option<serde_json::Value>,
    pub external_ref_payload: Option<serde_json::Value>,
    pub created_at: String,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct RunEventResponse {
    pub id: String,
    pub flow_run_id: String,
    pub node_run_id: Option<String>,
    pub sequence: i64,
    pub event_type: String,
    pub payload: serde_json::Value,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct AnswerSnapshotResponse {
    pub kind: String,
    pub text: String,
    pub output_payload: serde_json::Value,
    pub complete: bool,
    pub materialized_from: String,
    pub answer_node_id: String,
    pub answer_node_run_id: String,
    pub waiting_node_id: Option<String>,
    pub waiting_node_run_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct ApplicationRunDetailResponse {
    pub run: application_logs::ApplicationRunLogResponse,
    pub statistics: application_logs::ApplicationRunStatisticsResponse,
    pub detail: application_logs::ApplicationRunTypedDetailResponse,
    pub flow_run: FlowRunResponse,
    pub answer_snapshot: Option<AnswerSnapshotResponse>,
    pub node_runs: Vec<NodeRunResponse>,
    pub checkpoints: Vec<CheckpointResponse>,
    pub callback_tasks: Vec<CallbackTaskResponse>,
    pub events: Vec<RunEventResponse>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RuntimeDebugStreamResponse {
    pub parts: Vec<RuntimeDebugStreamPartResponse>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RuntimeDebugStreamPartResponse {
    pub id: String,
    pub flow_run_id: String,
    pub item_id: Option<String>,
    pub span_id: Option<String>,
    pub part_type: String,
    pub status: String,
    pub trust_level: String,
    pub payload: serde_json::Value,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct NodeLastRunResponse {
    pub flow_run: FlowRunResponse,
    pub node_run: NodeRunResponse,
    pub checkpoints: Vec<CheckpointResponse>,
    pub events: Vec<RunEventResponse>,
}

pub fn router() -> Router<Arc<ApiState>> {
    Router::new()
        .route(
            "/applications/:id/orchestration/debug-runs",
            post(start_flow_debug_run),
        )
        .route(
            "/applications/:id/orchestration/debug-runs/stream",
            post(start_flow_debug_run_stream),
        )
        .route(
            "/applications/:id/orchestration/runs/:run_id/debug-stream",
            get(subscribe_flow_debug_run_stream),
        )
        .route(
            "/applications/:id/orchestration/runs/:run_id/resume",
            post(resume_flow_run),
        )
        .route(
            "/applications/:id/orchestration/runs/:run_id/cancel",
            post(cancel_flow_run),
        )
        .route(
            "/applications/:id/orchestration/callback-tasks/:callback_task_id/complete",
            post(complete_callback_task),
        )
        .route(
            "/applications/:id/orchestration/nodes/:node_id/debug-runs",
            post(start_node_debug_preview),
        )
        .route(
            "/applications/:id/orchestration/debug-variable-snapshot",
            get(get_debug_variable_snapshot),
        )
        .route(
            "/applications/:id/orchestration/debug-variable-cache",
            put(upsert_debug_variable_cache_entry).delete(delete_debug_variable_cache_entries),
        )
        .route(
            "/applications/:id/orchestration/debug-artifacts/:artifact_id",
            get(get_runtime_debug_artifact),
        )
        .route("/applications/:id/logs/runs", get(list_application_runs))
        .route(
            "/applications/:id/monitoring/run-metrics",
            get(application_monitoring::get_application_run_monitoring_report),
        )
        .route(
            "/applications/:id/monitoring/runtime-activity",
            get(application_monitoring::get_application_runtime_activity),
        )
        .route(
            "/applications/:id/logs/conversations/:conversation_id/messages",
            get(list_application_conversation_messages),
        )
        .route(
            "/applications/:id/logs/runs/:run_id/conversation/messages",
            get(list_application_run_conversation_messages),
        )
        .route(
            "/applications/:id/logs/runs/:run_id/nodes/:node_id",
            get(get_application_run_node_last_run),
        )
        .route(
            "/applications/:id/logs/runs/:run_id",
            get(get_application_run_detail),
        )
        .route(
            "/applications/:id/logs/runs/:run_id/debug-stream",
            get(get_runtime_debug_stream),
        )
        .route(
            "/applications/:id/orchestration/nodes/:node_id/last-run",
            get(get_node_last_run),
        )
}

fn format_time(value: time::OffsetDateTime) -> String {
    value.format(&Rfc3339).unwrap()
}

fn format_optional_time(value: Option<time::OffsetDateTime>) -> Option<String> {
    value.map(format_time)
}

fn to_flow_run_summary_response(
    application: &domain::ApplicationRecord,
    summary: domain::ApplicationRunSummary,
    statistics: application_logs::ApplicationRunStatisticsResponse,
) -> FlowRunSummaryResponse {
    let application_type = application.application_type.as_str().to_string();
    let run_object_kind = application.sections.logs.run_object_kind.clone();
    let subject = application_logs::ApplicationRunSubjectResponse {
        kind: application_type.clone(),
        id: Some(application.id.to_string()),
        draft_id: None,
        target_node_id: summary.target_node_id.clone(),
    };
    let actor = application_logs::actor_from_console_user(
        summary.user_id.clone(),
        summary.authorized_account.clone(),
    );
    let correlation = application_logs::ApplicationRunCorrelationResponse {
        api_key_id: summary.api_key_id.map(|value| value.to_string()),
        publication_version_id: summary
            .publication_version_id
            .map(|value| value.to_string()),
        external_user: summary.user_id.clone(),
        external_conversation_id: summary.external_conversation_id.clone(),
        external_trace_id: summary.external_trace_id.clone(),
        compatibility_mode: summary.compatibility_mode.clone(),
        idempotency_key: summary.idempotency_key.clone(),
    };

    FlowRunSummaryResponse {
        id: summary.id.to_string(),
        application_id: application.id.to_string(),
        application_type,
        run_object_kind,
        run_kind: summary.run_mode.as_str().to_string(),
        run_mode: summary.run_mode.as_str().to_string(),
        status: summary.status.as_str().to_string(),
        target_node_id: summary.target_node_id,
        title: summary.title,
        expand_id: summary.user_id,
        authorized_account: summary.authorized_account,
        source: application_logs::source_for_run(summary.api_key_id),
        compatibility_mode: summary.compatibility_mode,
        subject,
        actor,
        correlation,
        statistics,
        started_at: format_time(summary.started_at),
        finished_at: format_optional_time(summary.finished_at),
        created_at: format_time(summary.created_at),
        updated_at: format_time(summary.updated_at),
    }
}

fn usage_token_value(value: &serde_json::Value) -> Option<i64> {
    value
        .as_i64()
        .or_else(|| value.as_u64().and_then(|value| i64::try_from(value).ok()))
}

fn usage_total_tokens(usage: &serde_json::Value) -> Option<i64> {
    if let Some(total_tokens) = usage.get("total_tokens").and_then(usage_token_value) {
        return Some(total_tokens);
    }

    let segments = ["input_tokens", "output_tokens", "reasoning_tokens"];
    let mut total = 0_i64;
    let mut has_segment = false;

    for segment in segments {
        if let Some(tokens) = usage.get(segment).and_then(usage_token_value) {
            total += tokens;
            has_segment = true;
        }
    }

    has_segment.then_some(total)
}

fn metrics_payload_total_tokens(metrics_payload: &serde_json::Value) -> Option<i64> {
    metrics_payload.get("usage").and_then(usage_total_tokens)
}

fn callback_task_tool_callback_count(task: &domain::CallbackTaskRecord) -> i64 {
    if task.callback_kind != "llm_tool_calls" {
        return 0;
    }

    task.request_payload
        .get("tool_calls")
        .and_then(serde_json::Value::as_array)
        .map(|tool_calls| tool_calls.len() as i64)
        .or_else(|| {
            task.request_payload
                .get("tool_calls")
                .and_then(|value| value.get("tool_call_count"))
                .and_then(serde_json::Value::as_i64)
        })
        .unwrap_or(0)
}

fn application_run_statistics(
    detail: &domain::ApplicationRunDetail,
) -> application_logs::ApplicationRunStatisticsResponse {
    let mut unique_node_ids = HashSet::new();
    let mut total_tokens = None;

    for node_run in &detail.node_runs {
        unique_node_ids.insert(node_run.node_id.as_str());

        if let Some(node_tokens) = metrics_payload_total_tokens(&node_run.metrics_payload) {
            total_tokens = Some(total_tokens.unwrap_or(0) + node_tokens);
        }
    }

    application_logs::ApplicationRunStatisticsResponse {
        total_tokens,
        unique_node_count: unique_node_ids.len() as i64,
        tool_callback_count: detail
            .callback_tasks
            .iter()
            .map(callback_task_tool_callback_count)
            .sum(),
    }
}

fn application_runs_created_after(query: &ApplicationRunsQuery) -> Option<OffsetDateTime> {
    let days = query.time_range_days?;

    if days <= 0 {
        return None;
    }

    Some(OffsetDateTime::now_utc() - Duration::days(days))
}

fn normalize_application_run_sort_by(input: Option<&str>) -> &'static str {
    match input.unwrap_or("created_at") {
        "created_at" => "created_at",
        "started_at" => "started_at",
        "finished_at" => "finished_at",
        "updated_at" => "updated_at",
        _ => "created_at",
    }
}

fn normalize_application_run_sort_order(input: Option<&str>) -> &'static str {
    match input.unwrap_or("desc").to_ascii_lowercase().as_str() {
        "asc" => "asc",
        _ => "desc",
    }
}

fn should_refresh_application_run_logs(cache_mode: Option<&str>) -> bool {
    matches!(cache_mode, Some("refresh"))
}

fn to_flow_run_response(run: domain::FlowRunRecord) -> FlowRunResponse {
    FlowRunResponse {
        id: run.id.to_string(),
        application_id: run.application_id.to_string(),
        flow_id: run.flow_id.to_string(),
        draft_id: run.draft_id.to_string(),
        compiled_plan_id: run.compiled_plan_id.map(|value| value.to_string()),
        run_mode: run.run_mode.as_str().to_string(),
        status: run.status.as_str().to_string(),
        target_node_id: run.target_node_id,
        title: run.title,
        expand_id: run.external_user,
        authorized_account: run.authorized_account,
        external_conversation_id: run.external_conversation_id,
        query: application_run_query(&run.input_payload),
        model: application_run_model(&run.input_payload),
        input_payload: run.input_payload,
        output_payload: run.output_payload,
        error_payload: run.error_payload,
        created_by: run.created_by.to_string(),
        started_at: format_time(run.started_at),
        finished_at: format_optional_time(run.finished_at),
        created_at: format_time(run.created_at),
        updated_at: format_time(run.updated_at),
    }
}

fn to_application_conversation_message_response(
    run: domain::FlowRunRecord,
    current_run_id: Option<Uuid>,
) -> ApplicationConversationMessageResponse {
    let run_id = run.id.to_string();

    ApplicationConversationMessageResponse {
        run_id: run_id.clone(),
        detail_run_id: Some(run_id),
        can_open_detail: true,
        role: None,
        content: None,
        started_at: format_time(run.started_at),
        finished_at: format_optional_time(run.finished_at),
        status: run.status.as_str().to_string(),
        query: application_run_query(&run.input_payload),
        model: application_run_model(&run.input_payload),
        answer: application_run_answer(&run.output_payload)
            .or_else(|| run.error_payload.as_ref().and_then(application_run_answer)),
        is_current: current_run_id == Some(run.id),
    }
}

fn parse_optional_uuid_cursor(value: Option<&str>) -> Option<Uuid> {
    value.and_then(|value| Uuid::parse_str(value).ok())
}

async fn conversation_messages_from_single_run<F, Fut>(
    run: &domain::FlowRunRecord,
    query: &ApplicationConversationMessagesQuery,
    load_debug_artifact: F,
) -> ApplicationConversationMessagesPageResponse
where
    F: Fn(Uuid) -> Fut,
    Fut: Future<Output = Option<serde_json::Value>>,
{
    let limit = query.limit.unwrap_or(5).clamp(1, 50) as usize;
    let mut items = imported_context_messages_from_run(run, load_debug_artifact).await;
    let system_items = if query.before.is_none() && query.after.is_none() {
        items
            .iter()
            .filter(|item| item.role.as_deref() == Some("system"))
            .cloned()
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    items.retain(|item| item.role.as_deref() != Some("system"));
    items.push(to_application_conversation_message_response(
        run.clone(),
        Some(run.id),
    ));

    let total = items.len();
    let (start, end) = imported_context_window(run.id, total, limit, query);

    ApplicationConversationMessagesPageResponse {
        items: system_items
            .into_iter()
            .chain(
                items
                    .into_iter()
                    .skip(start)
                    .take(end.saturating_sub(start)),
            )
            .collect(),
        page: ApplicationConversationMessagesPageInfoResponse {
            has_before: start > 0,
            has_after: end < total,
            before_cursor: (start > 0).then(|| imported_context_cursor(run.id, start)),
            after_cursor: (end < total).then(|| imported_context_cursor(run.id, end - 1)),
        },
    }
}

async fn imported_context_messages_from_run<F, Fut>(
    run: &domain::FlowRunRecord,
    load_debug_artifact: F,
) -> Vec<ApplicationConversationMessageResponse>
where
    F: Fn(Uuid) -> Fut,
    Fut: Future<Output = Option<serde_json::Value>>,
{
    let source = resolve_runtime_debug_artifact_value(&run.input_payload, &load_debug_artifact)
        .await
        .unwrap_or_else(|| run.input_payload.clone());
    let start_payload = start_input_payload(&source);
    let mut items = Vec::new();

    if let Some(system) = run_level_system_content(&source, &load_debug_artifact).await {
        items.push(imported_context_item(run, items.len(), "system", system));
    }

    let Some(history_value) = start_payload
        .get("history")
        .or_else(|| start_payload.get("messages"))
    else {
        return items;
    };
    let history_source =
        match resolve_runtime_debug_artifact_value(history_value, &load_debug_artifact).await {
            Some(value) => value,
            None => history_value.clone(),
        };
    let Some(history) = history_source.as_array() else {
        return items;
    };

    for message in history {
        let role = message
            .get("role")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();
        let Some(content) = conversation_message_content(message) else {
            continue;
        };

        match role {
            "system"
                if !items
                    .iter()
                    .any(|item| item.role.as_deref() == Some("system")) =>
            {
                items.push(imported_context_item(run, items.len(), role, content))
            }
            "user" | "assistant" => {
                items.push(imported_context_item(run, items.len(), role, content))
            }
            _ => {}
        }
    }

    items
}

async fn resolve_runtime_debug_artifact_value<F, Fut>(
    value: &serde_json::Value,
    load_debug_artifact: &F,
) -> Option<serde_json::Value>
where
    F: Fn(Uuid) -> Fut,
    Fut: Future<Output = Option<serde_json::Value>>,
{
    let artifact_id = runtime_debug_artifact_id(value)?;

    load_debug_artifact(artifact_id)
        .await
        .or_else(|| decode_runtime_debug_artifact_preview(value))
}

fn runtime_debug_artifact_id(value: &serde_json::Value) -> Option<Uuid> {
    if !value
        .get("__runtime_debug_artifact")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
    {
        return None;
    }

    value
        .get("artifact_ref")
        .and_then(serde_json::Value::as_str)
        .and_then(|value| Uuid::parse_str(value).ok())
}

fn imported_context_window(
    run_id: Uuid,
    total: usize,
    limit: usize,
    query: &ApplicationConversationMessagesQuery,
) -> (usize, usize) {
    if total == 0 {
        return (0, 0);
    }

    if let Some(before) = query
        .before
        .as_deref()
        .and_then(|cursor| parse_imported_context_cursor(run_id, cursor))
    {
        let end = before.min(total);
        return (end.saturating_sub(limit), end);
    }

    if let Some(after) = query
        .after
        .as_deref()
        .and_then(|cursor| parse_imported_context_cursor(run_id, cursor))
    {
        let start = (after + 1).min(total);
        return (start, (start + limit).min(total));
    }

    (total.saturating_sub(limit), total)
}

fn imported_context_cursor(run_id: Uuid, index: usize) -> String {
    format!("{run_id}:context:{index}")
}

fn parse_imported_context_cursor(run_id: Uuid, cursor: &str) -> Option<usize> {
    let (prefix, index) = cursor.rsplit_once(":context:")?;
    if prefix != run_id.to_string() {
        return None;
    }

    index.parse().ok()
}

fn imported_context_item(
    run: &domain::FlowRunRecord,
    index: usize,
    role: &str,
    content: String,
) -> ApplicationConversationMessageResponse {
    ApplicationConversationMessageResponse {
        run_id: imported_context_cursor(run.id, index),
        detail_run_id: None,
        can_open_detail: false,
        role: Some(role.to_string()),
        content: Some(content),
        started_at: format_time(run.started_at),
        finished_at: format_optional_time(run.finished_at),
        status: "succeeded".to_string(),
        query: None,
        model: application_run_model(&run.input_payload),
        answer: None,
        is_current: false,
    }
}

async fn run_level_system_content<F, Fut>(
    payload: &serde_json::Value,
    load_debug_artifact: &F,
) -> Option<String>
where
    F: Fn(Uuid) -> Fut,
    Fut: Future<Output = Option<serde_json::Value>>,
{
    let start_payload = start_input_payload(payload);
    let system_value = start_payload
        .get("system")
        .or_else(|| payload.get("system"))?;
    let resolved_system =
        resolve_runtime_debug_artifact_value(system_value, load_debug_artifact).await;

    resolved_system
        .as_ref()
        .and_then(conversation_prompt_text)
        .or_else(|| conversation_prompt_text(system_value))
}

fn conversation_prompt_text(value: &serde_json::Value) -> Option<String> {
    if let Some(text) = value
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return Some(text.to_string());
    }

    if let Some(array) = value.as_array() {
        let text = array
            .iter()
            .filter_map(conversation_content_part_text)
            .collect::<Vec<_>>()
            .join("");
        return (!text.is_empty()).then_some(text);
    }

    conversation_content_part_text(value).or_else(|| conversation_preview_text(value))
}

fn conversation_preview_text(value: &serde_json::Value) -> Option<String> {
    let preview = value
        .get("preview")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())?;

    serde_json::from_str::<serde_json::Value>(preview)
        .ok()
        .as_ref()
        .and_then(conversation_prompt_text)
        .or_else(|| Some(preview.to_string()))
}

fn conversation_message_content(message: &serde_json::Value) -> Option<String> {
    let content = message.get("content")?;
    if let Some(text) = content
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return Some(text.to_string());
    }

    if let Some(array) = content.as_array() {
        let text = array
            .iter()
            .filter_map(conversation_content_part_text)
            .collect::<Vec<_>>()
            .join("");
        return (!text.is_empty()).then_some(text);
    }

    conversation_content_part_text(content)
}

fn conversation_content_part_text(part: &serde_json::Value) -> Option<String> {
    if let Some(text) = part
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return Some(text.to_string());
    }

    part.get("text")
        .or_else(|| part.get("content"))
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn to_node_run_response(run: domain::NodeRunRecord) -> NodeRunResponse {
    let (input_payload, output_payload) = normalize_node_run_payloads_for_logs(&run);
    let input_payload_view = node_input_payload_view(&input_payload);

    NodeRunResponse {
        id: run.id.to_string(),
        flow_run_id: run.flow_run_id.to_string(),
        node_id: run.node_id,
        node_type: run.node_type,
        node_alias: run.node_alias,
        status: run.status.as_str().to_string(),
        input_payload,
        input_payload_view,
        output_payload,
        error_payload: run.error_payload,
        metrics_payload: run.metrics_payload,
        debug_payload: run.debug_payload,
        started_at: format_time(run.started_at),
        finished_at: format_optional_time(run.finished_at),
    }
}

fn normalize_node_run_payloads_for_logs(
    run: &domain::NodeRunRecord,
) -> (serde_json::Value, serde_json::Value) {
    if run.node_type == "start" {
        let input_payload = if run
            .input_payload
            .as_object()
            .is_some_and(serde_json::Map::is_empty)
            && run
                .output_payload
                .as_object()
                .is_some_and(|object| !object.is_empty())
        {
            run.output_payload.clone()
        } else {
            run.input_payload.clone()
        };

        return (input_payload, serde_json::json!({}));
    }

    (run.input_payload.clone(), run.output_payload.clone())
}

fn node_input_payload_view(payload: &serde_json::Value) -> serde_json::Value {
    payload.clone()
}

fn decode_runtime_debug_artifact_preview(payload: &serde_json::Value) -> Option<serde_json::Value> {
    let object = payload.as_object()?;
    if !object
        .get("__runtime_debug_artifact")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
    {
        return None;
    }

    object
        .get("preview")
        .and_then(serde_json::Value::as_str)
        .and_then(|preview| serde_json::from_str(preview).ok())
}

fn start_input_payload(payload: &serde_json::Value) -> &serde_json::Value {
    payload
        .get("node-start")
        .or_else(|| payload.get("start"))
        .unwrap_or(payload)
}

fn to_checkpoint_response(checkpoint: domain::CheckpointRecord) -> CheckpointResponse {
    CheckpointResponse {
        id: checkpoint.id.to_string(),
        flow_run_id: checkpoint.flow_run_id.to_string(),
        node_run_id: checkpoint.node_run_id.map(|value| value.to_string()),
        status: checkpoint.status,
        reason: checkpoint.reason,
        locator_payload: checkpoint.locator_payload,
        variable_snapshot: checkpoint.variable_snapshot,
        external_ref_payload: checkpoint.external_ref_payload,
        created_at: format_time(checkpoint.created_at),
    }
}

fn to_callback_task_response(task: domain::CallbackTaskRecord) -> CallbackTaskResponse {
    CallbackTaskResponse {
        id: task.id.to_string(),
        flow_run_id: task.flow_run_id.to_string(),
        node_run_id: task.node_run_id.to_string(),
        callback_kind: task.callback_kind,
        status: task.status.as_str().to_string(),
        request_payload: task.request_payload,
        response_payload: task.response_payload,
        external_ref_payload: task.external_ref_payload,
        created_at: format_time(task.created_at),
        completed_at: format_optional_time(task.completed_at),
    }
}

fn to_run_event_response(event: domain::RunEventRecord) -> RunEventResponse {
    RunEventResponse {
        id: event.id.to_string(),
        flow_run_id: event.flow_run_id.to_string(),
        node_run_id: event.node_run_id.map(|value| value.to_string()),
        sequence: event.sequence,
        event_type: event.event_type,
        payload: event.payload,
        created_at: format_time(event.created_at),
    }
}

fn is_waiting_prefix_answer_node_run(run: &domain::NodeRunRecord) -> bool {
    if run.node_type != "answer" {
        return false;
    }

    let input_marker = run
        .input_payload
        .get("presentation")
        .and_then(serde_json::Value::as_object)
        .and_then(|presentation| presentation.get("materialized_from"))
        .and_then(serde_json::Value::as_str);
    let debug_marker = run
        .debug_payload
        .get("answer_presentation")
        .and_then(serde_json::Value::as_object)
        .and_then(|presentation| presentation.get("materialized_from"))
        .and_then(serde_json::Value::as_str);

    input_marker == Some("waiting_prefix") || debug_marker == Some("waiting_prefix")
}

fn split_answer_snapshot_node_runs(
    detail: &domain::ApplicationRunDetail,
) -> (Option<domain::NodeRunRecord>, Vec<domain::NodeRunRecord>) {
    let mut answer_snapshot = None;
    let mut node_runs = Vec::new();

    for node_run in detail.node_runs.iter().cloned() {
        if is_waiting_prefix_answer_node_run(&node_run) {
            answer_snapshot = Some(node_run);
        } else {
            node_runs.push(node_run);
        }
    }

    (answer_snapshot, node_runs)
}

fn waiting_node_for_answer_snapshot(
    detail: &domain::ApplicationRunDetail,
) -> (Option<String>, Option<String>) {
    if let Some(checkpoint) = detail
        .checkpoints
        .iter()
        .rev()
        .find(|checkpoint| checkpoint.status.starts_with("waiting"))
    {
        let waiting_node_id = checkpoint
            .locator_payload
            .get("node_id")
            .and_then(serde_json::Value::as_str)
            .map(ToOwned::to_owned);
        let waiting_node_run_id = checkpoint.node_run_id.map(|value| value.to_string());
        return (waiting_node_id, waiting_node_run_id);
    }

    if let Some(task) = detail
        .callback_tasks
        .iter()
        .rev()
        .find(|task| task.status == domain::CallbackTaskStatus::Pending)
    {
        let waiting_node_run_id = task.node_run_id.to_string();
        let waiting_node_id = detail
            .node_runs
            .iter()
            .find(|node_run| node_run.id == task.node_run_id)
            .map(|node_run| node_run.node_id.clone());
        return (waiting_node_id, Some(waiting_node_run_id));
    }

    (None, None)
}

fn answer_snapshot_text(output_payload: &serde_json::Value) -> Option<String> {
    output_payload
        .get("answer")
        .or_else(|| output_payload.get("text"))
        .and_then(serde_json::Value::as_str)
        .filter(|text| !text.is_empty())
        .map(ToOwned::to_owned)
}

fn answer_snapshot_complete(run: &domain::NodeRunRecord) -> bool {
    if let Some(complete) = run
        .input_payload
        .get("presentation")
        .and_then(|presentation| presentation.get("complete"))
        .and_then(serde_json::Value::as_bool)
    {
        return complete;
    }

    !run.debug_payload
        .get("answer_presentation")
        .and_then(|presentation| presentation.get("partial"))
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
}

fn to_answer_snapshot_response(
    run: &domain::NodeRunRecord,
    detail: &domain::ApplicationRunDetail,
) -> Option<AnswerSnapshotResponse> {
    let text = answer_snapshot_text(&run.output_payload)?;
    let (waiting_node_id, waiting_node_run_id) = waiting_node_for_answer_snapshot(detail);

    Some(AnswerSnapshotResponse {
        kind: "answer".to_string(),
        text,
        output_payload: run.output_payload.clone(),
        complete: answer_snapshot_complete(run),
        materialized_from: "waiting_prefix".to_string(),
        answer_node_id: run.node_id.clone(),
        answer_node_run_id: run.id.to_string(),
        waiting_node_id,
        waiting_node_run_id,
    })
}

fn flow_run_can_expose_answer_snapshot(status: &domain::FlowRunStatus) -> bool {
    matches!(
        status,
        domain::FlowRunStatus::WaitingCallback | domain::FlowRunStatus::WaitingHuman
    )
}

fn to_application_run_detail_response(
    application: &domain::ApplicationRecord,
    detail: domain::ApplicationRunDetail,
) -> ApplicationRunDetailResponse {
    let (answer_snapshot_node_run, visible_node_run_records) =
        split_answer_snapshot_node_runs(&detail);
    let answer_snapshot = if flow_run_can_expose_answer_snapshot(&detail.flow_run.status) {
        answer_snapshot_node_run
            .as_ref()
            .and_then(|node_run| to_answer_snapshot_response(node_run, &detail))
    } else {
        None
    };
    let statistics = application_run_statistics(&domain::ApplicationRunDetail {
        node_runs: visible_node_run_records.clone(),
        ..detail.clone()
    });
    let flow_run = to_flow_run_response(detail.flow_run.clone());
    let node_runs = visible_node_run_records
        .into_iter()
        .map(to_node_run_response)
        .collect::<Vec<_>>();
    let checkpoints = detail
        .checkpoints
        .clone()
        .into_iter()
        .map(to_checkpoint_response)
        .collect::<Vec<_>>();
    let callback_tasks = detail
        .callback_tasks
        .clone()
        .into_iter()
        .map(to_callback_task_response)
        .collect::<Vec<_>>();
    let events = detail
        .events
        .clone()
        .into_iter()
        .map(to_run_event_response)
        .collect::<Vec<_>>();
    let application_type = application.application_type.as_str().to_string();
    let run = application_logs::ApplicationRunLogResponse {
        id: detail.flow_run.id.to_string(),
        application_id: application.id.to_string(),
        application_type: application_type.clone(),
        run_object_kind: application.sections.logs.run_object_kind.clone(),
        run_kind: detail.flow_run.run_mode.as_str().to_string(),
        status: detail.flow_run.status.as_str().to_string(),
        title: detail.flow_run.title.clone(),
        source: application_logs::source_for_run(detail.flow_run.api_key_id),
        compatibility_mode: detail.flow_run.compatibility_mode.clone(),
        subject: application_logs::ApplicationRunSubjectResponse {
            kind: application_type,
            id: Some(detail.flow_run.flow_id.to_string()),
            draft_id: Some(detail.flow_run.draft_id.to_string()),
            target_node_id: detail.flow_run.target_node_id.clone(),
        },
        actor: application_logs::actor_from_console_user(
            Some(detail.flow_run.created_by.to_string()),
            detail.flow_run.authorized_account.clone(),
        ),
        correlation: application_logs::ApplicationRunCorrelationResponse {
            api_key_id: detail.flow_run.api_key_id.map(|value| value.to_string()),
            publication_version_id: detail
                .flow_run
                .publication_version_id
                .map(|value| value.to_string()),
            external_user: detail.flow_run.external_user.clone(),
            external_conversation_id: detail.flow_run.external_conversation_id.clone(),
            external_trace_id: detail.flow_run.external_trace_id.clone(),
            compatibility_mode: detail.flow_run.compatibility_mode.clone(),
            idempotency_key: detail.flow_run.idempotency_key.clone(),
        },
        started_at: application_logs::format_time(detail.flow_run.started_at),
        finished_at: application_logs::format_optional_time(detail.flow_run.finished_at),
        created_at: application_logs::format_time(detail.flow_run.created_at),
        updated_at: application_logs::format_time(detail.flow_run.updated_at),
    };
    let typed_detail = application_logs::ApplicationRunTypedDetailResponse {
        kind: application.application_type.as_str().to_string(),
        flow_run: flow_run.clone(),
        answer_snapshot: answer_snapshot.clone(),
        node_runs: node_runs.clone(),
        checkpoints: checkpoints.clone(),
        callback_tasks: callback_tasks.clone(),
        events: events.clone(),
    };

    ApplicationRunDetailResponse {
        run,
        statistics,
        detail: typed_detail,
        flow_run,
        answer_snapshot,
        node_runs,
        checkpoints,
        callback_tasks,
        events,
    }
}

fn to_node_last_run_response(last_run: domain::NodeLastRun) -> NodeLastRunResponse {
    NodeLastRunResponse {
        flow_run: to_flow_run_response(last_run.flow_run),
        node_run: to_node_run_response(last_run.node_run),
        checkpoints: last_run
            .checkpoints
            .into_iter()
            .map(to_checkpoint_response)
            .collect(),
        events: last_run
            .events
            .into_iter()
            .map(to_run_event_response)
            .collect(),
    }
}

fn to_runtime_debug_stream_part_response(
    part: observability::DebugStreamPart,
) -> RuntimeDebugStreamPartResponse {
    RuntimeDebugStreamPartResponse {
        id: part.id.to_string(),
        flow_run_id: part.flow_run_id.to_string(),
        item_id: part.item_id.map(|value| value.to_string()),
        span_id: part.span_id.map(|value| value.to_string()),
        part_type: part.part_type,
        status: part.status,
        trust_level: part.trust_level.as_str().to_string(),
        payload: part.payload,
    }
}

async fn ensure_application_visible(
    state: &Arc<ApiState>,
    actor_user_id: Uuid,
    application_id: Uuid,
) -> Result<domain::ApplicationRecord, ApiError> {
    Ok(ApplicationService::new(state.store.clone())
        .get_application(actor_user_id, application_id)
        .await?)
}

fn parse_runtime_event_cursor(run_id: Uuid, event_id: &str) -> Option<i64> {
    if let Ok(sequence) = event_id.parse::<i64>() {
        return Some(sequence);
    }

    let (cursor_run_id, sequence) = event_id.rsplit_once(':')?;
    if cursor_run_id != run_id.to_string() {
        return None;
    }

    sequence.parse::<i64>().ok()
}

fn debug_run_stream_from_sequence(
    run_id: Uuid,
    query: &DebugRunStreamQuery,
    headers: &HeaderMap,
) -> Option<i64> {
    query.from_sequence.or_else(|| {
        query
            .last_event_id
            .as_deref()
            .and_then(|event_id| parse_runtime_event_cursor(run_id, event_id))
            .or_else(|| {
                headers
                    .get("last-event-id")
                    .and_then(|value| value.to_str().ok())
                    .and_then(|event_id| parse_runtime_event_cursor(run_id, event_id))
            })
    })
}

#[utoipa::path(
    post,
    path = "/api/console/applications/{id}/orchestration/debug-runs",
    request_body = StartFlowDebugRunBody,
    params(
        ("id" = String, Path, description = "Application id")
    ),
    responses(
        (status = 201, body = ApplicationRunDetailResponse),
        (status = 400, body = crate::error_response::ErrorBody),
        (status = 401, body = crate::error_response::ErrorBody),
        (status = 403, body = crate::error_response::ErrorBody),
        (status = 404, body = crate::error_response::ErrorBody)
    )
)]
pub async fn start_flow_debug_run(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(body): Json<StartFlowDebugRunBody>,
) -> Result<(StatusCode, Json<ApiSuccess<ApplicationRunDetailResponse>>), ApiError> {
    let _http_activity = state
        .runtime_activity
        .start(id, ApplicationActivityKind::HttpRequest);
    let context = require_session(&state, &headers).await?;
    require_csrf(&headers, &context.session)?;
    let application = ensure_application_visible(&state, context.user.id, id).await?;

    let runtime_service = OrchestrationRuntimeService::new(
        state.store.clone(),
        api_provider_runtime(&state),
        state.runtime_engine.clone(),
        state.provider_secret_master_key.clone(),
    );
    let detail = runtime_service
        .start_flow_debug_run(StartFlowDebugRunCommand {
            actor_user_id: context.user.id,
            application_id: id,
            input_payload: body.input_payload,
            document_snapshot: body.document,
            debug_session_id: body.debug_session_id,
        })
        .await?;
    let flow_run_id = detail.flow_run.id;
    let workspace_id = context.actor.current_workspace_id;
    let background_state = state.clone();

    tokio::spawn(async move {
        let _execution_activity = background_state
            .runtime_activity
            .start(id, ApplicationActivityKind::ApplicationExecution);
        let background_service = OrchestrationRuntimeService::new(
            background_state.store.clone(),
            api_provider_runtime(&background_state),
            background_state.runtime_engine.clone(),
            background_state.provider_secret_master_key.clone(),
        );
        let continue_result = scope_application_activity(
            id,
            background_service.continue_flow_debug_run(ContinueFlowDebugRunCommand {
                application_id: id,
                flow_run_id,
                workspace_id,
            }),
        )
        .await;
        match continue_result {
            Ok(detail) => {
                if let Err(error) = offload_application_run_detail_artifacts(
                    background_state.clone(),
                    workspace_id,
                    id,
                    detail,
                )
                .await
                {
                    error!(
                        application_id = %id,
                        flow_run_id = %flow_run_id,
                        error = %error.0,
                        "failed to offload flow debug artifacts"
                    );
                }
            }
            Err(error) => {
                error!(
                    application_id = %id,
                    flow_run_id = %flow_run_id,
                    error = %error,
                    "failed to continue flow debug run"
                );
            }
        }
    });

    Ok((
        StatusCode::CREATED,
        Json(ApiSuccess::new(to_application_run_detail_response(
            &application,
            detail,
        ))),
    ))
}

pub async fn start_flow_debug_run_stream(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Query(stream_query): Query<DebugRunStreamQuery>,
    Json(body): Json<StartFlowDebugRunBody>,
) -> Result<Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>>, ApiError> {
    let _http_activity = state
        .runtime_activity
        .start(id, ApplicationActivityKind::HttpRequest);
    let request_received_at = std::time::Instant::now();
    let context = require_session(&state, &headers).await?;
    require_csrf(&headers, &context.session)?;

    let runtime_service = OrchestrationRuntimeService::new(
        state.store.clone(),
        api_provider_runtime(&state),
        state.runtime_engine.clone(),
        state.provider_secret_master_key.clone(),
    );
    let shell = runtime_service
        .open_flow_debug_run_shell(StartFlowDebugRunCommand {
            actor_user_id: context.user.id,
            application_id: id,
            input_payload: body.input_payload.clone(),
            document_snapshot: body.document.clone(),
            debug_session_id: body.debug_session_id.clone(),
        })
        .await?;
    let run_id = shell.id;
    let workspace_id = context.actor.current_workspace_id;
    let actor_user_id = context.user.id;

    state
        .runtime_event_stream
        .open_run(run_id, RuntimeEventStreamPolicy::debug_default())
        .await?;
    let persister_handle = spawn_runtime_debug_event_persister(
        state.store.clone(),
        state.runtime_event_stream.clone(),
        run_id,
    );
    state
        .runtime_event_stream
        .append(run_id, debug_stream_events::flow_accepted(run_id))
        .await?;
    state
        .runtime_event_stream
        .append(run_id, debug_stream_events::heartbeat())
        .await?;

    let (sender, receiver) = mpsc::channel(32);
    tokio::spawn(debug_run_stream::send_runtime_event_stream(
        state.runtime_event_stream.clone(),
        Arc::new(state.store.clone()),
        run_id,
        debug_run_stream_from_sequence(run_id, &stream_query, &headers),
        sender,
    ));

    let background_state = state.clone();
    tokio::spawn(async move {
        let _execution_activity = background_state
            .runtime_activity
            .start(id, ApplicationActivityKind::ApplicationExecution);
        let background_service = OrchestrationRuntimeService::new(
            background_state.store.clone(),
            api_provider_runtime(&background_state),
            background_state.runtime_engine.clone(),
            background_state.provider_secret_master_key.clone(),
        )
        .with_runtime_event_stream(background_state.runtime_event_stream.clone());
        let prepare_result = scope_application_activity(
            id,
            background_service.prepare_flow_debug_run_from_shell(PrepareFlowDebugRunCommand {
                actor_user_id,
                application_id: id,
                flow_run_id: run_id,
                input_payload: body.input_payload,
                document_snapshot: body.document,
                debug_session_id: body.debug_session_id.unwrap_or_default(),
            }),
        )
        .await;
        let result = match prepare_result {
            Ok(_) => {
                scope_application_activity(
                    id,
                    background_service.continue_flow_debug_run(ContinueFlowDebugRunCommand {
                        application_id: id,
                        flow_run_id: run_id,
                        workspace_id,
                    }),
                )
                .await
            }
            Err(error) => Err(error),
        };

        match result {
            Ok(detail) => {
                if let Err(error) = offload_application_run_detail_artifacts(
                    background_state.clone(),
                    workspace_id,
                    id,
                    detail,
                )
                .await
                {
                    error!(
                        application_id = %id,
                        flow_run_id = %run_id,
                        error = %error.0,
                        "failed to offload streamed flow debug artifacts"
                    );
                }
            }
            Err(error) => {
                fail_runtime_event_stream_if_missing_terminal(
                    background_state.runtime_event_stream.clone(),
                    run_id,
                    &error,
                )
                .await;
                error!(
                    application_id = %id,
                    flow_run_id = %run_id,
                    error = %error,
                    "failed to prepare and continue streamed flow debug run"
                );
            }
        }
        wait_for_runtime_debug_event_persister(persister_handle, id, run_id).await;
    });

    tracing::info!(
        application_id = %id,
        flow_run_id = %run_id,
        http_to_sse_open_ms = request_received_at.elapsed().as_millis() as u64,
        "flow debug stream opened"
    );

    let sse_activity = state
        .runtime_activity
        .start(id, ApplicationActivityKind::SseConnection);
    let stream = debug_run_stream::DebugRunSseStream::new(receiver).map(move |event| {
        let _keep_alive = &sse_activity;
        event
    });

    Ok(Sse::new(stream).keep_alive(KeepAlive::default()))
}

pub async fn subscribe_flow_debug_run_stream(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path((id, run_id)): Path<(Uuid, Uuid)>,
    Query(stream_query): Query<DebugRunStreamQuery>,
) -> Result<Sse<debug_run_stream::DebugRunSseStream>, ApiError> {
    let context = require_session(&state, &headers).await?;
    ensure_application_visible(&state, context.user.id, id).await?;
    let flow_run = state
        .store
        .get_flow_run(id, run_id)
        .await?
        .ok_or(ControlPlaneError::NotFound("flow_run"))?;
    if flow_run.created_by != context.user.id {
        return Err(ControlPlaneError::NotFound("flow_run").into());
    }

    let (sender, receiver) = mpsc::channel(32);
    tokio::spawn(debug_run_stream::send_runtime_event_stream(
        state.runtime_event_stream.clone(),
        Arc::new(state.store.clone()),
        run_id,
        debug_run_stream_from_sequence(run_id, &stream_query, &headers),
        sender,
    ));

    Ok(Sse::new(debug_run_stream::DebugRunSseStream::new(receiver))
        .keep_alive(KeepAlive::default()))
}

#[utoipa::path(
    post,
    path = "/api/console/applications/{id}/orchestration/runs/{run_id}/cancel",
    params(
        ("id" = String, Path, description = "Application id"),
        ("run_id" = String, Path, description = "Flow run id")
    ),
    responses(
        (status = 200, body = ApplicationRunDetailResponse),
        (status = 400, body = crate::error_response::ErrorBody),
        (status = 401, body = crate::error_response::ErrorBody),
        (status = 403, body = crate::error_response::ErrorBody),
        (status = 404, body = crate::error_response::ErrorBody)
    )
)]
pub async fn cancel_flow_run(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path((id, run_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<ApiSuccess<ApplicationRunDetailResponse>>, ApiError> {
    let context = require_session(&state, &headers).await?;
    require_csrf(&headers, &context.session)?;
    let application = ensure_application_visible(&state, context.user.id, id).await?;

    let runtime_service = OrchestrationRuntimeService::new(
        state.store.clone(),
        api_provider_runtime(&state),
        state.runtime_engine.clone(),
        state.provider_secret_master_key.clone(),
    )
    .with_runtime_event_stream(state.runtime_event_stream.clone());

    let detail = runtime_service
        .cancel_flow_run(CancelFlowRunCommand {
            actor_user_id: context.user.id,
            application_id: id,
            flow_run_id: run_id,
        })
        .await?;
    let detail = offload_application_run_detail_artifacts(
        state.clone(),
        context.actor.current_workspace_id,
        id,
        detail,
    )
    .await?;

    Ok(Json(ApiSuccess::new(to_application_run_detail_response(
        &application,
        detail,
    ))))
}

#[utoipa::path(
    post,
    path = "/api/console/applications/{id}/orchestration/runs/{run_id}/resume",
    request_body = ResumeFlowRunBody,
    params(
        ("id" = String, Path, description = "Application id"),
        ("run_id" = String, Path, description = "Flow run id")
    ),
    responses(
        (status = 200, body = ApplicationRunDetailResponse),
        (status = 400, body = crate::error_response::ErrorBody),
        (status = 401, body = crate::error_response::ErrorBody),
        (status = 403, body = crate::error_response::ErrorBody),
        (status = 404, body = crate::error_response::ErrorBody)
    )
)]
pub async fn resume_flow_run(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path((id, run_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<ResumeFlowRunBody>,
) -> Result<Json<ApiSuccess<ApplicationRunDetailResponse>>, ApiError> {
    let _http_activity = state
        .runtime_activity
        .start(id, ApplicationActivityKind::HttpRequest);
    let context = require_session(&state, &headers).await?;
    require_csrf(&headers, &context.session)?;
    let application = ensure_application_visible(&state, context.user.id, id).await?;

    let checkpoint_id = Uuid::parse_str(&body.checkpoint_id)
        .map_err(|_| ControlPlaneError::InvalidInput("checkpoint_id"))?;
    let detail = scope_application_activity(
        id,
        OrchestrationRuntimeService::new(
            state.store.clone(),
            api_provider_runtime(&state),
            state.runtime_engine.clone(),
            state.provider_secret_master_key.clone(),
        )
        .resume_flow_run(ResumeFlowRunCommand {
            actor_user_id: context.user.id,
            application_id: id,
            flow_run_id: run_id,
            checkpoint_id,
            input_payload: body.input_payload,
        }),
    )
    .await?;
    let detail = offload_application_run_detail_artifacts(
        state.clone(),
        context.actor.current_workspace_id,
        id,
        detail,
    )
    .await?;

    Ok(Json(ApiSuccess::new(to_application_run_detail_response(
        &application,
        detail,
    ))))
}

#[utoipa::path(
    post,
    path = "/api/console/applications/{id}/orchestration/callback-tasks/{callback_task_id}/complete",
    request_body = CompleteCallbackTaskBody,
    params(
        ("id" = String, Path, description = "Application id"),
        ("callback_task_id" = String, Path, description = "Callback task id")
    ),
    responses(
        (status = 200, body = ApplicationRunDetailResponse),
        (status = 400, body = crate::error_response::ErrorBody),
        (status = 401, body = crate::error_response::ErrorBody),
        (status = 403, body = crate::error_response::ErrorBody),
        (status = 404, body = crate::error_response::ErrorBody)
    )
)]
pub async fn complete_callback_task(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path((id, callback_task_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<CompleteCallbackTaskBody>,
) -> Result<Json<ApiSuccess<ApplicationRunDetailResponse>>, ApiError> {
    let _http_activity = state
        .runtime_activity
        .start(id, ApplicationActivityKind::HttpRequest);
    let context = require_session(&state, &headers).await?;
    require_csrf(&headers, &context.session)?;
    let application = ensure_application_visible(&state, context.user.id, id).await?;

    let detail = scope_application_activity(
        id,
        OrchestrationRuntimeService::new(
            state.store.clone(),
            api_provider_runtime(&state),
            state.runtime_engine.clone(),
            state.provider_secret_master_key.clone(),
        )
        .complete_callback_task(CompleteCallbackTaskCommand {
            actor_user_id: context.user.id,
            application_id: id,
            callback_task_id,
            response_payload: body.response_payload,
        }),
    )
    .await?;
    let detail = offload_application_run_detail_artifacts(
        state.clone(),
        context.actor.current_workspace_id,
        id,
        detail,
    )
    .await?;

    Ok(Json(ApiSuccess::new(to_application_run_detail_response(
        &application,
        detail,
    ))))
}

#[utoipa::path(
    post,
    path = "/api/console/applications/{id}/orchestration/nodes/{node_id}/debug-runs",
    request_body = StartNodeDebugPreviewBody,
    params(
        ("id" = String, Path, description = "Application id"),
        ("node_id" = String, Path, description = "Node id")
    ),
    responses(
        (status = 201, body = NodeLastRunResponse),
        (status = 400, body = crate::error_response::ErrorBody),
        (status = 401, body = crate::error_response::ErrorBody),
        (status = 403, body = crate::error_response::ErrorBody),
        (status = 404, body = crate::error_response::ErrorBody)
    )
)]
pub async fn start_node_debug_preview(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path((id, node_id)): Path<(Uuid, String)>,
    Json(body): Json<StartNodeDebugPreviewBody>,
) -> Result<(StatusCode, Json<ApiSuccess<NodeLastRunResponse>>), ApiError> {
    let _http_activity = state
        .runtime_activity
        .start(id, ApplicationActivityKind::HttpRequest);
    let context = require_session(&state, &headers).await?;
    require_csrf(&headers, &context.session)?;

    let outcome = scope_application_activity(
        id,
        OrchestrationRuntimeService::new(
            state.store.clone(),
            api_provider_runtime(&state),
            state.runtime_engine.clone(),
            state.provider_secret_master_key.clone(),
        )
        .start_node_debug_preview(StartNodeDebugPreviewCommand {
            actor_user_id: context.user.id,
            application_id: id,
            node_id,
            input_payload: body.input_payload,
            document_snapshot: body.document,
            debug_session_id: body.debug_session_id,
        }),
    )
    .await?;

    let detail = offload_application_run_detail_artifacts(
        state.clone(),
        context.actor.current_workspace_id,
        id,
        domain::ApplicationRunDetail {
            flow_run: outcome.flow_run,
            node_runs: vec![outcome.node_run],
            checkpoints: Vec::new(),
            callback_tasks: Vec::new(),
            events: outcome.events,
        },
    )
    .await?;
    let node_run = detail
        .node_runs
        .into_iter()
        .next()
        .ok_or(ControlPlaneError::NotFound("node_run"))?;
    let response = to_node_last_run_response(domain::NodeLastRun {
        flow_run: detail.flow_run,
        node_run,
        checkpoints: Vec::new(),
        events: detail.events,
    });

    Ok((StatusCode::CREATED, Json(ApiSuccess::new(response))))
}

#[utoipa::path(
    get,
    path = "/api/console/applications/{id}/orchestration/debug-artifacts/{artifact_id}",
    params(
        ("id" = String, Path, description = "Application id"),
        ("artifact_id" = String, Path, description = "Runtime debug artifact id")
    ),
    responses(
        (status = 200, body = serde_json::Value),
        (status = 401, body = crate::error_response::ErrorBody),
        (status = 403, body = crate::error_response::ErrorBody),
        (status = 404, body = crate::error_response::ErrorBody)
    )
)]
pub async fn get_runtime_debug_artifact(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path((id, artifact_id)): Path<(Uuid, Uuid)>,
) -> Result<axum::response::Response, ApiError> {
    let context = require_session(&state, &headers).await?;
    ensure_application_visible(&state, context.user.id, id).await?;

    load_runtime_debug_artifact_response(state, context.actor.current_workspace_id, id, artifact_id)
        .await
}

#[utoipa::path(
    get,
    path = "/api/console/applications/{id}/logs/runs",
    params(
        ("id" = String, Path, description = "Application id"),
        ("page" = Option<i64>, Query, description = "1-based page number"),
        ("page_size" = Option<i64>, Query, description = "Page size"),
        ("time_range_days" = Option<i64>, Query, description = "Optional created-at day window"),
        ("sort_by" = Option<String>, Query, description = "Sort field: created_at, started_at, finished_at or updated_at"),
        ("sort_order" = Option<String>, Query, description = "Sort direction: asc or desc"),
        ("cache_mode" = Option<String>, Query, description = "Read mode: refresh bypasses application log cache reads")
    ),
    responses(
        (status = 200, body = FlowRunSummaryPageResponse),
        (status = 401, body = crate::error_response::ErrorBody),
        (status = 403, body = crate::error_response::ErrorBody),
        (status = 404, body = crate::error_response::ErrorBody)
    )
)]
pub async fn list_application_runs(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Query(query): Query<ApplicationRunsQuery>,
) -> Result<Json<ApiSuccess<FlowRunSummaryPageResponse>>, ApiError> {
    let context = require_session(&state, &headers).await?;
    let application = ensure_application_visible(&state, context.user.id, id).await?;
    let page = query.page.unwrap_or(1).max(1);
    let page_size = query.page_size.unwrap_or(20).clamp(1, 100);
    let created_after = application_runs_created_after(&query);
    let sort_by = normalize_application_run_sort_by(query.sort_by.as_deref()).to_string();
    let sort_order = normalize_application_run_sort_order(query.sort_order.as_deref()).to_string();
    let refresh_cache = should_refresh_application_run_logs(query.cache_mode.as_deref());
    let cache = state.infrastructure.cache_store();
    let cache_key = application_log_cache::summary_page_cache_key(
        context.actor.current_workspace_id,
        id,
        &query,
        page,
        page_size,
        &sort_by,
        &sort_order,
    );

    if !refresh_cache {
        if let Some(cached) =
            application_log_cache::read::<FlowRunSummaryPageResponse>(cache.as_ref(), &cache_key)
                .await
        {
            return Ok(Json(ApiSuccess::new(cached)));
        }
    }

    let runs_page =
        <MainDurableStore as OrchestrationRuntimeRepository>::list_application_run_logs_page(
            &state.store,
            id,
            control_plane::ports::ListApplicationRunsPageInput {
                page,
                page_size,
                created_after,
                sort_by: Some(sort_by),
                sort_order: Some(sort_order),
            },
        )
        .await?;

    let mut items = Vec::with_capacity(runs_page.items.len());

    for log_summary in runs_page.items {
        let statistics = application_logs::ApplicationRunStatisticsResponse {
            total_tokens: log_summary.total_tokens,
            unique_node_count: log_summary.unique_node_count,
            tool_callback_count: log_summary.tool_callback_count,
        };
        items.push(to_flow_run_summary_response(
            &application,
            log_summary.run,
            statistics,
        ));
    }

    let response = FlowRunSummaryPageResponse {
        items,
        total: runs_page.total,
        page: runs_page.page,
        page_size: runs_page.page_size,
    };

    if application_log_cache::summary_page_cacheable(&response) {
        application_log_cache::write(
            cache.as_ref(),
            &cache_key,
            &response,
            application_log_cache::summary_page_cache_ttl(page),
        )
        .await;
    }

    Ok(Json(ApiSuccess::new(response)))
}

#[utoipa::path(
    get,
    path = "/api/console/applications/{id}/logs/conversations/{conversation_id}/messages",
    params(
        ("id" = String, Path, description = "Application id"),
        ("conversation_id" = String, Path, description = "External conversation id"),
        ("around_run_id" = Option<String>, Query, description = "Flow run id to center the page around"),
        ("before" = Option<String>, Query, description = "Load runs before this cursor run id"),
        ("after" = Option<String>, Query, description = "Load runs after this cursor run id"),
        ("limit" = Option<i64>, Query, description = "Page size, defaults to 5")
    ),
    responses(
        (status = 200, body = ApplicationConversationMessagesPageResponse),
        (status = 401, body = crate::error_response::ErrorBody),
        (status = 403, body = crate::error_response::ErrorBody),
        (status = 404, body = crate::error_response::ErrorBody)
    )
)]
pub async fn list_application_conversation_messages(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path((id, conversation_id)): Path<(Uuid, String)>,
    Query(query): Query<ApplicationConversationMessagesQuery>,
) -> Result<Json<ApiSuccess<ApplicationConversationMessagesPageResponse>>, ApiError> {
    let context = require_session(&state, &headers).await?;
    ensure_application_visible(&state, context.user.id, id).await?;

    let page =
        <MainDurableStore as OrchestrationRuntimeRepository>::list_application_conversation_runs_page(
            &state.store,
            id,
            ListApplicationConversationRunsPageInput {
                external_conversation_id: conversation_id,
                around_run_id: query.around_run_id,
                before_run_id: parse_optional_uuid_cursor(query.before.as_deref()),
                after_run_id: parse_optional_uuid_cursor(query.after.as_deref()),
                limit: query.limit.unwrap_or(5),
            },
        )
        .await?;
    let current_run_id = query.around_run_id;

    Ok(Json(ApiSuccess::new(
        ApplicationConversationMessagesPageResponse {
            items: page
                .items
                .into_iter()
                .map(|run| to_application_conversation_message_response(run, current_run_id))
                .collect(),
            page: ApplicationConversationMessagesPageInfoResponse {
                has_before: page.has_before,
                has_after: page.has_after,
                before_cursor: page.before_cursor.map(|value| value.to_string()),
                after_cursor: page.after_cursor.map(|value| value.to_string()),
            },
        },
    )))
}

#[utoipa::path(
    get,
    path = "/api/console/applications/{id}/logs/runs/{run_id}/conversation/messages",
    params(
        ("id" = String, Path, description = "Application id"),
        ("run_id" = String, Path, description = "Flow run id"),
        ("before" = Option<String>, Query, description = "Load messages before this cursor"),
        ("after" = Option<String>, Query, description = "Load messages after this cursor"),
        ("limit" = Option<i64>, Query, description = "Page size, defaults to 5")
    ),
    responses(
        (status = 200, body = ApplicationConversationMessagesPageResponse),
        (status = 401, body = crate::error_response::ErrorBody),
        (status = 403, body = crate::error_response::ErrorBody),
        (status = 404, body = crate::error_response::ErrorBody)
    )
)]
pub async fn list_application_run_conversation_messages(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path((id, run_id)): Path<(Uuid, Uuid)>,
    Query(query): Query<ApplicationConversationMessagesQuery>,
) -> Result<Json<ApiSuccess<ApplicationConversationMessagesPageResponse>>, ApiError> {
    let context = require_session(&state, &headers).await?;
    ensure_application_visible(&state, context.user.id, id).await?;

    let detail = <MainDurableStore as OrchestrationRuntimeRepository>::get_application_run_detail(
        &state.store,
        id,
        run_id,
    )
    .await?
    .ok_or(ControlPlaneError::NotFound("flow_run"))?;

    if let Some(conversation_id) = detail.flow_run.external_conversation_id.clone() {
        let page = <MainDurableStore as OrchestrationRuntimeRepository>::list_application_conversation_runs_page(
            &state.store,
            id,
            ListApplicationConversationRunsPageInput {
                external_conversation_id: conversation_id,
                around_run_id: query.around_run_id.or(Some(run_id)),
                before_run_id: parse_optional_uuid_cursor(query.before.as_deref()),
                after_run_id: parse_optional_uuid_cursor(query.after.as_deref()),
                limit: query.limit.unwrap_or(5),
            },
        )
        .await?;
        let workspace_id = context.actor.current_workspace_id;
        let system_context_items =
            imported_context_messages_from_run(&detail.flow_run, |artifact_id| {
                let state = state.clone();

                async move {
                    load_runtime_debug_artifact_json_value(state, workspace_id, id, artifact_id)
                        .await
                        .ok()
                }
            })
            .await
            .into_iter()
            .filter(|item| item.role.as_deref() == Some("system"));

        return Ok(Json(ApiSuccess::new(
            ApplicationConversationMessagesPageResponse {
                items: system_context_items
                    .chain(
                        page.items.into_iter().map(|run| {
                            to_application_conversation_message_response(run, Some(run_id))
                        }),
                    )
                    .collect(),
                page: ApplicationConversationMessagesPageInfoResponse {
                    has_before: page.has_before,
                    has_after: page.has_after,
                    before_cursor: page.before_cursor.map(|value| value.to_string()),
                    after_cursor: page.after_cursor.map(|value| value.to_string()),
                },
            },
        )));
    }

    let workspace_id = context.actor.current_workspace_id;
    let fallback_page =
        conversation_messages_from_single_run(&detail.flow_run, &query, |artifact_id| {
            let state = state.clone();

            async move {
                load_runtime_debug_artifact_json_value(state, workspace_id, id, artifact_id)
                    .await
                    .ok()
            }
        })
        .await;

    Ok(Json(ApiSuccess::new(fallback_page)))
}

#[utoipa::path(
    get,
    path = "/api/console/applications/{id}/logs/runs/{run_id}",
    params(
        ("id" = String, Path, description = "Application id"),
        ("run_id" = String, Path, description = "Flow run id")
    ),
    responses(
        (status = 200, body = ApplicationRunDetailResponse),
        (status = 401, body = crate::error_response::ErrorBody),
        (status = 403, body = crate::error_response::ErrorBody),
        (status = 404, body = crate::error_response::ErrorBody)
    )
)]
pub async fn get_application_run_detail(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path((id, run_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<ApiSuccess<ApplicationRunDetailResponse>>, ApiError> {
    let context = require_session(&state, &headers).await?;
    let application = ensure_application_visible(&state, context.user.id, id).await?;
    let detail = <MainDurableStore as OrchestrationRuntimeRepository>::get_application_run_detail(
        &state.store,
        id,
        run_id,
    )
    .await?
    .ok_or(ControlPlaneError::NotFound("flow_run"))?;
    let detail = offload_application_run_detail_artifacts(
        state.clone(),
        context.actor.current_workspace_id,
        id,
        detail,
    )
    .await?;
    let response = to_application_run_detail_response(&application, detail);

    Ok(Json(ApiSuccess::new(response)))
}

#[utoipa::path(
    get,
    path = "/api/console/applications/{id}/logs/runs/{run_id}/nodes/{node_id}",
    params(
        ("id" = String, Path, description = "Application id"),
        ("run_id" = String, Path, description = "Flow run id"),
        ("node_id" = String, Path, description = "Flow node id")
    ),
    responses(
        (status = 200, body = NodeLastRunResponse),
        (status = 401, body = crate::error_response::ErrorBody),
        (status = 403, body = crate::error_response::ErrorBody),
        (status = 404, body = crate::error_response::ErrorBody)
    )
)]
pub async fn get_application_run_node_last_run(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path((id, run_id, node_id)): Path<(Uuid, Uuid, String)>,
) -> Result<Json<ApiSuccess<Option<NodeLastRunResponse>>>, ApiError> {
    let context = require_session(&state, &headers).await?;
    ensure_application_visible(&state, context.user.id, id).await?;

    let detail = <MainDurableStore as OrchestrationRuntimeRepository>::get_application_run_detail(
        &state.store,
        id,
        run_id,
    )
    .await?
    .ok_or(ControlPlaneError::NotFound("flow_run"))?;
    let detail = offload_application_run_detail_artifacts(
        state.clone(),
        context.actor.current_workspace_id,
        id,
        detail,
    )
    .await?;

    let Some(node_run) = detail
        .node_runs
        .into_iter()
        .rev()
        .find(|candidate| candidate.node_id == node_id)
    else {
        return Ok(Json(ApiSuccess::new(None)));
    };

    let node_run_id = node_run.id;
    let checkpoints = detail
        .checkpoints
        .into_iter()
        .filter(|checkpoint| checkpoint.node_run_id == Some(node_run_id))
        .collect();
    let events = detail
        .events
        .into_iter()
        .filter(|event| event.node_run_id == Some(node_run_id))
        .collect();

    Ok(Json(ApiSuccess::new(Some(to_node_last_run_response(
        domain::NodeLastRun {
            flow_run: detail.flow_run,
            node_run,
            checkpoints,
            events,
        },
    )))))
}

#[utoipa::path(
    get,
    path = "/api/console/applications/{id}/logs/runs/{run_id}/debug-stream",
    params(
        ("id" = String, Path, description = "Application id"),
        ("run_id" = String, Path, description = "Flow run id")
    ),
    responses(
        (status = 200, body = RuntimeDebugStreamResponse),
        (status = 401, body = crate::error_response::ErrorBody),
        (status = 403, body = crate::error_response::ErrorBody),
        (status = 404, body = crate::error_response::ErrorBody)
    )
)]
pub async fn get_runtime_debug_stream(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path((id, run_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<ApiSuccess<RuntimeDebugStreamResponse>>, ApiError> {
    let context = require_session(&state, &headers).await?;
    ensure_application_visible(&state, context.user.id, id).await?;

    <MainDurableStore as OrchestrationRuntimeRepository>::get_flow_run(&state.store, id, run_id)
        .await?
        .ok_or(ControlPlaneError::NotFound("flow_run"))?;

    let parts = <MainDurableStore as OrchestrationRuntimeRepository>::list_runtime_events(
        &state.store,
        run_id,
        0,
    )
    .await?
    .iter()
    .filter_map(|event| {
        control_plane::runtime_observability::debug_read_model::fold_event_to_debug_part(
            run_id, event,
        )
    })
    .map(to_runtime_debug_stream_part_response)
    .collect();

    Ok(Json(ApiSuccess::new(RuntimeDebugStreamResponse { parts })))
}

#[utoipa::path(
    get,
    path = "/api/console/applications/{id}/orchestration/nodes/{node_id}/last-run",
    params(
        ("id" = String, Path, description = "Application id"),
        ("node_id" = String, Path, description = "Node id")
    ),
    responses(
        (status = 200, body = NodeLastRunResponse),
        (status = 401, body = crate::error_response::ErrorBody),
        (status = 403, body = crate::error_response::ErrorBody),
        (status = 404, body = crate::error_response::ErrorBody)
    )
)]
pub async fn get_node_last_run(
    State(state): State<Arc<ApiState>>,
    headers: HeaderMap,
    Path((id, node_id)): Path<(Uuid, String)>,
) -> Result<Json<ApiSuccess<Option<NodeLastRunResponse>>>, ApiError> {
    let context = require_session(&state, &headers).await?;
    ensure_application_visible(&state, context.user.id, id).await?;

    let last_run = <MainDurableStore as OrchestrationRuntimeRepository>::get_latest_node_run(
        &state.store,
        id,
        &node_id,
    )
    .await?;
    let last_run = if let Some(last_run) = last_run {
        let checkpoints = last_run.checkpoints;
        let detail = offload_application_run_detail_artifacts(
            state.clone(),
            context.actor.current_workspace_id,
            id,
            domain::ApplicationRunDetail {
                flow_run: last_run.flow_run,
                node_runs: vec![last_run.node_run],
                checkpoints: checkpoints.clone(),
                callback_tasks: Vec::new(),
                events: last_run.events,
            },
        )
        .await?;
        let node_run = detail
            .node_runs
            .into_iter()
            .next()
            .ok_or(ControlPlaneError::NotFound("node_run"))?;
        Some(to_node_last_run_response(domain::NodeLastRun {
            flow_run: detail.flow_run,
            node_run,
            checkpoints,
            events: detail.events,
        }))
    } else {
        None
    };

    Ok(Json(ApiSuccess::new(last_run)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    #[test]
    fn runtime_event_cursor_accepts_numeric_and_run_scoped_event_ids() {
        let run_id = Uuid::now_v7();

        assert_eq!(parse_runtime_event_cursor(run_id, "7"), Some(7));
        assert_eq!(
            parse_runtime_event_cursor(run_id, &format!("{run_id}:8")),
            Some(8)
        );
        assert_eq!(
            parse_runtime_event_cursor(Uuid::now_v7(), &format!("{run_id}:8")),
            None
        );
        assert_eq!(parse_runtime_event_cursor(run_id, "not-a-cursor"), None);
    }

    #[test]
    fn debug_run_stream_cursor_prefers_query_before_last_event_id_header() {
        let run_id = Uuid::now_v7();
        let mut headers = HeaderMap::new();
        headers.insert(
            "last-event-id",
            HeaderValue::from_str(&format!("{run_id}:11")).unwrap(),
        );

        assert_eq!(
            debug_run_stream_from_sequence(
                run_id,
                &DebugRunStreamQuery {
                    from_sequence: Some(9),
                    last_event_id: Some(format!("{run_id}:10")),
                },
                &headers,
            ),
            Some(9)
        );
        assert_eq!(
            debug_run_stream_from_sequence(
                run_id,
                &DebugRunStreamQuery {
                    from_sequence: None,
                    last_event_id: Some(format!("{run_id}:10")),
                },
                &headers,
            ),
            Some(10)
        );
        assert_eq!(
            debug_run_stream_from_sequence(
                run_id,
                &DebugRunStreamQuery {
                    from_sequence: None,
                    last_event_id: None,
                },
                &headers,
            ),
            Some(11)
        );
    }

    #[test]
    fn usage_total_tokens_uses_total_or_known_segments() {
        assert_eq!(
            usage_total_tokens(&serde_json::json!({
                "total_tokens": 128,
                "input_tokens": 40,
                "output_tokens": 12
            })),
            Some(128)
        );
        assert_eq!(
            usage_total_tokens(&serde_json::json!({
                "input_tokens": 40,
                "output_tokens": 12,
                "reasoning_tokens": 6
            })),
            Some(58)
        );
        assert_eq!(usage_total_tokens(&serde_json::json!({})), None);
    }

    #[test]
    fn callback_task_tool_callback_count_reads_offloaded_tool_call_count() {
        let base_task = domain::CallbackTaskRecord {
            id: Uuid::now_v7(),
            flow_run_id: Uuid::now_v7(),
            node_run_id: Uuid::now_v7(),
            callback_kind: "llm_tool_calls".to_string(),
            status: domain::CallbackTaskStatus::Completed,
            request_payload: serde_json::json!({
                "tool_calls": [
                    { "id": "call-1" },
                    { "id": "call-2" }
                ]
            }),
            response_payload: None,
            external_ref_payload: None,
            created_at: OffsetDateTime::UNIX_EPOCH,
            completed_at: Some(OffsetDateTime::UNIX_EPOCH),
        };
        assert_eq!(callback_task_tool_callback_count(&base_task), 2);

        let offloaded_task = domain::CallbackTaskRecord {
            request_payload: serde_json::json!({
                "tool_calls": {
                    "__runtime_debug_artifact": true,
                    "artifact_ref": Uuid::now_v7().to_string(),
                    "tool_call_count": 3
                }
            }),
            ..base_task
        };
        assert_eq!(callback_task_tool_callback_count(&offloaded_task), 3);
    }

    #[test]
    fn start_node_response_moves_legacy_output_payload_into_input() {
        let run = domain::NodeRunRecord {
            id: Uuid::now_v7(),
            flow_run_id: Uuid::now_v7(),
            node_id: "node-start".to_string(),
            node_type: "start".to_string(),
            node_alias: "Start".to_string(),
            status: domain::NodeRunStatus::Succeeded,
            input_payload: serde_json::json!({}),
            output_payload: serde_json::json!({
                "query": "ping",
                "tools": [
                    {
                        "name": "read_file",
                        "source": "openai_compatible"
                    }
                ]
            }),
            error_payload: None,
            metrics_payload: serde_json::json!({}),
            debug_payload: serde_json::json!({}),
            started_at: OffsetDateTime::UNIX_EPOCH,
            finished_at: Some(OffsetDateTime::UNIX_EPOCH),
        };

        let response = to_node_run_response(run);

        assert_eq!(response.input_payload["query"], serde_json::json!("ping"));
        assert_eq!(
            response.input_payload["tools"][0]["name"],
            serde_json::json!("read_file")
        );
        assert_eq!(response.output_payload, serde_json::json!({}));
    }

    #[test]
    fn start_node_response_exposes_input_payload_truth_view() {
        let artifact_ref = Uuid::now_v7().to_string();
        let run = domain::NodeRunRecord {
            id: Uuid::now_v7(),
            flow_run_id: Uuid::now_v7(),
            node_id: "node-start".to_string(),
            node_type: "start".to_string(),
            node_alias: "Start".to_string(),
            status: domain::NodeRunStatus::Succeeded,
            input_payload: serde_json::json!({
                "query": "say hello",
                "model": "deepseek-chat",
                "files": [{ "name": "brief.md" }],
                "sys": {
                    "workflow_run_id": "run-1"
                },
                "env": {
                    "ApiBaseUrl": "https://api.example.com"
                },
                "history": {
                    "__runtime_debug_artifact": true,
                    "artifact_ref": artifact_ref,
                    "is_truncated": true,
                    "field_path": ["history"],
                    "preview": "[{\"role\":\"user\",\"content\":\"old"
                },
                "tools": []
            }),
            output_payload: serde_json::json!({ "query": "say hello" }),
            error_payload: None,
            metrics_payload: serde_json::json!({}),
            debug_payload: serde_json::json!({}),
            started_at: OffsetDateTime::UNIX_EPOCH,
            finished_at: Some(OffsetDateTime::UNIX_EPOCH),
        };

        let response = to_node_run_response(run);

        assert_eq!(response.input_payload["query"], "say hello");
        assert_eq!(response.input_payload["model"], "deepseek-chat");
        assert_eq!(response.input_payload["sys"]["workflow_run_id"], "run-1");
        assert_eq!(
            response.input_payload["env"]["ApiBaseUrl"],
            "https://api.example.com"
        );
        assert_eq!(
            response.input_payload["history"]["field_path"],
            serde_json::json!(["history"])
        );
        assert_eq!(response.input_payload_view, response.input_payload);
    }

    fn test_application_record() -> domain::ApplicationRecord {
        domain::ApplicationRecord {
            id: Uuid::now_v7(),
            workspace_id: Uuid::now_v7(),
            application_type: domain::ApplicationType::AgentFlow,
            name: "Support Agent".to_string(),
            description: "runtime".to_string(),
            icon: None,
            icon_type: None,
            icon_background: None,
            created_by: Uuid::now_v7(),
            updated_at: OffsetDateTime::UNIX_EPOCH,
            tags: Vec::new(),
            sections: domain::ApplicationSections {
                orchestration: domain::ApplicationOrchestrationSection {
                    status: "enabled".to_string(),
                    subject_kind: "flow".to_string(),
                    subject_status: "draft".to_string(),
                    current_subject_id: Some(Uuid::now_v7()),
                    current_draft_id: Some(Uuid::now_v7()),
                },
                api: domain::ApplicationApiSection {
                    status: "enabled".to_string(),
                    credential_kind: "api_key".to_string(),
                    invoke_routing_mode: "application".to_string(),
                    invoke_path_template: None,
                    api_capability_status: "enabled".to_string(),
                    credentials_status: "enabled".to_string(),
                },
                logs: domain::ApplicationLogsSection {
                    status: "enabled".to_string(),
                    runs_capability_status: "enabled".to_string(),
                    run_object_kind: "application_run".to_string(),
                    log_retention_status: "default".to_string(),
                },
                monitoring: domain::ApplicationMonitoringSection {
                    status: "enabled".to_string(),
                    metrics_capability_status: "enabled".to_string(),
                    metrics_object_kind: "application_run".to_string(),
                    tracing_config_status: "default".to_string(),
                },
            },
        }
    }

    fn test_flow_run_record(
        application_id: Uuid,
        flow_run_id: Uuid,
        status: domain::FlowRunStatus,
        output_payload: serde_json::Value,
    ) -> domain::FlowRunRecord {
        domain::FlowRunRecord {
            id: flow_run_id,
            application_id,
            flow_id: Uuid::now_v7(),
            draft_id: Uuid::now_v7(),
            compiled_plan_id: Some(Uuid::now_v7()),
            debug_session_id: "debug-session".to_string(),
            flow_schema_version: "1flowbase.flow/v2".to_string(),
            document_hash: "hash".to_string(),
            run_mode: domain::FlowRunMode::DebugFlowRun,
            target_node_id: None,
            title: "天气？".to_string(),
            status,
            input_payload: serde_json::json!({
                "node-start": {
                    "query": "天气？"
                }
            }),
            output_payload,
            error_payload: None,
            created_by: Uuid::now_v7(),
            authorized_account: Some("root".to_string()),
            api_key_id: None,
            publication_version_id: None,
            external_user: None,
            external_conversation_id: None,
            external_trace_id: None,
            compatibility_mode: None,
            idempotency_key: None,
            started_at: OffsetDateTime::UNIX_EPOCH,
            finished_at: None,
            created_at: OffsetDateTime::UNIX_EPOCH,
            updated_at: OffsetDateTime::UNIX_EPOCH,
        }
    }

    #[test]
    fn run_detail_response_moves_waiting_prefix_answer_into_answer_snapshot() {
        let application = test_application_record();
        let flow_run_id = Uuid::now_v7();
        let waiting_node_run_id = Uuid::now_v7();
        let virtual_answer_node_run_id = Uuid::now_v7();
        let detail = domain::ApplicationRunDetail {
            flow_run: test_flow_run_record(
                application.id,
                flow_run_id,
                domain::FlowRunStatus::WaitingCallback,
                serde_json::json!({ "answer": "LLM1 final\n----\n" }),
            ),
            node_runs: vec![
                domain::NodeRunRecord {
                    id: waiting_node_run_id,
                    flow_run_id,
                    node_id: "node-llm-2".to_string(),
                    node_type: "llm".to_string(),
                    node_alias: "LLM2".to_string(),
                    status: domain::NodeRunStatus::WaitingCallback,
                    input_payload: serde_json::json!({}),
                    output_payload: serde_json::json!({ "tool_calls": [] }),
                    error_payload: None,
                    metrics_payload: serde_json::json!({}),
                    debug_payload: serde_json::json!({}),
                    started_at: OffsetDateTime::UNIX_EPOCH,
                    finished_at: None,
                },
                domain::NodeRunRecord {
                    id: virtual_answer_node_run_id,
                    flow_run_id,
                    node_id: "node-answer".to_string(),
                    node_type: "answer".to_string(),
                    node_alias: "Answer".to_string(),
                    status: domain::NodeRunStatus::Succeeded,
                    input_payload: serde_json::json!({
                        "presentation": {
                            "kind": "answer",
                            "complete": false,
                            "materialized_from": "waiting_prefix"
                        }
                    }),
                    output_payload: serde_json::json!({
                        "answer": "LLM1 final\n----\n"
                    }),
                    error_payload: None,
                    metrics_payload: serde_json::json!({}),
                    debug_payload: serde_json::json!({
                        "answer_presentation": {
                            "partial": true,
                            "materialized_from": "waiting_prefix"
                        }
                    }),
                    started_at: OffsetDateTime::UNIX_EPOCH,
                    finished_at: Some(OffsetDateTime::UNIX_EPOCH),
                },
            ],
            checkpoints: vec![domain::CheckpointRecord {
                id: Uuid::now_v7(),
                flow_run_id,
                node_run_id: Some(waiting_node_run_id),
                status: "waiting_callback".to_string(),
                reason: "等待 callback 回填".to_string(),
                locator_payload: serde_json::json!({
                    "node_id": "node-llm-2",
                    "next_node_index": 2
                }),
                variable_snapshot: serde_json::json!({}),
                external_ref_payload: None,
                created_at: OffsetDateTime::UNIX_EPOCH,
            }],
            callback_tasks: Vec::new(),
            events: Vec::new(),
        };

        let response = to_application_run_detail_response(&application, detail);

        assert_eq!(response.node_runs.len(), 1);
        assert_eq!(response.node_runs[0].node_id, "node-llm-2");
        let answer_snapshot = response
            .answer_snapshot
            .expect("waiting_prefix answer should become answer_snapshot");
        assert_eq!(answer_snapshot.text, "LLM1 final\n----\n");
        assert!(!answer_snapshot.complete);
        assert_eq!(answer_snapshot.materialized_from, "waiting_prefix");
        assert_eq!(answer_snapshot.answer_node_id, "node-answer");
        assert_eq!(
            answer_snapshot.answer_node_run_id,
            virtual_answer_node_run_id.to_string()
        );
        assert_eq!(
            answer_snapshot.waiting_node_id.as_deref(),
            Some("node-llm-2")
        );
        assert_eq!(
            answer_snapshot.waiting_node_run_id.as_deref(),
            Some(waiting_node_run_id.to_string().as_str())
        );
        assert!(response
            .node_runs
            .iter()
            .all(|node_run| node_run.node_id != "node-answer"));
    }

    #[test]
    fn run_detail_response_hides_historical_waiting_prefix_after_run_finishes() {
        let application = test_application_record();
        let flow_run_id = Uuid::now_v7();
        let waiting_node_run_id = Uuid::now_v7();
        let virtual_answer_node_run_id = Uuid::now_v7();
        let final_answer_node_run_id = Uuid::now_v7();
        let detail = domain::ApplicationRunDetail {
            flow_run: test_flow_run_record(
                application.id,
                flow_run_id,
                domain::FlowRunStatus::Succeeded,
                serde_json::json!({ "answer": "final answer" }),
            ),
            node_runs: vec![
                domain::NodeRunRecord {
                    id: waiting_node_run_id,
                    flow_run_id,
                    node_id: "node-llm-2".to_string(),
                    node_type: "llm".to_string(),
                    node_alias: "LLM2".to_string(),
                    status: domain::NodeRunStatus::Succeeded,
                    input_payload: serde_json::json!({}),
                    output_payload: serde_json::json!({ "text": "final answer" }),
                    error_payload: None,
                    metrics_payload: serde_json::json!({}),
                    debug_payload: serde_json::json!({}),
                    started_at: OffsetDateTime::UNIX_EPOCH,
                    finished_at: Some(OffsetDateTime::UNIX_EPOCH),
                },
                domain::NodeRunRecord {
                    id: virtual_answer_node_run_id,
                    flow_run_id,
                    node_id: "node-answer".to_string(),
                    node_type: "answer".to_string(),
                    node_alias: "Answer".to_string(),
                    status: domain::NodeRunStatus::Succeeded,
                    input_payload: serde_json::json!({
                        "presentation": {
                            "kind": "answer",
                            "complete": false,
                            "materialized_from": "waiting_prefix"
                        }
                    }),
                    output_payload: serde_json::json!({
                        "answer": "prefix answer"
                    }),
                    error_payload: None,
                    metrics_payload: serde_json::json!({}),
                    debug_payload: serde_json::json!({
                        "answer_presentation": {
                            "partial": true,
                            "materialized_from": "waiting_prefix"
                        }
                    }),
                    started_at: OffsetDateTime::UNIX_EPOCH,
                    finished_at: Some(OffsetDateTime::UNIX_EPOCH),
                },
                domain::NodeRunRecord {
                    id: final_answer_node_run_id,
                    flow_run_id,
                    node_id: "node-answer".to_string(),
                    node_type: "answer".to_string(),
                    node_alias: "Answer".to_string(),
                    status: domain::NodeRunStatus::Succeeded,
                    input_payload: serde_json::json!({}),
                    output_payload: serde_json::json!({
                        "answer": "final answer"
                    }),
                    error_payload: None,
                    metrics_payload: serde_json::json!({}),
                    debug_payload: serde_json::json!({}),
                    started_at: OffsetDateTime::UNIX_EPOCH,
                    finished_at: Some(OffsetDateTime::UNIX_EPOCH),
                },
            ],
            checkpoints: vec![domain::CheckpointRecord {
                id: Uuid::now_v7(),
                flow_run_id,
                node_run_id: Some(waiting_node_run_id),
                status: "waiting_callback".to_string(),
                reason: "历史等待点".to_string(),
                locator_payload: serde_json::json!({
                    "node_id": "node-llm-2"
                }),
                variable_snapshot: serde_json::json!({}),
                external_ref_payload: None,
                created_at: OffsetDateTime::UNIX_EPOCH,
            }],
            callback_tasks: Vec::new(),
            events: Vec::new(),
        };

        let response = to_application_run_detail_response(&application, detail);

        assert!(response.answer_snapshot.is_none());
        assert!(response
            .node_runs
            .iter()
            .all(|node_run| node_run.id != virtual_answer_node_run_id.to_string()));
        assert!(response
            .node_runs
            .iter()
            .any(|node_run| node_run.id == final_answer_node_run_id.to_string()));
    }

    #[test]
    fn flow_run_response_exposes_query_and_model_short_fields() {
        let run = domain::FlowRunRecord {
            id: Uuid::now_v7(),
            application_id: Uuid::now_v7(),
            flow_id: Uuid::now_v7(),
            draft_id: Uuid::now_v7(),
            compiled_plan_id: None,
            debug_session_id: "debug-session".to_string(),
            flow_schema_version: "1flowbase.flow/v2".to_string(),
            document_hash: "hash".to_string(),
            run_mode: domain::FlowRunMode::PublishedApiRun,
            target_node_id: None,
            title: "say hello".to_string(),
            status: domain::FlowRunStatus::Succeeded,
            input_payload: serde_json::json!({
                "node-start": {
                    "query": "say hello",
                    "model": "deepseek-chat"
                }
            }),
            output_payload: serde_json::json!({ "answer": "hello" }),
            error_payload: None,
            created_by: Uuid::now_v7(),
            authorized_account: Some("root".to_string()),
            api_key_id: None,
            publication_version_id: None,
            external_user: Some("user-1".to_string()),
            external_conversation_id: Some("conversation-1".to_string()),
            external_trace_id: None,
            compatibility_mode: None,
            idempotency_key: None,
            started_at: OffsetDateTime::UNIX_EPOCH,
            finished_at: Some(OffsetDateTime::UNIX_EPOCH),
            created_at: OffsetDateTime::UNIX_EPOCH,
            updated_at: OffsetDateTime::UNIX_EPOCH,
        };

        let response = to_flow_run_response(run);

        assert_eq!(response.query.as_deref(), Some("say hello"));
        assert_eq!(response.model.as_deref(), Some("deepseek-chat"));
        assert_eq!(
            response.external_conversation_id.as_deref(),
            Some("conversation-1")
        );
    }

    #[tokio::test]
    async fn run_conversation_without_external_conversation_id_reads_imported_history_and_current_turn(
    ) {
        let run_id = Uuid::now_v7();
        let run = domain::FlowRunRecord {
            id: run_id,
            application_id: Uuid::now_v7(),
            flow_id: Uuid::now_v7(),
            draft_id: Uuid::now_v7(),
            compiled_plan_id: None,
            debug_session_id: "debug-session".to_string(),
            flow_schema_version: "1flowbase.flow/v2".to_string(),
            document_hash: "hash".to_string(),
            run_mode: domain::FlowRunMode::PublishedApiRun,
            target_node_id: None,
            title: "current question".to_string(),
            status: domain::FlowRunStatus::Succeeded,
            input_payload: serde_json::json!({
                "node-start": {
                    "query": "current question",
                    "model": "deepseek-chat",
                    "history": [
                        { "role": "system", "content": "hidden" },
                        { "role": "user", "content": "old question 1" },
                        { "role": "assistant", "content": "old answer 1" },
                        { "role": "tool", "content": "tool payload" },
                        { "role": "user", "content": "old question 2" },
                        { "role": "assistant", "content": "old answer 2" }
                    ]
                }
            }),
            output_payload: serde_json::json!({ "answer": "current answer" }),
            error_payload: None,
            created_by: Uuid::now_v7(),
            authorized_account: Some("root".to_string()),
            api_key_id: None,
            publication_version_id: None,
            external_user: None,
            external_conversation_id: None,
            external_trace_id: None,
            compatibility_mode: None,
            idempotency_key: None,
            started_at: OffsetDateTime::UNIX_EPOCH,
            finished_at: Some(OffsetDateTime::UNIX_EPOCH),
            created_at: OffsetDateTime::UNIX_EPOCH,
            updated_at: OffsetDateTime::UNIX_EPOCH,
        };

        let page = conversation_messages_from_single_run(
            &run,
            &ApplicationConversationMessagesQuery {
                around_run_id: None,
                before: None,
                after: None,
                limit: Some(2),
            },
            |_| async { None::<serde_json::Value> },
        )
        .await;

        assert_eq!(page.items.len(), 3);
        assert!(page.page.has_before);
        assert!(!page.page.has_after);
        assert_eq!(page.items[0].role.as_deref(), Some("system"));
        assert_eq!(page.items[0].content.as_deref(), Some("hidden"));
        assert_eq!(page.items[0].query, None);
        assert_eq!(page.items[0].answer, None);
        assert!(!page.items[0].can_open_detail);
        assert_eq!(page.items[0].detail_run_id, None);
        assert_eq!(page.items[1].role.as_deref(), Some("assistant"));
        assert_eq!(page.items[1].content.as_deref(), Some("old answer 2"));
        assert_eq!(page.items[1].query, None);
        assert_eq!(page.items[1].answer, None);
        assert!(!page.items[1].can_open_detail);
        assert_eq!(page.items[1].detail_run_id, None);
        assert_eq!(page.items[2].run_id, run_id.to_string());
        assert_eq!(page.items[2].role, None);
        assert_eq!(page.items[2].content, None);
        assert_eq!(page.items[2].query.as_deref(), Some("current question"));
        assert_eq!(page.items[2].answer.as_deref(), Some("current answer"));
        assert!(page.items[2].can_open_detail);
        let run_id_string = run_id.to_string();
        assert_eq!(
            page.items[2].detail_run_id.as_deref(),
            Some(run_id_string.as_str())
        );
    }

    #[tokio::test]
    async fn run_conversation_without_external_conversation_id_reads_artifact_backed_imported_history(
    ) {
        let run_id = Uuid::now_v7();
        let artifact_id = Uuid::now_v7();
        let run = domain::FlowRunRecord {
            id: run_id,
            application_id: Uuid::now_v7(),
            flow_id: Uuid::now_v7(),
            draft_id: Uuid::now_v7(),
            compiled_plan_id: None,
            debug_session_id: "debug-session".to_string(),
            flow_schema_version: "1flowbase.flow/v2".to_string(),
            document_hash: "hash".to_string(),
            run_mode: domain::FlowRunMode::PublishedApiRun,
            target_node_id: None,
            title: "current question".to_string(),
            status: domain::FlowRunStatus::Succeeded,
            input_payload: serde_json::json!({
                "__runtime_debug_artifact": true,
                "artifact_ref": artifact_id.to_string(),
                "is_truncated": true,
                "query": "current question",
                "model": "deepseek-chat",
                "preview": "{\"node-start\":{\"query\":\"current question\""
            }),
            output_payload: serde_json::json!({ "answer": "current answer" }),
            error_payload: None,
            created_by: Uuid::now_v7(),
            authorized_account: Some("root".to_string()),
            api_key_id: None,
            publication_version_id: None,
            external_user: None,
            external_conversation_id: None,
            external_trace_id: None,
            compatibility_mode: None,
            idempotency_key: None,
            started_at: OffsetDateTime::UNIX_EPOCH,
            finished_at: Some(OffsetDateTime::UNIX_EPOCH),
            created_at: OffsetDateTime::UNIX_EPOCH,
            updated_at: OffsetDateTime::UNIX_EPOCH,
        };

        let page = conversation_messages_from_single_run(
            &run,
            &ApplicationConversationMessagesQuery {
                around_run_id: None,
                before: None,
                after: None,
                limit: Some(5),
            },
            move |requested_artifact_id: Uuid| async move {
                (requested_artifact_id == artifact_id).then(|| {
                    serde_json::json!({
                        "node-start": {
                            "query": "current question",
                            "model": "deepseek-chat",
                            "history": [
                                { "role": "system", "content": "hidden" },
                                { "role": "user", "content": "old question" },
                                { "role": "assistant", "content": "old answer" }
                            ]
                        }
                    })
                })
            },
        )
        .await;

        assert_eq!(page.items.len(), 4);
        assert_eq!(page.items[0].role.as_deref(), Some("system"));
        assert_eq!(page.items[0].content.as_deref(), Some("hidden"));
        assert!(!page.items[0].can_open_detail);
        assert_eq!(page.items[1].role.as_deref(), Some("user"));
        assert_eq!(page.items[1].content.as_deref(), Some("old question"));
        assert_eq!(page.items[2].role.as_deref(), Some("assistant"));
        assert_eq!(page.items[2].content.as_deref(), Some("old answer"));
        assert_eq!(page.items[3].run_id, run_id.to_string());
        assert_eq!(page.items[3].query.as_deref(), Some("current question"));
        assert!(page.items[3].can_open_detail);
    }
}
