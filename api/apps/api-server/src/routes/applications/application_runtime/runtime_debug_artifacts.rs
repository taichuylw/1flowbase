use std::{collections::HashMap, future::Future, pin::Pin, sync::Arc};

use control_plane::{
    errors::ControlPlaneError,
    orchestration_runtime::debug_artifacts::{
        build_runtime_debug_artifact_object_path, build_runtime_debug_artifact_preview,
        inline_budget_for_kind, RUNTIME_DEBUG_ARTIFACT_CONTENT_TYPE_JSON,
        RUNTIME_DEBUG_ARTIFACT_RETENTION_ACTIVE,
    },
    ports::{
        CreateRuntimeDebugArtifactInput, FileManagementRepository, OrchestrationRuntimeRepository,
        UpdateCallbackTaskPayloadsInput, UpdateCheckpointPayloadsInput, UpdateFlowRunPayloadsInput,
        UpdateNodeRunPayloadsInput, UpdateRunEventPayloadInput,
    },
};
use serde_json::{json, Map, Value};
use storage_durable::MainDurableStore;
use uuid::Uuid;

use crate::{app_state::ApiState, error_response::ApiError};

type RuntimeDebugArtifactOffloadFuture<'a> =
    Pin<Box<dyn Future<Output = Result<(Value, bool), ApiError>> + Send + 'a>>;

mod payloads;
#[cfg(test)]
mod tests;
mod visible_internal_llm_route_traces;

pub(super) use payloads::{
    application_run_model, application_run_query, load_runtime_debug_artifact_json_value,
    load_runtime_debug_artifact_response,
};
use payloads::{
    is_runtime_debug_artifact_payload, is_safe_to_persist_debug_artifact_previews,
    should_keep_runtime_payload_field_inline, with_application_run_input_summary,
    with_debug_artifact_field_path,
};
use visible_internal_llm_route_traces::{
    collect_visible_internal_llm_tool_route_traces,
    collect_visible_internal_llm_tool_route_traces_with_main_output,
};

struct RuntimeDebugArtifactScope {
    workspace_id: Uuid,
    application_id: Uuid,
    flow_run_id: Option<Uuid>,
    node_run_id: Option<Uuid>,
    run_event_id: Option<Uuid>,
}

struct RuntimeDebugArtifactWriter {
    state: Arc<ApiState>,
    storage: domain::FileStorageRecord,
    driver: Arc<dyn storage_object::FileStorageDriver>,
    llm_tool_callback_runtime_facts: HashMap<String, LlmToolCallbackRuntimeFacts>,
}

#[derive(Clone)]
struct LlmToolCallbackRuntimeFacts {
    callback_payload: Value,
    duration_ms: Option<i64>,
}

#[derive(Clone)]
struct LlmToolCallbackArtifact {
    id: String,
    name: String,
    request_payload: Value,
    callback_payload: Option<Value>,
    request_round_index: Option<i64>,
    result_round_index: Option<i64>,
    call_usage: Option<Value>,
    result_context_usage: Option<Value>,
    duration_ms: Option<i64>,
}

impl LlmToolCallbackArtifact {
    fn callback_status(&self) -> &'static str {
        if self.callback_payload.is_some() {
            "returned"
        } else {
            "waiting_callback"
        }
    }

    fn detail_payload(&self) -> Value {
        json!({
            "id": self.id,
            "name": self.name,
            "callback_status": self.callback_status(),
            "execution_status": execution_status_from_callback_payload(self.callback_payload.as_ref()),
            "request_payload": self.request_payload,
            "callback_payload": self.callback_payload,
            "parsed_result": self.callback_payload.as_ref().map(parsed_tool_callback_payload),
            "request_round_index": self.request_round_index,
            "result_round_index": self.result_round_index,
            "call_usage": self.call_usage,
            "result_context_usage": self.result_context_usage,
            "duration_ms": self.duration_ms,
        })
    }

    fn summary_payload(&self, artifact_id: Uuid) -> Value {
        json!({
            "id": self.id,
            "name": self.name,
            "callback_status": self.callback_status(),
            "execution_status": execution_status_from_callback_payload(self.callback_payload.as_ref()),
            "request_round_index": self.request_round_index,
            "result_round_index": self.result_round_index,
            "artifact_ref": artifact_id.to_string(),
            "call_usage": self.call_usage,
            "result_context_usage": self.result_context_usage,
            "duration_ms": self.duration_ms,
        })
    }
}

fn is_llm_rounds_field_path(field_path: &[String]) -> bool {
    field_path.len() == 1 && field_path[0] == "llm_rounds"
}

fn is_tool_calls_field_path(field_path: &[String]) -> bool {
    field_path.last().is_some_and(|key| key == "tool_calls")
}

fn with_array_item_count(mut payload: Value, full_value: &Value, field_name: &str) -> Value {
    let Some(count) = full_value.as_array().map(|items| items.len() as i64) else {
        return payload;
    };
    let Some(object) = payload.as_object_mut() else {
        return payload;
    };
    object.insert(field_name.to_string(), json!(count));
    payload
}

fn is_llm_rounds_debug_artifact_missing_tool_index(value: &Value) -> bool {
    is_runtime_debug_artifact_payload(value) && value.get("tool_callbacks").is_none()
}

fn value_object(value: &Value) -> Option<&Map<String, Value>> {
    value.as_object()
}

fn record_field<'a>(record: &'a Map<String, Value>, keys: &[&str]) -> Option<&'a Value> {
    keys.iter().find_map(|key| record.get(*key))
}

fn record_string_field(record: &Map<String, Value>, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        record
            .get(*key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
    })
}

fn record_value_field(record: &Map<String, Value>, keys: &[&str]) -> Option<Value> {
    keys.iter().find_map(|key| record.get(*key).cloned())
}

fn record_i64_field(record: &Map<String, Value>, keys: &[&str]) -> Option<i64> {
    keys.iter().find_map(|key| {
        let value = record.get(*key)?;
        if let Some(value) = value.as_i64() {
            return Some(value);
        }

        value.as_u64().and_then(|value| i64::try_from(value).ok())
    })
}

fn round_index(round: &Map<String, Value>, fallback_index: usize) -> i64 {
    round
        .get("round_index")
        .and_then(Value::as_i64)
        .unwrap_or(fallback_index as i64)
}

fn read_round_tool_calls(round: &Map<String, Value>) -> Vec<Value> {
    let assistant_tool_calls = record_field(round, &["assistant", "assistant_message"])
        .and_then(value_object)
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

fn tool_call_id(tool_call: &Map<String, Value>, round_number: i64, index: usize) -> String {
    record_string_field(tool_call, &["id", "tool_call_id", "call_id"])
        .unwrap_or_else(|| format!("tool-{}-{}", round_number + 1, index + 1))
}

fn tool_result_id(tool_result: &Map<String, Value>, round_number: i64, index: usize) -> String {
    record_string_field(tool_result, &["tool_call_id", "id", "call_id"])
        .unwrap_or_else(|| format!("tool-result-{}-{}", round_number + 1, index + 1))
}

fn callback_duration_ms(task: &domain::CallbackTaskRecord) -> Option<i64> {
    let completed_at = task.completed_at?;
    let duration = completed_at - task.created_at;
    if duration < time::Duration::ZERO {
        return None;
    }

    i64::try_from(duration.whole_milliseconds()).ok()
}

fn collect_llm_tool_callback_runtime_facts(
    callback_tasks: &[domain::CallbackTaskRecord],
) -> HashMap<String, LlmToolCallbackRuntimeFacts> {
    let mut facts_by_tool_call_id = HashMap::new();

    for task in callback_tasks {
        if task.callback_kind != "llm_tool_calls" {
            continue;
        }

        let Some(response_payload) = task.response_payload.as_ref() else {
            continue;
        };
        let duration_ms = callback_duration_ms(task);

        for callback_payload in read_callback_response_tool_payloads(response_payload) {
            let Some(callback_payload_object) = callback_payload.as_object() else {
                continue;
            };
            let Some(tool_call_id) =
                record_string_field(callback_payload_object, &["tool_call_id", "id", "call_id"])
            else {
                continue;
            };

            facts_by_tool_call_id.insert(
                tool_call_id,
                LlmToolCallbackRuntimeFacts {
                    callback_payload,
                    duration_ms,
                },
            );
        }
    }

    facts_by_tool_call_id
}

fn read_callback_response_tool_payloads(response_payload: &Value) -> Vec<Value> {
    if let Some(tool_results) = response_payload
        .get("tool_results")
        .and_then(Value::as_array)
        .cloned()
    {
        return tool_results;
    }

    response_payload
        .as_object()
        .and_then(|object| {
            record_string_field(object, &["tool_call_id", "id", "call_id"])
                .map(|_| vec![response_payload.clone()])
        })
        .unwrap_or_default()
}

fn execution_status_from_callback_payload(callback_payload: Option<&Value>) -> &'static str {
    let Some(callback_payload) = callback_payload else {
        return "unknown";
    };
    let Some(callback_payload_object) = callback_payload.as_object() else {
        return "unknown";
    };

    if let Some(status) = callback_payload_object
        .get("execution")
        .and_then(Value::as_object)
        .and_then(|execution| execution.get("status"))
        .and_then(Value::as_str)
        .and_then(normalized_execution_status)
    {
        return status;
    }
    if let Some(status) = callback_payload_object
        .get("execution_status")
        .and_then(Value::as_str)
        .and_then(normalized_execution_status)
    {
        return status;
    }
    if callback_payload_object
        .get("timed_out")
        .and_then(Value::as_bool)
        == Some(true)
    {
        return "timed_out";
    }
    if callback_payload_object
        .get("cancelled")
        .and_then(Value::as_bool)
        == Some(true)
    {
        return "cancelled";
    }
    if let Some(exit_code) = callback_payload_object
        .get("exit_code")
        .and_then(Value::as_i64)
    {
        return if exit_code == 0 {
            "succeeded"
        } else {
            "failed"
        };
    }
    if let Some(http_status) = callback_payload_object
        .get("http_status")
        .and_then(Value::as_i64)
    {
        return if (200..300).contains(&http_status) {
            "succeeded"
        } else {
            "failed"
        };
    }
    if callback_payload_object
        .get("is_error")
        .and_then(Value::as_bool)
        == Some(true)
        || callback_payload_object
            .get("error")
            .is_some_and(|value| !value.is_null())
    {
        return "failed";
    }

    "unknown"
}

fn normalized_execution_status(status: &str) -> Option<&'static str> {
    match status {
        "succeeded" => Some("succeeded"),
        "failed" => Some("failed"),
        "timed_out" => Some("timed_out"),
        "cancelled" | "canceled" => Some("cancelled"),
        "unknown" => Some("unknown"),
        _ => None,
    }
}

fn parsed_tool_callback_payload(callback_payload: &Value) -> Value {
    let Some(callback_payload_object) = callback_payload.as_object() else {
        return json!({ "raw": callback_payload });
    };

    let mut parsed_payload = Map::new();
    for key in [
        "tool_call_id",
        "id",
        "call_id",
        "name",
        "content",
        "stdout",
        "stderr",
        "error",
        "exit_code",
        "http_status",
        "is_error",
        "timed_out",
        "cancelled",
        "execution",
        "execution_status",
    ] {
        if let Some(value) = callback_payload_object.get(key) {
            parsed_payload.insert(key.to_string(), value.clone());
        }
    }

    Value::Object(parsed_payload)
}

fn collect_llm_tool_callbacks(
    llm_rounds: &Value,
    runtime_facts: &HashMap<String, LlmToolCallbackRuntimeFacts>,
) -> Vec<LlmToolCallbackArtifact> {
    let Some(rounds) = llm_rounds.as_array() else {
        return Vec::new();
    };
    let mut callbacks: Vec<LlmToolCallbackArtifact> = Vec::new();
    let mut index_by_id = std::collections::HashMap::<String, usize>::new();

    for (fallback_round_index, round) in rounds.iter().enumerate() {
        let Some(round) = round.as_object() else {
            continue;
        };
        let current_round_index = round_index(round, fallback_round_index);
        let current_usage = round.get("usage").cloned();
        let next_usage = rounds
            .get(fallback_round_index + 1)
            .and_then(Value::as_object)
            .and_then(|round| round.get("usage"))
            .cloned();

        for (tool_call_index, tool_call) in read_round_tool_calls(round).into_iter().enumerate() {
            let Some(tool_call_object) = tool_call.as_object() else {
                continue;
            };
            let id = tool_call_id(tool_call_object, current_round_index, tool_call_index);
            let name =
                record_string_field(tool_call_object, &["name"]).unwrap_or_else(|| "Tool".into());

            upsert_llm_tool_callback(
                &mut callbacks,
                &mut index_by_id,
                LlmToolCallbackArtifact {
                    callback_payload: runtime_facts
                        .get(&id)
                        .map(|facts| facts.callback_payload.clone()),
                    duration_ms: runtime_facts.get(&id).and_then(|facts| facts.duration_ms),
                    id,
                    name,
                    call_usage: record_value_field(tool_call_object, &["call_usage"])
                        .or_else(|| current_usage.clone()),
                    result_context_usage: None,
                    request_payload: tool_call,
                    request_round_index: Some(current_round_index),
                    result_round_index: None,
                },
            );
        }

        for (tool_result_index, tool_result) in
            read_round_tool_results(round).into_iter().enumerate()
        {
            let Some(tool_result_object) = tool_result.as_object() else {
                continue;
            };
            let id = tool_result_id(tool_result_object, current_round_index, tool_result_index);
            let name =
                record_string_field(tool_result_object, &["name"]).unwrap_or_else(|| "Tool".into());

            upsert_llm_tool_callback(
                &mut callbacks,
                &mut index_by_id,
                LlmToolCallbackArtifact {
                    callback_payload: runtime_facts
                        .get(&id)
                        .map(|facts| facts.callback_payload.clone())
                        .or_else(|| Some(tool_result.clone())),
                    duration_ms: record_i64_field(tool_result_object, &["duration_ms"])
                        .or_else(|| runtime_facts.get(&id).and_then(|facts| facts.duration_ms)),
                    id,
                    name,
                    call_usage: record_value_field(tool_result_object, &["call_usage"]),
                    result_context_usage: record_value_field(
                        tool_result_object,
                        &["result_context_usage"],
                    )
                    .or_else(|| next_usage.clone()),
                    request_payload: json!({}),
                    request_round_index: None,
                    result_round_index: Some(current_round_index),
                },
            );
        }
    }

    callbacks
}

fn upsert_llm_tool_callback(
    callbacks: &mut Vec<LlmToolCallbackArtifact>,
    index_by_id: &mut std::collections::HashMap<String, usize>,
    next: LlmToolCallbackArtifact,
) {
    let Some(index) = index_by_id.get(&next.id).copied() else {
        index_by_id.insert(next.id.clone(), callbacks.len());
        callbacks.push(next);
        return;
    };

    let current = &mut callbacks[index];
    if next
        .request_payload
        .as_object()
        .is_some_and(|object| !object.is_empty())
    {
        current.request_payload = next.request_payload;
    }
    if next.callback_payload.is_some() {
        current.callback_payload = next.callback_payload;
    }
    if current.name == "Tool" && next.name != "Tool" {
        current.name = next.name;
    }
    if next.request_round_index.is_some() {
        current.request_round_index = next.request_round_index;
    }
    if next.result_round_index.is_some() {
        current.result_round_index = next.result_round_index;
    }
    if next.call_usage.is_some() {
        current.call_usage = next.call_usage;
    }
    if next.result_context_usage.is_some() {
        current.result_context_usage = next.result_context_usage;
    }
    if next.duration_ms.is_some() {
        current.duration_ms = next.duration_ms;
    }
}

fn with_llm_tool_callback_runtime_facts(
    llm_rounds: Value,
    runtime_facts: &HashMap<String, LlmToolCallbackRuntimeFacts>,
) -> (Value, bool) {
    let Value::Array(rounds) = llm_rounds else {
        return (llm_rounds, false);
    };
    let mut changed = false;
    let rounds = rounds
        .into_iter()
        .enumerate()
        .map(|(fallback_round_index, round)| {
            let Some(mut round_object) = round.as_object().cloned() else {
                return round;
            };
            let current_round_index = round_index(&round_object, fallback_round_index);
            let Some(tool_results) = round_object
                .get_mut("tool_results")
                .and_then(Value::as_array_mut)
            else {
                return Value::Object(round_object);
            };

            for (tool_result_index, tool_result) in tool_results.iter_mut().enumerate() {
                let Some(tool_result_object) = tool_result.as_object_mut() else {
                    continue;
                };
                if tool_result_object.contains_key("duration_ms") {
                    continue;
                }
                let id = tool_result_id(tool_result_object, current_round_index, tool_result_index);
                let Some(duration_ms) = runtime_facts.get(&id).and_then(|facts| facts.duration_ms)
                else {
                    continue;
                };

                tool_result_object.insert("duration_ms".to_string(), json!(duration_ms));
                changed = true;
            }

            Value::Object(round_object)
        })
        .collect();

    (Value::Array(rounds), changed)
}

impl RuntimeDebugArtifactWriter {
    async fn new(state: Arc<ApiState>) -> Result<Self, ApiError> {
        let storage =
            <MainDurableStore as FileManagementRepository>::get_default_file_storage(&state.store)
                .await?
                .ok_or(ControlPlaneError::Conflict("file_storage_default_missing"))?;
        if !storage.enabled {
            return Err(ControlPlaneError::Conflict("file_storage_disabled").into());
        }
        let driver = state
            .file_storage_registry
            .get(&storage.driver_type)
            .ok_or(ControlPlaneError::Conflict("storage_driver_not_registered"))?;

        Ok(Self {
            state,
            storage,
            driver,
            llm_tool_callback_runtime_facts: HashMap::new(),
        })
    }

    async fn offload_value(
        &self,
        scope: &RuntimeDebugArtifactScope,
        artifact_kind: &str,
        value: Value,
    ) -> Result<(Value, bool), ApiError> {
        let artifact_id = Uuid::now_v7();
        let Some(preview) = build_runtime_debug_artifact_preview(
            artifact_id,
            &value,
            inline_budget_for_kind(artifact_kind),
        )?
        else {
            return Ok((value, false));
        };
        let storage_ref = build_runtime_debug_artifact_object_path(
            scope.workspace_id,
            scope.application_id,
            scope.flow_run_id,
            preview.artifact_id,
        );

        self.driver
            .put_object(storage_object::FileStoragePutInput {
                config_json: &self.storage.config_json,
                object_path: &storage_ref,
                content_type: Some(RUNTIME_DEBUG_ARTIFACT_CONTENT_TYPE_JSON),
                bytes: &preview.full_bytes,
            })
            .await?;
        <MainDurableStore as OrchestrationRuntimeRepository>::create_runtime_debug_artifact(
            &self.state.store,
            &CreateRuntimeDebugArtifactInput {
                artifact_id: preview.artifact_id,
                workspace_id: scope.workspace_id,
                application_id: scope.application_id,
                flow_run_id: scope.flow_run_id,
                node_run_id: scope.node_run_id,
                run_event_id: scope.run_event_id,
                artifact_kind: artifact_kind.to_string(),
                content_type: RUNTIME_DEBUG_ARTIFACT_CONTENT_TYPE_JSON.to_string(),
                original_size_bytes: preview.original_size_bytes,
                preview_size_bytes: preview.preview_size_bytes,
                storage_id: self.storage.id,
                storage_ref,
                retention_state: RUNTIME_DEBUG_ARTIFACT_RETENTION_ACTIVE.to_string(),
            },
        )
        .await?;

        Ok((preview.preview_value, true))
    }

    async fn persist_value_artifact(
        &self,
        scope: &RuntimeDebugArtifactScope,
        artifact_kind: &str,
        value: &Value,
    ) -> Result<Uuid, ApiError> {
        let artifact_id = Uuid::now_v7();
        let bytes = serde_json::to_vec(value)?;
        let storage_ref = build_runtime_debug_artifact_object_path(
            scope.workspace_id,
            scope.application_id,
            scope.flow_run_id,
            artifact_id,
        );

        self.driver
            .put_object(storage_object::FileStoragePutInput {
                config_json: &self.storage.config_json,
                object_path: &storage_ref,
                content_type: Some(RUNTIME_DEBUG_ARTIFACT_CONTENT_TYPE_JSON),
                bytes: &bytes,
            })
            .await?;
        <MainDurableStore as OrchestrationRuntimeRepository>::create_runtime_debug_artifact(
            &self.state.store,
            &CreateRuntimeDebugArtifactInput {
                artifact_id,
                workspace_id: scope.workspace_id,
                application_id: scope.application_id,
                flow_run_id: scope.flow_run_id,
                node_run_id: scope.node_run_id,
                run_event_id: scope.run_event_id,
                artifact_kind: artifact_kind.to_string(),
                content_type: RUNTIME_DEBUG_ARTIFACT_CONTENT_TYPE_JSON.to_string(),
                original_size_bytes: bytes.len() as i64,
                preview_size_bytes: 0,
                storage_id: self.storage.id,
                storage_ref,
                retention_state: RUNTIME_DEBUG_ARTIFACT_RETENTION_ACTIVE.to_string(),
            },
        )
        .await?;

        Ok(artifact_id)
    }

    async fn with_llm_tool_callback_index(
        &self,
        scope: &RuntimeDebugArtifactScope,
        mut payload: Value,
        llm_rounds: &Value,
    ) -> Result<Value, ApiError> {
        let Some(object) = payload.as_object() else {
            return Ok(payload);
        };
        if object.contains_key("tool_callbacks") {
            return Ok(payload);
        }

        let callbacks =
            collect_llm_tool_callbacks(llm_rounds, &self.llm_tool_callback_runtime_facts);
        if callbacks.is_empty() {
            return Ok(payload);
        }

        let mut callback_summaries = Vec::with_capacity(callbacks.len());
        for callback in callbacks {
            let detail_payload = callback.detail_payload();
            let artifact_id = self
                .persist_value_artifact(scope, "node_debug_tool_callback", &detail_payload)
                .await?;

            callback_summaries.push(callback.summary_payload(artifact_id));
        }

        if let Some(object) = payload.as_object_mut() {
            object.insert(
                "tool_callbacks".to_string(),
                Value::Array(callback_summaries),
            );
        }
        Ok(payload)
    }

    async fn with_visible_internal_llm_tool_trace_index(
        &self,
        scope: &RuntimeDebugArtifactScope,
        mut payload: Value,
    ) -> Result<Value, ApiError> {
        let Some(object) = payload.as_object() else {
            return Ok(payload);
        };
        if object.contains_key("visible_internal_llm_tool_trace") {
            return Ok(payload);
        }

        let traces = collect_visible_internal_llm_tool_route_traces(&payload);
        if traces.is_empty() {
            return Ok(payload);
        }

        let mut summaries = Vec::with_capacity(traces.len());
        for trace in traces {
            let artifact_id = self
                .persist_value_artifact(
                    scope,
                    "node_debug_visible_internal_llm_tool_trace",
                    &trace.detail_payload(),
                )
                .await?;
            summaries.push(trace.summary_payload(artifact_id));
        }

        if let Some(object) = payload.as_object_mut() {
            object.insert(
                "visible_internal_llm_tool_trace".to_string(),
                Value::Array(summaries),
            );
        }
        Ok(payload)
    }

    async fn offload_node_debug_payload(
        &self,
        scope: &RuntimeDebugArtifactScope,
        payload: Value,
    ) -> Result<(Value, bool), ApiError> {
        let original_payload = payload.clone();
        let payload = self
            .with_visible_internal_llm_tool_trace_index(scope, payload)
            .await?;
        let trace_changed = payload != original_payload;
        let (payload, fields_changed) = self
            .offload_payload_fields(scope, "node_debug_payload", payload, Vec::new())
            .await?;

        Ok((payload, trace_changed || fields_changed))
    }

    async fn enrich_existing_llm_rounds_preview(
        &self,
        scope: &RuntimeDebugArtifactScope,
        payload: Value,
    ) -> Result<(Value, bool), ApiError> {
        if !is_llm_rounds_debug_artifact_missing_tool_index(&payload) {
            return Ok((payload, false));
        }

        let Some(artifact_id) = payload
            .get("artifact_ref")
            .and_then(Value::as_str)
            .and_then(|value| Uuid::parse_str(value).ok())
        else {
            return Ok((payload, false));
        };
        let full_llm_rounds = load_runtime_debug_artifact_json_value(
            self.state.clone(),
            scope.workspace_id,
            scope.application_id,
            artifact_id,
        )
        .await?;
        let payload = self
            .with_llm_tool_callback_index(scope, payload, &full_llm_rounds)
            .await?;

        Ok((payload, true))
    }

    fn offload_payload_fields<'a>(
        &'a self,
        scope: &'a RuntimeDebugArtifactScope,
        artifact_kind: &'a str,
        value: Value,
        field_path: Vec<String>,
    ) -> RuntimeDebugArtifactOffloadFuture<'a> {
        Box::pin(async move {
            if is_runtime_debug_artifact_payload(&value) {
                if is_llm_rounds_field_path(&field_path) {
                    return self.enrich_existing_llm_rounds_preview(scope, value).await;
                }

                return Ok((value, false));
            }

            if should_keep_runtime_payload_field_inline(&field_path) {
                return Ok((value, false));
            }

            match value {
                Value::Object(object) => {
                    let mut changed = false;
                    let mut next = Map::with_capacity(object.len());
                    for (key, child) in object {
                        let mut child_path = field_path.clone();
                        child_path.push(key.clone());
                        let (child, child_changed) = self
                            .offload_payload_fields(scope, artifact_kind, child, child_path)
                            .await?;
                        changed |= child_changed;
                        next.insert(key, child);
                    }
                    Ok((Value::Object(next), changed))
                }
                Value::Array(_) | Value::String(_) => {
                    let full_value = value.clone();
                    let (payload, changed) =
                        self.offload_value(scope, artifact_kind, value).await?;
                    let payload = if changed {
                        with_debug_artifact_field_path(payload, &field_path)
                    } else {
                        payload
                    };
                    let payload = if changed && is_tool_calls_field_path(&field_path) {
                        with_array_item_count(payload, &full_value, "tool_call_count")
                    } else {
                        payload
                    };
                    let payload = if changed && is_llm_rounds_field_path(&field_path) {
                        self.with_llm_tool_callback_index(scope, payload, &full_value)
                            .await?
                    } else {
                        payload
                    };
                    if !changed && is_llm_rounds_field_path(&field_path) {
                        let (payload, runtime_facts_changed) = with_llm_tool_callback_runtime_facts(
                            payload,
                            &self.llm_tool_callback_runtime_facts,
                        );
                        return Ok((payload, runtime_facts_changed));
                    }
                    Ok((payload, changed))
                }
                value => Ok((value, false)),
            }
        })
    }
}

pub async fn offload_application_run_detail_artifacts(
    state: Arc<ApiState>,
    workspace_id: Uuid,
    application_id: Uuid,
    mut detail: domain::ApplicationRunDetail,
) -> Result<domain::ApplicationRunDetail, ApiError> {
    if !is_safe_to_persist_debug_artifact_previews(detail.flow_run.status) {
        return Ok(detail);
    }

    let mut writer = RuntimeDebugArtifactWriter::new(state.clone()).await?;
    writer.llm_tool_callback_runtime_facts =
        collect_llm_tool_callback_runtime_facts(&detail.callback_tasks);
    let flow_scope = RuntimeDebugArtifactScope {
        workspace_id,
        application_id,
        flow_run_id: Some(detail.flow_run.id),
        node_run_id: None,
        run_event_id: None,
    };
    let flow_input_query = application_run_query(&detail.flow_run.input_payload);
    let flow_input_model = application_run_model(&detail.flow_run.input_payload);
    let (flow_input_payload, flow_input_changed) = writer
        .offload_value(
            &flow_scope,
            "flow_input_payload",
            detail.flow_run.input_payload.clone(),
        )
        .await?;
    let flow_input_payload = if flow_input_changed {
        with_application_run_input_summary(
            flow_input_payload,
            flow_input_query.as_deref(),
            flow_input_model.as_deref(),
        )
    } else {
        flow_input_payload
    };
    let (flow_output_payload, flow_output_changed) = writer
        .offload_payload_fields(
            &flow_scope,
            "flow_output_payload",
            detail.flow_run.output_payload.clone(),
            Vec::new(),
        )
        .await?;
    let (flow_error_payload, flow_error_changed) = match detail.flow_run.error_payload.clone() {
        Some(error_payload) => {
            let (payload, changed) = writer
                .offload_value(&flow_scope, "flow_error_payload", error_payload)
                .await?;
            (Some(payload), changed)
        }
        None => (None, false),
    };
    if flow_input_changed || flow_output_changed || flow_error_changed {
        detail.flow_run =
            <MainDurableStore as OrchestrationRuntimeRepository>::update_flow_run_payloads(
                &state.store,
                &UpdateFlowRunPayloadsInput {
                    flow_run_id: detail.flow_run.id,
                    input_payload: flow_input_payload,
                    output_payload: flow_output_payload,
                    error_payload: flow_error_payload,
                },
            )
            .await?;
    }

    for node_run in &mut detail.node_runs {
        let node_scope = RuntimeDebugArtifactScope {
            workspace_id,
            application_id,
            flow_run_id: Some(detail.flow_run.id),
            node_run_id: Some(node_run.id),
            run_event_id: None,
        };
        let (input_payload, input_changed) = writer
            .offload_payload_fields(
                &node_scope,
                "node_input_payload",
                node_run.input_payload.clone(),
                Vec::new(),
            )
            .await?;
        let (output_payload, output_changed) = writer
            .offload_payload_fields(
                &node_scope,
                "node_output_payload",
                node_run.output_payload.clone(),
                Vec::new(),
            )
            .await?;
        let (error_payload, error_changed) = match node_run.error_payload.clone() {
            Some(error_payload) => {
                let (payload, changed) = writer
                    .offload_payload_fields(
                        &node_scope,
                        "node_error_payload",
                        error_payload,
                        Vec::new(),
                    )
                    .await?;
                (Some(payload), changed)
            }
            None => (None, false),
        };
        let (metrics_payload, metrics_changed) = writer
            .offload_payload_fields(
                &node_scope,
                "node_metrics_payload",
                node_run.metrics_payload.clone(),
                Vec::new(),
            )
            .await?;
        let (debug_payload, debug_changed) = writer
            .offload_node_debug_payload(&node_scope, node_run.debug_payload.clone())
            .await?;

        if input_changed || output_changed || error_changed || metrics_changed || debug_changed {
            *node_run =
                <MainDurableStore as OrchestrationRuntimeRepository>::update_node_run_payloads(
                    &state.store,
                    &UpdateNodeRunPayloadsInput {
                        node_run_id: node_run.id,
                        input_payload,
                        output_payload,
                        error_payload,
                        metrics_payload,
                        debug_payload,
                    },
                )
                .await?;
        }
    }

    for checkpoint in &mut detail.checkpoints {
        let checkpoint_scope = RuntimeDebugArtifactScope {
            workspace_id,
            application_id,
            flow_run_id: Some(detail.flow_run.id),
            node_run_id: checkpoint.node_run_id,
            run_event_id: None,
        };
        let (locator_payload, locator_changed) = writer
            .offload_payload_fields(
                &checkpoint_scope,
                "checkpoint_locator_payload",
                checkpoint.locator_payload.clone(),
                Vec::new(),
            )
            .await?;
        let (variable_snapshot, variable_changed) = writer
            .offload_payload_fields(
                &checkpoint_scope,
                "checkpoint_variable_snapshot",
                checkpoint.variable_snapshot.clone(),
                Vec::new(),
            )
            .await?;
        let (external_ref_payload, external_changed) = match checkpoint.external_ref_payload.clone()
        {
            Some(external_ref_payload) => {
                let (payload, changed) = writer
                    .offload_payload_fields(
                        &checkpoint_scope,
                        "checkpoint_external_ref_payload",
                        external_ref_payload,
                        Vec::new(),
                    )
                    .await?;
                (Some(payload), changed)
            }
            None => (None, false),
        };

        if locator_changed || variable_changed || external_changed {
            *checkpoint =
                <MainDurableStore as OrchestrationRuntimeRepository>::update_checkpoint_payloads(
                    &state.store,
                    &UpdateCheckpointPayloadsInput {
                        checkpoint_id: checkpoint.id,
                        locator_payload,
                        variable_snapshot,
                        external_ref_payload,
                    },
                )
                .await?;
        }
    }

    for callback_task in &mut detail.callback_tasks {
        let callback_scope = RuntimeDebugArtifactScope {
            workspace_id,
            application_id,
            flow_run_id: Some(detail.flow_run.id),
            node_run_id: Some(callback_task.node_run_id),
            run_event_id: None,
        };
        let (request_payload, request_changed) = writer
            .offload_payload_fields(
                &callback_scope,
                "callback_task_request_payload",
                callback_task.request_payload.clone(),
                Vec::new(),
            )
            .await?;
        let (response_payload, response_changed) = match callback_task.response_payload.clone() {
            Some(response_payload) => {
                let (payload, changed) = writer
                    .offload_payload_fields(
                        &callback_scope,
                        "callback_task_response_payload",
                        response_payload,
                        Vec::new(),
                    )
                    .await?;
                (Some(payload), changed)
            }
            None => (None, false),
        };
        let (external_ref_payload, external_changed) =
            match callback_task.external_ref_payload.clone() {
                Some(external_ref_payload) => {
                    let (payload, changed) = writer
                        .offload_payload_fields(
                            &callback_scope,
                            "callback_task_external_ref_payload",
                            external_ref_payload,
                            Vec::new(),
                        )
                        .await?;
                    (Some(payload), changed)
                }
                None => (None, false),
            };

        if request_changed || response_changed || external_changed {
            *callback_task =
                <MainDurableStore as OrchestrationRuntimeRepository>::update_callback_task_payloads(
                    &state.store,
                    &UpdateCallbackTaskPayloadsInput {
                        callback_task_id: callback_task.id,
                        request_payload,
                        response_payload,
                        external_ref_payload,
                    },
                )
                .await?;
        }
    }

    for event in &mut detail.events {
        let event_scope = RuntimeDebugArtifactScope {
            workspace_id,
            application_id,
            flow_run_id: Some(detail.flow_run.id),
            node_run_id: event.node_run_id,
            run_event_id: Some(event.id),
        };
        let (payload, changed) = writer
            .offload_value(&event_scope, "run_event_payload", event.payload.clone())
            .await?;
        if changed {
            *event =
                <MainDurableStore as OrchestrationRuntimeRepository>::update_run_event_payload(
                    &state.store,
                    &UpdateRunEventPayloadInput {
                        run_event_id: event.id,
                        payload,
                    },
                )
                .await?;
        }
    }

    Ok(detail)
}

pub fn enrich_application_run_detail_visible_internal_llm_route_traces(
    mut detail: domain::ApplicationRunDetail,
    runtime_events: &[domain::RuntimeEventRecord],
) -> domain::ApplicationRunDetail {
    enrich_node_runs_visible_internal_llm_route_traces(&mut detail.node_runs, runtime_events);

    for trace in &mut detail.stitched_trace {
        enrich_node_runs_visible_internal_llm_route_traces(
            &mut trace.node_runs,
            &trace.runtime_events,
        );
    }

    detail
}

pub fn enrich_node_last_run_visible_internal_llm_route_traces(
    mut last_run: domain::NodeLastRun,
    runtime_events: &[domain::RuntimeEventRecord],
) -> domain::NodeLastRun {
    let runtime_events_by_node_run_id = visible_internal_llm_tool_runtime_events_by_node_run_id(
        std::slice::from_ref(&last_run.node_run),
        runtime_events,
    );
    let debug_payload = runtime_events_by_node_run_id
        .get(&last_run.node_run.id)
        .map(|runtime_events| {
            with_runtime_visible_internal_llm_tool_events(
                last_run.node_run.debug_payload.clone(),
                runtime_events,
            )
        })
        .unwrap_or_else(|| last_run.node_run.debug_payload.clone());
    last_run.node_run.debug_payload =
        with_inline_visible_internal_llm_tool_trace_index_with_main_output(
            debug_payload,
            Some(&last_run.node_run.output_payload),
        );

    last_run
}

fn enrich_node_runs_visible_internal_llm_route_traces(
    node_runs: &mut [domain::NodeRunRecord],
    runtime_events: &[domain::RuntimeEventRecord],
) {
    let runtime_events_by_node_run_id =
        visible_internal_llm_tool_runtime_events_by_node_run_id(node_runs, runtime_events);

    for node_run in node_runs {
        let debug_payload = runtime_events_by_node_run_id
            .get(&node_run.id)
            .map(|runtime_events| {
                with_runtime_visible_internal_llm_tool_events(
                    node_run.debug_payload.clone(),
                    runtime_events,
                )
            })
            .unwrap_or_else(|| node_run.debug_payload.clone());
        node_run.debug_payload = with_inline_visible_internal_llm_tool_trace_index_with_main_output(
            debug_payload,
            Some(&node_run.output_payload),
        );
    }
}

fn visible_internal_llm_tool_runtime_events_by_node_run_id(
    node_runs: &[domain::NodeRunRecord],
    runtime_events: &[domain::RuntimeEventRecord],
) -> HashMap<Uuid, Vec<Value>> {
    let mut latest_node_run_id_by_node_id = HashMap::<String, Uuid>::new();
    let mut node_run_ids = std::collections::HashSet::<Uuid>::new();
    for node_run in node_runs {
        latest_node_run_id_by_node_id.insert(node_run.node_id.clone(), node_run.id);
        node_run_ids.insert(node_run.id);
    }

    let mut events_by_node_run_id = HashMap::<Uuid, Vec<Value>>::new();
    for event in runtime_events {
        let Some(payload) = visible_internal_llm_tool_runtime_event_payload(event) else {
            continue;
        };
        let explicit_node_run_id =
            visible_internal_llm_tool_runtime_event_node_run_id(event, &payload);
        let owner_node_run_id = match explicit_node_run_id {
            Some(node_run_id) if node_run_ids.contains(&node_run_id) => Some(node_run_id),
            Some(_) => None,
            None => visible_internal_llm_tool_runtime_event_owner_node_id(&payload)
                .and_then(|node_id| latest_node_run_id_by_node_id.get(node_id).copied()),
        };
        let Some(owner_node_run_id) = owner_node_run_id else {
            continue;
        };

        events_by_node_run_id
            .entry(owner_node_run_id)
            .or_default()
            .push(payload);
    }

    events_by_node_run_id
}

fn visible_internal_llm_tool_runtime_event_payload(
    event: &domain::RuntimeEventRecord,
) -> Option<Value> {
    if !event.event_type.starts_with("visible_internal_llm_tool_") {
        return None;
    }

    let mut payload = event.payload.as_object()?.clone();
    payload.insert("event_type".to_string(), json!(event.event_type));
    if !payload.contains_key("node_run_id") {
        if let Some(node_run_id) = event.node_run_id {
            payload.insert("node_run_id".to_string(), json!(node_run_id.to_string()));
        }
    }

    Some(Value::Object(payload))
}

fn visible_internal_llm_tool_runtime_event_owner_node_id(payload: &Value) -> Option<&str> {
    payload
        .get("main_node_id")
        .or_else(|| payload.get("node_id"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn visible_internal_llm_tool_runtime_event_node_run_id(
    event: &domain::RuntimeEventRecord,
    payload: &Value,
) -> Option<Uuid> {
    payload
        .get("node_run_id")
        .and_then(Value::as_str)
        .and_then(|value| Uuid::parse_str(value).ok())
        .or(event.node_run_id)
}

fn with_runtime_visible_internal_llm_tool_events(
    mut payload: Value,
    runtime_events: &[Value],
) -> Value {
    if runtime_events.is_empty() {
        return payload;
    }
    let Some(object) = payload.as_object_mut() else {
        return payload;
    };

    if !object.contains_key("visible_internal_llm_tool_events") {
        object.insert(
            "visible_internal_llm_tool_events".to_string(),
            Value::Array(runtime_events.to_vec()),
        );
    }

    with_synthetic_visible_internal_llm_tool_rounds(payload, runtime_events)
}

fn with_synthetic_visible_internal_llm_tool_rounds(
    mut payload: Value,
    runtime_events: &[Value],
) -> Value {
    let Some(object) = payload.as_object_mut() else {
        return payload;
    };
    if object.contains_key("llm_rounds") {
        return payload;
    }

    let rounds = synthetic_visible_internal_llm_tool_rounds(runtime_events);
    if rounds.is_empty() {
        return payload;
    }

    object.insert("llm_rounds".to_string(), Value::Array(rounds));
    payload
}

fn synthetic_visible_internal_llm_tool_rounds(runtime_events: &[Value]) -> Vec<Value> {
    let mut calls_by_id = std::collections::BTreeMap::<String, Value>::new();
    let mut results_by_id = std::collections::BTreeMap::<String, Value>::new();

    for event in runtime_events {
        let Some(event_object) = event.as_object() else {
            continue;
        };
        let Some(tool_call_id) =
            record_string_field(event_object, &["tool_call_id", "id", "call_id"])
        else {
            continue;
        };
        let tool_name = record_string_field(event_object, &["tool_name", "name"])
            .unwrap_or_else(|| "Tool".to_string());

        calls_by_id.entry(tool_call_id.clone()).or_insert_with(|| {
            let mut call = Map::new();
            call.insert("id".to_string(), json!(tool_call_id));
            call.insert("name".to_string(), json!(tool_name));
            call.insert("type".to_string(), json!("visible_internal_llm_tool"));
            if let Some(arguments) = event_object.get("arguments") {
                call.insert("arguments".to_string(), arguments.clone());
            }
            Value::Object(call)
        });

        match event_object
            .get("event_type")
            .and_then(Value::as_str)
            .unwrap_or_default()
        {
            "visible_internal_llm_tool_completed" => {
                results_by_id.insert(
                    tool_call_id.clone(),
                    synthetic_visible_internal_llm_tool_result(
                        &tool_call_id,
                        &tool_name,
                        event_object,
                        false,
                    ),
                );
            }
            "visible_internal_llm_tool_failed" => {
                results_by_id.insert(
                    tool_call_id.clone(),
                    synthetic_visible_internal_llm_tool_result(
                        &tool_call_id,
                        &tool_name,
                        event_object,
                        true,
                    ),
                );
            }
            _ => {}
        }
    }

    if calls_by_id.is_empty() {
        return Vec::new();
    }

    let mut rounds = vec![json!({
        "round_index": 0,
        "assistant": {
            "role": "assistant",
            "tool_calls": calls_by_id.into_values().collect::<Vec<_>>()
        }
    })];

    if !results_by_id.is_empty() {
        rounds.push(json!({
            "round_index": 1,
            "tool_results": results_by_id.into_values().collect::<Vec<_>>()
        }));
    }

    rounds
}

fn synthetic_visible_internal_llm_tool_result(
    tool_call_id: &str,
    tool_name: &str,
    event_object: &Map<String, Value>,
    is_error: bool,
) -> Value {
    let mut result = Map::new();
    result.insert("role".to_string(), json!("tool"));
    result.insert("tool_call_id".to_string(), json!(tool_call_id));
    result.insert("name".to_string(), json!(tool_name));
    result.insert("is_error".to_string(), json!(is_error));

    if is_error {
        if let Some(error_payload) = event_object.get("error_payload") {
            result.insert("error".to_string(), error_payload.clone());
            result.insert("content".to_string(), error_payload.clone());
        }
    } else if let Some(content) = record_value_field(
        event_object,
        &[
            "content",
            "output",
            "output_payload",
            "result",
            "response_payload",
        ],
    ) {
        result.insert("content".to_string(), content);
    } else {
        result.insert("content".to_string(), json!(null));
    }

    Value::Object(result)
}

fn with_inline_visible_internal_llm_tool_trace_index_with_main_output(
    mut payload: Value,
    main_resume_output_fallback: Option<&Value>,
) -> Value {
    let Some(object) = payload.as_object() else {
        return payload;
    };
    if object.contains_key("visible_internal_llm_tool_trace") {
        return payload;
    }
    let runtime_events = object
        .get("visible_internal_llm_tool_events")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    payload = with_synthetic_visible_internal_llm_tool_rounds(payload, &runtime_events);

    let traces = collect_visible_internal_llm_tool_route_traces_with_main_output(
        &payload,
        main_resume_output_fallback,
    );
    if traces.is_empty() {
        return payload;
    }

    let summaries = traces
        .into_iter()
        .map(|trace| trace.inline_summary_payload())
        .collect::<Vec<_>>();
    if let Some(object) = payload.as_object_mut() {
        object.insert(
            "visible_internal_llm_tool_trace".to_string(),
            Value::Array(summaries),
        );
    }

    payload
}
