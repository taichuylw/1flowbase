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
    application_run_model, application_run_query, load_runtime_debug_artifact_json_value,
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
    fn metrics_payload_cache_hit_tokens_accepts_cache_read_tokens() {
        assert_eq!(
            metrics_payload_cache_hit_tokens(&serde_json::json!({
                "usage": {
                    "input_cache_hit_tokens": null,
                    "cache_read_tokens": 29_504
                }
            })),
            Some(29_504)
        );
        assert_eq!(
            metrics_payload_cache_hit_tokens(&serde_json::json!({
                "usage": {
                    "input_cache_hit_tokens": 11,
                    "cache_read_tokens": 29_504
                }
            })),
            Some(11)
        );
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

        let load_debug_artifact = |_| async { None::<serde_json::Value> };
        let page = conversation_messages_from_single_run(
            &run,
            &ApplicationConversationMessagesQuery {
                around_run_id: None,
                before: None,
                after: None,
                limit: Some(2),
            },
            &load_debug_artifact,
        )
        .await;

        assert_eq!(page.items.len(), 2);
        assert!(page.page.has_before);
        assert!(!page.page.has_after);
        let before_cursor = page
            .page
            .before_cursor
            .clone()
            .expect("initial page should expose earlier context cursor");
        assert_eq!(page.items[0].role.as_deref(), Some("assistant"));
        assert_eq!(page.items[0].content.as_deref(), Some("old answer 2"));
        assert_eq!(page.items[0].query, None);
        assert_eq!(page.items[0].answer, None);
        assert!(!page.items[0].can_open_detail);
        assert_eq!(page.items[0].detail_run_id, None);
        assert_eq!(page.items[1].run_id, run_id.to_string());
        assert_eq!(page.items[1].role, None);
        assert_eq!(page.items[1].content, None);
        assert_eq!(page.items[1].query.as_deref(), Some("current question"));
        assert_eq!(page.items[1].answer.as_deref(), Some("current answer"));
        assert!(page.items[1].can_open_detail);
        let run_id_string = run_id.to_string();
        assert_eq!(
            page.items[1].detail_run_id.as_deref(),
            Some(run_id_string.as_str())
        );

        let previous_page = conversation_messages_from_single_run(
            &run,
            &ApplicationConversationMessagesQuery {
                around_run_id: None,
                before: Some(before_cursor),
                after: None,
                limit: Some(5),
            },
            &load_debug_artifact,
        )
        .await;
        assert_eq!(previous_page.items.len(), 4);
        assert!(!previous_page.page.has_before);
        assert!(previous_page.page.has_after);
        assert_eq!(previous_page.items[0].role.as_deref(), Some("system"));
        assert_eq!(previous_page.items[0].content.as_deref(), Some("hidden"));
        assert_eq!(previous_page.items[1].role.as_deref(), Some("user"));
        assert_eq!(
            previous_page.items[1].content.as_deref(),
            Some("old question 1")
        );
    }

    #[tokio::test]
    async fn run_conversation_hides_claude_code_control_history_from_imported_context() {
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
                    "query": "那你帮我拉一下最新代码",
                    "history": [
                        {
                            "role": "user",
                            "content": "This session is being continued from a previous conversation that ran out of context. The summary below covers the earlier portion of the conversation.\n\nSummary: hi\n\nIf you need specific details from before compaction (like exact code snippets, error messages, or content you generated), read the full transcript at: C:\\Users\\Lw\\.claude\\projects\\repo\\session.jsonl"
                        },
                        {
                            "role": "assistant",
                            "content": "已恢复上下文。"
                        },
                        { "role": "user", "content": "visible old question" },
                        { "role": "assistant", "content": "visible old answer" }
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
            compatibility_mode: Some("anthropic-messages-v1".to_string()),
            idempotency_key: None,
            started_at: OffsetDateTime::UNIX_EPOCH,
            finished_at: Some(OffsetDateTime::UNIX_EPOCH),
            created_at: OffsetDateTime::UNIX_EPOCH,
            updated_at: OffsetDateTime::UNIX_EPOCH,
        };

        let load_debug_artifact = |_| async { None::<serde_json::Value> };
        let page = conversation_messages_from_single_run(
            &run,
            &ApplicationConversationMessagesQuery {
                around_run_id: None,
                before: None,
                after: None,
                limit: Some(5),
            },
            &load_debug_artifact,
        )
        .await;

        let visible_text = page
            .items
            .iter()
            .flat_map(|item| {
                [
                    item.content.clone(),
                    item.query.clone(),
                    item.answer.clone(),
                ]
                .into_iter()
                .flatten()
            })
            .collect::<Vec<_>>()
            .join("\n");

        assert!(!visible_text.contains("This session is being continued"));
        assert!(!visible_text.contains("已恢复上下文"));
        assert!(visible_text.contains("visible old question"));
        assert!(visible_text.contains("visible old answer"));
        assert!(visible_text.contains("那你帮我拉一下最新代码"));
        assert!(visible_text.contains("current answer"));
    }

    #[tokio::test]
    async fn run_conversation_reads_llm_system_when_run_input_system_is_split_from_provider_messages(
    ) {
        let run_id = Uuid::now_v7();
        let application_id = Uuid::now_v7();
        let flow_run = domain::FlowRunRecord {
            id: run_id,
            application_id,
            flow_id: Uuid::now_v7(),
            draft_id: Uuid::now_v7(),
            compiled_plan_id: None,
            debug_session_id: "debug-session".to_string(),
            flow_schema_version: "1flowbase.flow/v2".to_string(),
            document_hash: "hash".to_string(),
            run_mode: domain::FlowRunMode::PublishedApiRun,
            target_node_id: None,
            title: "hi ?".to_string(),
            status: domain::FlowRunStatus::Succeeded,
            input_payload: serde_json::json!({
                "node-start": {
                    "query": "hi ?",
                    "model": "1flowbase"
                }
            }),
            output_payload: serde_json::json!({ "answer": "Hello!" }),
            error_payload: None,
            created_by: Uuid::now_v7(),
            authorized_account: Some("root".to_string()),
            api_key_id: None,
            publication_version_id: None,
            external_user: None,
            external_conversation_id: Some("conversation-1".to_string()),
            external_trace_id: None,
            compatibility_mode: Some("anthropic-compatible".to_string()),
            idempotency_key: None,
            started_at: OffsetDateTime::UNIX_EPOCH,
            finished_at: Some(OffsetDateTime::UNIX_EPOCH),
            created_at: OffsetDateTime::UNIX_EPOCH,
            updated_at: OffsetDateTime::UNIX_EPOCH,
        };
        let detail = domain::ApplicationRunDetail {
            flow_run,
            node_runs: vec![domain::NodeRunRecord {
                id: Uuid::now_v7(),
                flow_run_id: run_id,
                node_id: "node-llm".to_string(),
                node_type: "llm".to_string(),
                node_alias: "LLM".to_string(),
                status: domain::NodeRunStatus::Succeeded,
                input_payload: serde_json::json!({
                    "prompt_messages": [
                        {
                            "id": "user-1",
                            "role": "user",
                            "content": "hi ?"
                        }
                    ]
                }),
                output_payload: serde_json::json!({ "answer": "Hello!" }),
                error_payload: None,
                metrics_payload: serde_json::json!({}),
                debug_payload: serde_json::json!({
                    "llm_context": {
                        "effective_system": "Use the image-aware system policy.",
                        "provider_messages": [
                            { "role": "user", "content": "hi ?" }
                        ]
                    }
                }),
                started_at: OffsetDateTime::UNIX_EPOCH,
                finished_at: Some(OffsetDateTime::UNIX_EPOCH),
            }],
            checkpoints: Vec::new(),
            callback_tasks: Vec::new(),
            events: Vec::new(),
        };

        let load_debug_artifact = |_| async { None::<serde_json::Value> };
        let page = conversation_messages_from_run_detail(
            &detail,
            &ApplicationConversationMessagesQuery {
                around_run_id: None,
                before: None,
                after: None,
                limit: Some(5),
            },
            &load_debug_artifact,
        )
        .await;

        assert_eq!(page.items.len(), 2);
        assert!(!page.page.has_before);
        assert!(!page.page.has_after);
        assert_eq!(page.items[0].role.as_deref(), Some("system"));
        assert_eq!(
            page.items[0].content.as_deref(),
            Some("Use the image-aware system policy.")
        );
        assert!(!page.items[0].can_open_detail);
        assert_eq!(page.items[1].run_id, run_id.to_string());
        assert_eq!(page.items[1].query.as_deref(), Some("hi ?"));
        assert_eq!(page.items[1].answer.as_deref(), Some("Hello!"));
    }

    #[tokio::test]
    async fn llm_prompt_messages_system_content_reads_system_prompt_message() {
        let payload = serde_json::json!({
            "prompt_messages": [
                {
                    "id": "system-1",
                    "role": "system",
                    "content": "Use the node policy."
                },
                {
                    "id": "user-1",
                    "role": "user",
                    "content": "hi ?"
                }
            ]
        });
        let load_debug_artifact = |_| async { None::<serde_json::Value> };

        assert_eq!(
            llm_prompt_messages_system_content(&payload, &load_debug_artifact)
                .await
                .as_deref(),
            Some("Use the node policy.")
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

        let load_debug_artifact = move |requested_artifact_id: Uuid| async move {
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
        };
        let page = conversation_messages_from_single_run(
            &run,
            &ApplicationConversationMessagesQuery {
                around_run_id: None,
                before: None,
                after: None,
                limit: Some(5),
            },
            &load_debug_artifact,
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

    #[tokio::test]
    async fn run_conversation_hydrates_artifact_backed_current_answer() {
        let run_id = Uuid::now_v7();
        let artifact_id = Uuid::now_v7();
        let full_answer = "full final answer from artifact";
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
                    "model": "deepseek-chat"
                }
            }),
            output_payload: serde_json::json!({
                "answer": {
                    "__runtime_debug_artifact": true,
                    "artifact_ref": artifact_id.to_string(),
                    "field_path": ["answer"],
                    "preview": "preview final answer"
                }
            }),
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

        let load_debug_artifact = move |requested_artifact_id: Uuid| async move {
            (requested_artifact_id == artifact_id)
                .then(|| serde_json::Value::String(full_answer.to_string()))
        };
        let page = conversation_messages_from_single_run(
            &run,
            &ApplicationConversationMessagesQuery {
                around_run_id: None,
                before: None,
                after: None,
                limit: Some(5),
            },
            &load_debug_artifact,
        )
        .await;

        let current = page.items.last().expect("current run message exists");
        assert_eq!(current.answer.as_deref(), Some(full_answer));
    }
}
