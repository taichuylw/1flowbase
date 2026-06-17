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
    application_run_model, application_run_query,
    enrich_application_run_detail_visible_internal_llm_route_traces,
    enrich_application_run_detail_visible_internal_llm_route_trace_summaries,
    enrich_node_last_run_visible_internal_llm_route_traces, load_runtime_debug_artifact_json_value,
    load_runtime_debug_artifact_response, offload_application_run_detail_artifacts,
};

fn api_provider_runtime(state: &ApiState) -> ApiProviderRuntime {
    ApiProviderRuntime::new_with_activity(
        state.provider_runtime.clone(),
        state.runtime_activity.clone(),
    )
}

include!("application_runtime/types.rs");

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
            "/applications/:id/logs/runs/:run_id/conversation-log",
            get(get_application_run_conversation_log_detail),
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

include!("application_runtime/summary_responses.rs");

include!("application_runtime/conversation_helpers.rs");

include!("application_runtime/detail_responses.rs");

include!("application_runtime/debug_handlers.rs");

include!("application_runtime/log_handlers.rs");

#[cfg(test)]
mod tests;
