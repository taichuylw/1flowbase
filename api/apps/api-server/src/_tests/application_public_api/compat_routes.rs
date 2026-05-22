use crate::{
    _tests::support::{
        login_and_capture_cookie, test_api_state_with_database_url, test_app, test_config,
    },
    app_state::ApiState,
    host_infrastructure::LocalRuntimeEventStream,
    routes::application_public_api::tool_callback_ids::encode_openai_callback_tool_call_id,
};
use async_trait::async_trait;
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
    Router,
};
use control_plane::{
    application_public_api::compat::openai::run_id_from_response_id,
    orchestration_runtime::debug_stream_events,
    ports::{
        CreateCallbackTaskInput, CreateNodeRunInput, OrchestrationRuntimeRepository,
        RuntimeEventCloseReason, RuntimeEventEnvelope, RuntimeEventPayload, RuntimeEventStream,
        RuntimeEventStreamPolicy, RuntimeEventSubscription, RuntimeEventTrimPolicy,
        UpdateFlowRunInput,
    },
};
use serde_json::{json, Value};
use std::sync::Arc;
use time::OffsetDateTime;
use tokio::time::{timeout, Duration};
use tower::ServiceExt;

struct DropTerminalRuntimeEventStream {
    inner: LocalRuntimeEventStream,
}

struct NeverCloseDropTerminalRuntimeEventStream {
    inner: DropTerminalRuntimeEventStream,
}

impl DropTerminalRuntimeEventStream {
    fn new() -> Self {
        Self {
            inner: LocalRuntimeEventStream::new(),
        }
    }
}

impl NeverCloseDropTerminalRuntimeEventStream {
    fn new() -> Self {
        Self {
            inner: DropTerminalRuntimeEventStream::new(),
        }
    }
}

#[async_trait]
impl RuntimeEventStream for DropTerminalRuntimeEventStream {
    async fn open_run(
        &self,
        run_id: uuid::Uuid,
        policy: RuntimeEventStreamPolicy,
    ) -> anyhow::Result<()> {
        self.inner.open_run(run_id, policy).await
    }

    async fn append(
        &self,
        run_id: uuid::Uuid,
        event: RuntimeEventPayload,
    ) -> anyhow::Result<RuntimeEventEnvelope> {
        if is_terminal_runtime_event(&event.event_type) {
            return Ok(RuntimeEventEnvelope::new(run_id, 0, event));
        }
        self.inner.append(run_id, event).await
    }

    async fn subscribe(
        &self,
        run_id: uuid::Uuid,
        from_sequence: Option<i64>,
    ) -> anyhow::Result<RuntimeEventSubscription> {
        self.inner.subscribe(run_id, from_sequence).await
    }

    async fn replay(
        &self,
        run_id: uuid::Uuid,
        from_sequence: Option<i64>,
        limit: usize,
    ) -> anyhow::Result<Vec<RuntimeEventEnvelope>> {
        self.inner.replay(run_id, from_sequence, limit).await
    }

    async fn close_run(
        &self,
        run_id: uuid::Uuid,
        reason: RuntimeEventCloseReason,
    ) -> anyhow::Result<()> {
        self.inner.close_run(run_id, reason).await
    }

    async fn trim(&self, run_id: uuid::Uuid, policy: RuntimeEventTrimPolicy) -> anyhow::Result<()> {
        self.inner.trim(run_id, policy).await
    }
}

#[async_trait]
impl RuntimeEventStream for NeverCloseDropTerminalRuntimeEventStream {
    async fn open_run(
        &self,
        run_id: uuid::Uuid,
        policy: RuntimeEventStreamPolicy,
    ) -> anyhow::Result<()> {
        self.inner.open_run(run_id, policy).await
    }

    async fn append(
        &self,
        run_id: uuid::Uuid,
        event: RuntimeEventPayload,
    ) -> anyhow::Result<RuntimeEventEnvelope> {
        self.inner.append(run_id, event).await
    }

    async fn subscribe(
        &self,
        run_id: uuid::Uuid,
        from_sequence: Option<i64>,
    ) -> anyhow::Result<RuntimeEventSubscription> {
        self.inner.subscribe(run_id, from_sequence).await
    }

    async fn replay(
        &self,
        run_id: uuid::Uuid,
        from_sequence: Option<i64>,
        limit: usize,
    ) -> anyhow::Result<Vec<RuntimeEventEnvelope>> {
        self.inner.replay(run_id, from_sequence, limit).await
    }

    async fn close_run(
        &self,
        _run_id: uuid::Uuid,
        _reason: RuntimeEventCloseReason,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    async fn trim(&self, run_id: uuid::Uuid, policy: RuntimeEventTrimPolicy) -> anyhow::Result<()> {
        self.inner.trim(run_id, policy).await
    }
}

fn is_terminal_runtime_event(event_type: &str) -> bool {
    matches!(
        event_type,
        "flow_finished" | "flow_failed" | "flow_cancelled" | "waiting_human" | "waiting_callback"
    )
}

async fn response_json(response: axum::response::Response) -> Value {
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&body).unwrap()
}

async fn create_application(app: &Router, cookie: &str, csrf: &str, name: &str) -> String {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/console/applications")
                .header("cookie", cookie)
                .header("x-csrf-token", csrf)
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "application_type": "agent_flow",
                        "name": name,
                        "description": "compatible public route test",
                        "icon": null,
                        "icon_type": null,
                        "icon_background": null
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
    response_json(response).await["data"]["id"]
        .as_str()
        .unwrap()
        .to_string()
}

async fn create_application_key(
    app: &Router,
    cookie: &str,
    csrf: &str,
    application_id: &str,
) -> String {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/console/applications/{application_id}/api-keys"
                ))
                .header("cookie", cookie)
                .header("x-csrf-token", csrf)
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "name": "Compatible route key",
                        "expires_at": null
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
    response_json(response).await["data"]["token"]
        .as_str()
        .unwrap()
        .to_string()
}

async fn publish_application(app: &Router, cookie: &str, csrf: &str, application_id: &str) {
    let state = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!(
                    "/api/console/applications/{application_id}/orchestration"
                ))
                .header("cookie", cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(state.status(), StatusCode::OK);
    let mut document = response_json(state).await["data"]["draft"]["document"].clone();
    let start_node = document["graph"]["nodes"]
        .as_array_mut()
        .expect("nodes array")
        .iter_mut()
        .find(|node| node["type"] == "start")
        .expect("default draft should include a start node");
    start_node["config"]["model_list"] = json!([
        { "id": "qwen3.6-35b-a3b", "name": "Qwen 3.6 35B" },
        "deepseek-v4-flash"
    ]);

    let save = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!(
                    "/api/console/applications/{application_id}/orchestration/draft"
                ))
                .header("cookie", cookie)
                .header("x-csrf-token", csrf)
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "document": document,
                        "change_kind": "logical",
                        "summary": "Configure compatible model list"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(save.status(), StatusCode::OK);

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/console/applications/{application_id}/api-publications"
                ))
                .header("cookie", cookie)
                .header("x-csrf-token", csrf)
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "mapping": {
                            "input": {
                                "query_target": "node-start.query",
                                "model_target": null,
                                "inputs_target": null,
                                "history_target": "node-start.history",
                                "attachments_target": null
                            },
                            "output": {
                                "answer_selector": "answer",
                                "usage_selector": null,
                                "files_selector": null,
                                "error_selector": null
                            }
                        },
                        "api_enabled": true
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
}

async fn setup_published_app(app: &Router, name: &str) -> String {
    let (cookie, csrf) = login_and_capture_cookie(app, "root", "change-me").await;
    let application_id = create_application(app, &cookie, &csrf, name).await;
    let token = create_application_key(app, &cookie, &csrf, &application_id).await;
    publish_application(app, &cookie, &csrf, &application_id).await;
    token
}

async fn setup_unpublished_app_key(app: &Router, name: &str) -> String {
    let (cookie, csrf) = login_and_capture_cookie(app, "root", "change-me").await;
    let application_id = create_application(app, &cookie, &csrf, name).await;
    create_application_key(app, &cookie, &csrf, &application_id).await
}

async fn test_app_with_state() -> (Router, std::sync::Arc<crate::app_state::ApiState>) {
    let (state, _) = test_api_state_with_database_url().await;
    let config = test_config();
    let app = crate::app_with_state_and_config(state.clone(), &config);
    (app, state)
}

async fn test_app_with_runtime_event_stream(
    runtime_event_stream: Arc<dyn RuntimeEventStream>,
) -> (Router, Arc<ApiState>) {
    let (base_state, _) = test_api_state_with_database_url().await;
    let config = test_config();
    let state = Arc::new(ApiState {
        store: base_state.store.clone(),
        infrastructure: base_state.infrastructure.clone(),
        file_storage_registry: base_state.file_storage_registry.clone(),
        runtime_engine: base_state.runtime_engine.clone(),
        provider_runtime: base_state.provider_runtime.clone(),
        process_started_at: base_state.process_started_at,
        api_runtime_profile: base_state.api_runtime_profile.clone(),
        plugin_runner_system: base_state.plugin_runner_system.clone(),
        official_plugin_source: base_state.official_plugin_source.clone(),
        provider_install_root: base_state.provider_install_root.clone(),
        provider_secret_master_key: base_state.provider_secret_master_key.clone(),
        host_extension_dropin_root: base_state.host_extension_dropin_root.clone(),
        allow_unverified_filesystem_dropins: base_state.allow_unverified_filesystem_dropins,
        allow_uploaded_host_extensions: base_state.allow_uploaded_host_extensions,
        session_store: base_state.session_store.clone(),
        runtime_event_stream,
        api_docs: base_state.api_docs.clone(),
        cookie_name: base_state.cookie_name.clone(),
        session_ttl_days: base_state.session_ttl_days,
        bootstrap_workspace_name: base_state.bootstrap_workspace_name.clone(),
    });
    let app = crate::app_with_state_and_config(state.clone(), &config);
    (app, state)
}

async fn seed_llm_callback_for_response_run(
    state: &crate::app_state::ApiState,
    flow_run_id: uuid::Uuid,
) -> domain::CallbackTaskRecord {
    state
        .store
        .update_flow_run(&UpdateFlowRunInput {
            flow_run_id,
            status: domain::FlowRunStatus::WaitingCallback,
            output_payload: json!({
                "tool_calls": [
                    {
                        "id": "call_inventory",
                        "name": "lookup_inventory",
                        "arguments": { "sku": "sku_123" }
                    }
                ]
            }),
            error_payload: None,
            finished_at: None,
        })
        .await
        .unwrap();
    let node_run = state
        .store
        .create_node_run(&CreateNodeRunInput {
            flow_run_id,
            node_id: "node-llm".to_string(),
            node_type: "llm".to_string(),
            node_alias: "LLM".to_string(),
            status: domain::NodeRunStatus::WaitingCallback,
            input_payload: json!({}),
            debug_payload: json!({ "llm_rounds": [] }),
            started_at: OffsetDateTime::now_utc(),
        })
        .await
        .unwrap();

    state
        .store
        .create_callback_task(&CreateCallbackTaskInput {
            flow_run_id,
            node_run_id: node_run.id,
            callback_kind: "llm_tool_calls".to_string(),
            request_payload: json!({
                "tool_calls": [
                    {
                        "id": "call_inventory",
                        "name": "lookup_inventory",
                        "arguments": { "sku": "sku_123" }
                    }
                ],
                "finish_reason": "tool_call"
            }),
            external_ref_payload: None,
        })
        .await
        .unwrap()
}

async fn post_json(
    app: &Router,
    uri: &str,
    token_header: (&str, String),
    body: Value,
) -> axum::response::Response {
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(uri)
                .header(token_header.0, token_header.1)
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap()
}

async fn get_models(app: &Router, uri: &str, token: &str) -> axum::response::Response {
    app.clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(uri)
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
}

fn openai_body(stream: bool) -> Value {
    json!({
        "model": "provider/custom-model:latest",
        "stream": stream,
        "messages": [
            {"role": "system", "content": "Use the support playbook."},
            {"role": "user", "content": "Earlier question"},
            {"role": "assistant", "content": "Earlier answer"},
            {"role": "user", "content": "Final question"}
        ],
        "metadata": {
            "trace_id": "trace-openai"
        }
    })
}

fn responses_body(stream: bool) -> Value {
    json!({
        "model": "provider/custom-model:latest",
        "stream": stream,
        "input": "Final question",
        "user": "external-user-123",
        "metadata": {
            "trace_id": "trace-responses"
        }
    })
}

fn anthropic_body(stream: bool) -> Value {
    json!({
        "model": "anthropic/custom-model:latest",
        "max_tokens": 512,
        "stream": stream,
        "system": "Use the support playbook.",
        "messages": [
            {"role": "user", "content": "Earlier question"},
            {"role": "assistant", "content": "Earlier answer"},
            {"role": "user", "content": [{"type": "text", "text": "Final question"}]}
        ],
        "metadata": {
            "expand_id": "external-user-123"
        }
    })
}

#[tokio::test]
async fn compatible_routes_require_application_api_key() {
    let app = test_app().await;

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/chat/completions")
                .header("content-type", "application/json")
                .body(Body::from(openai_body(false).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let payload = response_json(response).await;
    assert_eq!(payload["error"]["code"], json!("not_authenticated"));

    let responses = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/responses")
                .header("content-type", "application/json")
                .body(Body::from(responses_body(false).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(responses.status(), StatusCode::UNAUTHORIZED);
    let payload = response_json(responses).await;
    assert_eq!(payload["error"]["code"], json!("not_authenticated"));
}

#[tokio::test]
async fn compatible_routes_return_not_published_for_unpublished_key_application() {
    let app = test_app().await;
    let token = setup_unpublished_app_key(&app, "Compatible Unpublished Route App").await;

    let response = post_json(
        &app,
        "/v1/messages",
        ("x-api-key", token),
        anthropic_body(false),
    )
    .await;

    assert_eq!(response.status(), StatusCode::CONFLICT);
    let payload = response_json(response).await;
    assert_eq!(payload["error"]["type"], json!("application_not_published"));
}

#[tokio::test]
async fn openai_chat_completions_accepts_bearer_and_preserves_model() {
    let app = test_app().await;
    let token = setup_published_app(&app, "OpenAI Compatible Route App").await;

    let response = post_json(
        &app,
        "/v1/chat/completions",
        ("authorization", format!("Bearer {token}")),
        openai_body(false),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let payload = response_json(response).await;
    assert_eq!(payload["object"], json!("chat.completion"));
    assert_eq!(payload["model"], json!("provider/custom-model:latest"));
    assert_eq!(payload["choices"][0]["message"]["role"], json!("assistant"));
}

#[tokio::test]
async fn openai_chat_completions_accepts_root_endpoint_for_plain_base_url_clients() {
    let app = test_app().await;
    let token = setup_published_app(&app, "OpenAI Plain Base URL Compatible Route App").await;

    let response = post_json(
        &app,
        "/chat/completions",
        ("authorization", format!("Bearer {token}")),
        openai_body(false),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let payload = response_json(response).await;
    assert_eq!(payload["object"], json!("chat.completion"));
    assert_eq!(payload["model"], json!("provider/custom-model:latest"));
}

#[tokio::test]
async fn openai_chat_completions_accepts_prefixed_openai_alias() {
    let app = test_app().await;
    let token = setup_published_app(&app, "OpenAI Prefixed Alias Compatible Route App").await;

    let response = post_json(
        &app,
        "/openai/v1/chat/completions",
        ("authorization", format!("Bearer {token}")),
        openai_body(false),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let payload = response_json(response).await;
    assert_eq!(payload["object"], json!("chat.completion"));
    assert_eq!(payload["model"], json!("provider/custom-model:latest"));
}

#[tokio::test]
async fn openai_responses_accepts_blocking_text_input() {
    let app = test_app().await;
    let token = setup_published_app(&app, "OpenAI Responses Blocking App").await;

    let response = post_json(
        &app,
        "/v1/responses",
        ("authorization", format!("Bearer {token}")),
        responses_body(false),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let payload = response_json(response).await;
    assert_eq!(payload["object"], json!("response"));
    assert_eq!(payload["status"], json!("completed"));
    assert_eq!(payload["model"], json!("provider/custom-model:latest"));
    assert!(payload["id"].as_str().unwrap().starts_with("resp_"));
    assert_eq!(payload["output"][0]["type"], json!("message"));
    assert_eq!(
        payload["output"][0]["content"][0]["type"],
        json!("output_text")
    );
    assert!(payload["output_text"].is_string());
}

#[tokio::test]
async fn openai_responses_continues_from_previous_response_id() {
    let app = test_app().await;
    let token = setup_published_app(&app, "OpenAI Responses Continuation App").await;

    let first = post_json(
        &app,
        "/v1/responses",
        ("authorization", format!("Bearer {token}")),
        responses_body(false),
    )
    .await;
    assert_eq!(first.status(), StatusCode::OK);
    let first_payload = response_json(first).await;
    let previous_response_id = first_payload["id"].as_str().unwrap().to_string();

    let mut next_body = responses_body(false);
    next_body["input"] = json!("Follow up");
    next_body["previous_response_id"] = json!(previous_response_id);
    let next = post_json(
        &app,
        "/v1/responses",
        ("authorization", format!("Bearer {token}")),
        next_body,
    )
    .await;

    assert_eq!(next.status(), StatusCode::OK);
    let next_payload = response_json(next).await;
    assert_eq!(next_payload["previous_response_id"], first_payload["id"]);
    assert_ne!(next_payload["id"], first_payload["id"]);
}

#[tokio::test]
async fn openai_responses_rejects_invalid_previous_response_id() {
    let app = test_app().await;
    let token = setup_published_app(&app, "OpenAI Responses Invalid Previous App").await;
    let mut body = responses_body(false);
    body["previous_response_id"] = json!("resp_not-a-native-run-id");

    let response = post_json(
        &app,
        "/v1/responses",
        ("authorization", format!("Bearer {token}")),
        body,
    )
    .await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let payload = response_json(response).await;
    assert_eq!(payload["error"]["param"], json!("previous_response_id"));
    assert_eq!(payload["error"]["code"], json!("invalid_request"));
}

#[tokio::test]
async fn openai_responses_rejects_previous_response_from_another_api_key() {
    let app = test_app().await;
    let first_token = setup_published_app(&app, "OpenAI Responses Previous Owner App").await;
    let second_token = setup_published_app(&app, "OpenAI Responses Previous Consumer App").await;

    let first = post_json(
        &app,
        "/v1/responses",
        ("authorization", format!("Bearer {first_token}")),
        responses_body(false),
    )
    .await;
    assert_eq!(first.status(), StatusCode::OK);
    let first_payload = response_json(first).await;

    let mut body = responses_body(false);
    body["previous_response_id"] = first_payload["id"].clone();
    let response = post_json(
        &app,
        "/v1/responses",
        ("authorization", format!("Bearer {second_token}")),
        body,
    )
    .await;

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let payload = response_json(response).await;
    assert_eq!(payload["error"]["code"], json!("application_run_forbidden"));
}

#[tokio::test]
async fn openai_responses_rejects_function_call_output_when_previous_response_mismatches_callback_run(
) {
    let (app, state) = test_app_with_state().await;
    let token = setup_published_app(&app, "OpenAI Responses Callback Binding App").await;

    let first = post_json(
        &app,
        "/v1/responses",
        ("authorization", format!("Bearer {token}")),
        responses_body(false),
    )
    .await;
    assert_eq!(first.status(), StatusCode::OK);
    let first_payload = response_json(first).await;
    let first_run_id = run_id_from_response_id(first_payload["id"].as_str().unwrap()).unwrap();
    let callback_task = seed_llm_callback_for_response_run(state.as_ref(), first_run_id).await;

    let mut second_body = responses_body(false);
    second_body["input"] = json!("Different response");
    let second = post_json(
        &app,
        "/v1/responses",
        ("authorization", format!("Bearer {token}")),
        second_body,
    )
    .await;
    assert_eq!(second.status(), StatusCode::OK);
    let second_payload = response_json(second).await;

    let body = json!({
        "model": "provider/custom-model:latest",
        "previous_response_id": second_payload["id"],
        "input": [
            {
                "type": "function_call_output",
                "call_id": encode_openai_callback_tool_call_id(callback_task.id, "call_inventory"),
                "output": { "stock": 7 }
            }
        ]
    });
    let response = post_json(
        &app,
        "/v1/responses",
        ("authorization", format!("Bearer {token}")),
        body,
    )
    .await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let payload = response_json(response).await;
    assert_eq!(payload["error"]["param"], json!("previous_response_id"));
    assert_eq!(payload["error"]["code"], json!("invalid_request"));
    let stored_task = state
        .store
        .get_callback_task(callback_task.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored_task.status, domain::CallbackTaskStatus::Pending);
}

#[tokio::test]
async fn openai_models_lists_start_node_configured_models() {
    let app = test_app().await;
    let token = setup_published_app(&app, "OpenAI Compatible Models App").await;

    let response = get_models(&app, "/v1/models", &token).await;

    assert_eq!(response.status(), StatusCode::OK);
    let payload = response_json(response).await;
    assert_eq!(payload["object"], json!("list"));
    assert_eq!(payload["data"][0]["id"], json!("qwen3.6-35b-a3b"));
    assert_eq!(payload["data"][0]["name"], json!("Qwen 3.6 35B"));
    assert_eq!(payload["data"][0]["object"], json!("model"));
    assert_eq!(payload["data"][1]["id"], json!("deepseek-v4-flash"));
}

#[tokio::test]
async fn openai_models_accepts_full_chat_completions_base_url_alias() {
    let app = test_app().await;
    let token = setup_published_app(&app, "OpenAI Full Endpoint Base URL App").await;

    let response = get_models(&app, "/v1/chat/completions/models", &token).await;

    assert_eq!(response.status(), StatusCode::OK);
    let payload = response_json(response).await;
    assert_eq!(payload["data"][0]["id"], json!("qwen3.6-35b-a3b"));
}

#[tokio::test]
async fn openai_chat_completions_accepts_tools_for_agent_framework_compatibility() {
    let app = test_app().await;
    let token = setup_published_app(&app, "OpenAI Tool Compatible Route App").await;
    let mut body = openai_body(false);
    body["tools"] = json!([{"type": "function", "function": {"name": "lookup"}}]);
    body["tool_choice"] = json!("auto");

    let response = post_json(
        &app,
        "/v1/chat/completions",
        ("authorization", format!("Bearer {token}")),
        body,
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let payload = response_json(response).await;
    assert_eq!(payload["object"], json!("chat.completion"));
    assert_eq!(payload["model"], json!("provider/custom-model:latest"));
}

#[tokio::test]
async fn anthropic_messages_accepts_x_api_key_and_preserves_model() {
    let app = test_app().await;
    let token = setup_published_app(&app, "Anthropic Compatible Route App").await;

    let response = post_json(
        &app,
        "/v1/messages",
        ("x-api-key", token),
        anthropic_body(false),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let payload = response_json(response).await;
    assert_eq!(payload["type"], json!("message"));
    assert_eq!(payload["model"], json!("anthropic/custom-model:latest"));
    assert_eq!(payload["content"][0]["type"], json!("text"));
}

#[tokio::test]
async fn anthropic_messages_accepts_agent_tool_definitions() {
    let app = test_app().await;
    let token = setup_published_app(&app, "Anthropic Tool Compatible Route App").await;
    let mut body = anthropic_body(false);
    body["tools"] = json!([
        {
            "name": "lookup_order",
            "description": "Find an order",
            "input_schema": {
                "type": "object",
                "properties": {
                    "order_id": {"type": "string"}
                }
            }
        }
    ]);
    body["tool_choice"] = json!({"type": "auto"});

    let response = post_json(&app, "/v1/messages", ("x-api-key", token), body).await;

    assert_eq!(response.status(), StatusCode::OK);
    let payload = response_json(response).await;
    assert_eq!(payload["type"], json!("message"));
    assert_eq!(payload["model"], json!("anthropic/custom-model:latest"));
}

#[tokio::test]
async fn compatible_streaming_routes_return_protocol_sse() {
    let app = test_app().await;
    let token = setup_published_app(&app, "Compatible Streaming Route App").await;

    let openai = post_json(
        &app,
        "/v1/chat/completions",
        ("authorization", format!("Bearer {token}")),
        openai_body(true),
    )
    .await;
    assert_eq!(openai.status(), StatusCode::OK);
    assert_eq!(
        openai.headers().get("content-type").unwrap(),
        "text/event-stream"
    );
    let openai_body = timeout(
        Duration::from_secs(5),
        to_bytes(openai.into_body(), usize::MAX),
    )
    .await
    .expect("OpenAI compatible SSE should finish")
    .unwrap();
    let openai_body = String::from_utf8(openai_body.to_vec()).unwrap();
    assert!(openai_body.contains("[DONE]"), "{openai_body}");
    assert!(
        !openai_body.contains("event: workflow.event"),
        "{openai_body}"
    );

    let responses = post_json(
        &app,
        "/v1/responses",
        ("authorization", format!("Bearer {token}")),
        responses_body(true),
    )
    .await;
    assert_eq!(responses.status(), StatusCode::OK);
    assert_eq!(
        responses.headers().get("content-type").unwrap(),
        "text/event-stream"
    );
    let responses_body = timeout(
        Duration::from_secs(5),
        to_bytes(responses.into_body(), usize::MAX),
    )
    .await
    .expect("OpenAI Responses SSE should finish")
    .unwrap();
    let responses_body = String::from_utf8(responses_body.to_vec()).unwrap();
    assert!(
        responses_body.contains("event: response.created"),
        "{responses_body}"
    );
    assert!(
        responses_body.contains("event: response.completed")
            || responses_body.contains("event: response.failed"),
        "{responses_body}"
    );
    assert!(
        !responses_body.contains("event: workflow.event"),
        "{responses_body}"
    );

    let anthropic = post_json(
        &app,
        "/v1/messages",
        ("x-api-key", token),
        anthropic_body(true),
    )
    .await;
    assert_eq!(anthropic.status(), StatusCode::OK);
    assert_eq!(
        anthropic.headers().get("content-type").unwrap(),
        "text/event-stream"
    );
    let anthropic_body = timeout(
        Duration::from_secs(5),
        to_bytes(anthropic.into_body(), usize::MAX),
    )
    .await
    .expect("Anthropic compatible SSE should finish")
    .unwrap();
    let anthropic_body = String::from_utf8(anthropic_body.to_vec()).unwrap();
    assert!(
        anthropic_body.contains("event: message_stop") || anthropic_body.contains("event: error"),
        "{anthropic_body}"
    );
    assert!(
        !anthropic_body.contains("event: workflow.event"),
        "{anthropic_body}"
    );
}

#[tokio::test]
async fn openai_chat_streaming_tool_resume_returns_done_on_current_connection() {
    let (app, state) = test_app_with_state().await;
    let token = setup_published_app(&app, "OpenAI Streaming Tool Resume App").await;

    let created = post_json(
        &app,
        "/v1/chat/completions",
        ("authorization", format!("Bearer {token}")),
        openai_body(false),
    )
    .await;
    assert_eq!(created.status(), StatusCode::OK);
    let created_payload = response_json(created).await;
    let run_id = created_payload["id"]
        .as_str()
        .and_then(|id| id.strip_prefix("chatcmpl-"))
        .and_then(|id| uuid::Uuid::parse_str(id).ok())
        .expect("chat completion id should include run id");
    let callback_task = seed_llm_callback_for_response_run(state.as_ref(), run_id).await;
    let tool_call_id = encode_openai_callback_tool_call_id(callback_task.id, "call_inventory");
    state
        .runtime_event_stream
        .open_run(run_id, RuntimeEventStreamPolicy::debug_default())
        .await
        .unwrap();
    state
        .runtime_event_stream
        .append(run_id, debug_stream_events::flow_started(run_id))
        .await
        .unwrap();
    state
        .runtime_event_stream
        .append(
            run_id,
            debug_stream_events::waiting_callback_with_task(
                run_id,
                callback_task.node_run_id,
                "node-llm",
                &callback_task,
            ),
        )
        .await
        .unwrap();

    let response = post_json(
        &app,
        "/v1/chat/completions",
        ("authorization", format!("Bearer {token}")),
        json!({
            "model": "provider/custom-model:latest",
            "stream": true,
            "messages": [
                {
                    "role": "assistant",
                    "content": null,
                    "tool_calls": [{
                        "id": tool_call_id,
                        "type": "function",
                        "function": {
                            "name": "lookup_inventory",
                            "arguments": "{\"sku\":\"sku_123\"}"
                        }
                    }]
                },
                {
                    "role": "tool",
                    "tool_call_id": tool_call_id,
                    "content": "{\"stock\":7}"
                }
            ]
        }),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "text/event-stream"
    );
    let body = timeout(
        Duration::from_secs(5),
        to_bytes(response.into_body(), usize::MAX),
    )
    .await
    .expect("OpenAI streaming tool resume SSE should finish on current connection")
    .unwrap();
    let body = String::from_utf8(body.to_vec()).unwrap();

    assert!(body.contains("[DONE]"), "{body}");
    assert!(
        !body.contains("\"finish_reason\":\"tool_calls\""),
        "resume stream replayed a stale waiting_callback instead of the resumed turn: {body}"
    );
    assert!(
        !body.contains("lookup_inventory"),
        "resume stream sent the stale tool call again: {body}"
    );
    assert!(
        body.contains("runtime_error") || body.contains("\"finish_reason\":\"stop\""),
        "{body}"
    );
}

#[tokio::test]
async fn compatible_streaming_routes_emit_terminal_fallback_after_runtime_stream_closes() {
    let (app, _) =
        test_app_with_runtime_event_stream(Arc::new(DropTerminalRuntimeEventStream::new())).await;
    let token = setup_published_app(&app, "Compatible Terminal Fallback Route App").await;

    let openai = post_json(
        &app,
        "/v1/chat/completions",
        ("authorization", format!("Bearer {token}")),
        openai_body(true),
    )
    .await;
    assert_eq!(openai.status(), StatusCode::OK);

    let openai_body = timeout(
        Duration::from_secs(5),
        to_bytes(openai.into_body(), usize::MAX),
    )
    .await
    .expect("OpenAI compatible SSE should finish from durable terminal fallback")
    .unwrap();
    let openai_body = String::from_utf8(openai_body.to_vec()).unwrap();

    assert!(openai_body.contains("[DONE]"), "{openai_body}");
}

#[tokio::test]
async fn compatible_streaming_routes_emit_terminal_fallback_when_runtime_stream_stays_open() {
    let (app, _) = test_app_with_runtime_event_stream(Arc::new(
        NeverCloseDropTerminalRuntimeEventStream::new(),
    ))
    .await;
    let token = setup_published_app(&app, "Compatible Stuck Runtime Stream Route App").await;

    let openai = post_json(
        &app,
        "/v1/chat/completions",
        ("authorization", format!("Bearer {token}")),
        openai_body(true),
    )
    .await;
    assert_eq!(openai.status(), StatusCode::OK);

    let openai_body = timeout(
        Duration::from_secs(5),
        to_bytes(openai.into_body(), usize::MAX),
    )
    .await
    .expect("OpenAI compatible SSE should finish from durable terminal polling")
    .unwrap();
    let openai_body = String::from_utf8(openai_body.to_vec()).unwrap();

    assert!(openai_body.contains("[DONE]"), "{openai_body}");
}
