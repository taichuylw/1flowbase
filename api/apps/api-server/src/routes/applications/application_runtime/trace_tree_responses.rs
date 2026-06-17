const TRACE_NODE_KIND_NODE_RUN: &str = "node_run";
const TRACE_NODE_KIND_CALLBACK_TASK: &str = "callback_task";
const TRACE_NODE_NODE_RUN_PREFIX: &str = "node_run:";
const TRACE_NODE_CALLBACK_TASK_PREFIX: &str = "callback_task:";

enum ParsedTraceNodeId {
    NodeRun(Uuid),
    CallbackTask(Uuid),
}

fn node_run_trace_node_id(id: Uuid) -> String {
    format!("{TRACE_NODE_NODE_RUN_PREFIX}{id}")
}

fn callback_task_trace_node_id(id: Uuid) -> String {
    format!("{TRACE_NODE_CALLBACK_TASK_PREFIX}{id}")
}

fn parse_trace_node_id(value: &str) -> Result<ParsedTraceNodeId, ControlPlaneError> {
    if let Some(raw_id) = value.strip_prefix(TRACE_NODE_NODE_RUN_PREFIX) {
        return Uuid::parse_str(raw_id)
            .map(ParsedTraceNodeId::NodeRun)
            .map_err(|_| ControlPlaneError::InvalidInput("trace_node_id"));
    }

    if let Some(raw_id) = value.strip_prefix(TRACE_NODE_CALLBACK_TASK_PREFIX) {
        return Uuid::parse_str(raw_id)
            .map(ParsedTraceNodeId::CallbackTask)
            .map_err(|_| ControlPlaneError::InvalidInput("trace_node_id"));
    }

    Err(ControlPlaneError::InvalidInput("trace_node_id"))
}

fn trace_node_duration_ms(
    started_at: OffsetDateTime,
    finished_at: Option<OffsetDateTime>,
) -> Option<i64> {
    finished_at.map(|finished| {
        (finished - started_at)
            .whole_milliseconds()
            .max(0)
            .try_into()
            .unwrap_or(i64::MAX)
    })
}

fn trace_tree_visible_node_runs(
    detail: &domain::ApplicationRunDetail,
) -> Vec<domain::NodeRunRecord> {
    let (_, node_runs) = split_answer_snapshot_node_runs(detail);
    let stitched_node_runs = detail
        .stitched_trace
        .iter()
        .flat_map(|trace| trace.node_runs.iter())
        .filter(|node_run| stitched_trace_node_run_is_trace_step(node_run))
        .cloned();

    node_runs.into_iter().chain(stitched_node_runs).collect()
}

fn trace_node_has_callback_children(
    detail: &domain::ApplicationRunDetail,
    node_run_id: Uuid,
) -> bool {
    detail
        .callback_tasks
        .iter()
        .chain(
            detail
                .stitched_trace
                .iter()
                .flat_map(|trace| trace.callback_tasks.iter()),
        )
        .any(|task| task.node_run_id == node_run_id)
}

fn to_trace_node_summary_from_node_run(
    detail: &domain::ApplicationRunDetail,
    parent_trace_node_id: Option<String>,
    node_run: &domain::NodeRunRecord,
) -> ApplicationRunTraceNodeSummaryResponse {
    ApplicationRunTraceNodeSummaryResponse {
        trace_node_id: node_run_trace_node_id(node_run.id),
        parent_trace_node_id,
        node_kind: TRACE_NODE_KIND_NODE_RUN.to_string(),
        flow_run_id: node_run.flow_run_id.to_string(),
        node_run_id: Some(node_run.id.to_string()),
        callback_task_id: None,
        node_id: Some(node_run.node_id.clone()),
        node_type: Some(node_run.node_type.clone()),
        node_alias: node_run.node_alias.clone(),
        status: node_run.status.as_str().to_string(),
        started_at: format_time(node_run.started_at),
        finished_at: format_optional_time(node_run.finished_at),
        duration_ms: trace_node_duration_ms(node_run.started_at, node_run.finished_at),
        metrics_payload: node_run.metrics_payload.clone(),
        has_children: trace_node_has_callback_children(detail, node_run.id),
        has_content: true,
    }
}

fn to_trace_node_summary_from_callback_task(
    parent_trace_node_id: String,
    task: &domain::CallbackTaskRecord,
) -> ApplicationRunTraceNodeSummaryResponse {
    ApplicationRunTraceNodeSummaryResponse {
        trace_node_id: callback_task_trace_node_id(task.id),
        parent_trace_node_id: Some(parent_trace_node_id),
        node_kind: TRACE_NODE_KIND_CALLBACK_TASK.to_string(),
        flow_run_id: task.flow_run_id.to_string(),
        node_run_id: Some(task.node_run_id.to_string()),
        callback_task_id: Some(task.id.to_string()),
        node_id: None,
        node_type: Some(task.callback_kind.clone()),
        node_alias: task.callback_kind.clone(),
        status: task.status.as_str().to_string(),
        started_at: format_time(task.created_at),
        finished_at: format_optional_time(task.completed_at),
        duration_ms: trace_node_duration_ms(task.created_at, task.completed_at),
        metrics_payload: serde_json::json!({}),
        has_children: false,
        has_content: true,
    }
}

fn application_run_log_response_for_trace_tree(
    application: &domain::ApplicationRecord,
    flow_run: &domain::FlowRunRecord,
) -> application_logs::ApplicationRunLogResponse {
    let application_type = application.application_type.as_str().to_string();

    application_logs::ApplicationRunLogResponse {
        id: flow_run.id.to_string(),
        application_id: application.id.to_string(),
        application_type: application_type.clone(),
        run_object_kind: application.sections.logs.run_object_kind.clone(),
        run_kind: flow_run.run_mode.as_str().to_string(),
        status: flow_run.status.as_str().to_string(),
        title: flow_run.title.clone(),
        source: application_logs::source_for_run(flow_run.api_key_id),
        compatibility_mode: flow_run.compatibility_mode.clone(),
        subject: application_logs::ApplicationRunSubjectResponse {
            kind: application_type,
            id: Some(flow_run.flow_id.to_string()),
            draft_id: Some(flow_run.draft_id.to_string()),
            target_node_id: flow_run.target_node_id.clone(),
        },
        actor: application_logs::actor_from_console_user(
            Some(flow_run.created_by.to_string()),
            flow_run.authorized_account.clone(),
        ),
        correlation: application_logs::ApplicationRunCorrelationResponse {
            api_key_id: flow_run.api_key_id.map(|value| value.to_string()),
            publication_version_id: flow_run
                .publication_version_id
                .map(|value| value.to_string()),
            external_user: flow_run.external_user.clone(),
            external_conversation_id: flow_run.external_conversation_id.clone(),
            external_trace_id: flow_run.external_trace_id.clone(),
            compatibility_mode: flow_run.compatibility_mode.clone(),
            idempotency_key: flow_run.idempotency_key.clone(),
        },
        started_at: application_logs::format_time(flow_run.started_at),
        finished_at: application_logs::format_optional_time(flow_run.finished_at),
        created_at: application_logs::format_time(flow_run.created_at),
        updated_at: application_logs::format_time(flow_run.updated_at),
    }
}

fn answer_snapshot_for_trace_tree(
    detail: &domain::ApplicationRunDetail,
) -> Option<AnswerSnapshotResponse> {
    let (answer_snapshot_node_run, _) = split_answer_snapshot_node_runs(detail);

    if !flow_run_can_expose_answer_snapshot(&detail.flow_run.status) {
        return None;
    }

    answer_snapshot_node_run
        .as_ref()
        .and_then(|node_run| to_answer_snapshot_response(node_run, detail))
}

fn to_application_run_trace_tree_response(
    application: &domain::ApplicationRecord,
    detail: domain::ApplicationRunDetail,
) -> ApplicationRunTraceTreeResponse {
    let visible_node_runs = trace_tree_visible_node_runs(&detail);
    let (_, current_visible_node_runs) = split_answer_snapshot_node_runs(&detail);
    let statistics = application_run_statistics(&domain::ApplicationRunDetail {
        node_runs: current_visible_node_runs,
        ..detail.clone()
    });
    let nodes = visible_node_runs
        .iter()
        .map(|node_run| to_trace_node_summary_from_node_run(&detail, None, node_run))
        .collect();

    ApplicationRunTraceTreeResponse {
        run: application_run_log_response_for_trace_tree(application, &detail.flow_run),
        statistics,
        flow_run: to_flow_run_response(detail.flow_run.clone()),
        answer_snapshot: answer_snapshot_for_trace_tree(&detail),
        nodes,
    }
}

fn trace_node_children_response(
    detail: domain::ApplicationRunDetail,
    parent_trace_node_id: &str,
) -> Result<ApplicationRunTraceNodeChildrenResponse, ControlPlaneError> {
    let items = match parse_trace_node_id(parent_trace_node_id)? {
        ParsedTraceNodeId::NodeRun(node_run_id) => {
            let parent_trace_node_id = node_run_trace_node_id(node_run_id);
            if !trace_node_run_exists(&detail, node_run_id) {
                return Err(ControlPlaneError::NotFound("trace_node"));
            }

            detail
                .callback_tasks
                .iter()
                .chain(
                    detail
                        .stitched_trace
                        .iter()
                        .flat_map(|trace| trace.callback_tasks.iter()),
                )
                .filter(|task| task.node_run_id == node_run_id)
                .map(|task| to_trace_node_summary_from_callback_task(parent_trace_node_id.clone(), task))
                .collect()
        }
        ParsedTraceNodeId::CallbackTask(callback_task_id) => {
            if trace_callback_task_content(&detail, callback_task_id).is_none() {
                return Err(ControlPlaneError::NotFound("trace_node"));
            }
            Vec::new()
        }
    };

    Ok(ApplicationRunTraceNodeChildrenResponse { items })
}

fn trace_node_run_exists(detail: &domain::ApplicationRunDetail, node_run_id: Uuid) -> bool {
    detail
        .node_runs
        .iter()
        .any(|node_run| node_run.id == node_run_id)
        || detail.stitched_trace.iter().any(|trace| {
            trace
                .node_runs
                .iter()
                .filter(|node_run| stitched_trace_node_run_is_trace_step(node_run))
                .any(|node_run| node_run.id == node_run_id)
        })
}

fn trace_node_run_content(
    detail: &domain::ApplicationRunDetail,
    node_run_id: Uuid,
) -> Option<(
    domain::NodeRunRecord,
    Vec<domain::CheckpointRecord>,
    Vec<domain::RunEventRecord>,
)> {
    if let Some(node_run) = detail
        .node_runs
        .iter()
        .find(|candidate| candidate.id == node_run_id)
    {
        let checkpoints = detail
            .checkpoints
            .iter()
            .filter(|checkpoint| checkpoint.node_run_id == Some(node_run_id))
            .cloned()
            .collect();
        let events = detail
            .events
            .iter()
            .filter(|event| event.node_run_id == Some(node_run_id))
            .cloned()
            .collect();

        return Some((node_run.clone(), checkpoints, events));
    }

    for trace in &detail.stitched_trace {
        let Some(node_run) = trace
            .node_runs
            .iter()
            .filter(|candidate| stitched_trace_node_run_is_trace_step(candidate))
            .find(|candidate| candidate.id == node_run_id)
        else {
            continue;
        };
        let events = trace
            .events
            .iter()
            .filter(|event| event.node_run_id == Some(node_run_id))
            .cloned()
            .collect();

        return Some((node_run.clone(), Vec::new(), events));
    }

    None
}

fn trace_callback_task_content(
    detail: &domain::ApplicationRunDetail,
    callback_task_id: Uuid,
) -> Option<domain::CallbackTaskRecord> {
    detail
        .callback_tasks
        .iter()
        .chain(
            detail
                .stitched_trace
                .iter()
                .flat_map(|trace| trace.callback_tasks.iter()),
        )
        .find(|candidate| candidate.id == callback_task_id)
        .cloned()
}

fn trace_node_content_response(
    detail: domain::ApplicationRunDetail,
    trace_node_id: &str,
) -> Result<ApplicationRunTraceNodeContentResponse, ControlPlaneError> {
    match parse_trace_node_id(trace_node_id)? {
        ParsedTraceNodeId::NodeRun(node_run_id) => {
            let Some((node_run, checkpoints, events)) =
                trace_node_run_content(&detail, node_run_id)
            else {
                return Err(ControlPlaneError::NotFound("trace_node"));
            };

            Ok(ApplicationRunTraceNodeContentResponse {
                trace_node_id: trace_node_id.to_string(),
                node_kind: TRACE_NODE_KIND_NODE_RUN.to_string(),
                node_run: Some(to_node_run_response(node_run)),
                callback_task: None,
                flow_run: None,
                checkpoints: checkpoints.into_iter().map(to_checkpoint_response).collect(),
                events: events.into_iter().map(to_run_event_response).collect(),
            })
        }
        ParsedTraceNodeId::CallbackTask(callback_task_id) => {
            let Some(task) = trace_callback_task_content(&detail, callback_task_id)
            else {
                return Err(ControlPlaneError::NotFound("trace_node"));
            };

            Ok(ApplicationRunTraceNodeContentResponse {
                trace_node_id: trace_node_id.to_string(),
                node_kind: TRACE_NODE_KIND_CALLBACK_TASK.to_string(),
                node_run: None,
                callback_task: Some(to_callback_task_response(task)),
                flow_run: None,
                checkpoints: Vec::new(),
                events: Vec::new(),
            })
        }
    }
}
