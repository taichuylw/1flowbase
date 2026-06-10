use std::collections::BTreeSet;

use orchestration_runtime::compiled_plan::{CompiledNode, CompiledPlan};
use serde_json::{json, Map, Value};

use super::helpers::materialize_ready_answer_node_run;
use super::*;

pub(super) struct WaitingNodeContext<'a, R, H> {
    pub(super) service: &'a OrchestrationRuntimeService<R, H>,
    pub(super) command: &'a ContinueFlowDebugRunCommand,
    pub(super) flow_run: &'a domain::FlowRunRecord,
    pub(super) compiled_plan: &'a CompiledPlan,
    pub(super) variable_pool: &'a Map<String, Value>,
    pub(super) active_node_ids: &'a BTreeSet<String>,
    pub(super) node_id: &'a str,
    pub(super) node: &'a CompiledNode,
    pub(super) node_run: &'a domain::NodeRunRecord,
}

pub(super) async fn wait_for_human_input<R, H>(
    context: WaitingNodeContext<'_, R, H>,
    rendered_templates: &Map<String, Value>,
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
    let service = context.service;
    let command = context.command;
    let flow_run = context.flow_run;
    let node = context.node;
    let node_run = context.node_run;

    update_node_run_and_emit(
        service,
        flow_run.id,
        &UpdateNodeRunInput {
            node_run_id: node_run.id,
            status: domain::NodeRunStatus::WaitingHuman,
            output_payload: json!({}),
            error_payload: None,
            metrics_payload: json!({ "preview_mode": true, "waiting": "human_input" }),
            debug_payload: json!({}),
            finished_at: None,
        },
    )
    .await?;

    if is_run_cancelled(&service.repository, command.application_id, flow_run.id).await? {
        return load_run_detail(&service.repository, command.application_id, flow_run.id).await;
    }

    let prompt = rendered_templates
        .get("prompt")
        .and_then(Value::as_str)
        .unwrap_or("请提供人工输入");
    ensure_flow_run_transition(
        domain::FlowRunStatus::Running,
        domain::FlowRunStatus::WaitingHuman,
        "continue_flow_debug_run",
    )?;
    let answer_output_payload = materialize_ready_answer_node_run(
        service,
        flow_run.id,
        context.compiled_plan,
        context.variable_pool,
    )
    .await?
    .unwrap_or_else(|| json!({}));
    let next_index = next_node_index(context.compiled_plan, context.node_id)?;
    let mut checkpoint_active_node_ids = context.active_node_ids.clone();
    orchestration_runtime::execution_engine::branching::activate_downstream_nodes(
        context.compiled_plan,
        &mut checkpoint_active_node_ids,
        node,
        None,
    );
    service
        .repository
        .create_checkpoint(&CreateCheckpointInput {
            flow_run_id: flow_run.id,
            node_run_id: Some(node_run.id),
            status: "waiting_human".to_string(),
            reason: "等待人工输入".to_string(),
            locator_payload: CheckpointLocatorPayload::from_runtime_position(
                &node.node_id,
                next_index,
                orchestration_runtime::execution_engine::branching::checkpoint_active_node_ids(
                    &checkpoint_active_node_ids,
                ),
            )
            .into_json(),
            variable_snapshot: Value::Object(context.variable_pool.clone()),
            external_ref_payload: Some(json!({ "prompt": prompt })),
        })
        .await?;
    if service
        .repository
        .update_flow_run_if_status(
            &UpdateFlowRunInput {
                flow_run_id: flow_run.id,
                status: domain::FlowRunStatus::WaitingHuman,
                output_payload: answer_output_payload,
                error_payload: None,
                finished_at: None,
            },
            domain::FlowRunStatus::Running,
        )
        .await?
        .is_none()
    {
        return load_run_detail(&service.repository, command.application_id, flow_run.id).await;
    }
    append_runtime_event(
        service,
        flow_run.id,
        debug_stream_events::waiting_human(flow_run.id, node_run.id, &node.node_id),
    )
    .await;
    close_runtime_event_stream(service, flow_run.id, RuntimeEventCloseReason::WaitingHuman).await;
    load_run_detail(&service.repository, command.application_id, flow_run.id).await
}

pub(super) async fn wait_for_tool_callback<R, H>(
    context: WaitingNodeContext<'_, R, H>,
    resolved_inputs: &Map<String, Value>,
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
    let service = context.service;
    let command = context.command;
    let flow_run = context.flow_run;
    let node = context.node;
    let node_run = context.node_run;

    let request_payload = Value::Object(resolved_inputs.clone());
    update_node_run_and_emit(
        service,
        flow_run.id,
        &UpdateNodeRunInput {
            node_run_id: node_run.id,
            status: domain::NodeRunStatus::WaitingCallback,
            output_payload: json!({}),
            error_payload: None,
            metrics_payload: json!({ "preview_mode": true, "waiting": node.node_type }),
            debug_payload: json!({}),
            finished_at: None,
        },
    )
    .await?;

    if is_run_cancelled(&service.repository, command.application_id, flow_run.id).await? {
        return load_run_detail(&service.repository, command.application_id, flow_run.id).await;
    }

    ensure_flow_run_transition(
        domain::FlowRunStatus::Running,
        domain::FlowRunStatus::WaitingCallback,
        "continue_flow_debug_run",
    )?;
    let answer_output_payload = materialize_ready_answer_node_run(
        service,
        flow_run.id,
        context.compiled_plan,
        context.variable_pool,
    )
    .await?
    .unwrap_or_else(|| json!({}));
    let next_index = next_node_index(context.compiled_plan, context.node_id)?;
    let mut checkpoint_active_node_ids = context.active_node_ids.clone();
    orchestration_runtime::execution_engine::branching::activate_downstream_nodes(
        context.compiled_plan,
        &mut checkpoint_active_node_ids,
        node,
        None,
    );
    service
        .repository
        .create_checkpoint(&CreateCheckpointInput {
            flow_run_id: flow_run.id,
            node_run_id: Some(node_run.id),
            status: "waiting_callback".to_string(),
            reason: "等待 callback 回填".to_string(),
            locator_payload: CheckpointLocatorPayload::from_runtime_position(
                &node.node_id,
                next_index,
                orchestration_runtime::execution_engine::branching::checkpoint_active_node_ids(
                    &checkpoint_active_node_ids,
                ),
            )
            .into_json(),
            variable_snapshot: Value::Object(context.variable_pool.clone()),
            external_ref_payload: Some(request_payload.clone()),
        })
        .await?;
    let callback_task = service
        .repository
        .create_callback_task(&CreateCallbackTaskInput {
            flow_run_id: flow_run.id,
            node_run_id: node_run.id,
            callback_kind: node.node_type.clone(),
            request_payload: request_payload.clone(),
            external_ref_payload: Some(request_payload),
        })
        .await?;
    if service
        .repository
        .update_flow_run_if_status(
            &UpdateFlowRunInput {
                flow_run_id: flow_run.id,
                status: domain::FlowRunStatus::WaitingCallback,
                output_payload: answer_output_payload,
                error_payload: None,
                finished_at: None,
            },
            domain::FlowRunStatus::Running,
        )
        .await?
        .is_none()
    {
        return load_run_detail(&service.repository, command.application_id, flow_run.id).await;
    }
    append_runtime_event(
        service,
        flow_run.id,
        debug_stream_events::waiting_callback_with_task(
            flow_run.id,
            node_run.id,
            &node.node_id,
            &callback_task,
        ),
    )
    .await;
    close_runtime_event_stream(
        service,
        flow_run.id,
        RuntimeEventCloseReason::WaitingCallback,
    )
    .await;
    load_run_detail(&service.repository, command.application_id, flow_run.id).await
}
