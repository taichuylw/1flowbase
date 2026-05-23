use std::{collections::HashMap, future::Future, pin::Pin, sync::Arc};

use axum::{
    body::Body,
    http::{header::CONTENT_TYPE, Response, StatusCode},
};
use control_plane::{
    errors::ControlPlaneError,
    orchestration_runtime::debug_artifacts::{
        build_runtime_debug_artifact_object_path, build_runtime_debug_artifact_preview,
        inline_budget_for_kind, RUNTIME_DEBUG_ARTIFACT_CONTENT_TYPE_JSON,
        RUNTIME_DEBUG_ARTIFACT_RETENTION_ACTIVE,
    },
    ports::{
        CreateRuntimeDebugArtifactInput, FileManagementRepository, GetRuntimeDebugArtifactInput,
        OrchestrationRuntimeRepository, UpdateFlowRunPayloadsInput, UpdateNodeRunPayloadsInput,
        UpdateRunEventPayloadInput,
    },
};
use serde_json::{json, Map, Value};
use storage_durable::MainDurableStore;
use uuid::Uuid;

use crate::{app_state::ApiState, error_response::ApiError};

type RuntimeDebugArtifactOffloadFuture<'a> =
    Pin<Box<dyn Future<Output = Result<(Value, bool), ApiError>> + Send + 'a>>;

const APPLICATION_INPUT_QUERY_KEYS: &[&str] = &["query", "question", "prompt", "message", "input"];

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
    llm_tool_callback_raw_payloads: HashMap<String, Value>,
}

#[derive(Clone)]
struct LlmToolCallbackArtifact {
    id: String,
    name: String,
    request_payload: Value,
    callback_payload: Option<Value>,
    request_round_index: Option<i64>,
    result_round_index: Option<i64>,
    call_input_tokens: Option<u64>,
    call_cached_input_tokens: Option<u64>,
    call_output_tokens: Option<u64>,
    result_input_tokens: Option<u64>,
    result_context_input_tokens: Option<u64>,
    result_context_cached_input_tokens: Option<u64>,
    token_count_method: Option<String>,
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
            "call_input_tokens": self.call_input_tokens,
            "call_cached_input_tokens": self.call_cached_input_tokens,
            "call_output_tokens": self.call_output_tokens,
            "result_input_tokens": self.result_input_tokens,
            "result_context_input_tokens": self.result_context_input_tokens,
            "result_context_cached_input_tokens": self.result_context_cached_input_tokens,
            "token_count_method": self.token_count_method.clone(),
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
            "call_input_tokens": self.call_input_tokens,
            "call_cached_input_tokens": self.call_cached_input_tokens,
            "call_output_tokens": self.call_output_tokens,
            "result_input_tokens": self.result_input_tokens,
            "result_context_input_tokens": self.result_context_input_tokens,
            "result_context_cached_input_tokens": self.result_context_cached_input_tokens,
            "token_count_method": self.token_count_method.clone(),
        })
    }
}

fn is_llm_rounds_field_path(field_path: &[String]) -> bool {
    field_path.len() == 1 && field_path[0] == "llm_rounds"
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

fn record_u64_field(record: &Map<String, Value>, keys: &[&str]) -> Option<u64> {
    keys.iter()
        .find_map(|key| record.get(*key).and_then(Value::as_u64))
}

fn usage_input_tokens(record: Option<&Map<String, Value>>) -> Option<u64> {
    record.and_then(|record| record_u64_field(record, &["input_tokens"]))
}

fn usage_cached_input_tokens(record: Option<&Map<String, Value>>) -> Option<u64> {
    record.and_then(|record| {
        record_u64_field(record, &["input_cache_hit_tokens", "cache_read_tokens"])
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

fn collect_llm_tool_callback_raw_payloads(
    callback_tasks: &[domain::CallbackTaskRecord],
) -> HashMap<String, Value> {
    let mut payload_by_tool_call_id = HashMap::new();

    for task in callback_tasks {
        if task.callback_kind != "llm_tool_calls" {
            continue;
        }

        let Some(response_payload) = task.response_payload.as_ref() else {
            continue;
        };

        for callback_payload in read_callback_response_tool_payloads(response_payload) {
            let Some(callback_payload_object) = callback_payload.as_object() else {
                continue;
            };
            let Some(tool_call_id) =
                record_string_field(callback_payload_object, &["tool_call_id", "id", "call_id"])
            else {
                continue;
            };

            payload_by_tool_call_id.insert(tool_call_id, callback_payload);
        }
    }

    payload_by_tool_call_id
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
    raw_payloads: &HashMap<String, Value>,
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
        let current_usage = round.get("usage").and_then(Value::as_object);
        let next_usage = rounds
            .get(fallback_round_index + 1)
            .and_then(Value::as_object)
            .and_then(|round| round.get("usage"))
            .and_then(Value::as_object);

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
                    callback_payload: raw_payloads.get(&id).cloned(),
                    id,
                    name,
                    call_input_tokens: record_u64_field(tool_call_object, &["call_input_tokens"])
                        .or_else(|| usage_input_tokens(current_usage)),
                    call_cached_input_tokens: record_u64_field(
                        tool_call_object,
                        &["call_cached_input_tokens"],
                    )
                    .or_else(|| usage_cached_input_tokens(current_usage)),
                    call_output_tokens: record_u64_field(tool_call_object, &["call_output_tokens"]),
                    result_input_tokens: record_u64_field(
                        tool_call_object,
                        &["result_input_tokens"],
                    ),
                    result_context_input_tokens: None,
                    result_context_cached_input_tokens: None,
                    token_count_method: record_string_field(
                        tool_call_object,
                        &["token_count_method"],
                    ),
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
                    callback_payload: raw_payloads
                        .get(&id)
                        .cloned()
                        .or_else(|| Some(tool_result.clone())),
                    id,
                    name,
                    call_input_tokens: record_u64_field(tool_result_object, &["call_input_tokens"]),
                    call_cached_input_tokens: record_u64_field(
                        tool_result_object,
                        &["call_cached_input_tokens"],
                    ),
                    call_output_tokens: record_u64_field(
                        tool_result_object,
                        &["call_output_tokens"],
                    ),
                    result_input_tokens: record_u64_field(
                        tool_result_object,
                        &["result_input_tokens"],
                    ),
                    result_context_input_tokens: record_u64_field(
                        tool_result_object,
                        &["result_context_input_tokens"],
                    )
                    .or_else(|| usage_input_tokens(next_usage)),
                    result_context_cached_input_tokens: record_u64_field(
                        tool_result_object,
                        &["result_context_cached_input_tokens"],
                    )
                    .or_else(|| usage_cached_input_tokens(next_usage)),
                    token_count_method: record_string_field(
                        tool_result_object,
                        &["token_count_method"],
                    ),
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
    if next.call_input_tokens.is_some() {
        current.call_input_tokens = next.call_input_tokens;
    }
    if next.call_cached_input_tokens.is_some() {
        current.call_cached_input_tokens = next.call_cached_input_tokens;
    }
    if next.call_output_tokens.is_some() {
        current.call_output_tokens = next.call_output_tokens;
    }
    if next.result_input_tokens.is_some() {
        current.result_input_tokens = next.result_input_tokens;
    }
    if next.result_context_input_tokens.is_some() {
        current.result_context_input_tokens = next.result_context_input_tokens;
    }
    if next.result_context_cached_input_tokens.is_some() {
        current.result_context_cached_input_tokens = next.result_context_cached_input_tokens;
    }
    if next.token_count_method.is_some() {
        current.token_count_method = next.token_count_method;
    }
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
            llm_tool_callback_raw_payloads: HashMap::new(),
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
            collect_llm_tool_callbacks(llm_rounds, &self.llm_tool_callback_raw_payloads);
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
                    let payload = if changed && is_llm_rounds_field_path(&field_path) {
                        self.with_llm_tool_callback_index(scope, payload, &full_value)
                            .await?
                    } else {
                        payload
                    };
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
    writer.llm_tool_callback_raw_payloads =
        collect_llm_tool_callback_raw_payloads(&detail.callback_tasks);
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
        .offload_value(
            &flow_scope,
            "flow_output_payload",
            detail.flow_run.output_payload.clone(),
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
            .offload_payload_fields(
                &node_scope,
                "node_debug_payload",
                node_run.debug_payload.clone(),
                Vec::new(),
            )
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

pub(super) fn application_run_query(payload: &Value) -> Option<String> {
    if let Some(query) = string_field(payload, "query") {
        return Some(query);
    }

    if let Some(input_text) = string_field(payload, "input_text") {
        return Some(input_text);
    }

    for selector in [
        "node-start.query",
        "node-start.question",
        "node-start.prompt",
        "node-start.message",
        "node-start.input",
        "start.query",
        "start.question",
        "start.prompt",
        "start.message",
        "start.input",
    ] {
        if let Some(value) = string_field(payload, selector) {
            return Some(value);
        }
    }

    let object = payload.as_object()?;
    for key in ["node-start", "start"] {
        if let Some(value) = object.get(key).and_then(immediate_input_text) {
            return Some(value);
        }
    }

    for value in object.values() {
        if let Some(value) = immediate_input_text(value) {
            return Some(value);
        }
    }

    None
}

pub(super) fn application_run_model(payload: &Value) -> Option<String> {
    if let Some(model) = string_field(payload, "model") {
        return Some(model);
    }

    for selector in ["node-start.model", "start.model"] {
        if let Some(value) = string_field(payload, selector) {
            return Some(value);
        }
    }

    let object = payload.as_object()?;
    for key in ["node-start", "start"] {
        if let Some(value) = object.get(key).and_then(immediate_model_value) {
            return Some(value);
        }
    }

    for value in object.values() {
        if let Some(value) = immediate_model_value(value) {
            return Some(value);
        }
    }

    None
}

pub(super) fn application_run_answer(payload: &Value) -> Option<String> {
    for key in ["answer", "text", "content", "message"] {
        if let Some(value) = string_field(payload, key) {
            return Some(value);
        }
        if let Some(value) = payload
            .get(key)
            .and_then(runtime_debug_artifact_preview_text)
        {
            return Some(value);
        }
    }

    if is_runtime_debug_artifact_payload(payload) {
        return runtime_debug_artifact_preview_text(payload);
    }

    let object = payload.as_object()?;
    if let Some(error) = object.get("error").and_then(|value| value.get("message")) {
        if let Some(error) = error
            .as_str()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            return Some(error.to_string());
        }
    }

    None
}

fn immediate_input_text(value: &Value) -> Option<String> {
    let object = value.as_object()?;
    for key in APPLICATION_INPUT_QUERY_KEYS {
        if let Some(value) = object
            .get(*key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            return Some(value.to_string());
        }
    }
    None
}

fn immediate_model_value(value: &Value) -> Option<String> {
    let object = value.as_object()?;
    object
        .get("model")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn is_runtime_debug_artifact_payload(value: &Value) -> bool {
    value
        .get("__runtime_debug_artifact")
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn runtime_debug_artifact_preview_text(value: &Value) -> Option<String> {
    if !is_runtime_debug_artifact_payload(value) {
        return None;
    }

    let preview = string_field(value, "preview")?;
    if let Ok(decoded) = serde_json::from_str::<Value>(&preview) {
        if let Some(text) = decoded
            .as_str()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            return Some(text.to_string());
        }
    }

    Some(preview.trim_start_matches('"').to_string())
}

fn with_debug_artifact_field_path(mut payload: Value, field_path: &[String]) -> Value {
    let Some(object) = payload.as_object_mut() else {
        return payload;
    };
    if field_path.is_empty() {
        return payload;
    }

    object.insert(
        "artifact_scope".to_string(),
        Value::String("field".to_string()),
    );
    object.insert(
        "field_path".to_string(),
        Value::Array(field_path.iter().cloned().map(Value::String).collect()),
    );
    payload
}

fn string_field(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn with_application_run_input_summary(
    mut payload: Value,
    query: Option<&str>,
    model: Option<&str>,
) -> Value {
    let Some(object) = payload.as_object_mut() else {
        return payload;
    };
    if let Some(query) = query {
        object.insert("query".to_string(), Value::String(query.to_string()));
    }
    if let Some(model) = model {
        object.insert("model".to_string(), Value::String(model.to_string()));
    }
    payload
}

fn should_keep_runtime_payload_field_inline(field_path: &[String]) -> bool {
    field_path
        .iter()
        .any(|key| matches!(key.as_str(), "query" | "model" | "files" | "sys" | "env"))
}

fn is_safe_to_persist_debug_artifact_previews(status: domain::FlowRunStatus) -> bool {
    matches!(
        status,
        domain::FlowRunStatus::Succeeded
            | domain::FlowRunStatus::Failed
            | domain::FlowRunStatus::Cancelled
            | domain::FlowRunStatus::WaitingCallback
            | domain::FlowRunStatus::WaitingHuman
    )
}

pub async fn load_runtime_debug_artifact_response(
    state: Arc<ApiState>,
    workspace_id: Uuid,
    application_id: Uuid,
    artifact_id: Uuid,
) -> Result<Response<Body>, ApiError> {
    let artifact =
        <MainDurableStore as OrchestrationRuntimeRepository>::get_runtime_debug_artifact(
            &state.store,
            &GetRuntimeDebugArtifactInput {
                workspace_id,
                application_id,
                artifact_id,
            },
        )
        .await?
        .ok_or(ControlPlaneError::NotFound("runtime_debug_artifact"))?;
    let storage = <MainDurableStore as FileManagementRepository>::get_file_storage(
        &state.store,
        artifact.storage_id,
    )
    .await?
    .ok_or(ControlPlaneError::NotFound("file_storage"))?;
    if !storage.enabled {
        return Err(ControlPlaneError::Conflict("file_storage_disabled").into());
    }
    let driver = state
        .file_storage_registry
        .get(&storage.driver_type)
        .ok_or(ControlPlaneError::Conflict("storage_driver_not_registered"))?;
    let object = driver
        .open_read(storage_object::OpenReadInput {
            config_json: &storage.config_json,
            object_path: &artifact.storage_ref,
        })
        .await?;
    let content_type = object.content_type.unwrap_or(artifact.content_type);

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, content_type)
        .body(Body::from(object.bytes))
        .unwrap())
}

pub(super) async fn load_runtime_debug_artifact_json_value(
    state: Arc<ApiState>,
    workspace_id: Uuid,
    application_id: Uuid,
    artifact_id: Uuid,
) -> Result<Value, ApiError> {
    let artifact =
        <MainDurableStore as OrchestrationRuntimeRepository>::get_runtime_debug_artifact(
            &state.store,
            &GetRuntimeDebugArtifactInput {
                workspace_id,
                application_id,
                artifact_id,
            },
        )
        .await?
        .ok_or(ControlPlaneError::NotFound("runtime_debug_artifact"))?;
    let storage = <MainDurableStore as FileManagementRepository>::get_file_storage(
        &state.store,
        artifact.storage_id,
    )
    .await?
    .ok_or(ControlPlaneError::NotFound("file_storage"))?;
    if !storage.enabled {
        return Err(ControlPlaneError::Conflict("file_storage_disabled").into());
    }
    let driver = state
        .file_storage_registry
        .get(&storage.driver_type)
        .ok_or(ControlPlaneError::Conflict("storage_driver_not_registered"))?;
    let object = driver
        .open_read(storage_object::OpenReadInput {
            config_json: &storage.config_json,
            object_path: &artifact.storage_ref,
        })
        .await?;

    serde_json::from_slice(&object.bytes)
        .map_err(|_| ControlPlaneError::Conflict("runtime_debug_artifact_not_json").into())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn application_query_prefers_start_query_over_tool_schema_payload() {
        let payload = json!({
            "node-start": {
                "query": "ping",
                "model": "gpt-test",
                "compatibility": {
                    "tools": [
                        {
                            "function": {
                                "description": "path to the file to read.",
                                "parameters": {
                                    "properties": {
                                        "file_path": {
                                            "description": "path to the file to read."
                                        }
                                    }
                                }
                            }
                        }
                    ]
                }
            }
        });

        assert_eq!(application_run_query(&payload), Some("ping".into()));
        assert_eq!(application_run_model(&payload), Some("gpt-test".into()));
    }

    #[test]
    fn application_query_reads_persisted_artifact_preview_summary() {
        let payload = json!({
            "__runtime_debug_artifact": true,
            "artifact_ref": Uuid::now_v7().to_string(),
            "preview": "{\"node-start\":{\"compatibility\":{\"tools\":[]}}}",
            "query": "总结退款政策",
            "model": "deepseek-chat"
        });

        assert_eq!(application_run_query(&payload), Some("总结退款政策".into()));
        assert_eq!(
            application_run_model(&payload),
            Some("deepseek-chat".into())
        );
    }

    #[test]
    fn flow_input_artifact_preview_keeps_application_query_and_model() {
        let preview = json!({
            "__runtime_debug_artifact": true,
            "artifact_ref": Uuid::now_v7().to_string(),
            "preview": "{\"node-start\":{\"compatibility\":{\"tools\":[]}}}"
        });

        let preview = with_application_run_input_summary(preview, Some("ping"), Some("gpt-test"));

        assert_eq!(preview["query"], json!("ping"));
        assert_eq!(preview["model"], json!("gpt-test"));
    }

    #[test]
    fn runtime_payload_field_artifact_selection_keeps_truth_fields_inline() {
        assert!(should_keep_runtime_payload_field_inline(&["query".into()]));
        assert!(should_keep_runtime_payload_field_inline(&[
            "node-start".into(),
            "model".into()
        ]));
        assert!(should_keep_runtime_payload_field_inline(&[
            "input".into(),
            "sys".into(),
            "workflow_run_id".into()
        ]));
        assert!(should_keep_runtime_payload_field_inline(&[
            "env".into(),
            "ApiBaseUrl".into()
        ]));
        assert!(should_keep_runtime_payload_field_inline(&["files".into()]));
        assert!(!should_keep_runtime_payload_field_inline(&[
            "history".into()
        ]));
        assert!(!should_keep_runtime_payload_field_inline(&[
            "compatibility".into(),
            "tools".into()
        ]));
    }

    #[test]
    fn runtime_payload_field_artifact_wrapper_keeps_original_field_path() {
        let payload = json!({
            "__runtime_debug_artifact": true,
            "artifact_ref": Uuid::now_v7().to_string(),
            "preview": "[{\"role\":\"user\"}]"
        });

        let payload = with_debug_artifact_field_path(payload, &["history".into()]);

        assert_eq!(payload["artifact_scope"], json!("field"));
        assert_eq!(payload["field_path"], json!(["history"]));
    }

    #[test]
    fn paused_callback_statuses_can_persist_debug_artifact_previews() {
        assert!(is_safe_to_persist_debug_artifact_previews(
            domain::FlowRunStatus::WaitingCallback
        ));
        assert!(is_safe_to_persist_debug_artifact_previews(
            domain::FlowRunStatus::WaitingHuman
        ));
        assert!(!is_safe_to_persist_debug_artifact_previews(
            domain::FlowRunStatus::Running
        ));
    }

    #[test]
    fn llm_tool_callback_execution_status_requires_explicit_execution_fact() {
        assert_eq!(
            execution_status_from_callback_payload(Some(&json!({
                "tool_call_id": "call_weather",
                "content": "{\"temperature\":21}"
            }))),
            "unknown"
        );
        assert_eq!(
            execution_status_from_callback_payload(Some(&json!({
                "tool_call_id": "call_read",
                "execution": {
                    "status": "failed"
                }
            }))),
            "failed"
        );
        assert_eq!(
            execution_status_from_callback_payload(Some(&json!({
                "tool_call_id": "call_glob",
                "exit_code": 0,
                "content": []
            }))),
            "succeeded"
        );
        assert_eq!(
            execution_status_from_callback_payload(Some(&json!({
                "tool_call_id": "call_http",
                "http_status": 500
            }))),
            "failed"
        );
    }

    #[test]
    fn application_answer_reads_preferred_output_fields() {
        let payload = json!({
            "answer": "退款政策摘要"
        });

        assert_eq!(
            application_run_answer(&payload),
            Some("退款政策摘要".into())
        );
    }

    #[test]
    fn application_answer_reads_field_level_artifact_preview() {
        let payload = json!({
            "answer": {
                "__runtime_debug_artifact": true,
                "artifact_ref": Uuid::now_v7().to_string(),
                "field_path": ["answer"],
                "preview": "退款政策摘要..."
            },
            "sys": {
                "workflow_run_id": "run-1"
            },
            "env": {}
        });

        assert_eq!(
            application_run_answer(&payload),
            Some("退款政策摘要...".into())
        );
    }
}
