use std::collections::{HashMap, HashSet};

use anyhow::Result;
use sha2::{Digest, Sha256};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::ports::{
    ApplicationRunTraceNodeContentProjectionInput, ApplicationRunTraceNodeProjectionInput,
    ReplaceApplicationRunTraceProjectionInput,
};

pub const APPLICATION_RUN_TRACE_PROJECTION_VERSION: i32 = 3;

pub fn trace_node_id_for_locator(flow_run_id: Uuid, stable_locator: &str) -> Uuid {
    let mut hasher = Sha256::new();
    hasher.update(b"1flowbase.application_run_trace_node.v1");
    hasher.update(flow_run_id.as_bytes());
    hasher.update(stable_locator.as_bytes());

    let digest = hasher.finalize();
    let mut bytes = [0_u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    bytes[6] = (bytes[6] & 0x0f) | 0x80;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Uuid::from_bytes(bytes)
}

pub fn legacy_locator_component(
    source_path: &str,
    order_key: &str,
    source_payload: &serde_json::Value,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"1flowbase.trace_legacy_locator.v1");
    hasher.update(source_path.as_bytes());
    hasher.update(order_key.as_bytes());
    hasher.update(source_payload.to_string().as_bytes());
    let digest = hasher.finalize();
    format!(
        "legacy:{:x}",
        &digest[..8]
            .iter()
            .fold(0_u64, |acc, byte| { (acc << 8) | u64::from(*byte) })
    )
}

pub fn build_application_run_trace_projection(
    detail: &domain::ApplicationRunDetail,
) -> Result<ReplaceApplicationRunTraceProjectionInput> {
    let source_watermark = trace_projection_source_watermark(detail);
    let mut builder = TraceProjectionBuilder::new(detail.flow_run.id, source_watermark);
    let current_node_groups = trace_visible_current_node_run_groups(detail);

    for (index, node_runs) in current_node_groups.iter().enumerate() {
        builder.push_node_run_root(index, node_runs, detail)?;
    }

    if !detail.stitched_trace.is_empty() {
        builder.push_stitched_context_group(current_node_groups.len(), &detail.stitched_trace)?;
    }

    Ok(builder.finish())
}

pub fn projection_status_needs_lazy_rebuild(
    status: Option<&domain::ApplicationRunTraceProjectionStatusRecord>,
    current_source_watermark: &str,
) -> bool {
    let Some(status) = status else {
        return true;
    };

    if status.projection_version != APPLICATION_RUN_TRACE_PROJECTION_VERSION {
        return true;
    }

    match status.status {
        domain::ApplicationRunTraceProjectionStatus::Succeeded => {
            status.source_watermark != current_source_watermark
        }
        domain::ApplicationRunTraceProjectionStatus::Stale
        | domain::ApplicationRunTraceProjectionStatus::Partial => true,
        domain::ApplicationRunTraceProjectionStatus::Pending
        | domain::ApplicationRunTraceProjectionStatus::Running
        | domain::ApplicationRunTraceProjectionStatus::Failed => false,
    }
}

pub fn trace_projection_source_watermark(detail: &domain::ApplicationRunDetail) -> String {
    format!(
        "flow_run_updated_at:{}/node_runs:{}/callback_tasks:{}/events:{}/stitched:{}",
        detail.flow_run.updated_at.unix_timestamp_nanos(),
        detail.node_runs.len(),
        detail.callback_tasks.len(),
        detail.events.len(),
        detail.stitched_trace.len()
    )
}

fn trace_visible_current_node_runs(
    detail: &domain::ApplicationRunDetail,
) -> Vec<domain::NodeRunRecord> {
    detail
        .node_runs
        .iter()
        .filter(|node_run| !is_waiting_prefix_answer_node_run(node_run))
        .cloned()
        .collect()
}

fn trace_visible_current_node_run_groups(
    detail: &domain::ApplicationRunDetail,
) -> Vec<Vec<domain::NodeRunRecord>> {
    let mut groups = Vec::<Vec<domain::NodeRunRecord>>::new();
    let mut llm_group_index_by_node = HashMap::<(Uuid, String), usize>::new();

    for node_run in trace_visible_current_node_runs(detail) {
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

fn is_waiting_prefix_answer_node_run(node_run: &domain::NodeRunRecord) -> bool {
    let input_marker = node_run
        .input_payload
        .get("presentation")
        .and_then(serde_json::Value::as_object)
        .and_then(|presentation| presentation.get("materialized_from"))
        .and_then(serde_json::Value::as_str);
    let debug_marker = node_run
        .debug_payload
        .get("answer_presentation")
        .and_then(serde_json::Value::as_object)
        .and_then(|presentation| presentation.get("materialized_from"))
        .and_then(serde_json::Value::as_str);

    input_marker == Some("waiting_prefix") || debug_marker == Some("waiting_prefix")
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

struct TraceProjectionBuilder {
    flow_run_id: Uuid,
    source_watermark: String,
    nodes: Vec<ApplicationRunTraceNodeProjectionInput>,
    contents: Vec<ApplicationRunTraceNodeContentProjectionInput>,
}

impl TraceProjectionBuilder {
    fn new(flow_run_id: Uuid, source_watermark: String) -> Self {
        Self {
            flow_run_id,
            source_watermark,
            nodes: Vec::new(),
            contents: Vec::new(),
        }
    }

    fn finish(self) -> ReplaceApplicationRunTraceProjectionInput {
        ReplaceApplicationRunTraceProjectionInput {
            flow_run_id: self.flow_run_id,
            projection_version: APPLICATION_RUN_TRACE_PROJECTION_VERSION,
            source_watermark: self.source_watermark,
            nodes: self.nodes,
            contents: self.contents,
        }
    }

    fn push_node_run_root(
        &mut self,
        index: usize,
        node_runs: &[domain::NodeRunRecord],
        detail: &domain::ApplicationRunDetail,
    ) -> Result<()> {
        let first_node_run = &node_runs[0];
        let summary_node_run = merge_node_run_group(node_runs);
        let order_key = root_order_key(index);
        let stable_locator = if node_runs.len() == 1 {
            format!("run:{}/node:{}", self.flow_run_id, first_node_run.id)
        } else {
            format!("run:{}/node_group:{}", self.flow_run_id, first_node_run.id)
        };
        let trace_node_id = trace_node_id_for_locator(self.flow_run_id, &stable_locator);
        let node_run_ids = node_runs
            .iter()
            .map(|node_run| node_run.id)
            .collect::<HashSet<_>>();
        let callback_tasks = callback_tasks_for_node_run_ids(detail, &node_run_ids);
        let has_tool_calls = callback_tasks
            .iter()
            .any(|task| task.callback_kind == "llm_tool_calls");
        let non_tool_callback_count = callback_tasks
            .iter()
            .filter(|task| task.callback_kind != "llm_tool_calls")
            .count();
        let child_count = i64::try_from(non_tool_callback_count + usize::from(has_tool_calls))
            .unwrap_or(i64::MAX);

        self.nodes.push(ApplicationRunTraceNodeProjectionInput {
            trace_node_id,
            parent_trace_node_id: None,
            stable_locator: stable_locator.clone(),
            node_kind: "node_run".to_string(),
            owner_kind: Some(if node_runs.len() == 1 {
                "node_run".to_string()
            } else {
                "node_run_group".to_string()
            }),
            owner_id: Some(first_node_run.id.to_string()),
            order_key: order_key.clone(),
            node_id: Some(first_node_run.node_id.clone()),
            node_type: Some(first_node_run.node_type.clone()),
            node_alias: first_node_run.node_alias.clone(),
            status: summary_node_run.status.as_str().to_string(),
            started_at: first_node_run.started_at,
            finished_at: summary_node_run.finished_at,
            duration_ms: trace_node_group_duration_ms(node_runs),
            metrics_payload: summary_node_run.metrics_payload.clone(),
            has_children: child_count > 0,
            child_count,
            has_content: true,
            content_ref: None,
        });
        self.contents
            .push(node_run_group_content(trace_node_id, node_runs, detail)?);

        self.push_callback_children(
            &order_key,
            trace_node_id,
            &stable_locator,
            node_runs,
            &callback_tasks,
        )?;
        Ok(())
    }

    fn push_callback_children(
        &mut self,
        parent_order_key: &str,
        parent_trace_node_id: Uuid,
        parent_stable_locator: &str,
        parent_node_runs: &[domain::NodeRunRecord],
        callback_tasks: &[domain::CallbackTaskRecord],
    ) -> Result<()> {
        let mut child_index = 0_usize;
        let tool_tasks: Vec<&domain::CallbackTaskRecord> = callback_tasks
            .iter()
            .filter(|task| task.callback_kind == "llm_tool_calls")
            .collect();
        let synthetic_tool_calls =
            synthetic_tool_calls_not_in_callback_tasks(parent_node_runs, &tool_tasks);

        if !tool_tasks.is_empty() {
            child_index += 1;
            self.push_tool_group(
                child_order_key(parent_order_key, child_index),
                parent_trace_node_id,
                parent_stable_locator,
                parent_node_runs,
                &tool_tasks,
                &synthetic_tool_calls,
            )?;
        } else if !synthetic_tool_calls.is_empty() {
            child_index += 1;
            self.push_synthetic_tool_group(
                child_order_key(parent_order_key, child_index),
                parent_trace_node_id,
                parent_stable_locator,
                parent_node_runs,
                &synthetic_tool_calls,
            )?;
        }

        for task in callback_tasks
            .iter()
            .filter(|task| task.callback_kind != "llm_tool_calls")
        {
            child_index += 1;
            self.push_callback_task_node(
                child_order_key(parent_order_key, child_index),
                parent_trace_node_id,
                parent_stable_locator,
                task,
            )?;
        }

        Ok(())
    }

    fn push_tool_group(
        &mut self,
        order_key: String,
        parent_trace_node_id: Uuid,
        parent_stable_locator: &str,
        parent_node_runs: &[domain::NodeRunRecord],
        tool_tasks: &[&domain::CallbackTaskRecord],
        synthetic_tool_calls: &[serde_json::Value],
    ) -> Result<()> {
        let stable_locator = format!("{parent_stable_locator}/tools");
        let trace_node_id = trace_node_id_for_locator(self.flow_run_id, &stable_locator);
        let tool_call_count = tool_tasks
            .iter()
            .flat_map(|task| tool_calls_from_callback_task(task))
            .count()
            + synthetic_tool_calls.len();

        self.nodes.push(ApplicationRunTraceNodeProjectionInput {
            trace_node_id,
            parent_trace_node_id: Some(parent_trace_node_id),
            stable_locator: stable_locator.clone(),
            node_kind: "tool_group".to_string(),
            owner_kind: Some("node_run_tools".to_string()),
            owner_id: Some(parent_trace_node_id.to_string()),
            order_key: order_key.clone(),
            node_id: None,
            node_type: Some("tools".to_string()),
            node_alias: "Tools".to_string(),
            status: tool_group_status(tool_tasks),
            started_at: tool_tasks
                .iter()
                .map(|task| task.created_at)
                .min()
                .unwrap_or(OffsetDateTime::UNIX_EPOCH),
            finished_at: tool_tasks.iter().filter_map(|task| task.completed_at).max(),
            duration_ms: None,
            metrics_payload: serde_json::json!({}),
            has_children: tool_call_count > 0,
            child_count: i64::try_from(tool_call_count).unwrap_or(i64::MAX),
            has_content: false,
            content_ref: None,
        });

        let mut tool_index = 0_usize;
        for task in tool_tasks {
            for tool_call in tool_calls_from_callback_task(task) {
                tool_index += 1;
                self.push_tool_callback_node(
                    child_order_key(&order_key, tool_index),
                    trace_node_id,
                    &stable_locator,
                    parent_node_runs,
                    task,
                    &tool_call,
                )?;
            }
        }
        for tool_call in synthetic_tool_calls {
            tool_index += 1;
            self.push_synthetic_tool_callback_node(
                child_order_key(&order_key, tool_index),
                trace_node_id,
                &stable_locator,
                parent_node_runs,
                tool_call,
            )?;
        }

        Ok(())
    }

    fn push_tool_callback_node(
        &mut self,
        order_key: String,
        parent_trace_node_id: Uuid,
        parent_stable_locator: &str,
        parent_node_runs: &[domain::NodeRunRecord],
        task: &domain::CallbackTaskRecord,
        tool_call: &serde_json::Value,
    ) -> Result<()> {
        let tool_call_id = tool_call
            .get("id")
            .and_then(serde_json::Value::as_str)
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| legacy_locator_component("tool_call", &order_key, tool_call));
        let tool_name = tool_call
            .get("name")
            .and_then(serde_json::Value::as_str)
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| tool_call_id.clone());
        let stable_locator = format!("{parent_stable_locator}/tool:{tool_call_id}");
        let trace_node_id = trace_node_id_for_locator(self.flow_run_id, &stable_locator);
        let tool_result = tool_result_for_call(task, &tool_call_id);
        let route_trace = route_trace_for_tool_call(parent_node_runs, &tool_call_id);
        let payload = tool_callback_content_payload(
            Some(task),
            &tool_call_id,
            &tool_name,
            tool_call,
            tool_result.as_ref(),
            route_trace.as_ref(),
        );
        let has_route_child = route_trace.is_some();

        self.nodes.push(ApplicationRunTraceNodeProjectionInput {
            trace_node_id,
            parent_trace_node_id: Some(parent_trace_node_id),
            stable_locator: stable_locator.clone(),
            node_kind: "tool_callback".to_string(),
            owner_kind: Some("tool_call".to_string()),
            owner_id: Some(tool_call_id.clone()),
            order_key: order_key.clone(),
            node_id: None,
            node_type: Some("tool".to_string()),
            node_alias: tool_name,
            status: callback_task_trace_node_status(task),
            started_at: task.created_at,
            finished_at: task.completed_at,
            duration_ms: trace_node_duration_ms(task.created_at, task.completed_at),
            metrics_payload: serde_json::json!({}),
            has_children: has_route_child,
            child_count: i64::from(has_route_child),
            has_content: true,
            content_ref: None,
        });
        self.contents
            .push(ApplicationRunTraceNodeContentProjectionInput {
                trace_node_id,
                content_kind: "tool_callback".to_string(),
                payload,
                source_refs: serde_json::json!([{
                    "source_kind": "callback_task",
                    "source_locator": task.id
                }]),
            });
        if let Some(route_trace) = route_trace.as_ref() {
            self.push_tool_route_node(
                child_order_key(&order_key, 1),
                trace_node_id,
                &stable_locator,
                task.created_at,
                task.completed_at,
                route_trace,
            )?;
        }

        Ok(())
    }

    fn push_synthetic_tool_group(
        &mut self,
        order_key: String,
        parent_trace_node_id: Uuid,
        parent_stable_locator: &str,
        parent_node_runs: &[domain::NodeRunRecord],
        tool_calls: &[serde_json::Value],
    ) -> Result<()> {
        let stable_locator = format!("{parent_stable_locator}/tools");
        let trace_node_id = trace_node_id_for_locator(self.flow_run_id, &stable_locator);
        let first_node_run = &parent_node_runs[0];

        self.nodes.push(ApplicationRunTraceNodeProjectionInput {
            trace_node_id,
            parent_trace_node_id: Some(parent_trace_node_id),
            stable_locator: stable_locator.clone(),
            node_kind: "tool_group".to_string(),
            owner_kind: Some("node_run_tools".to_string()),
            owner_id: Some(parent_trace_node_id.to_string()),
            order_key: order_key.clone(),
            node_id: None,
            node_type: Some("tools".to_string()),
            node_alias: "Tools".to_string(),
            status: trace_node_group_status(parent_node_runs)
                .as_str()
                .to_string(),
            started_at: first_node_run.started_at,
            finished_at: trace_node_group_finished_at(parent_node_runs),
            duration_ms: None,
            metrics_payload: serde_json::json!({}),
            has_children: true,
            child_count: i64::try_from(tool_calls.len()).unwrap_or(i64::MAX),
            has_content: false,
            content_ref: None,
        });

        for (index, tool_call) in tool_calls.iter().enumerate() {
            self.push_synthetic_tool_callback_node(
                child_order_key(&order_key, index + 1),
                trace_node_id,
                &stable_locator,
                parent_node_runs,
                tool_call,
            )?;
        }

        Ok(())
    }

    fn push_synthetic_tool_callback_node(
        &mut self,
        order_key: String,
        parent_trace_node_id: Uuid,
        parent_stable_locator: &str,
        parent_node_runs: &[domain::NodeRunRecord],
        tool_call: &serde_json::Value,
    ) -> Result<()> {
        let tool_call_id = tool_call
            .get("id")
            .and_then(serde_json::Value::as_str)
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| legacy_locator_component("tool_call", &order_key, tool_call));
        let tool_name = tool_call
            .get("name")
            .and_then(serde_json::Value::as_str)
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| tool_call_id.clone());
        let stable_locator = format!("{parent_stable_locator}/tool:{tool_call_id}");
        let trace_node_id = trace_node_id_for_locator(self.flow_run_id, &stable_locator);
        let route_trace = route_trace_for_tool_call(parent_node_runs, &tool_call_id);
        let payload = tool_callback_content_payload(
            None,
            &tool_call_id,
            &tool_name,
            tool_call,
            None,
            route_trace.as_ref(),
        );
        let has_route_child = route_trace.is_some();

        self.nodes.push(ApplicationRunTraceNodeProjectionInput {
            trace_node_id,
            parent_trace_node_id: Some(parent_trace_node_id),
            stable_locator: stable_locator.clone(),
            node_kind: "tool_callback".to_string(),
            owner_kind: Some("tool_call".to_string()),
            owner_id: Some(tool_call_id.clone()),
            order_key: order_key.clone(),
            node_id: None,
            node_type: Some("tool".to_string()),
            node_alias: tool_name,
            status: trace_node_group_status(parent_node_runs)
                .as_str()
                .to_string(),
            started_at: parent_node_runs[0].started_at,
            finished_at: trace_node_group_finished_at(parent_node_runs),
            duration_ms: None,
            metrics_payload: serde_json::json!({}),
            has_children: has_route_child,
            child_count: i64::from(has_route_child),
            has_content: true,
            content_ref: None,
        });
        self.contents
            .push(ApplicationRunTraceNodeContentProjectionInput {
                trace_node_id,
                content_kind: "tool_callback".to_string(),
                payload,
                source_refs: serde_json::json!([{
                    "source_kind": "node_run_tool_call",
                    "source_locator": tool_call_id
                }]),
            });
        if let Some(route_trace) = route_trace.as_ref() {
            self.push_tool_route_node(
                child_order_key(&order_key, 1),
                trace_node_id,
                &stable_locator,
                parent_node_runs[0].started_at,
                trace_node_group_finished_at(parent_node_runs),
                route_trace,
            )?;
        }

        Ok(())
    }

    fn push_tool_route_node(
        &mut self,
        order_key: String,
        parent_trace_node_id: Uuid,
        parent_stable_locator: &str,
        started_at: OffsetDateTime,
        finished_at: Option<OffsetDateTime>,
        route_trace: &serde_json::Value,
    ) -> Result<()> {
        let node_kind = route_trace_node_kind(route_trace).to_string();
        let locator_component = route_trace_locator_component(route_trace, &node_kind, &order_key);
        let stable_locator = format!("{parent_stable_locator}/{node_kind}:{locator_component}");
        let trace_node_id = trace_node_id_for_locator(self.flow_run_id, &stable_locator);
        let branch_traces = route_trace_branch_traces(route_trace);
        let child_count = i64::try_from(branch_traces.len()).unwrap_or(i64::MAX);

        self.nodes.push(ApplicationRunTraceNodeProjectionInput {
            trace_node_id,
            parent_trace_node_id: Some(parent_trace_node_id),
            stable_locator: stable_locator.clone(),
            node_kind: node_kind.clone(),
            owner_kind: Some(node_kind.clone()),
            owner_id: Some(locator_component.clone()),
            order_key: order_key.clone(),
            node_id: route_trace
                .get("route_id")
                .or_else(|| route_trace.get("node_id"))
                .and_then(serde_json::Value::as_str)
                .map(ToOwned::to_owned),
            node_type: Some(node_kind.clone()),
            node_alias: route_trace_node_alias(route_trace, &node_kind),
            status: route_trace_status(route_trace),
            started_at,
            finished_at,
            duration_ms: trace_node_duration_ms(started_at, finished_at),
            metrics_payload: route_trace_metrics_payload(route_trace),
            has_children: child_count > 0,
            child_count,
            has_content: true,
            content_ref: None,
        });
        self.contents
            .push(ApplicationRunTraceNodeContentProjectionInput {
                trace_node_id,
                content_kind: node_kind,
                payload: route_trace.clone(),
                source_refs: serde_json::json!([{
                    "source_kind": "visible_internal_llm_tool_trace",
                    "source_locator": locator_component
                }]),
            });

        for (index, branch_trace) in branch_traces.iter().enumerate() {
            self.push_route_branch_node(
                child_order_key(&order_key, index + 1),
                trace_node_id,
                &stable_locator,
                started_at,
                finished_at,
                branch_trace,
            )?;
        }

        Ok(())
    }

    fn push_route_branch_node(
        &mut self,
        order_key: String,
        parent_trace_node_id: Uuid,
        parent_stable_locator: &str,
        started_at: OffsetDateTime,
        finished_at: Option<OffsetDateTime>,
        branch_trace: &serde_json::Value,
    ) -> Result<()> {
        let locator_component = branch_locator_component(branch_trace, &order_key);
        let stable_locator = format!("{parent_stable_locator}/branch:{locator_component}");
        let trace_node_id = trace_node_id_for_locator(self.flow_run_id, &stable_locator);

        self.nodes.push(ApplicationRunTraceNodeProjectionInput {
            trace_node_id,
            parent_trace_node_id: Some(parent_trace_node_id),
            stable_locator,
            node_kind: "branch".to_string(),
            owner_kind: Some("branch".to_string()),
            owner_id: Some(locator_component.clone()),
            order_key,
            node_id: branch_trace
                .get("node_id")
                .and_then(serde_json::Value::as_str)
                .map(ToOwned::to_owned),
            node_type: branch_trace
                .get("node_type")
                .and_then(serde_json::Value::as_str)
                .map(ToOwned::to_owned)
                .or_else(|| Some("branch".to_string())),
            node_alias: branch_trace_node_alias(branch_trace),
            status: branch_trace_status(branch_trace),
            started_at,
            finished_at,
            duration_ms: trace_node_duration_ms(started_at, finished_at),
            metrics_payload: route_trace_metrics_payload(branch_trace),
            has_children: false,
            child_count: 0,
            has_content: true,
            content_ref: None,
        });
        self.contents
            .push(ApplicationRunTraceNodeContentProjectionInput {
                trace_node_id,
                content_kind: "branch".to_string(),
                payload: branch_trace.clone(),
                source_refs: serde_json::json!([{
                    "source_kind": "visible_internal_llm_tool_branch",
                    "source_locator": locator_component
                }]),
            });

        Ok(())
    }

    fn push_callback_task_node(
        &mut self,
        order_key: String,
        parent_trace_node_id: Uuid,
        parent_stable_locator: &str,
        task: &domain::CallbackTaskRecord,
    ) -> Result<()> {
        let stable_locator = format!("{parent_stable_locator}/callback_task:{}", task.id);
        let trace_node_id = trace_node_id_for_locator(self.flow_run_id, &stable_locator);

        self.nodes.push(ApplicationRunTraceNodeProjectionInput {
            trace_node_id,
            parent_trace_node_id: Some(parent_trace_node_id),
            stable_locator,
            node_kind: "callback_task".to_string(),
            owner_kind: Some("callback_task".to_string()),
            owner_id: Some(task.id.to_string()),
            order_key,
            node_id: None,
            node_type: Some(task.callback_kind.clone()),
            node_alias: task.callback_kind.clone(),
            status: callback_task_trace_node_status(task),
            started_at: task.created_at,
            finished_at: task.completed_at,
            duration_ms: trace_node_duration_ms(task.created_at, task.completed_at),
            metrics_payload: serde_json::json!({}),
            has_children: false,
            child_count: 0,
            has_content: true,
            content_ref: None,
        });
        self.contents
            .push(ApplicationRunTraceNodeContentProjectionInput {
                trace_node_id,
                content_kind: "callback_task".to_string(),
                payload: serde_json::to_value(task)?,
                source_refs: serde_json::json!([{
                    "source_kind": "callback_task",
                    "source_locator": task.id
                }]),
            });

        Ok(())
    }

    fn push_stitched_context_group(
        &mut self,
        root_index: usize,
        stitched_trace: &[domain::ApplicationRunStitchedTrace],
    ) -> Result<()> {
        let order_key = root_order_key(root_index);
        let stable_locator = format!("run:{}/stitched_context", self.flow_run_id);
        let trace_node_id = trace_node_id_for_locator(self.flow_run_id, &stable_locator);

        self.nodes.push(ApplicationRunTraceNodeProjectionInput {
            trace_node_id,
            parent_trace_node_id: None,
            stable_locator: stable_locator.clone(),
            node_kind: "stitched_context".to_string(),
            owner_kind: Some("stitched_context".to_string()),
            owner_id: Some(self.flow_run_id.to_string()),
            order_key: order_key.clone(),
            node_id: None,
            node_type: Some("stitched_context".to_string()),
            node_alias: "Stitched context".to_string(),
            status: "succeeded".to_string(),
            started_at: stitched_trace
                .iter()
                .map(|trace| trace.source_flow_run.started_at)
                .min()
                .unwrap_or(OffsetDateTime::UNIX_EPOCH),
            finished_at: stitched_trace
                .iter()
                .filter_map(|trace| trace.source_flow_run.finished_at)
                .max(),
            duration_ms: None,
            metrics_payload: serde_json::json!({}),
            has_children: true,
            child_count: i64::try_from(stitched_trace.len()).unwrap_or(i64::MAX),
            has_content: false,
            content_ref: None,
        });

        for (index, trace) in stitched_trace.iter().enumerate() {
            self.push_stitched_run_summary(
                child_order_key(&order_key, index + 1),
                trace_node_id,
                &stable_locator,
                trace,
            )?;
        }

        Ok(())
    }

    fn push_stitched_run_summary(
        &mut self,
        order_key: String,
        parent_trace_node_id: Uuid,
        parent_stable_locator: &str,
        trace: &domain::ApplicationRunStitchedTrace,
    ) -> Result<()> {
        let source_run = &trace.source_flow_run;
        let stable_locator = format!("{parent_stable_locator}/run:{}", source_run.id);
        let trace_node_id = trace_node_id_for_locator(self.flow_run_id, &stable_locator);

        self.nodes.push(ApplicationRunTraceNodeProjectionInput {
            trace_node_id,
            parent_trace_node_id: Some(parent_trace_node_id),
            stable_locator,
            node_kind: "stitched_run".to_string(),
            owner_kind: Some("flow_run".to_string()),
            owner_id: Some(source_run.id.to_string()),
            order_key,
            node_id: None,
            node_type: Some("flow_run".to_string()),
            node_alias: source_run.title.clone(),
            status: source_run.status.as_str().to_string(),
            started_at: source_run.started_at,
            finished_at: source_run.finished_at,
            duration_ms: trace_node_duration_ms(source_run.started_at, source_run.finished_at),
            metrics_payload: serde_json::json!({}),
            has_children: !trace.node_runs.is_empty(),
            child_count: i64::try_from(trace.node_runs.len()).unwrap_or(i64::MAX),
            has_content: true,
            content_ref: None,
        });
        self.contents
            .push(ApplicationRunTraceNodeContentProjectionInput {
                trace_node_id,
                content_kind: "stitched_run".to_string(),
                payload: serde_json::to_value(source_run)?,
                source_refs: serde_json::json!([{
                    "source_kind": "flow_run",
                    "source_locator": source_run.id
                }]),
            });

        Ok(())
    }
}

fn json_object_has_keys(value: &serde_json::Value) -> bool {
    value.as_object().is_some_and(|object| !object.is_empty())
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

fn trace_node_group_finished_at(node_runs: &[domain::NodeRunRecord]) -> Option<OffsetDateTime> {
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
        merged.insert(
            "llm_rounds".to_string(),
            serde_json::Value::Array(llm_rounds),
        );
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

fn merge_metric_usage_value(node_runs: &[domain::NodeRunRecord], usage_key: &str) -> Option<i64> {
    let mut total = None;

    for node_run in node_runs {
        if let Some(value) = node_run
            .metrics_payload
            .get("usage")
            .and_then(|usage| usage.get(usage_key))
            .and_then(serde_json::Value::as_i64)
        {
            total = Some(total.unwrap_or(0_i64).saturating_add(value));
        }
    }

    total
}

fn merge_metrics_payloads(node_runs: &[domain::NodeRunRecord]) -> serde_json::Value {
    let mut usage = serde_json::Map::new();

    for key in [
        "total_tokens",
        "input_tokens",
        "output_tokens",
        "input_cache_hit_tokens",
        "cache_read_tokens",
    ] {
        if let Some(value) = merge_metric_usage_value(node_runs, key) {
            usage.insert(key.to_string(), serde_json::json!(value));
        }
    }

    if usage.is_empty() {
        return last_non_empty_json(node_runs, |node_run| &node_run.metrics_payload);
    }

    serde_json::json!({ "usage": usage })
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
    merged.metrics_payload = merge_metrics_payloads(node_runs);
    merged.debug_payload = merge_debug_payloads(node_runs);

    merged
}

fn trace_node_content_debug_payload_without_tool_index(
    debug_payload: serde_json::Value,
) -> serde_json::Value {
    let serde_json::Value::Object(mut object) = debug_payload else {
        return debug_payload;
    };

    object.remove("llm_rounds");
    object.remove("tool_callbacks");

    serde_json::Value::Object(object)
}

fn callback_tasks_for_node_run_ids(
    detail: &domain::ApplicationRunDetail,
    node_run_ids: &HashSet<Uuid>,
) -> Vec<domain::CallbackTaskRecord> {
    detail
        .callback_tasks
        .iter()
        .filter(|task| node_run_ids.contains(&task.node_run_id))
        .cloned()
        .collect()
}

fn synthetic_tool_calls_not_in_callback_tasks(
    node_runs: &[domain::NodeRunRecord],
    tool_tasks: &[&domain::CallbackTaskRecord],
) -> Vec<serde_json::Value> {
    if tool_tasks.is_empty() {
        return tool_calls_from_node_runs(node_runs);
    }

    let callback_tool_call_keys = tool_tasks
        .iter()
        .flat_map(|task| tool_calls_from_callback_task(task))
        .map(|tool_call| tool_call_dedup_key(&tool_call))
        .collect::<HashSet<_>>();

    tool_calls_from_node_runs(node_runs)
        .into_iter()
        .filter(|tool_call| !callback_tool_call_keys.contains(&tool_call_dedup_key(tool_call)))
        .collect()
}

fn tool_call_dedup_key(tool_call: &serde_json::Value) -> String {
    tool_call
        .get("id")
        .or_else(|| tool_call.get("tool_call_id"))
        .or_else(|| tool_call.get("call_id"))
        .and_then(serde_json::Value::as_str)
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| tool_call.to_string())
}

fn tool_calls_from_callback_task(task: &domain::CallbackTaskRecord) -> Vec<serde_json::Value> {
    task.request_payload
        .get("tool_calls")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default()
}

fn tool_calls_from_node_runs(node_runs: &[domain::NodeRunRecord]) -> Vec<serde_json::Value> {
    let mut tool_calls = Vec::new();
    let mut seen_tool_call_ids = HashSet::<String>::new();

    for node_run in node_runs {
        for tool_call in tool_calls_from_node_payload(&node_run.output_payload)
            .into_iter()
            .chain(tool_calls_from_node_debug_payload(&node_run.debug_payload))
        {
            let tool_call_id = tool_call
                .get("id")
                .or_else(|| tool_call.get("tool_call_id"))
                .or_else(|| tool_call.get("call_id"))
                .and_then(serde_json::Value::as_str)
                .map(ToOwned::to_owned)
                .unwrap_or_else(|| {
                    legacy_locator_component(
                        "node_run_tool_call",
                        &node_run.id.to_string(),
                        &tool_call,
                    )
                });

            if seen_tool_call_ids.insert(tool_call_id) {
                tool_calls.push(tool_call);
            }
        }
    }

    tool_calls
}

fn tool_calls_from_node_payload(payload: &serde_json::Value) -> Vec<serde_json::Value> {
    payload
        .get("tool_calls")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default()
}

fn tool_calls_from_node_debug_payload(payload: &serde_json::Value) -> Vec<serde_json::Value> {
    payload
        .get("llm_rounds")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|round| round.get("assistant"))
        .filter_map(|assistant| assistant.get("tool_calls"))
        .filter_map(serde_json::Value::as_array)
        .flatten()
        .cloned()
        .collect()
}

fn tool_result_for_call(
    task: &domain::CallbackTaskRecord,
    tool_call_id: &str,
) -> Option<serde_json::Value> {
    task.response_payload
        .as_ref()
        .and_then(|payload| payload.get("tool_results"))
        .and_then(serde_json::Value::as_array)
        .and_then(|items| {
            items
                .iter()
                .find(|item| {
                    item.get("tool_call_id")
                        .or_else(|| item.get("id"))
                        .and_then(serde_json::Value::as_str)
                        == Some(tool_call_id)
                })
                .cloned()
        })
}

fn route_trace_for_tool_call(
    parent_node_runs: &[domain::NodeRunRecord],
    tool_call_id: &str,
) -> Option<serde_json::Value> {
    parent_node_runs
        .iter()
        .filter_map(|node_run| {
            node_run
                .debug_payload
                .get("visible_internal_llm_tool_trace")
                .and_then(serde_json::Value::as_array)
        })
        .flatten()
        .find(|trace| {
            trace
                .get("tool_call_id")
                .and_then(serde_json::Value::as_str)
                == Some(tool_call_id)
        })
        .cloned()
}

fn route_trace_node_kind(route_trace: &serde_json::Value) -> &'static str {
    match route_trace
        .get("route_kind")
        .or_else(|| route_trace.get("kind"))
        .and_then(serde_json::Value::as_str)
    {
        Some("fusion") | Some("visible_internal_llm_tool_fusion") => "fusion",
        _ => "route",
    }
}

fn route_trace_locator_component(
    route_trace: &serde_json::Value,
    node_kind: &str,
    order_key: &str,
) -> String {
    route_trace
        .get("route_ref")
        .or_else(|| route_trace.get("route_id"))
        .or_else(|| route_trace.get("fusion_ref"))
        .or_else(|| route_trace.get("fusion_id"))
        .or_else(|| route_trace.get("tool_call_id"))
        .and_then(serde_json::Value::as_str)
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| legacy_locator_component(node_kind, order_key, route_trace))
}

fn route_trace_branch_traces(route_trace: &serde_json::Value) -> Vec<serde_json::Value> {
    route_trace
        .get("branch_traces")
        .or_else(|| route_trace.get("branch_summaries"))
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default()
}

fn route_trace_node_alias(route_trace: &serde_json::Value, node_kind: &str) -> String {
    route_trace
        .get("route_alias")
        .or_else(|| route_trace.get("fusion_alias"))
        .or_else(|| route_trace.get("tool_name"))
        .or_else(|| route_trace.get("route_model"))
        .and_then(serde_json::Value::as_str)
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| {
            if node_kind == "fusion" {
                "Fusion".to_string()
            } else {
                "Route".to_string()
            }
        })
}

fn branch_locator_component(branch_trace: &serde_json::Value, order_key: &str) -> String {
    branch_trace
        .get("branch_ref")
        .or_else(|| branch_trace.get("branch_id"))
        .or_else(|| branch_trace.get("node_run_id"))
        .or_else(|| branch_trace.get("node_id"))
        .and_then(serde_json::Value::as_str)
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| legacy_locator_component("branch", order_key, branch_trace))
}

fn branch_trace_node_alias(branch_trace: &serde_json::Value) -> String {
    branch_trace
        .get("node_alias")
        .or_else(|| branch_trace.get("branch_alias"))
        .or_else(|| branch_trace.get("node_id"))
        .and_then(serde_json::Value::as_str)
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| "Branch".to_string())
}

fn route_trace_status(route_trace: &serde_json::Value) -> String {
    route_trace
        .get("status")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("completed")
        .to_string()
}

fn branch_trace_status(branch_trace: &serde_json::Value) -> String {
    branch_trace
        .get("status")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("completed")
        .to_string()
}

fn route_trace_metrics_payload(route_trace: &serde_json::Value) -> serde_json::Value {
    route_trace
        .get("metrics_payload")
        .or_else(|| route_trace.get("usage"))
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}))
}

fn callback_status(task: &domain::CallbackTaskRecord) -> &'static str {
    if task.response_payload.is_some() {
        "returned"
    } else {
        "waiting_callback"
    }
}

fn tool_result_execution_status(tool_result: Option<&serde_json::Value>) -> Option<String> {
    tool_result
        .and_then(|value| value.get("execution_status"))
        .and_then(serde_json::Value::as_str)
        .map(ToOwned::to_owned)
}

fn tool_callback_content_payload(
    task: Option<&domain::CallbackTaskRecord>,
    tool_call_id: &str,
    tool_name: &str,
    tool_call: &serde_json::Value,
    tool_result: Option<&serde_json::Value>,
    route_trace: Option<&serde_json::Value>,
) -> serde_json::Value {
    serde_json::json!({
        "id": tool_call_id,
        "name": tool_name,
        "callback_task_id": task.map(|task| task.id),
        "tool_call_id": tool_call_id,
        "callback_status": task.map(callback_status).unwrap_or("waiting_callback"),
        "execution_status": tool_result_execution_status(tool_result),
        "request_payload": tool_call,
        "callback_payload": tool_result,
        "parsed_result": tool_result,
        "duration_ms": task.and_then(|task| trace_node_duration_ms(task.created_at, task.completed_at)),
        "route_trace": route_trace,
        "tool_call": tool_call,
        "tool_result": tool_result,
    })
}

fn callback_task_trace_node_status(task: &domain::CallbackTaskRecord) -> String {
    match task.status {
        domain::CallbackTaskStatus::Pending => domain::NodeRunStatus::WaitingCallback,
        domain::CallbackTaskStatus::Completed => domain::NodeRunStatus::Succeeded,
        domain::CallbackTaskStatus::Cancelled => domain::NodeRunStatus::Failed,
    }
    .as_str()
    .to_string()
}

fn tool_group_status(tool_tasks: &[&domain::CallbackTaskRecord]) -> String {
    if tool_tasks
        .iter()
        .any(|task| task.status == domain::CallbackTaskStatus::Pending)
    {
        return domain::NodeRunStatus::WaitingCallback.as_str().to_string();
    }

    if tool_tasks
        .iter()
        .any(|task| task.status == domain::CallbackTaskStatus::Cancelled)
    {
        return domain::NodeRunStatus::Failed.as_str().to_string();
    }

    domain::NodeRunStatus::Succeeded.as_str().to_string()
}

fn node_run_group_content(
    trace_node_id: Uuid,
    node_runs: &[domain::NodeRunRecord],
    detail: &domain::ApplicationRunDetail,
) -> Result<ApplicationRunTraceNodeContentProjectionInput> {
    let node_run_ids = node_runs
        .iter()
        .map(|node_run| node_run.id)
        .collect::<HashSet<_>>();
    let mut merged_node_run = merge_node_run_group(node_runs);
    merged_node_run.debug_payload =
        trace_node_content_debug_payload_without_tool_index(merged_node_run.debug_payload);
    let checkpoints: Vec<&domain::CheckpointRecord> = detail
        .checkpoints
        .iter()
        .filter(|checkpoint| {
            checkpoint
                .node_run_id
                .is_some_and(|node_run_id| node_run_ids.contains(&node_run_id))
        })
        .collect();
    let events: Vec<&domain::RunEventRecord> = detail
        .events
        .iter()
        .filter(|event| {
            event
                .node_run_id
                .is_some_and(|node_run_id| node_run_ids.contains(&node_run_id))
        })
        .collect();
    let source_refs = node_runs
        .iter()
        .map(|node_run| {
            serde_json::json!({
                "source_kind": "node_run",
                "source_locator": node_run.id
            })
        })
        .collect::<Vec<_>>();

    Ok(ApplicationRunTraceNodeContentProjectionInput {
        trace_node_id,
        content_kind: "node_run".to_string(),
        payload: serde_json::json!({
            "node_run": merged_node_run,
            "checkpoints": checkpoints,
            "events": events
        }),
        source_refs: serde_json::Value::Array(source_refs),
    })
}

fn root_order_key(index: usize) -> String {
    format!("{:06}", index + 1)
}

fn child_order_key(parent_order_key: &str, index: usize) -> String {
    format!("{parent_order_key}/{index:06}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn stable_trace_node_id_is_deterministic_for_locator() {
        let flow_run_id = Uuid::now_v7();
        let locator = format!("run:{flow_run_id}/node:{}", Uuid::now_v7());

        let first = trace_node_id_for_locator(flow_run_id, &locator);
        let second = trace_node_id_for_locator(flow_run_id, &locator);
        let other = trace_node_id_for_locator(flow_run_id, &(locator.clone() + "/tools"));

        assert_eq!(first, second);
        assert_ne!(first, other);
    }

    #[test]
    fn legacy_locator_component_uses_source_payload_fingerprint() {
        let first = legacy_locator_component(
            "visible_internal_llm_tool_trace",
            "000001/000001",
            &json!({ "route": "image", "branch": 1 }),
        );
        let second = legacy_locator_component(
            "visible_internal_llm_tool_trace",
            "000001/000001",
            &json!({ "route": "image", "branch": 1 }),
        );
        let changed = legacy_locator_component(
            "visible_internal_llm_tool_trace",
            "000001/000001",
            &json!({ "route": "image", "branch": 2 }),
        );

        assert!(first.starts_with("legacy:"));
        assert_eq!(first, second);
        assert_ne!(first, changed);
    }

    #[test]
    fn builder_projects_node_run_tool_group_and_tool_callbacks() {
        let flow_run_id = Uuid::now_v7();
        let node_run_id = Uuid::now_v7();
        let callback_task_id = Uuid::now_v7();
        let now = OffsetDateTime::UNIX_EPOCH;
        let detail = domain::ApplicationRunDetail {
            flow_run: flow_run(flow_run_id, now),
            node_runs: vec![domain::NodeRunRecord {
                id: node_run_id,
                flow_run_id,
                node_id: "node-llm".to_string(),
                node_type: "llm".to_string(),
                node_alias: "Main LLM".to_string(),
                status: domain::NodeRunStatus::Succeeded,
                input_payload: json!({ "prompt": "weather" }),
                output_payload: json!({ "answer": "done" }),
                error_payload: None,
                metrics_payload: json!({ "usage": { "total_tokens": 12 } }),
                debug_payload: json!({
                    "tool_callbacks": [
                        {
                            "id": "call-weather",
                            "name": "weather"
                        }
                    ],
                    "llm_rounds": [
                        {
                            "round_index": 0,
                            "assistant": {
                                "tool_calls": [
                                    {
                                        "id": "call-weather",
                                        "name": "weather"
                                    }
                                ]
                            }
                        }
                    ],
                    "debug_summary": {
                        "kept": true
                    }
                }),
                started_at: now,
                finished_at: Some(now + time::Duration::seconds(2)),
            }],
            checkpoints: Vec::new(),
            callback_tasks: vec![domain::CallbackTaskRecord {
                id: callback_task_id,
                flow_run_id,
                node_run_id,
                callback_kind: "llm_tool_calls".to_string(),
                status: domain::CallbackTaskStatus::Completed,
                request_payload: json!({
                    "tool_calls": [
                        { "id": "call-weather", "name": "weather" }
                    ]
                }),
                response_payload: Some(json!({
                    "tool_results": [
                        {
                            "tool_call_id": "call-weather",
                            "content": "22c"
                        }
                    ]
                })),
                external_ref_payload: None,
                created_at: now + time::Duration::seconds(1),
                completed_at: Some(now + time::Duration::seconds(2)),
            }],
            events: Vec::new(),
            stitched_trace: Vec::new(),
        };

        let projection = build_application_run_trace_projection(&detail).unwrap();
        let locators: Vec<&str> = projection
            .nodes
            .iter()
            .map(|node| node.stable_locator.as_str())
            .collect();

        assert_eq!(
            projection.projection_version,
            APPLICATION_RUN_TRACE_PROJECTION_VERSION
        );
        assert!(locators.contains(&format!("run:{flow_run_id}/node:{node_run_id}").as_str()));
        assert!(locators.contains(&format!("run:{flow_run_id}/node:{node_run_id}/tools").as_str()));
        assert!(locators.contains(
            &format!("run:{flow_run_id}/node:{node_run_id}/tools/tool:call-weather").as_str()
        ));
        assert_eq!(projection.contents.len(), 2);
        let node_run_content = projection
            .contents
            .iter()
            .find(|content| content.content_kind == "node_run")
            .expect("node run content should be projected");
        let projected_debug_payload = &node_run_content.payload["node_run"]["debug_payload"];
        assert!(projected_debug_payload.get("tool_callbacks").is_none());
        assert!(projected_debug_payload.get("llm_rounds").is_none());
        assert_eq!(
            projected_debug_payload["debug_summary"]["kept"],
            json!(true)
        );
        assert!(projection.contents.iter().any(|content| {
            content.content_kind == "tool_callback"
                && content.payload["tool_call_id"] == json!("call-weather")
                && content.payload["tool_result"]["content"] == json!("22c")
        }));
    }

    #[test]
    fn builder_projects_tool_route_fusion_and_branch_nodes() {
        let flow_run_id = Uuid::now_v7();
        let node_run_id = Uuid::now_v7();
        let callback_task_id = Uuid::now_v7();
        let now = OffsetDateTime::UNIX_EPOCH;
        let detail = domain::ApplicationRunDetail {
            flow_run: flow_run(flow_run_id, now),
            node_runs: vec![domain::NodeRunRecord {
                id: node_run_id,
                flow_run_id,
                node_id: "node-llm".to_string(),
                node_type: "llm".to_string(),
                node_alias: "Main LLM".to_string(),
                status: domain::NodeRunStatus::Succeeded,
                input_payload: json!({ "prompt": "review" }),
                output_payload: json!({ "answer": "done" }),
                error_payload: None,
                metrics_payload: json!({}),
                debug_payload: json!({
                    "visible_internal_llm_tool_trace": [
                        {
                            "kind": "visible_internal_llm_tool_trace",
                            "route_kind": "fusion",
                            "tool_call_id": "call-review",
                            "tool_name": "problem_review",
                            "status": "succeeded",
                            "route_model": "mimo-v2.5",
                            "branch_traces": [
                                {
                                    "branch_ref": "panel-a",
                                    "node_id": "node-panel-a",
                                    "node_alias": "LLM2",
                                    "node_type": "llm",
                                    "status": "succeeded",
                                    "output_summary": {
                                        "preview": "panel A says strict"
                                    }
                                }
                            ]
                        }
                    ]
                }),
                started_at: now,
                finished_at: Some(now + time::Duration::seconds(2)),
            }],
            checkpoints: Vec::new(),
            callback_tasks: vec![domain::CallbackTaskRecord {
                id: callback_task_id,
                flow_run_id,
                node_run_id,
                callback_kind: "llm_tool_calls".to_string(),
                status: domain::CallbackTaskStatus::Completed,
                request_payload: json!({
                    "tool_calls": [
                        { "id": "call-review", "name": "problem_review" }
                    ]
                }),
                response_payload: Some(json!({
                    "tool_results": [
                        {
                            "tool_call_id": "call-review",
                            "content": "review complete"
                        }
                    ]
                })),
                external_ref_payload: None,
                created_at: now + time::Duration::seconds(1),
                completed_at: Some(now + time::Duration::seconds(2)),
            }],
            events: Vec::new(),
            stitched_trace: Vec::new(),
        };

        let projection = build_application_run_trace_projection(&detail).unwrap();
        let tool_callback = projection
            .nodes
            .iter()
            .find(|node| node.node_kind == "tool_callback")
            .expect("tool callback node should be projected");
        let fusion = projection
            .nodes
            .iter()
            .find(|node| node.node_kind == "fusion")
            .expect("fusion route node should be projected");
        let branch = projection
            .nodes
            .iter()
            .find(|node| node.node_kind == "branch")
            .expect("branch node should be projected");

        assert!(tool_callback.has_children);
        assert_eq!(tool_callback.child_count, 1);
        assert_eq!(
            fusion.parent_trace_node_id,
            Some(tool_callback.trace_node_id)
        );
        assert!(fusion.has_children);
        assert_eq!(fusion.child_count, 1);
        assert_eq!(branch.parent_trace_node_id, Some(fusion.trace_node_id));
        assert!(fusion.stable_locator.ends_with("/fusion:call-review"));
        assert!(branch.stable_locator.ends_with("/branch:panel-a"));
        assert!(projection.contents.iter().any(|content| {
            content.trace_node_id == fusion.trace_node_id
                && content.content_kind == "fusion"
                && content.payload["route_model"] == json!("mimo-v2.5")
        }));
        assert!(projection.contents.iter().any(|content| {
            content.trace_node_id == branch.trace_node_id
                && content.content_kind == "branch"
                && content.payload["output_summary"]["preview"] == json!("panel A says strict")
        }));
    }

    #[test]
    fn builder_merges_callback_task_tools_with_internal_route_tools() {
        let flow_run_id = Uuid::now_v7();
        let callback_node_run_id = Uuid::now_v7();
        let route_node_run_id = Uuid::now_v7();
        let callback_task_id = Uuid::now_v7();
        let now = OffsetDateTime::UNIX_EPOCH;
        let ordinary_tool_calls = json!([
            { "id": "call-read-memory", "name": "Read" },
            { "id": "call-read-agents", "name": "Read" },
            { "id": "call-git-pull", "name": "Bash" },
            { "id": "call-git-log", "name": "Bash" },
            { "id": "call-git-log-detail", "name": "Bash" }
        ]);
        let branch_traces = json!([
            {
                "branch_ref": "panel-a",
                "node_id": "node-llm-2",
                "node_alias": "LLM2",
                "node_type": "llm",
                "status": "succeeded"
            },
            {
                "branch_ref": "panel-b",
                "node_id": "node-llm-3",
                "node_alias": "LLM3",
                "node_type": "llm",
                "status": "succeeded"
            },
            {
                "branch_ref": "panel-c",
                "node_id": "node-llm-4",
                "node_alias": "LLM4",
                "node_type": "llm",
                "status": "succeeded"
            },
            {
                "branch_ref": "panel-d",
                "node_id": "node-llm-5",
                "node_alias": "LLM5",
                "node_type": "llm",
                "status": "succeeded"
            }
        ]);
        let detail = domain::ApplicationRunDetail {
            flow_run: flow_run(flow_run_id, now),
            node_runs: vec![
                domain::NodeRunRecord {
                    id: callback_node_run_id,
                    flow_run_id,
                    node_id: "node-llm".to_string(),
                    node_type: "llm".to_string(),
                    node_alias: "Main LLM".to_string(),
                    status: domain::NodeRunStatus::Succeeded,
                    input_payload: json!({ "prompt": "prepare context" }),
                    output_payload: json!({ "tool_calls": ordinary_tool_calls }),
                    error_payload: None,
                    metrics_payload: json!({}),
                    debug_payload: json!({}),
                    started_at: now,
                    finished_at: Some(now + time::Duration::seconds(5)),
                },
                domain::NodeRunRecord {
                    id: route_node_run_id,
                    flow_run_id,
                    node_id: "node-llm".to_string(),
                    node_type: "llm".to_string(),
                    node_alias: "Main LLM".to_string(),
                    status: domain::NodeRunStatus::Succeeded,
                    input_payload: json!({ "prompt": "review latest commits" }),
                    output_payload: json!({ "answer": "review complete" }),
                    error_payload: None,
                    metrics_payload: json!({}),
                    debug_payload: json!({
                        "llm_rounds": [
                            {
                                "round_index": 3,
                                "assistant": {
                                    "tool_calls": [
                                        {
                                            "id": "call-problem-review",
                                            "name": "problem_review"
                                        }
                                    ]
                                },
                                "tool_results": [
                                    {
                                        "tool_call_id": "call-problem-review",
                                        "name": "problem_review",
                                        "content": "problem review result"
                                    }
                                ]
                            }
                        ],
                        "visible_internal_llm_tool_trace": [
                            {
                                "kind": "visible_internal_llm_tool_trace",
                                "route_kind": "fusion",
                                "tool_call_id": "call-problem-review",
                                "tool_name": "problem_review",
                                "status": "succeeded",
                                "route_model": "gemini-3-flash",
                                "branch_traces": branch_traces
                            }
                        ]
                    }),
                    started_at: now + time::Duration::seconds(6),
                    finished_at: Some(now + time::Duration::seconds(12)),
                },
            ],
            checkpoints: Vec::new(),
            callback_tasks: vec![domain::CallbackTaskRecord {
                id: callback_task_id,
                flow_run_id,
                node_run_id: callback_node_run_id,
                callback_kind: "llm_tool_calls".to_string(),
                status: domain::CallbackTaskStatus::Completed,
                request_payload: json!({
                    "tool_calls": ordinary_tool_calls
                }),
                response_payload: Some(json!({
                    "tool_results": [
                        { "tool_call_id": "call-read-memory", "content": "memory" },
                        { "tool_call_id": "call-read-agents", "content": "agents" },
                        { "tool_call_id": "call-git-pull", "content": "pulled" },
                        { "tool_call_id": "call-git-log", "content": "log" },
                        { "tool_call_id": "call-git-log-detail", "content": "details" }
                    ]
                })),
                external_ref_payload: None,
                created_at: now + time::Duration::seconds(1),
                completed_at: Some(now + time::Duration::seconds(5)),
            }],
            events: Vec::new(),
            stitched_trace: Vec::new(),
        };

        let projection = build_application_run_trace_projection(&detail).unwrap();
        let tools = projection
            .nodes
            .iter()
            .find(|node| node.node_kind == "tool_group")
            .expect("tool group should be projected");
        let tool_callbacks: Vec<_> = projection
            .nodes
            .iter()
            .filter(|node| node.parent_trace_node_id == Some(tools.trace_node_id))
            .collect();
        let problem_review = tool_callbacks
            .iter()
            .find(|node| node.node_alias == "problem_review")
            .expect("internal route tool should be projected beside callback task tools");
        let fusion = projection
            .nodes
            .iter()
            .find(|node| node.parent_trace_node_id == Some(problem_review.trace_node_id))
            .expect("problem_review should expose its fusion route");
        let branch_count = projection
            .nodes
            .iter()
            .filter(|node| node.parent_trace_node_id == Some(fusion.trace_node_id))
            .count();

        assert_eq!(tools.child_count, 6);
        assert_eq!(tool_callbacks.len(), 6);
        assert!(problem_review.has_children);
        assert_eq!(problem_review.child_count, 1);
        assert_eq!(fusion.node_kind, "fusion");
        assert_eq!(fusion.child_count, 4);
        assert_eq!(branch_count, 4);
    }

    #[test]
    fn builder_projects_stitched_trace_as_collapsed_context_group() {
        let flow_run_id = Uuid::now_v7();
        let prior_run_id = Uuid::now_v7();
        let now = OffsetDateTime::UNIX_EPOCH;
        let mut detail = domain::ApplicationRunDetail {
            flow_run: flow_run(flow_run_id, now),
            node_runs: Vec::new(),
            checkpoints: Vec::new(),
            callback_tasks: Vec::new(),
            events: Vec::new(),
            stitched_trace: Vec::new(),
        };
        detail
            .stitched_trace
            .push(domain::ApplicationRunStitchedTrace {
                source_flow_run: domain::FlowRunRecord {
                    id: prior_run_id,
                    title: "prior run".to_string(),
                    ..flow_run(prior_run_id, now - time::Duration::seconds(10))
                },
                node_runs: vec![domain::NodeRunRecord {
                    id: Uuid::now_v7(),
                    flow_run_id: prior_run_id,
                    node_id: "prior-node".to_string(),
                    node_type: "llm".to_string(),
                    node_alias: "Prior LLM".to_string(),
                    status: domain::NodeRunStatus::Succeeded,
                    input_payload: json!({}),
                    output_payload: json!({}),
                    error_payload: None,
                    metrics_payload: json!({}),
                    debug_payload: json!({}),
                    started_at: now,
                    finished_at: Some(now),
                }],
                callback_tasks: Vec::new(),
                events: Vec::new(),
                runtime_events: Vec::new(),
            });

        let projection = build_application_run_trace_projection(&detail).unwrap();
        let stitched_group = projection
            .nodes
            .iter()
            .find(|node| node.node_kind == "stitched_context")
            .expect("stitched context group should be projected");
        let prior_run = projection
            .nodes
            .iter()
            .find(|node| node.node_kind == "stitched_run")
            .expect("prior run summary should be child of stitched group");

        assert_eq!(stitched_group.parent_trace_node_id, None);
        assert_eq!(stitched_group.child_count, 1);
        assert_eq!(
            prior_run.parent_trace_node_id,
            Some(stitched_group.trace_node_id)
        );
        assert!(projection
            .nodes
            .iter()
            .all(|node| node.stable_locator != format!("run:{flow_run_id}/node:prior-node")));
    }

    #[test]
    fn projection_status_needs_rebuild_for_missing_stale_or_changed_source() {
        let flow_run_id = Uuid::now_v7();
        let now = OffsetDateTime::UNIX_EPOCH;
        let fresh = status_record(
            flow_run_id,
            APPLICATION_RUN_TRACE_PROJECTION_VERSION,
            domain::ApplicationRunTraceProjectionStatus::Succeeded,
            "source:1",
            now,
        );
        let stale_version = status_record(
            flow_run_id,
            APPLICATION_RUN_TRACE_PROJECTION_VERSION - 1,
            domain::ApplicationRunTraceProjectionStatus::Succeeded,
            "source:1",
            now,
        );
        let failed = status_record(
            flow_run_id,
            APPLICATION_RUN_TRACE_PROJECTION_VERSION,
            domain::ApplicationRunTraceProjectionStatus::Failed,
            "source:1",
            now,
        );

        assert!(projection_status_needs_lazy_rebuild(None, "source:1"));
        assert!(!projection_status_needs_lazy_rebuild(
            Some(&fresh),
            "source:1"
        ));
        assert!(projection_status_needs_lazy_rebuild(
            Some(&fresh),
            "source:2"
        ));
        assert!(projection_status_needs_lazy_rebuild(
            Some(&stale_version),
            "source:1"
        ));
        assert!(!projection_status_needs_lazy_rebuild(
            Some(&failed),
            "source:1"
        ));
    }

    fn flow_run(flow_run_id: Uuid, now: OffsetDateTime) -> domain::FlowRunRecord {
        domain::FlowRunRecord {
            id: flow_run_id,
            application_id: Uuid::now_v7(),
            flow_id: Uuid::now_v7(),
            draft_id: Uuid::now_v7(),
            compiled_plan_id: None,
            debug_session_id: "debug-session".to_string(),
            flow_schema_version: "1flowbase.flow/v2".to_string(),
            document_hash: "hash".to_string(),
            run_mode: domain::FlowRunMode::DebugFlowRun,
            target_node_id: None,
            title: "debug flow".to_string(),
            status: domain::FlowRunStatus::Succeeded,
            input_payload: json!({}),
            output_payload: json!({}),
            error_payload: None,
            created_by: Uuid::now_v7(),
            authorized_account: Some("owner@example.com".to_string()),
            api_key_id: None,
            publication_version_id: None,
            external_user: None,
            external_conversation_id: None,
            external_trace_id: None,
            compatibility_mode: None,
            idempotency_key: None,
            started_at: now,
            finished_at: Some(now + time::Duration::seconds(3)),
            created_at: now,
            updated_at: now + time::Duration::seconds(3),
        }
    }

    fn status_record(
        flow_run_id: Uuid,
        projection_version: i32,
        status: domain::ApplicationRunTraceProjectionStatus,
        source_watermark: &str,
        now: OffsetDateTime,
    ) -> domain::ApplicationRunTraceProjectionStatusRecord {
        domain::ApplicationRunTraceProjectionStatusRecord {
            flow_run_id,
            projection_version,
            status,
            source_watermark: source_watermark.to_string(),
            attempt_count: 1,
            last_attempt_at: Some(now),
            last_success_at: Some(now),
            last_error_code: None,
            last_error_stage: None,
            last_error_source_kind: None,
            last_error_source_locator: None,
            last_error_message: None,
            last_error_ref: None,
            retriable: false,
            created_at: now,
            updated_at: now,
        }
    }
}
