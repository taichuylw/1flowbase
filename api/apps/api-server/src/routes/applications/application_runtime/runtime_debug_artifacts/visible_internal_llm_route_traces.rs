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

#[derive(Default)]
struct VisibleInternalLlmToolTraceFacts {
    tool_call_id: String,
    tool_name: Option<String>,
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
}

pub(super) fn collect_visible_internal_llm_tool_route_traces(
    debug_payload: &Value,
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

    let mut facts_by_tool_call_id = collect_trace_event_facts(events);
    collect_trace_round_facts(
        debug_payload
            .get("llm_rounds")
            .and_then(Value::as_array)
            .map(Vec::as_slice)
            .unwrap_or_default(),
        &mut facts_by_tool_call_id,
    );

    facts_by_tool_call_id
        .into_values()
        .filter_map(route_trace_from_facts)
        .collect()
}

fn collect_trace_event_facts(
    events: &[Value],
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
            string_field(event_object, &["node_id", "waiting_node_id"]),
        );
        set_if_some(
            &mut entry.route_node_alias,
            string_field(event_object, &["node_alias", "waiting_node_alias"]),
        );
        if entry.arguments.is_none() {
            entry.arguments = event_object.get("arguments").cloned();
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
) -> Option<VisibleInternalLlmToolRouteTrace> {
    let route_output = facts
        .tool_result
        .as_ref()
        .and_then(|value| value.get("content"))
        .cloned();
    let route_output_summary = route_output
        .as_ref()
        .map(summarize_runtime_value)
        .unwrap_or_else(|| summarize_runtime_value(&Value::Null));
    let final_output = facts
        .main_resume_output
        .as_ref()
        .and_then(|value| value.get("content"))
        .cloned();
    let final_output_summary = final_output
        .as_ref()
        .map(summarize_runtime_value)
        .unwrap_or_else(|| summarize_runtime_value(&Value::Null));
    let returned_to_main = facts.tool_result.is_some();
    let main_resume = facts.main_resume_round_index.is_some();
    let status = route_trace_status(&facts, returned_to_main, main_resume);
    let tool_name = facts.tool_name.unwrap_or_else(|| "Tool".to_string());

    let summary_payload = json!({
        "kind": VISIBLE_INTERNAL_LLM_TOOL_TRACE_KIND,
        "preview_kind": VISIBLE_INTERNAL_LLM_TOOL_TRACE_KIND,
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
    });
    let detail_payload = json!({
        "kind": VISIBLE_INTERNAL_LLM_TOOL_TRACE_KIND,
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
        "main_resume_output": facts.main_resume_output,
        "final_output": final_output,
        "final_output_summary": summary_payload["final_output_summary"],
        "callback_requests": facts.waiting_events.iter().map(callback_request_detail).collect::<Vec<_>>(),
        "events": facts.events,
    });

    Some(VisibleInternalLlmToolRouteTrace {
        detail_payload,
        summary_payload,
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
}
