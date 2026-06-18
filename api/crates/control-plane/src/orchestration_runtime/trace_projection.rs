use anyhow::Result;
use sha2::{Digest, Sha256};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::ports::{
    ApplicationRunTraceNodeContentProjectionInput, ApplicationRunTraceNodeProjectionInput,
    ReplaceApplicationRunTraceProjectionInput,
};

pub const APPLICATION_RUN_TRACE_PROJECTION_VERSION: i32 = 1;

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
    let current_nodes = trace_visible_current_node_runs(detail);

    for (index, node_run) in current_nodes.iter().enumerate() {
        builder.push_node_run_root(index, node_run, detail)?;
    }

    if !detail.stitched_trace.is_empty() {
        builder.push_stitched_context_group(current_nodes.len(), &detail.stitched_trace)?;
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

fn trace_projection_source_watermark(detail: &domain::ApplicationRunDetail) -> String {
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
        node_run: &domain::NodeRunRecord,
        detail: &domain::ApplicationRunDetail,
    ) -> Result<()> {
        let order_key = root_order_key(index);
        let stable_locator = format!("run:{}/node:{}", self.flow_run_id, node_run.id);
        let trace_node_id = trace_node_id_for_locator(self.flow_run_id, &stable_locator);
        let callback_tasks = callback_tasks_for_node_run(detail, node_run.id);
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
            owner_kind: Some("node_run".to_string()),
            owner_id: Some(node_run.id.to_string()),
            order_key: order_key.clone(),
            node_id: Some(node_run.node_id.clone()),
            node_type: Some(node_run.node_type.clone()),
            node_alias: node_run.node_alias.clone(),
            status: node_run.status.as_str().to_string(),
            started_at: node_run.started_at,
            finished_at: node_run.finished_at,
            duration_ms: trace_node_duration_ms(node_run.started_at, node_run.finished_at),
            metrics_payload: node_run.metrics_payload.clone(),
            has_children: child_count > 0,
            child_count,
            has_content: true,
            content_ref: None,
        });
        self.contents
            .push(node_run_content(trace_node_id, node_run, detail)?);

        self.push_callback_children(&order_key, trace_node_id, &stable_locator, &callback_tasks)?;
        Ok(())
    }

    fn push_callback_children(
        &mut self,
        parent_order_key: &str,
        parent_trace_node_id: Uuid,
        parent_stable_locator: &str,
        callback_tasks: &[domain::CallbackTaskRecord],
    ) -> Result<()> {
        let mut child_index = 0_usize;
        let tool_tasks: Vec<&domain::CallbackTaskRecord> = callback_tasks
            .iter()
            .filter(|task| task.callback_kind == "llm_tool_calls")
            .collect();

        if !tool_tasks.is_empty() {
            child_index += 1;
            self.push_tool_group(
                child_order_key(parent_order_key, child_index),
                parent_trace_node_id,
                parent_stable_locator,
                &tool_tasks,
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
        tool_tasks: &[&domain::CallbackTaskRecord],
    ) -> Result<()> {
        let stable_locator = format!("{parent_stable_locator}/tools");
        let trace_node_id = trace_node_id_for_locator(self.flow_run_id, &stable_locator);
        let tool_call_count = tool_tasks
            .iter()
            .flat_map(|task| tool_calls_from_callback_task(task))
            .count();

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
                    task,
                    &tool_call,
                )?;
            }
        }

        Ok(())
    }

    fn push_tool_callback_node(
        &mut self,
        order_key: String,
        parent_trace_node_id: Uuid,
        parent_stable_locator: &str,
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
        let payload = serde_json::json!({
            "callback_task_id": task.id,
            "tool_call_id": tool_call_id,
            "tool_call": tool_call,
            "tool_result": tool_result_for_call(task, &tool_call_id),
            "callback_status": task.status.as_str()
        });

        self.nodes.push(ApplicationRunTraceNodeProjectionInput {
            trace_node_id,
            parent_trace_node_id: Some(parent_trace_node_id),
            stable_locator,
            node_kind: "tool_callback".to_string(),
            owner_kind: Some("tool_call".to_string()),
            owner_id: Some(tool_call_id),
            order_key,
            node_id: None,
            node_type: Some("tool".to_string()),
            node_alias: tool_name,
            status: task.status.as_str().to_string(),
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
                content_kind: "tool_callback".to_string(),
                payload,
                source_refs: serde_json::json!([{
                    "source_kind": "callback_task",
                    "source_locator": task.id
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
            status: task.status.as_str().to_string(),
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

fn callback_tasks_for_node_run(
    detail: &domain::ApplicationRunDetail,
    node_run_id: Uuid,
) -> Vec<domain::CallbackTaskRecord> {
    detail
        .callback_tasks
        .iter()
        .filter(|task| task.node_run_id == node_run_id)
        .cloned()
        .collect()
}

fn tool_calls_from_callback_task(task: &domain::CallbackTaskRecord) -> Vec<serde_json::Value> {
    task.request_payload
        .get("tool_calls")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default()
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

fn tool_group_status(tool_tasks: &[&domain::CallbackTaskRecord]) -> String {
    if tool_tasks
        .iter()
        .any(|task| task.status == domain::CallbackTaskStatus::Pending)
    {
        return domain::CallbackTaskStatus::Pending.as_str().to_string();
    }

    if tool_tasks
        .iter()
        .any(|task| task.status == domain::CallbackTaskStatus::Cancelled)
    {
        return domain::CallbackTaskStatus::Cancelled.as_str().to_string();
    }

    domain::CallbackTaskStatus::Completed.as_str().to_string()
}

fn node_run_content(
    trace_node_id: Uuid,
    node_run: &domain::NodeRunRecord,
    detail: &domain::ApplicationRunDetail,
) -> Result<ApplicationRunTraceNodeContentProjectionInput> {
    let checkpoints: Vec<&domain::CheckpointRecord> = detail
        .checkpoints
        .iter()
        .filter(|checkpoint| checkpoint.node_run_id == Some(node_run.id))
        .collect();
    let events: Vec<&domain::RunEventRecord> = detail
        .events
        .iter()
        .filter(|event| event.node_run_id == Some(node_run.id))
        .collect();

    Ok(ApplicationRunTraceNodeContentProjectionInput {
        trace_node_id,
        content_kind: "node_run".to_string(),
        payload: serde_json::json!({
            "node_run": node_run,
            "checkpoints": checkpoints,
            "events": events
        }),
        source_refs: serde_json::json!([{
            "source_kind": "node_run",
            "source_locator": node_run.id
        }]),
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
                debug_payload: json!({}),
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
        assert!(projection.contents.iter().any(|content| {
            content.content_kind == "tool_callback"
                && content.payload["tool_call_id"] == json!("call-weather")
                && content.payload["tool_result"]["content"] == json!("22c")
        }));
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
