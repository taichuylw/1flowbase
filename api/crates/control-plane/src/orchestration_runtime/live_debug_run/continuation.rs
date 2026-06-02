use std::sync::Arc;

use anyhow::{anyhow, Result};
use serde_json::{json, Map, Value};
use time::OffsetDateTime;
use tokio::sync::{mpsc, Mutex};

use crate::{
    errors::ControlPlaneError,
    ports::{
        AppendRunEventInput, CreateCallbackTaskInput, CreateCheckpointInput, CreateNodeRunInput,
        OrchestrationRuntimeRepository, RuntimeEventCloseReason, UpdateFlowRunInput,
        UpdateNodeRunInput,
    },
    runtime_observability::{append_host_span, AppendHostSpanInput},
    state_transition::{ensure_flow_run_transition, ensure_node_run_transition},
};

use super::super::{
    data_model_runtime, debug_stream_events, CancelFlowRunCommand, ContinueFlowDebugRunCommand,
    LiveProviderStreamEventSender, OrchestrationRuntimeService,
};
use super::super::{
    debug_variable_cache::{persist_debug_variable_cache_entries, public_node_variable_cache},
    llm_observability_refs::apply_llm_debug_observability_refs,
    payloads::persisted_node_output_payload,
};
use super::{
    append_runtime_event, close_runtime_event_stream, emit_flow_failed_and_close, fail_flow_run,
    first_output_key, is_run_cancelled, load_run_detail, next_node_index,
    persist_llm_context_observability, run_live_event_persister, update_node_run_and_emit,
};

pub(super) async fn continue_flow_debug_run<R, H>(
    service: &OrchestrationRuntimeService<R, H>,
    command: ContinueFlowDebugRunCommand,
) -> Result<domain::ApplicationRunDetail>
where
    R: crate::ports::ApplicationRepository
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
    continue_flow_debug_run_with_optional_live_provider_events(service, command, None).await
}

pub(super) async fn continue_flow_debug_run_with_live_provider_events<R, H>(
    service: &OrchestrationRuntimeService<R, H>,
    command: ContinueFlowDebugRunCommand,
    live_provider_events: LiveProviderStreamEventSender,
) -> Result<domain::ApplicationRunDetail>
where
    R: crate::ports::ApplicationRepository
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
    continue_flow_debug_run_with_optional_live_provider_events(
        service,
        command,
        Some(live_provider_events),
    )
    .await
}

async fn continue_flow_debug_run_with_optional_live_provider_events<R, H>(
    service: &OrchestrationRuntimeService<R, H>,
    command: ContinueFlowDebugRunCommand,
    live_provider_events: Option<LiveProviderStreamEventSender>,
) -> Result<domain::ApplicationRunDetail>
where
    R: crate::ports::ApplicationRepository
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
    let result = continue_flow_debug_run_inner(service, &command, live_provider_events).await;

    match result {
        Ok(detail) => Ok(detail),
        Err(error) => fail_flow_run(service, command.application_id, command.flow_run_id, &error)
            .await
            .or(Err(error)),
    }
}

pub(super) async fn cancel_flow_run<R, H>(
    service: &OrchestrationRuntimeService<R, H>,
    command: CancelFlowRunCommand,
) -> Result<domain::ApplicationRunDetail>
where
    R: crate::ports::ApplicationRepository
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
    let actor = crate::ports::ApplicationRepository::load_actor_context_for_user(
        &service.repository,
        command.actor_user_id,
    )
    .await?;
    service
        .repository
        .get_application(actor.current_workspace_id, command.application_id)
        .await?
        .ok_or(ControlPlaneError::NotFound("application"))?;
    let flow_run = service
        .repository
        .get_flow_run(command.application_id, command.flow_run_id)
        .await?
        .ok_or_else(|| anyhow!("flow run not found"))?;
    ensure_flow_run_transition(
        flow_run.status,
        domain::FlowRunStatus::Cancelled,
        "cancel_flow_run",
    )?;
    let updated = service
        .repository
        .update_flow_run_if_status(
            &UpdateFlowRunInput {
                flow_run_id: flow_run.id,
                status: domain::FlowRunStatus::Cancelled,
                output_payload: flow_run.output_payload.clone(),
                error_payload: flow_run.error_payload.clone(),
                finished_at: Some(OffsetDateTime::now_utc()),
            },
            flow_run.status,
        )
        .await?;
    let Some(flow_run) = updated else {
        return load_run_detail(&service.repository, command.application_id, flow_run.id).await;
    };
    append_runtime_event(
        service,
        flow_run.id,
        debug_stream_events::flow_cancelled(flow_run.id),
    )
    .await;
    close_runtime_event_stream(service, flow_run.id, RuntimeEventCloseReason::Cancelled).await;
    service
        .repository
        .append_run_event(&AppendRunEventInput {
            flow_run_id: flow_run.id,
            node_run_id: None,
            event_type: "flow_run_cancelled".to_string(),
            payload: json!({
                "reason": "manual_stop",
            }),
        })
        .await?;

    load_run_detail(&service.repository, command.application_id, flow_run.id).await
}

async fn continue_flow_debug_run_inner<R, H>(
    service: &OrchestrationRuntimeService<R, H>,
    command: &ContinueFlowDebugRunCommand,
    live_provider_events: Option<LiveProviderStreamEventSender>,
) -> Result<domain::ApplicationRunDetail>
where
    R: crate::ports::ApplicationRepository
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
    let flow_run = service
        .repository
        .get_flow_run(command.application_id, command.flow_run_id)
        .await?
        .ok_or_else(|| anyhow!("flow run not found"))?;
    if flow_run.status != domain::FlowRunStatus::Running {
        return load_run_detail(&service.repository, command.application_id, flow_run.id).await;
    }
    let actor = crate::ports::ApplicationRepository::load_actor_context_for_user(
        &service.repository,
        flow_run.created_by,
    )
    .await?;
    let application = service
        .repository
        .get_application(command.workspace_id, command.application_id)
        .await?
        .ok_or(ControlPlaneError::NotFound("application"))?;
    let compiled_plan_id = flow_run
        .compiled_plan_id
        .ok_or_else(|| anyhow!("flow run compiled plan is not attached"))?;
    let compiled_record = service
        .repository
        .get_compiled_plan(compiled_plan_id)
        .await?
        .ok_or_else(|| anyhow!("compiled plan not found"))?;
    let compiled_plan: orchestration_runtime::compiled_plan::CompiledPlan =
        serde_json::from_value(compiled_record.plan)?;
    let answer_presentation =
        super::super::answer_presentation::AnswerPresentationCursor::from_plan(&compiled_plan)
            .map(|cursor| Arc::new(Mutex::new(cursor)));
    let invoker = if let Some(live_provider_events) = live_provider_events {
        service.runtime_invoker_with_live_provider_events(
            application.workspace_id,
            live_provider_events,
        )
    } else {
        service.runtime_invoker(application.workspace_id)
    }
    .for_flow_run(flow_run.id);
    let invoker = if let Some(answer_presentation) = &answer_presentation {
        invoker.with_answer_presentation(answer_presentation.clone())
    } else {
        invoker
    };
    let mut variable_pool = flow_run
        .input_payload
        .as_object()
        .cloned()
        .ok_or_else(|| anyhow!("input payload must be an object"))?;
    if !variable_pool.contains_key("env") {
        let environment_variables = service
            .repository
            .list_application_environment_variables(application.workspace_id, application.id)
            .await?;
        inject_application_environment_variables(&mut variable_pool, &environment_variables);
    }
    inject_system_variables(
        &mut variable_pool,
        &flow_run,
        compiled_plan_start_node_id(&compiled_plan),
    );
    let runtime_context =
        orchestration_runtime::execution_engine::ExecutionRuntimeContext::from_plan_input(
            &compiled_plan,
            &variable_pool,
        );
    let mut last_output_payload = json!({});
    let mut pending_failure: Option<Value> = None;
    let flow_span = append_host_span(
        &service.repository,
        AppendHostSpanInput {
            flow_run_id: flow_run.id,
            node_run_id: None,
            parent_span_id: None,
            kind: domain::RuntimeSpanKind::Flow,
            name: "debug flow".to_string(),
            started_at: flow_run.started_at,
            metadata: json!({
                "application_id": command.application_id,
                "run_mode": flow_run.run_mode.as_str(),
                "trigger_event_type": "flow_run_continued",
            }),
        },
    )
    .await?;

    for (node_index, node_id) in compiled_plan.topological_order.iter().enumerate() {
        if is_run_cancelled(&service.repository, command.application_id, flow_run.id).await? {
            return load_run_detail(&service.repository, command.application_id, flow_run.id).await;
        }

        let node = compiled_plan
            .nodes
            .get(node_id)
            .ok_or_else(|| anyhow!("compiled node missing: {node_id}"))?;
        let resolved_inputs =
            orchestration_runtime::binding_runtime::resolve_node_inputs(node, &variable_pool)?;
        let rendered_templates = orchestration_runtime::binding_runtime::render_templated_bindings(
            node,
            &resolved_inputs,
        );
        let node_started_at = OffsetDateTime::now_utc();
        let node_input_payload = if node.node_type == "start" {
            start_node_input_payload(&variable_pool, node_id)
        } else {
            Value::Object(resolved_inputs.clone())
        };
        let node_run = service
            .repository
            .create_node_run(&CreateNodeRunInput {
                flow_run_id: flow_run.id,
                node_id: node.node_id.clone(),
                node_type: node.node_type.clone(),
                node_alias: node.alias.clone(),
                status: domain::NodeRunStatus::Running,
                input_payload: node_input_payload,
                debug_payload: json!({}),
                started_at: node_started_at,
            })
            .await?;
        append_runtime_event(
            service,
            flow_run.id,
            debug_stream_events::node_started(&node_run),
        )
        .await;
        let node_span = append_host_span(
            &service.repository,
            AppendHostSpanInput {
                flow_run_id: flow_run.id,
                node_run_id: Some(node_run.id),
                parent_span_id: Some(flow_span.id),
                kind: if node.node_type == "llm" {
                    domain::RuntimeSpanKind::LlmTurn
                } else {
                    domain::RuntimeSpanKind::Node
                },
                name: node.alias.clone(),
                started_at: node_started_at,
                metadata: json!({
                    "node_id": node.node_id,
                    "node_type": node.node_type,
                }),
            },
        )
        .await?;

        match node.node_type.as_str() {
            "start" => {
                last_output_payload = variable_pool
                    .get(node_id)
                    .cloned()
                    .unwrap_or_else(|| json!({}));
                update_node_run_and_emit(
                    service,
                    flow_run.id,
                    &UpdateNodeRunInput {
                        node_run_id: node_run.id,
                        status: domain::NodeRunStatus::Succeeded,
                        output_payload: json!({}),
                        error_payload: None,
                        metrics_payload: json!({ "preview_mode": true }),
                        debug_payload: json!({}),
                        finished_at: Some(OffsetDateTime::now_utc()),
                    },
                )
                .await?;
                emit_answer_presentation_for_node(
                    service,
                    flow_run.id,
                    answer_presentation.as_ref(),
                    &node.node_id,
                    node_run.id,
                    &last_output_payload,
                )
                .await;
            }
            "llm" => {
                let persist_text_run_events = service.runtime_event_stream.is_none();
                let (persist_sender, persist_receiver) = mpsc::unbounded_channel();
                let persist_handle = tokio::spawn(run_live_event_persister(
                    service.repository.clone(),
                    flow_run.id,
                    node_run.id,
                    node_span.id,
                    persist_text_run_events,
                    persist_receiver,
                ));
                let llm_invoker = invoker.for_live_llm_node_with_persist(
                    node.node_id.clone(),
                    node_run.id,
                    persist_sender,
                );
                let execution_result = orchestration_runtime::execution_engine::execute_llm_node(
                    node,
                    &resolved_inputs,
                    &rendered_templates,
                    &variable_pool,
                    &runtime_context,
                    &llm_invoker,
                )
                .await;
                drop(llm_invoker);
                persist_handle
                    .await
                    .map_err(|e| anyhow!("persist task panicked: {e}"))??;
                let execution = execution_result?;
                let mut debug_payload = execution.debug_payload.clone();

                let public_output_payload = persisted_node_output_payload(
                    &execution.output_payload,
                    &execution.metrics_payload,
                    execution.error_payload.as_ref(),
                    &execution.debug_payload,
                );
                let refs = persist_llm_context_observability(
                    &service.repository,
                    flow_run.id,
                    node_run.id,
                    node_span.id,
                    Value::Object(resolved_inputs.clone()),
                    &execution.metrics_payload,
                    execution.error_payload.as_ref(),
                )
                .await?;
                apply_llm_debug_observability_refs(&mut debug_payload, &refs);

                if let Some(error_payload) = execution.error_payload.clone() {
                    variable_pool.insert(node.node_id.clone(), public_output_payload.clone());
                    ensure_node_run_transition(
                        domain::NodeRunStatus::Running,
                        domain::NodeRunStatus::Failed,
                        "continue_flow_debug_run",
                    )?;
                    update_node_run_and_emit(
                        service,
                        flow_run.id,
                        &UpdateNodeRunInput {
                            node_run_id: node_run.id,
                            status: domain::NodeRunStatus::Failed,
                            output_payload: public_output_payload.clone(),
                            error_payload: Some(error_payload.clone()),
                            metrics_payload: execution.metrics_payload.clone(),
                            debug_payload: debug_payload.clone(),
                            finished_at: Some(OffsetDateTime::now_utc()),
                        },
                    )
                    .await?;
                    emit_answer_presentation_for_node(
                        service,
                        flow_run.id,
                        answer_presentation.as_ref(),
                        &node.node_id,
                        node_run.id,
                        &public_output_payload,
                    )
                    .await;
                    if can_continue_to_terminal_template_nodes(&compiled_plan, node_index) {
                        pending_failure = Some(error_payload.clone());
                        continue;
                    }
                    return fail_current_live_run_after_node_error(
                        service,
                        command,
                        &flow_run,
                        node_run.id,
                        last_output_payload.clone(),
                        error_payload,
                    )
                    .await;
                }

                if let Some(wait) =
                    orchestration_runtime::execution_engine::build_llm_tool_callback_wait(
                        node,
                        &resolved_inputs,
                        &variable_pool,
                        &execution.output_payload,
                    )
                {
                    let checkpoint_variable_pool = wait.checkpoint_variable_pool.clone();
                    ensure_node_run_transition(
                        domain::NodeRunStatus::Running,
                        domain::NodeRunStatus::WaitingCallback,
                        "continue_flow_debug_run",
                    )?;
                    update_node_run_and_emit(
                        service,
                        flow_run.id,
                        &UpdateNodeRunInput {
                            node_run_id: node_run.id,
                            status: domain::NodeRunStatus::WaitingCallback,
                            output_payload: public_output_payload,
                            error_payload: None,
                            metrics_payload: execution.metrics_payload.clone(),
                            debug_payload: debug_payload.clone(),
                            finished_at: None,
                        },
                    )
                    .await?;

                    if is_run_cancelled(&service.repository, command.application_id, flow_run.id)
                        .await?
                    {
                        return load_run_detail(
                            &service.repository,
                            command.application_id,
                            flow_run.id,
                        )
                        .await;
                    }

                    ensure_flow_run_transition(
                        domain::FlowRunStatus::Running,
                        domain::FlowRunStatus::WaitingCallback,
                        "continue_flow_debug_run",
                    )?;
                    let answer_output_payload = materialize_ready_answer_node_run(
                        service,
                        flow_run.id,
                        &compiled_plan,
                        &checkpoint_variable_pool,
                    )
                    .await?
                    .unwrap_or_else(|| json!({}));
                    service
                        .repository
                        .create_checkpoint(&CreateCheckpointInput {
                            flow_run_id: flow_run.id,
                            node_run_id: Some(node_run.id),
                            status: "waiting_callback".to_string(),
                            reason: "等待 LLM 工具回调".to_string(),
                            locator_payload: json!({
                                "node_id": node.node_id,
                                "next_node_index": node_index,
                            }),
                            variable_snapshot: Value::Object(wait.checkpoint_variable_pool),
                            external_ref_payload: Some(wait.request_payload.clone()),
                        })
                        .await?;
                    let callback_task = service
                        .repository
                        .create_callback_task(&CreateCallbackTaskInput {
                            flow_run_id: flow_run.id,
                            node_run_id: node_run.id,
                            callback_kind: "llm_tool_calls".to_string(),
                            request_payload: wait.request_payload.clone(),
                            external_ref_payload: Some(wait.request_payload),
                        })
                        .await?;
                    service
                        .repository
                        .update_flow_run(&UpdateFlowRunInput {
                            flow_run_id: flow_run.id,
                            status: domain::FlowRunStatus::WaitingCallback,
                            output_payload: answer_output_payload,
                            error_payload: None,
                            finished_at: None,
                        })
                        .await?;
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
                    return load_run_detail(
                        &service.repository,
                        command.application_id,
                        flow_run.id,
                    )
                    .await;
                }

                ensure_node_run_transition(
                    domain::NodeRunStatus::Running,
                    domain::NodeRunStatus::Succeeded,
                    "continue_flow_debug_run",
                )?;
                update_node_run_and_emit(
                    service,
                    flow_run.id,
                    &UpdateNodeRunInput {
                        node_run_id: node_run.id,
                        status: domain::NodeRunStatus::Succeeded,
                        output_payload: public_output_payload.clone(),
                        error_payload: None,
                        metrics_payload: execution.metrics_payload.clone(),
                        debug_payload: debug_payload.clone(),
                        finished_at: Some(OffsetDateTime::now_utc()),
                    },
                )
                .await?;
                if is_run_cancelled(&service.repository, command.application_id, flow_run.id)
                    .await?
                {
                    return load_run_detail(
                        &service.repository,
                        command.application_id,
                        flow_run.id,
                    )
                    .await;
                }
                last_output_payload = public_output_payload.clone();
                variable_pool.insert(node.node_id.clone(), public_output_payload);
                emit_answer_presentation_for_node(
                    service,
                    flow_run.id,
                    answer_presentation.as_ref(),
                    &node.node_id,
                    node_run.id,
                    &last_output_payload,
                )
                .await;
            }
            "plugin_node" => {
                let execution =
                    orchestration_runtime::execution_engine::execute_capability_plugin_node(
                        node,
                        &resolved_inputs,
                        &rendered_templates,
                        &invoker,
                    )
                    .await?;
                let public_output_payload = persisted_node_output_payload(
                    &execution.output_payload,
                    &execution.metrics_payload,
                    execution.error_payload.as_ref(),
                    &execution.debug_payload,
                );
                let node_status = if execution.error_payload.is_some() {
                    domain::NodeRunStatus::Failed
                } else {
                    domain::NodeRunStatus::Succeeded
                };
                ensure_node_run_transition(
                    domain::NodeRunStatus::Running,
                    node_status,
                    "continue_flow_debug_run",
                )?;
                update_node_run_and_emit(
                    service,
                    flow_run.id,
                    &UpdateNodeRunInput {
                        node_run_id: node_run.id,
                        status: node_status,
                        output_payload: public_output_payload.clone(),
                        error_payload: execution.error_payload.clone(),
                        metrics_payload: execution.metrics_payload.clone(),
                        debug_payload: execution.debug_payload.clone(),
                        finished_at: Some(OffsetDateTime::now_utc()),
                    },
                )
                .await?;

                if is_run_cancelled(&service.repository, command.application_id, flow_run.id)
                    .await?
                {
                    return load_run_detail(
                        &service.repository,
                        command.application_id,
                        flow_run.id,
                    )
                    .await;
                }

                if let Some(error_payload) = execution.error_payload {
                    ensure_flow_run_transition(
                        domain::FlowRunStatus::Running,
                        domain::FlowRunStatus::Failed,
                        "continue_flow_debug_run",
                    )?;
                    service
                        .repository
                        .update_flow_run(&UpdateFlowRunInput {
                            flow_run_id: flow_run.id,
                            status: domain::FlowRunStatus::Failed,
                            output_payload: last_output_payload.clone(),
                            error_payload: Some(error_payload.clone()),
                            finished_at: Some(OffsetDateTime::now_utc()),
                        })
                        .await?;
                    emit_flow_failed_and_close(service, flow_run.id, error_payload.clone()).await;
                    service
                        .repository
                        .append_run_event(&AppendRunEventInput {
                            flow_run_id: flow_run.id,
                            node_run_id: Some(node_run.id),
                            event_type: "flow_run_failed".to_string(),
                            payload: error_payload,
                        })
                        .await?;
                    return load_run_detail(
                        &service.repository,
                        command.application_id,
                        flow_run.id,
                    )
                    .await;
                }

                last_output_payload = public_output_payload.clone();
                variable_pool.insert(node.node_id.clone(), public_output_payload);
            }
            "data_model_list" | "data_model_get" | "data_model_create" | "data_model_update"
            | "data_model_delete" => {
                let execution = data_model_runtime::execute_data_model_node(
                    service.repository.clone(),
                    service.runtime_engine.clone(),
                    &actor,
                    node,
                    &resolved_inputs,
                    &data_model_runtime::DataModelRunContext {
                        workspace_id: application.workspace_id,
                        application_id: command.application_id,
                        draft_id: flow_run.draft_id,
                        flow_run_id: flow_run.id,
                        node_run_id: node_run.id,
                    },
                )
                .await;
                if let Some(confirmation) = execution.waiting_confirmation {
                    ensure_node_run_transition(
                        domain::NodeRunStatus::Running,
                        domain::NodeRunStatus::WaitingCallback,
                        "continue_flow_debug_run",
                    )?;
                    update_node_run_and_emit(
                        service,
                        flow_run.id,
                        &UpdateNodeRunInput {
                            node_run_id: node_run.id,
                            status: domain::NodeRunStatus::WaitingCallback,
                            output_payload: json!({}),
                            error_payload: None,
                            metrics_payload: execution.metrics_payload.clone(),
                            debug_payload: json!({
                                "side_effect_policy": "confirm_each_run",
                                "idempotency_key": confirmation.idempotency_key,
                                "payload_hash": confirmation.payload_hash,
                                "expires_at": confirmation.expires_at,
                            }),
                            finished_at: None,
                        },
                    )
                    .await?;

                    if is_run_cancelled(&service.repository, command.application_id, flow_run.id)
                        .await?
                    {
                        return load_run_detail(
                            &service.repository,
                            command.application_id,
                            flow_run.id,
                        )
                        .await;
                    }

                    ensure_flow_run_transition(
                        domain::FlowRunStatus::Running,
                        domain::FlowRunStatus::WaitingCallback,
                        "continue_flow_debug_run",
                    )?;
                    let answer_output_payload = materialize_ready_answer_node_run(
                        service,
                        flow_run.id,
                        &compiled_plan,
                        &variable_pool,
                    )
                    .await?
                    .unwrap_or_else(|| json!({}));
                    let confirmation_payload = json!({
                        "kind": "data_model_side_effect_confirmation",
                        "actor_user_id": actor.user_id,
                        "node_id": node.node_id,
                        "run_id": flow_run.id,
                        "payload_hash": confirmation.payload_hash,
                        "idempotency_key": confirmation.idempotency_key,
                        "expires_at": confirmation.expires_at,
                        "request_payload": confirmation.request_payload,
                    });
                    service
                        .repository
                        .create_checkpoint(&CreateCheckpointInput {
                            flow_run_id: flow_run.id,
                            node_run_id: Some(node_run.id),
                            status: "waiting_data_model_side_effect_confirmation".to_string(),
                            reason: "等待 Data Model 写入确认".to_string(),
                            locator_payload: json!({
                                "node_id": node.node_id,
                                "next_node_index": next_node_index(&compiled_plan, node_id)?,
                            }),
                            variable_snapshot: Value::Object(variable_pool.clone()),
                            external_ref_payload: Some(confirmation_payload.clone()),
                        })
                        .await?;
                    let callback_task = service
                        .repository
                        .create_callback_task(&CreateCallbackTaskInput {
                            flow_run_id: flow_run.id,
                            node_run_id: node_run.id,
                            callback_kind: "data_model_side_effect_confirmation".to_string(),
                            request_payload: confirmation_payload.clone(),
                            external_ref_payload: Some(confirmation_payload),
                        })
                        .await?;
                    service
                        .repository
                        .update_flow_run(&UpdateFlowRunInput {
                            flow_run_id: flow_run.id,
                            status: domain::FlowRunStatus::WaitingCallback,
                            output_payload: answer_output_payload,
                            error_payload: None,
                            finished_at: None,
                        })
                        .await?;
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
                    return load_run_detail(
                        &service.repository,
                        command.application_id,
                        flow_run.id,
                    )
                    .await;
                }
                let public_output_payload = persisted_node_output_payload(
                    &execution.output_payload,
                    &execution.metrics_payload,
                    execution.error_payload.as_ref(),
                    &json!({}),
                );
                let node_status = if execution.error_payload.is_some() {
                    domain::NodeRunStatus::Failed
                } else {
                    domain::NodeRunStatus::Succeeded
                };
                ensure_node_run_transition(
                    domain::NodeRunStatus::Running,
                    node_status,
                    "continue_flow_debug_run",
                )?;
                update_node_run_and_emit(
                    service,
                    flow_run.id,
                    &UpdateNodeRunInput {
                        node_run_id: node_run.id,
                        status: node_status,
                        output_payload: public_output_payload.clone(),
                        error_payload: execution.error_payload.clone(),
                        metrics_payload: execution.metrics_payload.clone(),
                        debug_payload: json!({}),
                        finished_at: Some(OffsetDateTime::now_utc()),
                    },
                )
                .await?;

                if let Some(error_payload) = execution.error_payload {
                    ensure_flow_run_transition(
                        domain::FlowRunStatus::Running,
                        domain::FlowRunStatus::Failed,
                        "continue_flow_debug_run",
                    )?;
                    service
                        .repository
                        .update_flow_run(&UpdateFlowRunInput {
                            flow_run_id: flow_run.id,
                            status: domain::FlowRunStatus::Failed,
                            output_payload: last_output_payload.clone(),
                            error_payload: Some(error_payload.clone()),
                            finished_at: Some(OffsetDateTime::now_utc()),
                        })
                        .await?;
                    emit_flow_failed_and_close(service, flow_run.id, error_payload.clone()).await;
                    service
                        .repository
                        .append_run_event(&AppendRunEventInput {
                            flow_run_id: flow_run.id,
                            node_run_id: Some(node_run.id),
                            event_type: "flow_run_failed".to_string(),
                            payload: error_payload,
                        })
                        .await?;
                    return load_run_detail(
                        &service.repository,
                        command.application_id,
                        flow_run.id,
                    )
                    .await;
                }

                last_output_payload = public_output_payload.clone();
                variable_pool.insert(node.node_id.clone(), public_output_payload);
            }
            "template_transform" | "answer" => {
                let output_key = first_output_key(node);
                let output_value =
                    rendered_templates
                        .values()
                        .next()
                        .cloned()
                        .unwrap_or_else(|| {
                            resolved_inputs
                                .values()
                                .next()
                                .cloned()
                                .unwrap_or(Value::Null)
                        });
                let output_payload =
                    template_output_payload(node, output_key, output_value, &variable_pool);
                last_output_payload = output_payload.clone();
                variable_pool.insert(node.node_id.clone(), output_payload.clone());
                update_node_run_and_emit(
                    service,
                    flow_run.id,
                    &UpdateNodeRunInput {
                        node_run_id: node_run.id,
                        status: domain::NodeRunStatus::Succeeded,
                        output_payload,
                        error_payload: None,
                        metrics_payload: json!({ "preview_mode": true }),
                        debug_payload: json!({}),
                        finished_at: Some(OffsetDateTime::now_utc()),
                    },
                )
                .await?;
            }
            "human_input" => {
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

                if is_run_cancelled(&service.repository, command.application_id, flow_run.id)
                    .await?
                {
                    return load_run_detail(
                        &service.repository,
                        command.application_id,
                        flow_run.id,
                    )
                    .await;
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
                    &compiled_plan,
                    &variable_pool,
                )
                .await?
                .unwrap_or_else(|| json!({}));
                service
                    .repository
                    .create_checkpoint(&CreateCheckpointInput {
                        flow_run_id: flow_run.id,
                        node_run_id: Some(node_run.id),
                        status: "waiting_human".to_string(),
                        reason: "等待人工输入".to_string(),
                        locator_payload: json!({
                            "node_id": node.node_id,
                            "next_node_index": next_node_index(&compiled_plan, node_id)?,
                        }),
                        variable_snapshot: Value::Object(variable_pool.clone()),
                        external_ref_payload: Some(json!({ "prompt": prompt })),
                    })
                    .await?;
                service
                    .repository
                    .update_flow_run(&UpdateFlowRunInput {
                        flow_run_id: flow_run.id,
                        status: domain::FlowRunStatus::WaitingHuman,
                        output_payload: answer_output_payload,
                        error_payload: None,
                        finished_at: None,
                    })
                    .await?;
                append_runtime_event(
                    service,
                    flow_run.id,
                    debug_stream_events::waiting_human(flow_run.id, node_run.id, &node.node_id),
                )
                .await;
                close_runtime_event_stream(
                    service,
                    flow_run.id,
                    RuntimeEventCloseReason::WaitingHuman,
                )
                .await;
                return load_run_detail(&service.repository, command.application_id, flow_run.id)
                    .await;
            }
            "tool" | "http_request" => {
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

                if is_run_cancelled(&service.repository, command.application_id, flow_run.id)
                    .await?
                {
                    return load_run_detail(
                        &service.repository,
                        command.application_id,
                        flow_run.id,
                    )
                    .await;
                }

                ensure_flow_run_transition(
                    domain::FlowRunStatus::Running,
                    domain::FlowRunStatus::WaitingCallback,
                    "continue_flow_debug_run",
                )?;
                let answer_output_payload = materialize_ready_answer_node_run(
                    service,
                    flow_run.id,
                    &compiled_plan,
                    &variable_pool,
                )
                .await?
                .unwrap_or_else(|| json!({}));
                service
                    .repository
                    .create_checkpoint(&CreateCheckpointInput {
                        flow_run_id: flow_run.id,
                        node_run_id: Some(node_run.id),
                        status: "waiting_callback".to_string(),
                        reason: "等待 callback 回填".to_string(),
                        locator_payload: json!({
                            "node_id": node.node_id,
                            "next_node_index": next_node_index(&compiled_plan, node_id)?,
                        }),
                        variable_snapshot: Value::Object(variable_pool.clone()),
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
                service
                    .repository
                    .update_flow_run(&UpdateFlowRunInput {
                        flow_run_id: flow_run.id,
                        status: domain::FlowRunStatus::WaitingCallback,
                        output_payload: answer_output_payload,
                        error_payload: None,
                        finished_at: None,
                    })
                    .await?;
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
                return load_run_detail(&service.repository, command.application_id, flow_run.id)
                    .await;
            }
            "code" => {
                let execution = orchestration_runtime::execution_engine::execute_code_node(
                    node,
                    &resolved_inputs,
                    &invoker,
                )
                .await?;
                let public_output_payload = persisted_node_output_payload(
                    &execution.output_payload,
                    &execution.metrics_payload,
                    execution.error_payload.as_ref(),
                    &execution.debug_payload,
                );
                let node_status = if execution.error_payload.is_some() {
                    domain::NodeRunStatus::Failed
                } else {
                    domain::NodeRunStatus::Succeeded
                };
                ensure_node_run_transition(
                    domain::NodeRunStatus::Running,
                    node_status,
                    "continue_flow_debug_run",
                )?;
                update_node_run_and_emit(
                    service,
                    flow_run.id,
                    &UpdateNodeRunInput {
                        node_run_id: node_run.id,
                        status: node_status,
                        output_payload: public_output_payload.clone(),
                        error_payload: execution.error_payload.clone(),
                        metrics_payload: execution.metrics_payload.clone(),
                        debug_payload: execution.debug_payload.clone(),
                        finished_at: Some(OffsetDateTime::now_utc()),
                    },
                )
                .await?;

                if is_run_cancelled(&service.repository, command.application_id, flow_run.id)
                    .await?
                {
                    return load_run_detail(
                        &service.repository,
                        command.application_id,
                        flow_run.id,
                    )
                    .await;
                }

                if let Some(error_payload) = execution.error_payload {
                    ensure_flow_run_transition(
                        domain::FlowRunStatus::Running,
                        domain::FlowRunStatus::Failed,
                        "continue_flow_debug_run",
                    )?;
                    service
                        .repository
                        .update_flow_run(&UpdateFlowRunInput {
                            flow_run_id: flow_run.id,
                            status: domain::FlowRunStatus::Failed,
                            output_payload: last_output_payload.clone(),
                            error_payload: Some(error_payload.clone()),
                            finished_at: Some(OffsetDateTime::now_utc()),
                        })
                        .await?;
                    emit_flow_failed_and_close(service, flow_run.id, error_payload.clone()).await;
                    service
                        .repository
                        .append_run_event(&AppendRunEventInput {
                            flow_run_id: flow_run.id,
                            node_run_id: Some(node_run.id),
                            event_type: "flow_run_failed".to_string(),
                            payload: error_payload,
                        })
                        .await?;
                    return load_run_detail(
                        &service.repository,
                        command.application_id,
                        flow_run.id,
                    )
                    .await;
                }

                last_output_payload = public_output_payload.clone();
                variable_pool.insert(node.node_id.clone(), public_output_payload);
            }
            other => {
                let error_payload = orchestration_runtime::node_errors::build_node_type_not_implemented_error_payload(
                    other,
                    "debug",
                );
                update_node_run_and_emit(
                    service,
                    flow_run.id,
                    &UpdateNodeRunInput {
                        node_run_id: node_run.id,
                        status: domain::NodeRunStatus::Failed,
                        output_payload: json!({}),
                        error_payload: Some(error_payload.clone()),
                        metrics_payload: json!({ "preview_mode": true }),
                        debug_payload: json!({}),
                        finished_at: Some(OffsetDateTime::now_utc()),
                    },
                )
                .await?;

                return Err(anyhow!("{}", error_payload));
            }
        }
    }

    if is_run_cancelled(&service.repository, command.application_id, flow_run.id).await? {
        return load_run_detail(&service.repository, command.application_id, flow_run.id).await;
    }

    if let Some(error_payload) = pending_failure {
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
                    output_payload: last_output_payload.clone(),
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
        let variable_cache = public_node_variable_cache(&compiled_plan, &variable_pool);
        persist_debug_variable_cache_entries(
            &service.repository,
            application.workspace_id,
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
                output_payload: last_output_payload.clone(),
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
        debug_stream_events::flow_finished(flow_run.id, last_output_payload.clone()),
    )
    .await;
    close_runtime_event_stream(service, flow_run.id, RuntimeEventCloseReason::Finished).await;
    service
        .repository
        .append_run_event(&AppendRunEventInput {
            flow_run_id: flow_run.id,
            node_run_id: None,
            event_type: "flow_run_completed".to_string(),
            payload: last_output_payload,
        })
        .await?;
    let updated_flow_run = updated.expect("updated flow run exists after is_none check");
    let variable_cache = public_node_variable_cache(&compiled_plan, &variable_pool);
    persist_debug_variable_cache_entries(
        &service.repository,
        application.workspace_id,
        &updated_flow_run,
        &variable_cache,
    )
    .await?;

    load_run_detail(&service.repository, command.application_id, flow_run.id).await
}

fn inject_system_variables(
    variable_pool: &mut serde_json::Map<String, Value>,
    flow_run: &domain::FlowRunRecord,
    start_node_id: Option<&str>,
) {
    let conversation_id = flow_run
        .external_conversation_id
        .as_deref()
        .filter(|value| !value.is_empty())
        .unwrap_or(&flow_run.debug_session_id);
    let model_parameters = variable_pool
        .get("sys")
        .and_then(|value| value.get("model_parameters"))
        .cloned();
    let has_model_parameters = model_parameters.is_some();
    let start_has_reasoning_effort = start_node_id
        .and_then(|node_id| variable_pool.get(node_id))
        .and_then(Value::as_object)
        .is_some_and(|payload| payload.contains_key("reasoning_effort"));
    let sys_has_reasoning_effort = variable_pool
        .get("sys")
        .and_then(Value::as_object)
        .is_some_and(|payload| payload.contains_key("reasoning_effort"));
    let reasoning_effort = variable_pool
        .get(start_node_id.unwrap_or("node-start"))
        .and_then(|value| value.get("reasoning_effort"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .or_else(|| {
            variable_pool
                .get("sys")
                .and_then(|value| value.get("reasoning_effort"))
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned)
        })
        .or_else(|| {
            model_parameters
                .as_ref()
                .and_then(external_reasoning_effort)
        });

    let mut sys = json!({
            "conversation_id": conversation_id,
            "dialog_count": 0,
            "user_id": flow_run.created_by.to_string(),
            "application_id": flow_run.application_id.to_string(),
            "workflow_id": flow_run.flow_id.to_string(),
            "workflow_run_id": flow_run.id.to_string(),
    });
    if let Some(model_parameters) = model_parameters {
        sys["model_parameters"] = model_parameters;
    }

    variable_pool.insert("sys".to_string(), sys);
    if start_has_reasoning_effort || sys_has_reasoning_effort || has_model_parameters {
        insert_start_reasoning_effort(
            variable_pool,
            start_node_id,
            reasoning_effort.unwrap_or_default(),
        );
    }
}

fn compiled_plan_start_node_id(
    compiled_plan: &orchestration_runtime::compiled_plan::CompiledPlan,
) -> Option<&str> {
    compiled_plan
        .nodes
        .values()
        .find(|node| node.node_type == "start")
        .map(|node| node.node_id.as_str())
}

fn insert_start_reasoning_effort(
    variable_pool: &mut serde_json::Map<String, Value>,
    start_node_id: Option<&str>,
    reasoning_effort: String,
) {
    let start_node_id = start_node_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("node-start");
    let start_payload = variable_pool
        .entry(start_node_id.to_string())
        .or_insert_with(|| Value::Object(Map::new()));

    if !start_payload.is_object() {
        *start_payload = Value::Object(Map::new());
    }
    if let Some(start_payload) = start_payload.as_object_mut() {
        start_payload.insert(
            "reasoning_effort".to_string(),
            Value::String(reasoning_effort),
        );
    }
}

fn external_reasoning_effort(model_parameters: &Value) -> Option<String> {
    model_parameters
        .get("reasoning")
        .and_then(|value| value.get("effort"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn start_node_input_payload(
    variable_pool: &serde_json::Map<String, Value>,
    node_id: &str,
) -> Value {
    let mut payload = variable_pool
        .get(node_id)
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();

    if let Some(sys) = variable_pool.get("sys") {
        payload.insert("sys".to_string(), sys.clone());
    }
    if let Some(env) = variable_pool.get("env") {
        payload.insert("env".to_string(), env.clone());
    }

    Value::Object(payload)
}

fn template_output_payload(
    node: &orchestration_runtime::compiled_plan::CompiledNode,
    output_key: String,
    output_value: Value,
    variable_pool: &serde_json::Map<String, Value>,
) -> Value {
    let mut payload = serde_json::Map::new();
    payload.insert(output_key, output_value);

    if node.node_type == "answer" {
        if let Some(sys) = variable_pool.get("sys") {
            payload.insert("sys".to_string(), sys.clone());
        }
        if let Some(env) = variable_pool.get("env") {
            payload.insert("env".to_string(), env.clone());
        }
    }

    Value::Object(payload)
}

fn can_continue_to_terminal_template_nodes(
    plan: &orchestration_runtime::compiled_plan::CompiledPlan,
    failed_node_index: usize,
) -> bool {
    let mut has_terminal_template_node = false;
    for node_id in plan.topological_order.iter().skip(failed_node_index + 1) {
        let Some(node) = plan.nodes.get(node_id) else {
            return false;
        };
        if !matches!(node.node_type.as_str(), "template_transform" | "answer") {
            return false;
        }
        has_terminal_template_node = true;
    }
    has_terminal_template_node
}

async fn emit_answer_presentation_for_node<R, H>(
    service: &OrchestrationRuntimeService<R, H>,
    flow_run_id: uuid::Uuid,
    answer_presentation: Option<
        &Arc<Mutex<super::super::answer_presentation::AnswerPresentationCursor>>,
    >,
    node_id: &str,
    node_run_id: uuid::Uuid,
    output_payload: &Value,
) where
    R: OrchestrationRuntimeRepository,
{
    let Some(answer_presentation) = answer_presentation else {
        return;
    };
    let events =
        answer_presentation
            .lock()
            .await
            .complete_node(node_id, node_run_id, output_payload);
    for event in events {
        append_runtime_event(service, flow_run_id, event).await;
    }
}

async fn materialize_ready_answer_node_run<R, H>(
    service: &OrchestrationRuntimeService<R, H>,
    flow_run_id: uuid::Uuid,
    compiled_plan: &orchestration_runtime::compiled_plan::CompiledPlan,
    variable_pool: &Map<String, Value>,
) -> Result<Option<Value>>
where
    R: OrchestrationRuntimeRepository,
{
    let Some(ready) = super::super::answer_presentation::ready_answer_output_from_variable_pool(
        compiled_plan,
        variable_pool,
    ) else {
        return Ok(None);
    };
    let Some(answer_node) = compiled_plan.nodes.get(&ready.answer_node_id) else {
        return Ok(None);
    };
    let started_at = OffsetDateTime::now_utc();
    let node_run = service
        .repository
        .create_node_run(&CreateNodeRunInput {
            flow_run_id,
            node_id: answer_node.node_id.clone(),
            node_type: answer_node.node_type.clone(),
            node_alias: answer_node.alias.clone(),
            status: domain::NodeRunStatus::Running,
            input_payload: json!({
                "presentation": {
                    "kind": "answer",
                    "complete": ready.complete,
                    "materialized_from": "waiting_prefix"
                }
            }),
            debug_payload: json!({}),
            started_at,
        })
        .await?;
    append_runtime_event(
        service,
        flow_run_id,
        debug_stream_events::node_started(&node_run),
    )
    .await;

    ensure_node_run_transition(
        domain::NodeRunStatus::Running,
        domain::NodeRunStatus::Succeeded,
        "materialize_waiting_answer_node",
    )?;
    let output_payload =
        super::super::answer_presentation::ready_answer_output_payload(&ready, variable_pool);
    update_node_run_and_emit(
        service,
        flow_run_id,
        &UpdateNodeRunInput {
            node_run_id: node_run.id,
            status: domain::NodeRunStatus::Succeeded,
            output_payload: output_payload.clone(),
            error_payload: None,
            metrics_payload: json!({
                "preview_mode": true,
                "answer_presentation": {
                    "partial": !ready.complete,
                    "materialized_from": "waiting_prefix"
                }
            }),
            debug_payload: json!({
                "answer_presentation": {
                    "partial": !ready.complete,
                    "materialized_from": "waiting_prefix"
                }
            }),
            finished_at: Some(started_at),
        },
    )
    .await?;

    if ready.text.is_empty() {
        Ok(None)
    } else {
        Ok(Some(output_payload))
    }
}

async fn fail_current_live_run_after_node_error<R, H>(
    service: &OrchestrationRuntimeService<R, H>,
    command: &ContinueFlowDebugRunCommand,
    flow_run: &domain::FlowRunRecord,
    node_run_id: uuid::Uuid,
    output_payload: Value,
    error_payload: Value,
) -> Result<domain::ApplicationRunDetail>
where
    R: OrchestrationRuntimeRepository,
{
    ensure_flow_run_transition(
        domain::FlowRunStatus::Running,
        domain::FlowRunStatus::Failed,
        "continue_flow_debug_run",
    )?;
    service
        .repository
        .update_flow_run(&UpdateFlowRunInput {
            flow_run_id: flow_run.id,
            status: domain::FlowRunStatus::Failed,
            output_payload,
            error_payload: Some(error_payload.clone()),
            finished_at: Some(OffsetDateTime::now_utc()),
        })
        .await?;
    emit_flow_failed_and_close(service, flow_run.id, error_payload.clone()).await;
    service
        .repository
        .append_run_event(&AppendRunEventInput {
            flow_run_id: flow_run.id,
            node_run_id: Some(node_run_id),
            event_type: "flow_run_failed".to_string(),
            payload: error_payload,
        })
        .await?;
    load_run_detail(&service.repository, command.application_id, flow_run.id).await
}

fn inject_application_environment_variables(
    variable_pool: &mut serde_json::Map<String, Value>,
    variables: &[domain::ApplicationEnvironmentVariable],
) {
    variable_pool.insert(
        "env".to_string(),
        Value::Object(
            variables
                .iter()
                .map(|variable| (variable.name.clone(), variable.value.clone()))
                .collect(),
        ),
    );
}
