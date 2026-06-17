use std::collections::BTreeMap;

use serde_json::{json, Map, Value};
use uuid::Uuid;

const VISIBLE_INTERNAL_LLM_TOOL_TRACE_KIND: &str = "visible_internal_llm_tool_trace";
const TEXT_PREVIEW_CHARS: usize = 180;

#[derive(Debug, Clone)]
pub(super) struct VisibleInternalLlmToolRouteTrace {
    detail_payload: Value,
    summary_payload: Value,
}

impl VisibleInternalLlmToolRouteTrace {
    pub(super) fn detail_payload(&self) -> Value {
        self.detail_payload.clone()
    }

    pub(super) fn inline_summary_payload(&self) -> Value {
        self.summary_payload.clone()
    }

    pub(super) fn inline_summary_with_branch_traces_payload(&self) -> Value {
        let mut summary = self.summary_payload.clone();
        if let (Some(summary_object), Some(branch_traces)) = (
            summary.as_object_mut(),
            self.detail_payload.get("branch_traces").cloned(),
        ) {
            summary_object.insert("branch_traces".to_string(), branch_traces);
        }
        summary
    }

    pub(super) fn summary_payload(&self, artifact_id: Uuid) -> Value {
        let mut summary = self.summary_payload.clone();
        let original_size_bytes = serde_json::to_vec(&self.detail_payload)
            .map(|bytes| bytes.len() as i64)
            .unwrap_or_default();
        let preview_size_bytes = serde_json::to_vec(&summary)
            .map(|bytes| bytes.len() as i64)
            .unwrap_or_default();

        if let Some(object) = summary.as_object_mut() {
            object.insert("__runtime_debug_artifact".to_string(), json!(true));
            object.insert("artifact_ref".to_string(), json!(artifact_id.to_string()));
            object.insert("content_type".to_string(), json!("application/json"));
            object.insert("is_truncated".to_string(), json!(true));
            object.insert(
                "original_size_bytes".to_string(),
                json!(original_size_bytes),
            );
            object.insert("preview_size_bytes".to_string(), json!(preview_size_bytes));
        }

        summary
    }
}

#[derive(Debug, Clone)]
pub(super) struct VisibleInternalLlmToolBranchNodeRunPayload {
    pub(super) node_run_id: String,
    pub(super) node_id: String,
    pub(super) node_alias: String,
    pub(super) node_type: String,
    pub(super) input_payload: Value,
    pub(super) output_payload: Value,
    pub(super) metrics_payload: Value,
    pub(super) debug_payload: Value,
}

struct VisibleInternalLlmToolBranchNodeRunIndex<'a> {
    by_node_run_id: BTreeMap<String, &'a VisibleInternalLlmToolBranchNodeRunPayload>,
    latest_by_node_id: BTreeMap<String, &'a VisibleInternalLlmToolBranchNodeRunPayload>,
}

impl<'a> VisibleInternalLlmToolBranchNodeRunIndex<'a> {
    fn new(branch_node_runs: &'a [VisibleInternalLlmToolBranchNodeRunPayload]) -> Self {
        let mut by_node_run_id = BTreeMap::new();
        let mut latest_by_node_id = BTreeMap::new();

        for node_run in branch_node_runs {
            by_node_run_id.insert(node_run.node_run_id.clone(), node_run);
            latest_by_node_id.insert(node_run.node_id.clone(), node_run);
        }

        Self {
            by_node_run_id,
            latest_by_node_id,
        }
    }

    fn find(
        &self,
        event_object: &Map<String, Value>,
        branch_node_id: Option<&str>,
    ) -> Option<&'a VisibleInternalLlmToolBranchNodeRunPayload> {
        if let Some(node_run_id) = string_field(
            event_object,
            &[
                "branch_node_run_id",
                "route_node_run_id",
                "target_node_run_id",
            ],
        ) {
            if let Some(node_run) = self.by_node_run_id.get(&node_run_id) {
                return Some(node_run);
            }
        }

        branch_node_id.and_then(|node_id| self.latest_by_node_id.get(node_id).copied())
    }
}

#[derive(Default)]
struct VisibleInternalLlmToolTraceFacts {
    tool_call_id: String,
    tool_name: Option<String>,
    route_kind: Option<String>,
    execution_mode: Option<String>,
    main_node_id: Option<String>,
    target_node_id: Option<String>,
    route_node_id: Option<String>,
    route_node_alias: Option<String>,
    route_model: Option<String>,
    provider_route: Option<Value>,
    arguments: Option<Value>,
    tool_call: Option<Value>,
    tool_result: Option<Value>,
    tool_call_round_index: Option<i64>,
    tool_result_round_index: Option<i64>,
    main_resume_round_index: Option<i64>,
    main_resume_output: Option<Value>,
    events: Vec<Value>,
    waiting_events: Vec<Value>,
    failed_event: Option<Value>,
    completed_event: Option<Value>,
    branch_traces: Vec<VisibleInternalLlmToolBranchTraceFacts>,
}

struct VisibleInternalLlmToolBranchTraceFacts {
    event_type: String,
    node_id: Option<String>,
    node_alias: Option<String>,
    node_type: Option<String>,
    status: String,
    route_model: Option<String>,
    provider_route: Option<Value>,
    input_payload: Option<Value>,
    output_payload: Value,
    output: Option<Value>,
    output_summary: Value,
    metrics_payload: Option<Value>,
    debug_payload: Option<Value>,
    debug_payload_ref: Option<Value>,
}

pub(super) fn collect_visible_internal_llm_tool_route_traces(
    debug_payload: &Value,
) -> Vec<VisibleInternalLlmToolRouteTrace> {
    collect_visible_internal_llm_tool_route_traces_with_main_output(debug_payload, None)
}

pub(super) fn collect_visible_internal_llm_tool_route_traces_with_main_output(
    debug_payload: &Value,
    main_resume_output_fallback: Option<&Value>,
) -> Vec<VisibleInternalLlmToolRouteTrace> {
    collect_visible_internal_llm_tool_route_traces_with_branch_node_runs(
        debug_payload,
        main_resume_output_fallback,
        &[],
    )
}

pub(super) fn collect_visible_internal_llm_tool_route_traces_with_branch_node_runs(
    debug_payload: &Value,
    main_resume_output_fallback: Option<&Value>,
    branch_node_runs: &[VisibleInternalLlmToolBranchNodeRunPayload],
) -> Vec<VisibleInternalLlmToolRouteTrace> {
    let Some(events) = debug_payload
        .get("visible_internal_llm_tool_events")
        .and_then(Value::as_array)
    else {
        return Vec::new();
    };
    if events.is_empty() {
        return Vec::new();
    }

    let branch_node_run_index = VisibleInternalLlmToolBranchNodeRunIndex::new(branch_node_runs);
    let mut facts_by_tool_call_id = collect_trace_event_facts(events, &branch_node_run_index);
    collect_trace_round_facts(
        debug_payload
            .get("llm_rounds")
            .and_then(Value::as_array)
            .map(Vec::as_slice)
            .unwrap_or_default(),
        &mut facts_by_tool_call_id,
    );
    let main_resume_output_fallback = if facts_by_tool_call_id.len() == 1 {
        main_resume_output_fallback.and_then(main_resume_output_from_node_output)
    } else {
        None
    };

    facts_by_tool_call_id
        .into_values()
        .filter_map(|facts| route_trace_from_facts(facts, main_resume_output_fallback.as_ref()))
        .collect()
}

fn collect_trace_event_facts(
    events: &[Value],
    branch_node_run_index: &VisibleInternalLlmToolBranchNodeRunIndex<'_>,
) -> BTreeMap<String, VisibleInternalLlmToolTraceFacts> {
    let mut facts_by_tool_call_id = BTreeMap::new();

    for event in events {
        let Some(event_object) = event.as_object() else {
            continue;
        };
        let Some(tool_call_id) = string_field(event_object, &["tool_call_id", "id", "call_id"])
        else {
            continue;
        };
        let entry = facts_by_tool_call_id
            .entry(tool_call_id.clone())
            .or_insert_with(|| VisibleInternalLlmToolTraceFacts {
                tool_call_id: tool_call_id.clone(),
                ..Default::default()
            });

        entry.events.push(event.clone());
        set_if_some(
            &mut entry.tool_name,
            string_field(event_object, &["tool_name"]),
        );
        set_if_some(&mut entry.route_kind, route_kind_from_event(event_object));
        set_if_some(
            &mut entry.execution_mode,
            string_field(event_object, &["execution_mode"]),
        );
        set_if_some(
            &mut entry.main_node_id,
            string_field(event_object, &["main_node_id"]),
        );
        set_if_some(
            &mut entry.target_node_id,
            string_field(event_object, &["target_node_id"]),
        );
        set_if_some(
            &mut entry.route_node_id,
            string_field(
                event_object,
                &["route_node_id", "node_id", "waiting_node_id"],
            ),
        );
        set_if_some(
            &mut entry.route_node_alias,
            string_field(
                event_object,
                &["route_node_alias", "node_alias", "waiting_node_alias"],
            ),
        );
        if entry.arguments.is_none() {
            entry.arguments = event_object.get("arguments").cloned();
        }
        if let Some(branch_trace) = branch_trace_from_event(event_object, branch_node_run_index) {
            entry.branch_traces.push(branch_trace);
        }

        match event_object
            .get("event_type")
            .and_then(Value::as_str)
            .unwrap_or_default()
        {
            "visible_internal_llm_tool_waiting_callback" => {
                entry.waiting_events.push(event.clone());
            }
            "visible_internal_llm_tool_completed" => {
                entry.completed_event = Some(event.clone());
                if entry.tool_result.is_none() {
                    if let Some(content) = value_field(
                        event_object,
                        &[
                            "content",
                            "output",
                            "output_payload",
                            "result",
                            "response_payload",
                        ],
                    ) {
                        entry.tool_result = Some(json!({
                            "role": "tool",
                            "tool_call_id": tool_call_id,
                            "name": entry.tool_name.clone().unwrap_or_else(|| "Tool".to_string()),
                            "is_error": false,
                            "content": content,
                        }));
                    }
                }
                if let Some(provider_route) = event_object.get("provider_route") {
                    entry.provider_route = Some(provider_route.clone());
                    set_if_some(
                        &mut entry.route_model,
                        provider_route
                            .as_object()
                            .and_then(|route| string_field(route, &["model"])),
                    );
                }
            }
            "visible_internal_llm_tool_failed" => {
                entry.failed_event = Some(event.clone());
            }
            _ => {}
        }
    }

    facts_by_tool_call_id
}

fn route_kind_from_event(event_object: &Map<String, Value>) -> Option<String> {
    let route_kind = string_field(event_object, &["route_kind", "tool_mode"])
        .or_else(|| string_field(event_object, &["execution_mode"]));
    match route_kind.as_deref() {
        Some("fusion") | Some("bounded_parallel_panel") => Some("fusion".to_string()),
        Some("agent") | Some("sequential_resume") | Some("route") => Some("route".to_string()),
        _ => None,
    }
}

fn branch_trace_from_event(
    event_object: &Map<String, Value>,
    branch_node_run_index: &VisibleInternalLlmToolBranchNodeRunIndex<'_>,
) -> Option<VisibleInternalLlmToolBranchTraceFacts> {
    let event_type = event_object.get("event_type").and_then(Value::as_str)?;
    let (status, node_id, node_alias) = match event_type {
        "visible_internal_llm_tool_completed" => (
            "succeeded",
            string_field(event_object, &["route_node_id", "node_id"]),
            string_field(event_object, &["route_node_alias", "node_alias"]),
        ),
        "visible_internal_llm_tool_waiting_callback" => (
            "waiting_callback",
            string_field(
                event_object,
                &["route_node_id", "waiting_node_id", "node_id"],
            ),
            string_field(
                event_object,
                &["route_node_alias", "waiting_node_alias", "node_alias"],
            ),
        ),
        "visible_internal_llm_tool_failed" => (
            "failed",
            string_field(
                event_object,
                &["route_node_id", "node_id", "waiting_node_id"],
            ),
            string_field(
                event_object,
                &["route_node_alias", "node_alias", "waiting_node_alias"],
            ),
        ),
        _ => return None,
    };
    if node_id.is_none() && event_type != "visible_internal_llm_tool_failed" {
        return None;
    }
    let provider_route = event_object.get("provider_route").cloned();
    let route_model = provider_route
        .as_ref()
        .and_then(Value::as_object)
        .and_then(|route| string_field(route, &["model"]));
    let output = value_field(
        event_object,
        &[
            "content",
            "output",
            "output_payload",
            "result",
            "response_payload",
        ],
    );
    let output_summary = output
        .as_ref()
        .map(summarize_runtime_value)
        .unwrap_or_else(|| summarize_runtime_value(&Value::Null));
    let output_payload =
        branch_output_payload(event_object, output.as_ref(), provider_route.as_ref());

    let mut branch_trace = VisibleInternalLlmToolBranchTraceFacts {
        event_type: event_type.to_string(),
        node_id: node_id.clone(),
        node_alias,
        node_type: string_field(event_object, &["node_type"]),
        status: status.to_string(),
        route_model,
        provider_route,
        input_payload: branch_input_payload(event_object),
        output_payload,
        output,
        output_summary,
        metrics_payload: event_object.get("metrics_payload").cloned(),
        debug_payload: event_object.get("debug_payload").cloned(),
        debug_payload_ref: event_object.get("debug_payload_ref").cloned(),
    };
    if let Some(node_run) = branch_node_run_index.find(event_object, node_id.as_deref()) {
        apply_branch_node_run_payload(&mut branch_trace, node_run);
    }

    Some(branch_trace)
}

fn branch_input_payload(event_object: &Map<String, Value>) -> Option<Value> {
    value_field(event_object, &["input_payload"])
        .or_else(|| historical_llm_input_payload_from_debug_context(event_object))
}

fn historical_llm_input_payload_from_debug_context(
    event_object: &Map<String, Value>,
) -> Option<Value> {
    if string_field(event_object, &["node_type"]).as_deref() != Some("llm") {
        return None;
    }

    let llm_context = event_object
        .get("debug_payload")?
        .get("llm_context")?
        .as_object()?;
    let mut prompt_messages = Vec::new();

    prompt_messages.extend(
        llm_context
            .get("provider_messages")?
            .as_array()?
            .iter()
            .filter_map(|message| {
                message
                    .as_object()
                    .map(|object| Value::Object(object.clone()))
            }),
    );

    if let Some(effective_system) = llm_context
        .get("effective_system")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .filter(|_| {
            !prompt_messages
                .iter()
                .filter_map(Value::as_object)
                .any(|message| string_field(message, &["role"]).as_deref() == Some("system"))
        })
    {
        prompt_messages.insert(
            0,
            json!({
                "role": "system",
                "content": effective_system,
            }),
        );
    }

    if prompt_messages.is_empty() {
        return None;
    }

    Some(json!({
        "prompt_messages": prompt_messages,
    }))
}

fn apply_branch_node_run_payload(
    branch_trace: &mut VisibleInternalLlmToolBranchTraceFacts,
    node_run: &VisibleInternalLlmToolBranchNodeRunPayload,
) {
    if branch_trace.node_alias.is_none() {
        branch_trace.node_alias = Some(node_run.node_alias.clone());
    }
    if branch_trace.node_type.is_none() {
        branch_trace.node_type = Some(node_run.node_type.clone());
    }
    if runtime_payload_has_detail(&node_run.input_payload) {
        branch_trace.input_payload = Some(node_run.input_payload.clone());
    }
    if runtime_payload_has_detail(&node_run.output_payload) {
        branch_trace.output_payload = node_run.output_payload.clone();
        branch_trace.output = branch_output_text(&node_run.output_payload);
        branch_trace.output_summary = branch_trace
            .output
            .as_ref()
            .map(summarize_runtime_value)
            .unwrap_or_else(|| summarize_runtime_value(&node_run.output_payload));
    }
    if runtime_payload_has_detail(&node_run.metrics_payload) {
        branch_trace.metrics_payload = Some(node_run.metrics_payload.clone());
    }
    if runtime_payload_has_detail(&node_run.debug_payload) {
        branch_trace.debug_payload = Some(node_run.debug_payload.clone());
    }
}

fn runtime_payload_has_detail(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Object(object) => !object.is_empty(),
        _ => true,
    }
}

fn branch_output_text(output_payload: &Value) -> Option<Value> {
    output_payload
        .as_object()
        .and_then(|object| string_field(object, &["text", "answer", "content"]))
        .map(Value::String)
}

fn collect_trace_round_facts(
    rounds: &[Value],
    facts_by_tool_call_id: &mut BTreeMap<String, VisibleInternalLlmToolTraceFacts>,
) {
    for (fallback_round_index, round) in rounds.iter().enumerate() {
        let Some(round_object) = round.as_object() else {
            continue;
        };
        let current_round_index = round_index(round_object, fallback_round_index);

        for (tool_call_index, tool_call) in
            read_round_tool_calls(round_object).into_iter().enumerate()
        {
            let Some(tool_call_object) = tool_call.as_object() else {
                continue;
            };
            let tool_call_id = tool_call_id(tool_call_object, current_round_index, tool_call_index);
            let Some(entry) = facts_by_tool_call_id.get_mut(&tool_call_id) else {
                continue;
            };
            entry.tool_call = Some(tool_call.clone());
            entry.tool_call_round_index = Some(current_round_index);
            set_if_some(
                &mut entry.tool_name,
                string_field(tool_call_object, &["name", "tool_name"]),
            );
            if entry.arguments.is_none() {
                entry.arguments = tool_call_object.get("arguments").cloned();
            }
        }

        for (tool_result_index, tool_result) in read_round_tool_results(round_object)
            .into_iter()
            .enumerate()
        {
            let Some(tool_result_object) = tool_result.as_object() else {
                continue;
            };
            let tool_call_id =
                tool_result_id(tool_result_object, current_round_index, tool_result_index);
            let Some(entry) = facts_by_tool_call_id.get_mut(&tool_call_id) else {
                continue;
            };
            entry.tool_result = Some(tool_result.clone());
            entry.tool_result_round_index = Some(current_round_index);
            set_if_some(
                &mut entry.tool_name,
                string_field(tool_result_object, &["name", "tool_name"]),
            );
        }
    }

    for facts in facts_by_tool_call_id.values_mut() {
        let Some(tool_result_round_index) = facts.tool_result_round_index else {
            continue;
        };
        for (fallback_round_index, round) in rounds.iter().enumerate() {
            let Some(round_object) = round.as_object() else {
                continue;
            };
            let current_round_index = round_index(round_object, fallback_round_index);
            if current_round_index <= tool_result_round_index {
                continue;
            }
            let Some(assistant) = round_object
                .get("assistant")
                .and_then(Value::as_object)
                .map(|assistant| Value::Object(assistant.clone()))
            else {
                continue;
            };
            facts.main_resume_round_index = Some(current_round_index);
            facts.main_resume_output = Some(assistant);
            break;
        }
    }
}

fn route_trace_from_facts(
    facts: VisibleInternalLlmToolTraceFacts,
    main_resume_output_fallback: Option<&Value>,
) -> Option<VisibleInternalLlmToolRouteTrace> {
    let route_kind = route_trace_kind(&facts);
    let branch_summaries = branch_summary_payloads(&facts.branch_traces, &route_kind);
    let branch_traces = branch_detail_payloads(&facts.branch_traces, &route_kind);
    let branch_count = branch_summaries.len() as i64;
    let route_output = facts
        .tool_result
        .as_ref()
        .and_then(|value| value.get("content"))
        .cloned();
    let route_output_summary = route_output
        .as_ref()
        .map(summarize_runtime_value)
        .unwrap_or_else(|| summarize_runtime_value(&Value::Null));
    let fallback_main_resume_output = if facts.main_resume_output.is_none()
        && facts.failed_event.is_none()
        && facts.completed_event.is_some()
    {
        main_resume_output_fallback.cloned()
    } else {
        None
    };
    let main_resume_output = facts
        .main_resume_output
        .clone()
        .or(fallback_main_resume_output);
    let final_output = main_resume_output
        .as_ref()
        .and_then(|value| value.get("content"))
        .cloned();
    let final_output_summary = final_output
        .as_ref()
        .map(summarize_runtime_value)
        .unwrap_or_else(|| summarize_runtime_value(&Value::Null));
    let returned_to_main = facts.tool_result.is_some()
        || (facts.completed_event.is_some() && facts.failed_event.is_none());
    let main_resume = main_resume_output.is_some();
    let status = route_trace_status(&facts, returned_to_main, main_resume);
    let tool_name = facts.tool_name.unwrap_or_else(|| "Tool".to_string());
    let fan_in = if route_kind == "fusion" {
        json!({
            "mode": facts.execution_mode,
            "branch_count": branch_count,
            "returned_to_main": returned_to_main,
            "main_resume": main_resume,
        })
    } else {
        Value::Null
    };

    let summary_payload = json!({
        "kind": VISIBLE_INTERNAL_LLM_TOOL_TRACE_KIND,
        "preview_kind": VISIBLE_INTERNAL_LLM_TOOL_TRACE_KIND,
        "route_kind": route_kind,
        "tool_call_id": facts.tool_call_id,
        "tool_name": tool_name,
        "status": status,
        "main_node_id": facts.main_node_id,
        "target_node_id": facts.target_node_id,
        "route_node_id": facts.route_node_id,
        "route_node_alias": facts.route_node_alias,
        "route_model": facts.route_model,
        "callback_count": facts.waiting_events.len() as i64,
        "returned_to_main": returned_to_main,
        "main_resume": main_resume,
        "tool_call_round_index": facts.tool_call_round_index,
        "tool_result_round_index": facts.tool_result_round_index,
        "main_resume_round_index": facts.main_resume_round_index,
        "route_output_summary": route_output_summary,
        "final_output_summary": final_output_summary,
        "branch_count": branch_count,
        "branch_summaries": branch_summaries,
        "fan_in": fan_in,
    });
    let detail_payload = json!({
        "kind": VISIBLE_INTERNAL_LLM_TOOL_TRACE_KIND,
        "route_kind": summary_payload["route_kind"],
        "tool_call_id": summary_payload["tool_call_id"],
        "tool_name": summary_payload["tool_name"],
        "status": summary_payload["status"],
        "main_node_id": summary_payload["main_node_id"],
        "target_node_id": summary_payload["target_node_id"],
        "route": {
            "node_id": summary_payload["route_node_id"],
            "node_alias": summary_payload["route_node_alias"],
            "model": summary_payload["route_model"],
            "provider_route": facts.provider_route,
        },
        "callback_count": summary_payload["callback_count"],
        "returned_to_main": returned_to_main,
        "main_resume": main_resume,
        "rounds": {
            "tool_call_round_index": facts.tool_call_round_index,
            "tool_result_round_index": facts.tool_result_round_index,
            "main_resume_round_index": facts.main_resume_round_index,
        },
        "tool_call": facts.tool_call,
        "arguments": facts.arguments,
        "tool_result": facts.tool_result,
        "route_output": route_output,
        "route_output_summary": summary_payload["route_output_summary"],
        "main_resume_output": main_resume_output,
        "final_output": final_output,
        "final_output_summary": summary_payload["final_output_summary"],
        "branch_traces": branch_traces,
        "fan_in": summary_payload["fan_in"],
        "callback_requests": facts.waiting_events.iter().map(callback_request_detail).collect::<Vec<_>>(),
        "events": facts.events,
    });

    Some(VisibleInternalLlmToolRouteTrace {
        detail_payload,
        summary_payload,
    })
}

fn route_trace_kind(facts: &VisibleInternalLlmToolTraceFacts) -> String {
    if facts.route_kind.as_deref() == Some("fusion") || facts.branch_traces.len() > 1 {
        "fusion".to_string()
    } else {
        "route".to_string()
    }
}

fn branch_summary_payloads(
    branch_traces: &[VisibleInternalLlmToolBranchTraceFacts],
    route_kind: &str,
) -> Vec<Value> {
    if route_kind != "fusion" {
        return Vec::new();
    }

    branch_traces
        .iter()
        .map(|branch| {
            json!({
                "event_type": branch.event_type,
                "node_id": branch.node_id,
                "node_alias": branch.node_alias,
                "node_type": branch.node_type,
                "status": branch.status,
                "route_model": branch.route_model,
                "output_summary": branch.output_summary,
            })
        })
        .collect()
}

fn branch_detail_payloads(
    branch_traces: &[VisibleInternalLlmToolBranchTraceFacts],
    route_kind: &str,
) -> Vec<Value> {
    if route_kind != "fusion" {
        return Vec::new();
    }

    branch_traces
        .iter()
        .map(|branch| {
            json!({
                "event_type": branch.event_type,
                "node_id": branch.node_id,
                "node_alias": branch.node_alias,
                "node_type": branch.node_type,
                "status": branch.status,
                "route_model": branch.route_model,
                "input_payload": branch.input_payload,
                "provider_route": branch.provider_route,
                "output_payload": branch.output_payload,
                "output": branch.output,
                "output_summary": branch.output_summary,
                "metrics_payload": branch.metrics_payload,
                "debug_payload": branch.debug_payload,
                "debug_payload_ref": branch.debug_payload_ref,
            })
        })
        .collect()
}

fn branch_output_payload(
    event_object: &Map<String, Value>,
    output: Option<&Value>,
    provider_route: Option<&Value>,
) -> Value {
    if let Some(output_payload) = value_field(event_object, &["output_payload"]) {
        return output_payload;
    }

    let mut payload = Map::new();
    payload.insert("text".to_string(), output.cloned().unwrap_or(Value::Null));
    payload.insert(
        "provider_route".to_string(),
        provider_route.cloned().unwrap_or(Value::Null),
    );
    Value::Object(payload)
}

fn main_resume_output_from_node_output(output_payload: &Value) -> Option<Value> {
    if let Some(text) = output_payload
        .as_object()
        .and_then(|object| string_field(object, &["text", "answer", "content"]))
    {
        return Some(json!({
            "role": "assistant",
            "content": text,
            "source": "node_output_payload",
        }));
    }

    if let Some(text) = output_payload
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return Some(json!({
            "role": "assistant",
            "content": text,
            "source": "node_output_payload",
        }));
    }

    output_payload
        .as_object()
        .filter(|object| !object.is_empty())
        .map(|_| {
            json!({
                "role": "assistant",
                "content": output_payload,
                "source": "node_output_payload",
            })
        })
}

fn route_trace_status(
    facts: &VisibleInternalLlmToolTraceFacts,
    returned_to_main: bool,
    main_resume: bool,
) -> &'static str {
    if facts.failed_event.is_some() {
        "failed"
    } else if returned_to_main && main_resume {
        "succeeded"
    } else if returned_to_main {
        "returned_to_main"
    } else if !facts.waiting_events.is_empty() {
        "waiting_callback"
    } else if facts.completed_event.is_some() {
        "route_completed"
    } else {
        "started"
    }
}

fn callback_request_detail(event: &Value) -> Value {
    let Some(event_object) = event.as_object() else {
        return event.clone();
    };

    json!({
        "event_type": event_object.get("event_type").cloned().unwrap_or(Value::Null),
        "waiting_node_id": event_object.get("waiting_node_id").cloned().unwrap_or(Value::Null),
        "waiting_node_alias": event_object.get("waiting_node_alias").cloned().unwrap_or(Value::Null),
        "request_payload": event_object.get("request_payload").cloned().unwrap_or(Value::Null),
    })
}

fn summarize_runtime_value(value: &Value) -> Value {
    if let Some(text) = value.as_str() {
        return summarize_text(text);
    }
    if let Some(blocks) = value.as_array() {
        return summarize_content_blocks(blocks);
    }

    json!({
        "kind": "json",
        "preview": preview_text(&value.to_string(), TEXT_PREVIEW_CHARS),
        "serialized_size_bytes": value.to_string().len() as i64,
    })
}

fn summarize_content_blocks(blocks: &[Value]) -> Value {
    let text = blocks
        .iter()
        .filter_map(|block| block.get("text").and_then(Value::as_str))
        .collect::<Vec<_>>()
        .join("\n");
    let mut media_types = Vec::<String>::new();
    let mut media_block_count = 0_i64;

    for block in blocks {
        let media_type = block.get("media_type").and_then(Value::as_str).or_else(|| {
            block
                .get("source")
                .and_then(|source| source.get("media_type"))
                .and_then(Value::as_str)
        });
        if let Some(media_type) = media_type {
            media_block_count += 1;
            if !media_types.iter().any(|existing| existing == media_type) {
                media_types.push(media_type.to_string());
            }
        }
    }

    json!({
        "kind": "content_blocks",
        "block_count": blocks.len() as i64,
        "media_block_count": media_block_count,
        "media_types": media_types,
        "text": summarize_text(&text),
    })
}

fn summarize_text(text: &str) -> Value {
    let char_count = text.chars().count();

    json!({
        "kind": "text",
        "preview": preview_text(text, TEXT_PREVIEW_CHARS),
        "char_count": char_count as i64,
        "truncated": char_count > TEXT_PREVIEW_CHARS,
    })
}

fn preview_text(text: &str, max_chars: usize) -> String {
    text.chars().take(max_chars).collect()
}

fn read_round_tool_calls(round: &Map<String, Value>) -> Vec<Value> {
    let assistant_tool_calls = round
        .get("assistant")
        .or_else(|| round.get("assistant_message"))
        .and_then(Value::as_object)
        .and_then(|assistant| assistant.get("tool_calls"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    if !assistant_tool_calls.is_empty() {
        return assistant_tool_calls;
    }

    round
        .get("tool_calls")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

fn read_round_tool_results(round: &Map<String, Value>) -> Vec<Value> {
    round
        .get("tool_results")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

fn round_index(round: &Map<String, Value>, fallback_index: usize) -> i64 {
    round
        .get("round_index")
        .and_then(Value::as_i64)
        .unwrap_or(fallback_index as i64)
}

fn tool_call_id(tool_call: &Map<String, Value>, round_number: i64, index: usize) -> String {
    string_field(tool_call, &["id", "tool_call_id", "call_id"])
        .unwrap_or_else(|| format!("tool-{}-{}", round_number + 1, index + 1))
}

fn tool_result_id(tool_result: &Map<String, Value>, round_number: i64, index: usize) -> String {
    string_field(tool_result, &["tool_call_id", "id", "call_id"])
        .unwrap_or_else(|| format!("tool-result-{}-{}", round_number + 1, index + 1))
}

fn string_field(record: &Map<String, Value>, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        record
            .get(*key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
    })
}

fn value_field(record: &Map<String, Value>, keys: &[&str]) -> Option<Value> {
    keys.iter().find_map(|key| record.get(*key).cloned())
}

fn set_if_some(target: &mut Option<String>, value: Option<String>) {
    if target.is_none() {
        *target = value;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn route_trace_summary_proves_return_to_main_without_large_payloads() {
        let route_output = "route image description ".repeat(48);
        let debug_payload = json!({
            "llm_rounds": [
                {
                    "round_index": 0,
                    "assistant": {
                        "role": "assistant",
                        "content": "need image",
                        "tool_calls": [
                            {
                                "id": "call_image",
                                "name": "image_llm",
                                "arguments": {
                                    "file": "uploads/agent-flow-node-detail-icon-aligned.png"
                                }
                            }
                        ]
                    }
                },
                {
                    "round_index": 1,
                    "tool_results": [
                        {
                            "role": "tool",
                            "tool_call_id": "call_image",
                            "name": "image_llm",
                            "content": route_output
                        }
                    ]
                },
                {
                    "round_index": 2,
                    "assistant": {
                        "role": "assistant",
                        "content": "main model saw the routed result and answered"
                    },
                    "usage": {
                        "total_tokens": 120
                    }
                }
            ],
            "visible_internal_llm_tool_events": [
                {
                    "event_type": "visible_internal_llm_tool_started",
                    "main_node_id": "node-llm",
                    "target_node_id": "node-llm-1",
                    "tool_name": "image_llm",
                    "tool_call_id": "call_image",
                    "arguments": {
                        "file": "uploads/agent-flow-node-detail-icon-aligned.png"
                    }
                },
                {
                    "event_type": "visible_internal_llm_tool_waiting_callback",
                    "main_node_id": "node-llm",
                    "target_node_id": "node-llm-1",
                    "tool_name": "image_llm",
                    "tool_call_id": "call_image",
                    "waiting_node_id": "node-llm-1",
                    "waiting_node_alias": "Read image",
                    "request_payload": {
                        "history": [
                            {
                                "role": "user",
                                "content_blocks": [
                                    {
                                        "type": "image",
                                        "source": {
                                            "type": "base64",
                                            "media_type": "image/png",
                                            "data": "large-base64-data-should-stay-in-detail"
                                        }
                                    }
                                ]
                            }
                        ]
                    }
                },
                {
                    "event_type": "visible_internal_llm_tool_completed",
                    "main_node_id": "node-llm",
                    "target_node_id": "node-llm-1",
                    "tool_name": "image_llm",
                    "tool_call_id": "call_image",
                    "node_id": "node-llm-1",
                    "provider_route": {
                        "model": "mimo-v2.5",
                        "provider_code": "anthropic"
                    }
                }
            ]
        });

        let traces = collect_visible_internal_llm_tool_route_traces(&debug_payload);

        assert_eq!(traces.len(), 1);
        let summary = traces[0].summary_payload(Uuid::nil());
        assert_eq!(summary["kind"], json!(VISIBLE_INTERNAL_LLM_TOOL_TRACE_KIND));
        assert_eq!(summary["tool_call_id"], json!("call_image"));
        assert_eq!(summary["tool_name"], json!("image_llm"));
        assert_eq!(summary["route_model"], json!("mimo-v2.5"));
        assert_eq!(summary["returned_to_main"], json!(true));
        assert_eq!(summary["main_resume"], json!(true));
        assert_eq!(summary["callback_count"], json!(1));
        assert_eq!(
            summary["route_output_summary"]["char_count"],
            json!(route_output.chars().count() as i64)
        );
        assert_eq!(summary["route_output_summary"]["truncated"], json!(true));
        assert_eq!(summary["__runtime_debug_artifact"], json!(true));
        assert_eq!(summary["artifact_ref"], json!(Uuid::nil().to_string()));
        assert!(!summary.to_string().contains("request_payload"));
        assert!(!summary
            .to_string()
            .contains("large-base64-data-should-stay-in-detail"));
        assert!(!summary.to_string().contains(&route_output));

        let detail = traces[0].detail_payload();
        assert_eq!(detail["route"]["model"], json!("mimo-v2.5"));
        assert_eq!(detail["route_output"], json!(route_output));
        assert_eq!(
            detail["main_resume_output"]["content"],
            json!("main model saw the routed result and answered")
        );
        assert_eq!(
            detail["callback_requests"][0]["request_payload"]["history"][0]["content_blocks"][0]
                ["source"]["data"],
            json!("large-base64-data-should-stay-in-detail")
        );
    }

    #[test]
    fn route_trace_ignores_plain_external_tool_rounds() {
        let debug_payload = json!({
            "llm_rounds": [
                {
                    "round_index": 0,
                    "assistant": {
                        "role": "assistant",
                        "tool_calls": [
                            {
                                "id": "call_external",
                                "name": "read_file"
                            }
                        ]
                    }
                },
                {
                    "round_index": 1,
                    "tool_results": [
                        {
                            "role": "tool",
                            "tool_call_id": "call_external",
                            "name": "read_file",
                            "content": "plain tool result"
                        }
                    ]
                }
            ],
            "visible_internal_llm_tool_events": []
        });

        assert!(collect_visible_internal_llm_tool_route_traces(&debug_payload).is_empty());
    }

    #[test]
    fn route_trace_uses_completed_event_content_without_persisted_rounds() {
        let debug_payload = json!({
            "visible_internal_llm_tool_events": [
                {
                    "event_type": "visible_internal_llm_tool_started",
                    "main_node_id": "node-llm",
                    "target_node_id": "node-llm-1",
                    "tool_name": "image_llm",
                    "tool_call_id": "call_image",
                    "arguments": {
                        "media": [
                            {
                                "kind": "image",
                                "path": "uploads/test-01.png",
                                "source": "workspace_path"
                            }
                        ]
                    }
                },
                {
                    "event_type": "visible_internal_llm_tool_completed",
                    "main_node_id": "node-llm",
                    "target_node_id": "node-llm-1",
                    "tool_name": "image_llm",
                    "tool_call_id": "call_image",
                    "node_id": "node-llm-1",
                    "provider_route": {
                        "model": "mimo-v2.5",
                        "provider_code": "anthropic"
                    },
                    "content": "图片是 1flowbase 顶部导航栏。"
                }
            ]
        });

        let traces = collect_visible_internal_llm_tool_route_traces(&debug_payload);

        assert_eq!(traces.len(), 1);
        let summary = traces[0].inline_summary_payload();
        assert_eq!(summary["status"], json!("returned_to_main"));
        assert_eq!(summary["route_model"], json!("mimo-v2.5"));
        assert_eq!(
            summary["route_output_summary"]["preview"],
            json!("图片是 1flowbase 顶部导航栏。")
        );
    }

    #[test]
    fn route_trace_projects_node_output_as_main_resume_for_current_run_sample() {
        let debug_payload = json!({
            "visible_internal_llm_tool_events": [
                {
                    "event_type": "visible_internal_llm_tool_started",
                    "main_node_id": "node-llm",
                    "target_node_id": "node-llm-1",
                    "tool_name": "image_llm",
                    "tool_call_id": "call_image"
                },
                {
                    "event_type": "visible_internal_llm_tool_completed",
                    "main_node_id": "node-llm",
                    "target_node_id": "node-llm-1",
                    "tool_name": "image_llm",
                    "tool_call_id": "call_image",
                    "node_id": "node-llm-1",
                    "provider_route": {
                        "model": "mimo-v2.5",
                        "provider_code": "anthropic"
                    }
                }
            ]
        });
        let node_output = json!({
            "text": "很好，图片分析出来了！这是一个 1flowbase 的顶部导航栏。"
        });

        let traces = collect_visible_internal_llm_tool_route_traces_with_main_output(
            &debug_payload,
            Some(&node_output),
        );

        assert_eq!(traces.len(), 1);
        let summary = traces[0].inline_summary_payload();
        assert_eq!(summary["status"], json!("succeeded"));
        assert_eq!(summary["returned_to_main"], json!(true));
        assert_eq!(summary["main_resume"], json!(true));
        assert_eq!(
            summary["final_output_summary"]["preview"],
            json!("很好，图片分析出来了！这是一个 1flowbase 的顶部导航栏。")
        );
    }

    #[test]
    fn fusion_trace_projects_panel_branch_summaries_and_fan_in_detail() {
        let debug_payload = json!({
            "llm_rounds": [
                {
                    "round_index": 0,
                    "assistant": {
                        "role": "assistant",
                        "tool_calls": [
                            {
                                "id": "call_fusion",
                                "name": "fusion_review",
                                "arguments": {
                                    "topic": "refund policy"
                                }
                            }
                        ]
                    }
                },
                {
                    "round_index": 1,
                    "tool_results": [
                        {
                            "role": "tool",
                            "tool_call_id": "call_fusion",
                            "name": "fusion_review",
                            "content": "panel A says strict\npanel B says flexible"
                        }
                    ]
                },
                {
                    "round_index": 2,
                    "assistant": {
                        "role": "assistant",
                        "content": "main merged the fusion panel"
                    }
                }
            ],
            "visible_internal_llm_tool_events": [
                {
                    "event_type": "visible_internal_llm_tool_started",
                    "main_node_id": "node-main-llm",
                    "target_node_id": "node-panel-a",
                    "tool_name": "fusion_review",
                    "tool_call_id": "call_fusion",
                    "tool_mode": "fusion",
                    "execution_mode": "bounded_parallel_panel"
                },
                {
                    "event_type": "visible_internal_llm_tool_completed",
                    "main_node_id": "node-main-llm",
                    "target_node_id": "node-panel-a",
                    "tool_name": "fusion_review",
                    "tool_call_id": "call_fusion",
                    "tool_mode": "fusion",
                    "execution_mode": "bounded_parallel_panel",
                    "node_id": "node-panel-a",
                    "node_alias": "Risk Panel",
                    "node_type": "llm",
                    "provider_route": {
                        "model": "risk-v1"
                    },
                    "input_payload": {
                        "user_prompt": "review refund policy risk",
                        "model": "risk-v1"
                    },
                    "content": "panel A says strict",
                    "debug_payload": {
                        "llm_rounds": [
                            {
                                "round_index": 0,
                                "assistant": {
                                    "content": "risk result"
                                }
                            }
                        ]
                    }
                },
                {
                    "event_type": "visible_internal_llm_tool_completed",
                    "main_node_id": "node-main-llm",
                    "target_node_id": "node-panel-a",
                    "tool_name": "fusion_review",
                    "tool_call_id": "call_fusion",
                    "tool_mode": "fusion",
                    "execution_mode": "bounded_parallel_panel",
                    "node_id": "node-panel-b",
                    "node_alias": "Support Panel",
                    "node_type": "llm",
                    "provider_route": {
                        "model": "support-v1"
                    },
                    "content": "panel B says flexible",
                    "debug_payload_ref": "artifact-panel-b"
                }
            ]
        });

        let traces = collect_visible_internal_llm_tool_route_traces(&debug_payload);

        assert_eq!(traces.len(), 1);
        let summary = traces[0].inline_summary_payload();
        assert_eq!(summary["route_kind"], json!("fusion"));
        assert_eq!(summary["branch_count"], json!(2));
        assert_eq!(
            summary["branch_summaries"][0]["node_id"],
            json!("node-panel-a")
        );
        assert_eq!(
            summary["branch_summaries"][0]["node_alias"],
            json!("Risk Panel")
        );
        assert_eq!(
            summary["branch_summaries"][0]["route_model"],
            json!("risk-v1")
        );
        assert_eq!(
            summary["branch_summaries"][0]["output_summary"]["preview"],
            json!("panel A says strict")
        );
        assert!(!summary.to_string().contains("risk result"));

        let detail = traces[0].detail_payload();
        assert_eq!(detail["route_kind"], json!("fusion"));
        assert_eq!(detail["fan_in"]["mode"], json!("bounded_parallel_panel"));
        assert_eq!(detail["fan_in"]["branch_count"], json!(2));
        assert_eq!(detail["fan_in"]["returned_to_main"], json!(true));
        assert_eq!(detail["fan_in"]["main_resume"], json!(true));
        assert_eq!(
            detail["branch_traces"][0]["input_payload"]["user_prompt"],
            json!("review refund policy risk")
        );
        assert_eq!(
            detail["branch_traces"][0]["debug_payload"]["llm_rounds"][0]["assistant"]["content"],
            json!("risk result")
        );
        assert_eq!(
            detail["branch_traces"][0]["output_payload"]["text"],
            json!("panel A says strict")
        );
        assert_eq!(
            detail["branch_traces"][0]["output_payload"]["provider_route"]["model"],
            json!("risk-v1")
        );
        assert_eq!(
            detail["branch_traces"][1]["debug_payload_ref"],
            json!("artifact-panel-b")
        );
    }

    #[test]
    fn fusion_trace_projects_historical_summary_llm_detail_from_debug_context() {
        let debug_payload = json!({
            "visible_internal_llm_tool_events": [
                {
                    "event_type": "visible_internal_llm_tool_started",
                    "main_node_id": "node-main-llm",
                    "target_node_id": "node-panel-a",
                    "tool_name": "fusion_review",
                    "tool_call_id": "call_fusion",
                    "tool_mode": "fusion",
                    "execution_mode": "bounded_parallel_panel"
                },
                {
                    "event_type": "visible_internal_llm_tool_completed",
                    "main_node_id": "node-main-llm",
                    "target_node_id": "node-panel-a",
                    "tool_name": "fusion_review",
                    "tool_call_id": "call_fusion",
                    "tool_mode": "fusion",
                    "execution_mode": "bounded_parallel_panel",
                    "node_id": "node-judge",
                    "node_alias": "LLM5",
                    "node_type": "llm",
                    "provider_route": {
                        "model": "gpt-5.4-mini",
                        "provider_code": "fixture_provider"
                    },
                    "metrics_payload": {
                        "usage": {
                            "input_tokens": 5513,
                            "output_tokens": 2455,
                            "total_tokens": 7968
                        }
                    },
                    "debug_payload": {
                        "llm_context": {
                            "effective_system": "You are the fusion judge.",
                            "provider_messages": [
                                {
                                    "role": "user",
                                    "content": "Merge panel answers."
                                }
                            ]
                        },
                        "assistant_message": {
                            "role": "assistant",
                            "content": "judge merged answer"
                        }
                    },
                    "content": "judge merged answer"
                }
            ]
        });

        let traces = collect_visible_internal_llm_tool_route_traces(&debug_payload);

        assert_eq!(traces.len(), 1);
        let detail = traces[0].detail_payload();
        let branch_trace = &detail["branch_traces"][0];
        assert_eq!(branch_trace["node_alias"], json!("LLM5"));
        assert_eq!(
            branch_trace["input_payload"]["prompt_messages"][0]["role"],
            json!("system")
        );
        assert_eq!(
            branch_trace["input_payload"]["prompt_messages"][0]["content"],
            json!("You are the fusion judge.")
        );
        assert_eq!(
            branch_trace["input_payload"]["prompt_messages"][1]["content"],
            json!("Merge panel answers.")
        );
        assert_eq!(
            branch_trace["output_payload"]["text"],
            json!("judge merged answer")
        );
        assert_eq!(
            branch_trace["metrics_payload"]["usage"]["total_tokens"],
            json!(7968)
        );
    }
}
