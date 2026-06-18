use std::collections::HashMap;

const TRACE_NODE_KIND_NODE_RUN: &str = "node_run";
const TRACE_NODE_KIND_CALLBACK_TASK: &str = "callback_task";
const TRACE_NODE_NODE_RUN_PREFIX: &str = "node_run:";
const TRACE_NODE_NODE_RUN_GROUP_PREFIX: &str = "node_run_group:";
const TRACE_NODE_CALLBACK_TASK_PREFIX: &str = "callback_task:";

enum ParsedTraceNodeId {
    NodeRun(Uuid),
    NodeRunGroup(Uuid),
    CallbackTask(Uuid),
}

fn node_run_trace_node_id(id: Uuid) -> String {
    format!("{TRACE_NODE_NODE_RUN_PREFIX}{id}")
}

fn callback_task_trace_node_id(id: Uuid) -> String {
    format!("{TRACE_NODE_CALLBACK_TASK_PREFIX}{id}")
}

fn node_run_group_trace_node_id(first_node_run_id: Uuid) -> String {
    format!("{TRACE_NODE_NODE_RUN_GROUP_PREFIX}{first_node_run_id}")
}

fn parse_trace_node_id(value: &str) -> Result<ParsedTraceNodeId, ControlPlaneError> {
    if let Some(raw_id) = value.strip_prefix(TRACE_NODE_NODE_RUN_PREFIX) {
        return Uuid::parse_str(raw_id)
            .map(ParsedTraceNodeId::NodeRun)
            .map_err(|_| ControlPlaneError::InvalidInput("trace_node_id"));
    }

    if let Some(raw_id) = value.strip_prefix(TRACE_NODE_NODE_RUN_GROUP_PREFIX) {
        return Uuid::parse_str(raw_id)
            .map(ParsedTraceNodeId::NodeRunGroup)
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

fn trace_tree_root_node_run_groups(
    detail: &domain::ApplicationRunDetail,
) -> Vec<Vec<domain::NodeRunRecord>> {
    let mut groups: Vec<Vec<domain::NodeRunRecord>> = Vec::new();
    let mut llm_group_index_by_node = HashMap::<(Uuid, String), usize>::new();

    for node_run in trace_tree_visible_node_runs(detail) {
        if node_run.node_type != "llm" {
            groups.push(vec![node_run]);
            continue;
        }

        let group_key = (node_run.flow_run_id, node_run.node_id.clone());
        if let Some(group_index) = llm_group_index_by_node.get(&group_key).copied() {
            groups[group_index].push(node_run);
            continue;
        }

        llm_group_index_by_node.insert(group_key, groups.len());
        groups.push(vec![node_run]);
    }

    groups
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
        .filter(|task| callback_task_is_trace_child(task))
        .any(|task| task.node_run_id == node_run_id)
}

fn callback_task_is_trace_child(task: &domain::CallbackTaskRecord) -> bool {
    task.callback_kind != "llm_tool_calls"
}

fn json_object_has_keys(value: &serde_json::Value) -> bool {
    value
        .as_object()
        .is_some_and(|object| !object.is_empty())
}

fn first_non_empty_json(
    node_runs: &[domain::NodeRunRecord],
    selector: impl Fn(&domain::NodeRunRecord) -> &serde_json::Value,
) -> serde_json::Value {
    node_runs
        .iter()
        .find_map(|node_run| {
            let payload = selector(node_run);
            json_object_has_keys(payload).then(|| payload.clone())
        })
        .unwrap_or_else(|| serde_json::json!({}))
}

fn last_non_empty_json(
    node_runs: &[domain::NodeRunRecord],
    selector: impl Fn(&domain::NodeRunRecord) -> &serde_json::Value,
) -> serde_json::Value {
    node_runs
        .iter()
        .rev()
        .find_map(|node_run| {
            let payload = selector(node_run);
            json_object_has_keys(payload).then(|| payload.clone())
        })
        .unwrap_or_else(|| serde_json::json!({}))
}

fn trace_node_group_status(node_runs: &[domain::NodeRunRecord]) -> domain::NodeRunStatus {
    if node_runs
        .iter()
        .any(|node_run| node_run.status == domain::NodeRunStatus::Failed)
    {
        return domain::NodeRunStatus::Failed;
    }

    if node_runs
        .iter()
        .any(|node_run| node_run.status == domain::NodeRunStatus::WaitingHuman)
    {
        return domain::NodeRunStatus::WaitingHuman;
    }

    if node_runs
        .iter()
        .any(|node_run| node_run.status == domain::NodeRunStatus::WaitingCallback)
    {
        return domain::NodeRunStatus::WaitingCallback;
    }

    if node_runs.iter().any(|node_run| {
        matches!(
            node_run.status,
            domain::NodeRunStatus::Running
                | domain::NodeRunStatus::Streaming
                | domain::NodeRunStatus::Retrying
                | domain::NodeRunStatus::WaitingTool
        )
    }) {
        return domain::NodeRunStatus::Running;
    }

    if node_runs
        .iter()
        .all(|node_run| node_run.status == domain::NodeRunStatus::Succeeded)
    {
        return domain::NodeRunStatus::Succeeded;
    }

    node_runs
        .last()
        .map(|node_run| node_run.status)
        .unwrap_or(domain::NodeRunStatus::Running)
}

fn trace_node_group_finished_at(
    node_runs: &[domain::NodeRunRecord],
) -> Option<OffsetDateTime> {
    if node_runs
        .iter()
        .any(|node_run| node_run.finished_at.is_none())
    {
        return None;
    }

    node_runs.last().and_then(|node_run| node_run.finished_at)
}

fn trace_node_group_duration_ms(node_runs: &[domain::NodeRunRecord]) -> Option<i64> {
    let durations: Vec<i64> = node_runs
        .iter()
        .filter_map(|node_run| trace_node_duration_ms(node_run.started_at, node_run.finished_at))
        .collect();

    if durations.is_empty() {
        return None;
    }

    Some(
        durations
            .into_iter()
            .fold(0_i64, |total, duration| total.saturating_add(duration)),
    )
}

fn merge_debug_payloads(node_runs: &[domain::NodeRunRecord]) -> serde_json::Value {
    let mut merged = serde_json::Map::new();
    let mut llm_rounds = Vec::<serde_json::Value>::new();
    let mut visible_internal_route_traces = Vec::<serde_json::Value>::new();
    let mut visible_internal_route_events = Vec::<serde_json::Value>::new();

    for node_run in node_runs {
        let Some(debug_payload) = node_run.debug_payload.as_object() else {
            continue;
        };

        for (key, value) in debug_payload {
            match key.as_str() {
                "llm_rounds" => {
                    if let Some(items) = value.as_array() {
                        llm_rounds.extend(items.iter().cloned());
                    } else if !merged.contains_key(key) {
                        merged.insert(key.clone(), value.clone());
                    }
                }
                "visible_internal_llm_tool_trace" => {
                    if let Some(items) = value.as_array() {
                        visible_internal_route_traces.extend(items.iter().cloned());
                    } else if !merged.contains_key(key) {
                        merged.insert(key.clone(), value.clone());
                    }
                }
                "visible_internal_llm_tool_events" => {
                    if let Some(items) = value.as_array() {
                        visible_internal_route_events.extend(items.iter().cloned());
                    } else if !merged.contains_key(key) {
                        merged.insert(key.clone(), value.clone());
                    }
                }
                _ => {
                    if !merged.contains_key(key) {
                        merged.insert(key.clone(), value.clone());
                    }
                }
            }
        }
    }

    if !llm_rounds.is_empty() {
        merged.insert("llm_rounds".to_string(), serde_json::Value::Array(llm_rounds));
    }
    if !visible_internal_route_traces.is_empty() {
        merged.insert(
            "visible_internal_llm_tool_trace".to_string(),
            serde_json::Value::Array(visible_internal_route_traces),
        );
    }
    if !visible_internal_route_events.is_empty() {
        merged.insert(
            "visible_internal_llm_tool_events".to_string(),
            serde_json::Value::Array(visible_internal_route_events),
        );
    }

    serde_json::Value::Object(merged)
}

fn merge_node_run_group(node_runs: &[domain::NodeRunRecord]) -> domain::NodeRunRecord {
    let mut merged = node_runs[0].clone();

    if node_runs.len() == 1 {
        return merged;
    }

    merged.status = trace_node_group_status(node_runs);
    merged.finished_at = trace_node_group_finished_at(node_runs);
    merged.input_payload = first_non_empty_json(node_runs, |node_run| &node_run.input_payload);
    merged.output_payload = last_non_empty_json(node_runs, |node_run| &node_run.output_payload);
    merged.error_payload = node_runs
        .iter()
        .rev()
        .find_map(|node_run| node_run.error_payload.clone());
    merged.metrics_payload = last_non_empty_json(node_runs, |node_run| &node_run.metrics_payload);
    merged.debug_payload = merge_debug_payloads(node_runs);

    merged
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

fn to_trace_node_summary_from_node_run_group(
    detail: &domain::ApplicationRunDetail,
    parent_trace_node_id: Option<String>,
    node_runs: &[domain::NodeRunRecord],
) -> ApplicationRunTraceNodeSummaryResponse {
    if node_runs.len() == 1 {
        return to_trace_node_summary_from_node_run(detail, parent_trace_node_id, &node_runs[0]);
    }

    let first_node_run = &node_runs[0];
    ApplicationRunTraceNodeSummaryResponse {
        trace_node_id: node_run_group_trace_node_id(first_node_run.id),
        parent_trace_node_id,
        node_kind: TRACE_NODE_KIND_NODE_RUN.to_string(),
        flow_run_id: first_node_run.flow_run_id.to_string(),
        node_run_id: Some(first_node_run.id.to_string()),
        callback_task_id: None,
        node_id: Some(first_node_run.node_id.clone()),
        node_type: Some(first_node_run.node_type.clone()),
        node_alias: first_node_run.node_alias.clone(),
        status: trace_node_group_status(node_runs).as_str().to_string(),
        started_at: format_time(first_node_run.started_at),
        finished_at: format_optional_time(trace_node_group_finished_at(node_runs)),
        duration_ms: trace_node_group_duration_ms(node_runs),
        metrics_payload: last_non_empty_json(node_runs, |node_run| &node_run.metrics_payload),
        has_children: node_runs
            .iter()
            .any(|node_run| trace_node_has_callback_children(detail, node_run.id)),
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

fn to_application_run_overview_response(
    application: &domain::ApplicationRecord,
    detail: domain::ApplicationRunDetail,
) -> ApplicationRunOverviewResponse {
    let (_, current_visible_node_runs) = split_answer_snapshot_node_runs(&detail);
    let statistics = application_run_statistics(&domain::ApplicationRunDetail {
        node_runs: current_visible_node_runs,
        ..detail.clone()
    });

    ApplicationRunOverviewResponse {
        run: application_run_log_response_for_trace_tree(application, &detail.flow_run),
        statistics,
        flow_run: to_flow_run_response(detail.flow_run.clone()),
        answer_snapshot: answer_snapshot_for_trace_tree(&detail),
    }
}

fn to_application_run_trace_tree_response(
    application: &domain::ApplicationRecord,
    detail: domain::ApplicationRunDetail,
) -> ApplicationRunTraceTreeResponse {
    let root_node_run_groups = trace_tree_root_node_run_groups(&detail);
    let (_, current_visible_node_runs) = split_answer_snapshot_node_runs(&detail);
    let statistics = application_run_statistics(&domain::ApplicationRunDetail {
        node_runs: current_visible_node_runs,
        ..detail.clone()
    });
    let nodes = root_node_run_groups
        .iter()
        .map(|node_runs| to_trace_node_summary_from_node_run_group(&detail, None, node_runs))
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
                .filter(|task| callback_task_is_trace_child(task))
                .filter(|task| task.node_run_id == node_run_id)
                .map(|task| to_trace_node_summary_from_callback_task(parent_trace_node_id.clone(), task))
                .collect()
        }
        ParsedTraceNodeId::NodeRunGroup(first_node_run_id) => {
            let Some(node_runs) = trace_node_run_group(&detail, first_node_run_id) else {
                return Err(ControlPlaneError::NotFound("trace_node"));
            };
            let parent_trace_node_id = node_run_group_trace_node_id(first_node_run_id);
            let node_run_ids = node_runs
                .iter()
                .map(|node_run| node_run.id)
                .collect::<HashSet<_>>();

            detail
                .callback_tasks
                .iter()
                .chain(
                    detail
                        .stitched_trace
                        .iter()
                        .flat_map(|trace| trace.callback_tasks.iter()),
                )
                .filter(|task| callback_task_is_trace_child(task))
                .filter(|task| node_run_ids.contains(&task.node_run_id))
                .map(|task| {
                    to_trace_node_summary_from_callback_task(parent_trace_node_id.clone(), task)
                })
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

fn trace_node_run_group(
    detail: &domain::ApplicationRunDetail,
    first_node_run_id: Uuid,
) -> Option<Vec<domain::NodeRunRecord>> {
    let visible_node_runs = trace_tree_visible_node_runs(detail);
    let first_node_run = visible_node_runs
        .iter()
        .find(|candidate| candidate.id == first_node_run_id)?;

    if first_node_run.node_type != "llm" {
        return Some(vec![first_node_run.clone()]);
    }

    let group_flow_run_id = first_node_run.flow_run_id;
    let group_node_id = first_node_run.node_id.clone();
    let node_runs = visible_node_runs
        .into_iter()
        .filter(|candidate| {
            candidate.flow_run_id == group_flow_run_id
                && candidate.node_id == group_node_id
                && candidate.node_type == "llm"
        })
        .collect();

    Some(node_runs)
}

fn trace_node_run_group_content(
    detail: &domain::ApplicationRunDetail,
    first_node_run_id: Uuid,
) -> Option<(
    domain::NodeRunRecord,
    Vec<domain::CheckpointRecord>,
    Vec<domain::RunEventRecord>,
)> {
    let node_runs = trace_node_run_group(detail, first_node_run_id)?;
    let node_run_ids = node_runs
        .iter()
        .map(|node_run| node_run.id)
        .collect::<HashSet<_>>();
    let checkpoints = detail
        .checkpoints
        .iter()
        .filter(|checkpoint| {
            checkpoint
                .node_run_id
                .is_some_and(|node_run_id| node_run_ids.contains(&node_run_id))
        })
        .cloned()
        .collect();
    let events = detail
        .events
        .iter()
        .chain(
            detail
                .stitched_trace
                .iter()
                .flat_map(|trace| trace.events.iter()),
        )
        .filter(|event| {
            event
                .node_run_id
                .is_some_and(|node_run_id| node_run_ids.contains(&node_run_id))
        })
        .cloned()
        .collect();

    let callback_tasks = trace_node_callback_tasks_for_node_run_ids(detail, &node_run_ids);
    let node_run = trace_node_run_with_llm_tool_callback_index(
        merge_node_run_group(&node_runs),
        &callback_tasks,
    );

    Some((node_run, checkpoints, events))
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

fn trace_node_callback_tasks_for_node_run_ids(
    detail: &domain::ApplicationRunDetail,
    node_run_ids: &HashSet<Uuid>,
) -> Vec<domain::CallbackTaskRecord> {
    detail
        .callback_tasks
        .iter()
        .chain(
            detail
                .stitched_trace
                .iter()
                .flat_map(|trace| trace.callback_tasks.iter()),
        )
        .filter(|task| node_run_ids.contains(&task.node_run_id))
        .cloned()
        .collect()
}

fn trace_node_run_with_llm_tool_callback_index(
    mut node_run: domain::NodeRunRecord,
    callback_tasks: &[domain::CallbackTaskRecord],
) -> domain::NodeRunRecord {
    if node_run.node_type == "llm" {
        node_run.debug_payload =
            with_inline_llm_tool_callback_index(node_run.debug_payload, callback_tasks);
        node_run.debug_payload =
            without_inline_visible_internal_llm_tool_trace(node_run.debug_payload);
    }

    node_run
}

fn trace_node_llm_tool_callback_sources(
    detail: &domain::ApplicationRunDetail,
    trace_node_id: &str,
) -> Result<(Vec<serde_json::Value>, Vec<domain::CallbackTaskRecord>), ControlPlaneError> {
    match parse_trace_node_id(trace_node_id)? {
        ParsedTraceNodeId::NodeRun(node_run_id) => {
            let Some((node_run, _, _)) = trace_node_run_content(detail, node_run_id) else {
                return Err(ControlPlaneError::NotFound("trace_node"));
            };
            if node_run.node_type != "llm" {
                return Err(ControlPlaneError::NotFound("tool_callback"));
            }

            let node_run_ids = std::iter::once(node_run_id).collect::<HashSet<_>>();
            let callback_tasks = trace_node_callback_tasks_for_node_run_ids(detail, &node_run_ids);

            Ok((vec![node_run.debug_payload], callback_tasks))
        }
        ParsedTraceNodeId::NodeRunGroup(first_node_run_id) => {
            let Some(node_runs) = trace_node_run_group(detail, first_node_run_id) else {
                return Err(ControlPlaneError::NotFound("trace_node"));
            };
            if node_runs.iter().any(|node_run| node_run.node_type != "llm") {
                return Err(ControlPlaneError::NotFound("tool_callback"));
            }

            let node_run_ids = node_runs
                .iter()
                .map(|node_run| node_run.id)
                .collect::<HashSet<_>>();
            let callback_tasks = trace_node_callback_tasks_for_node_run_ids(detail, &node_run_ids);
            let debug_payloads = node_runs
                .into_iter()
                .map(|node_run| node_run.debug_payload)
                .collect();

            Ok((debug_payloads, callback_tasks))
        }
        ParsedTraceNodeId::CallbackTask(_) => Err(ControlPlaneError::NotFound("tool_callback")),
    }
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
            let node_run_ids = std::iter::once(node_run_id).collect::<HashSet<_>>();
            let callback_tasks = trace_node_callback_tasks_for_node_run_ids(&detail, &node_run_ids);
            let node_run = trace_node_run_with_llm_tool_callback_index(node_run, &callback_tasks);

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
        ParsedTraceNodeId::NodeRunGroup(first_node_run_id) => {
            let Some((node_run, checkpoints, events)) =
                trace_node_run_group_content(&detail, first_node_run_id)
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

fn trace_tool_callback_content_response(
    detail: domain::ApplicationRunDetail,
    trace_node_id: &str,
    tool_call_id: &str,
) -> Result<ApplicationRunTraceToolCallbackContentResponse, ControlPlaneError> {
    let (debug_payloads, callback_tasks) =
        trace_node_llm_tool_callback_sources(&detail, trace_node_id)?;
    let Some(callback) = collect_llm_tool_callback_trace_items(&debug_payloads, &callback_tasks)
        .into_iter()
        .find(|callback| callback.id() == tool_call_id)
    else {
        return Err(ControlPlaneError::NotFound("tool_callback"));
    };

    Ok(ApplicationRunTraceToolCallbackContentResponse {
        trace_node_id: trace_node_id.to_string(),
        tool_call_id: tool_call_id.to_string(),
        payload: callback.detail_payload(),
    })
}
