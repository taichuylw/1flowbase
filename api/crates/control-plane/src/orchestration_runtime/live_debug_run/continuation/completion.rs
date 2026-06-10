use orchestration_runtime::compiled_plan::CompiledPlan;
use serde_json::{Map, Value};
use uuid::Uuid;

use super::*;

pub(super) struct ContinuedFlowCompletion<'a, R, H> {
    pub(super) service: &'a OrchestrationRuntimeService<R, H>,
    pub(super) command: &'a ContinueFlowDebugRunCommand,
    pub(super) flow_run: &'a domain::FlowRunRecord,
    pub(super) workspace_id: Uuid,
    pub(super) compiled_plan: &'a CompiledPlan,
    pub(super) variable_pool: &'a Map<String, Value>,
    pub(super) last_output_payload: Value,
    pub(super) pending_failure: Option<Value>,
}

pub(super) async fn finish_continued_flow_run<R, H>(
    completion: ContinuedFlowCompletion<'_, R, H>,
) -> Result<domain::ApplicationRunDetail>
where
    R: crate::ports::ApplicationRepository
        + crate::ports::FileManagementRepository
        + crate::ports::FlowRepository
        + OrchestrationRuntimeRepository
        + crate::ports::ModelDefinitionRepository
        + crate::ports::ModelProviderRepository
        + crate::ports::NodeContributionRepository
        + crate::ports::PluginRepository
        + Clone
        + Send
        + Sync
        + 'static,
    H: crate::ports::ProviderRuntimePort
        + crate::capability_plugin_runtime::CapabilityPluginRuntimePort
        + Clone,
{
    let service = completion.service;
    let command = completion.command;
    let flow_run = completion.flow_run;

    if is_run_cancelled(&service.repository, command.application_id, flow_run.id).await? {
        return load_run_detail(&service.repository, command.application_id, flow_run.id).await;
    }

    if let Some(error_payload) = completion.pending_failure {
        ensure_flow_run_transition(
            domain::FlowRunStatus::Running,
            domain::FlowRunStatus::Failed,
            "continue_flow_debug_run",
        )?;
        let updated = service
            .repository
            .update_flow_run_if_status(
                &UpdateFlowRunInput {
                    flow_run_id: flow_run.id,
                    status: domain::FlowRunStatus::Failed,
                    output_payload: completion.last_output_payload.clone(),
                    error_payload: Some(error_payload.clone()),
                    finished_at: Some(OffsetDateTime::now_utc()),
                },
                domain::FlowRunStatus::Running,
            )
            .await?;
        let Some(updated_flow_run) = updated else {
            return load_run_detail(&service.repository, command.application_id, flow_run.id).await;
        };
        emit_flow_failed_and_close(service, flow_run.id, error_payload.clone()).await;
        service
            .repository
            .append_run_event(&AppendRunEventInput {
                flow_run_id: flow_run.id,
                node_run_id: None,
                event_type: "flow_run_failed".to_string(),
                payload: error_payload,
            })
            .await?;
        let variable_cache =
            public_node_variable_cache(completion.compiled_plan, completion.variable_pool);
        persist_debug_variable_cache_entries(
            &service.repository,
            completion.workspace_id,
            &updated_flow_run,
            &variable_cache,
        )
        .await?;

        return load_run_detail(&service.repository, command.application_id, flow_run.id).await;
    }

    ensure_flow_run_transition(
        domain::FlowRunStatus::Running,
        domain::FlowRunStatus::Succeeded,
        "continue_flow_debug_run",
    )?;
    let updated = service
        .repository
        .update_flow_run_if_status(
            &UpdateFlowRunInput {
                flow_run_id: flow_run.id,
                status: domain::FlowRunStatus::Succeeded,
                output_payload: completion.last_output_payload.clone(),
                error_payload: None,
                finished_at: Some(OffsetDateTime::now_utc()),
            },
            domain::FlowRunStatus::Running,
        )
        .await?;
    if updated.is_none() {
        return load_run_detail(&service.repository, command.application_id, flow_run.id).await;
    }
    append_runtime_event(
        service,
        flow_run.id,
        debug_stream_events::flow_finished(flow_run.id, completion.last_output_payload.clone()),
    )
    .await;
    close_runtime_event_stream(service, flow_run.id, RuntimeEventCloseReason::Finished).await;
    service
        .repository
        .append_run_event(&AppendRunEventInput {
            flow_run_id: flow_run.id,
            node_run_id: None,
            event_type: "flow_run_completed".to_string(),
            payload: completion.last_output_payload,
        })
        .await?;
    let updated_flow_run = updated.expect("updated flow run exists after is_none check");
    let variable_cache =
        public_node_variable_cache(completion.compiled_plan, completion.variable_pool);
    persist_debug_variable_cache_entries(
        &service.repository,
        completion.workspace_id,
        &updated_flow_run,
        &variable_cache,
    )
    .await?;

    load_run_detail(&service.repository, command.application_id, flow_run.id).await
}
