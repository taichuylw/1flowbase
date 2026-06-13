use crate::{
    _tests::support::{login_and_capture_cookie, test_api_state_with_database_url, test_config},
    app_state::ApiState,
    routes::application_public_api::tool_callback_ids::encode_anthropic_callback_tool_use_id,
};
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
    Router,
};
use control_plane::ports::{
    CreateCallbackTaskInput, CreateNodeRunInput, OrchestrationRuntimeRepository, UpdateFlowRunInput,
};
use serde_json::{json, Value};
use std::sync::Arc;
use time::OffsetDateTime;
use tower::ServiceExt;
use uuid::Uuid;

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
                        "description": "anthropic compatible route test",
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
                        "name": "Anthropic compatible route key",
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
        {
            "id": "qwen3.6-35b-a3b",
            "name": "Qwen 3.6 35B",
            "context_window": 128000,
            "max_output_tokens": 32000,
            "auto_compact_token_limit": 110000,
            "capabilities": {
                "reasoning": true,
                "tool_call": true,
                "multimodal": false,
                "structured_output": true
            },
            "reasoning": {
                "default_effort": "medium",
                "supported_efforts": ["low", "medium", "high"]
            }
        },
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
                        "summary": "Configure anthropic compatible model list"
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

async fn test_app_with_state() -> (Router, Arc<ApiState>) {
    let (state, _) = test_api_state_with_database_url().await;
    let config = test_config();
    let app = crate::app_with_state_and_config(state.clone(), &config);
    (app, state)
}

async fn flow_run_count(state: &ApiState) -> i64 {
    sqlx::query_scalar("select count(*) from flow_runs")
        .fetch_one(state.store.pool())
        .await
        .unwrap()
}

async fn seed_pending_anthropic_llm_callback(
    state: &ApiState,
    flow_run_id: Uuid,
    tool_use_id: &str,
) -> domain::CallbackTaskRecord {
    state
        .store
        .update_flow_run(&UpdateFlowRunInput {
            flow_run_id,
            status: domain::FlowRunStatus::WaitingCallback,
            output_payload: json!({
                "tool_calls": [
                    {
                        "id": tool_use_id,
                        "name": "Grep",
                        "arguments": { "pattern": "image-1.png" }
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
                        "id": tool_use_id,
                        "name": "Grep",
                        "arguments": { "pattern": "image-1.png" }
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
    post_json_with_headers(app, uri, token_header, Vec::new(), body).await
}

async fn post_json_with_headers(
    app: &Router,
    uri: &str,
    token_header: (&str, String),
    extra_headers: Vec<(&str, String)>,
    body: Value,
) -> axum::response::Response {
    let mut request = Request::builder()
        .method("POST")
        .uri(uri)
        .header(token_header.0, token_header.1)
        .header("content-type", "application/json");
    for (name, value) in extra_headers {
        request = request.header(name, value);
    }

    app.clone()
        .oneshot(request.body(Body::from(body.to_string())).unwrap())
        .await
        .unwrap()
}

fn anthropic_body() -> Value {
    json!({
        "model": "anthropic/custom-model:latest",
        "max_tokens": 64,
        "messages": [
            {"role": "user", "content": "Earlier question"},
            {"role": "assistant", "content": "Earlier answer"},
            {"role": "user", "content": "Final question"}
        ],
        "metadata": {
            "expand_id": "external-user-123"
        }
    })
}

fn anthropic_multimodal_body() -> Value {
    let mut body = anthropic_body();
    body["messages"] = json!([
        {
            "role": "user",
            "content": [
                {"type": "text", "text": "Describe this image"},
                {
                    "type": "image",
                    "source": {
                        "type": "base64",
                        "media_type": "image/png",
                        "data": "iVBORw0KGgo="
                    }
                }
            ]
        }
    ]);
    body
}

#[tokio::test]
async fn anthropic_messages_accepts_x_api_key_and_preserves_model() {
    let (app, _) = test_app_with_state().await;
    let token = setup_published_app(&app, "Anthropic Compatible Route App").await;

    let response = post_json(&app, "/v1/messages", ("x-api-key", token), anthropic_body()).await;

    assert_eq!(response.status(), StatusCode::OK);
    let payload = response_json(response).await;
    assert_eq!(payload["type"], json!("message"));
    assert_eq!(payload["model"], json!("anthropic/custom-model:latest"));
    assert_eq!(payload["content"][0]["type"], json!("text"));
}

#[tokio::test]
async fn anthropic_messages_accepts_last_user_multimodal_content() {
    let (app, _) = test_app_with_state().await;
    let token = setup_published_app(&app, "Anthropic Multimodal Compatible Route App").await;

    let response = post_json(
        &app,
        "/v1/messages",
        ("x-api-key", token),
        anthropic_multimodal_body(),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let payload = response_json(response).await;
    assert_eq!(payload["type"], json!("message"));
    assert_ne!(
        payload["error"]["message"],
        json!("messages is not supported by this endpoint")
    );
}

#[tokio::test]
async fn anthropic_messages_accepts_agent_tool_definitions() {
    let (app, _) = test_app_with_state().await;
    let token = setup_published_app(&app, "Anthropic Tool Compatible Route App").await;
    let mut body = anthropic_body();
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
async fn anthropic_messages_rehydrates_session_history_from_durable_turns() {
    let (app, state) = test_app_with_state().await;
    let token = setup_published_app(&app, "Anthropic Session History Route App").await;
    let session_id = "claude-code-session-1".to_string();
    let metadata = json!({
        "user_id": "{\"account_uuid\":\"account-1\",\"device_id\":\"device-1\"}"
    });

    let first = post_json_with_headers(
        &app,
        "/v1/messages",
        ("x-api-key", token.clone()),
        vec![("x-claude-code-session-id", session_id.clone())],
        json!({
            "model": "anthropic/custom-model:latest",
            "max_tokens": 64,
            "messages": [
                {"role": "user", "content": "Describe uploads/agent-flow-preview-debug.png"}
            ],
            "metadata": metadata
        }),
    )
    .await;
    assert_eq!(first.status(), StatusCode::OK);

    let second = post_json_with_headers(
        &app,
        "/v1/messages",
        ("x-api-key", token.clone()),
        vec![("x-claude-code-session-id", session_id.clone())],
        json!({
            "model": "anthropic/custom-model:latest",
            "max_tokens": 64,
            "messages": [
                {"role": "user", "content": "Find the corresponding code"}
            ],
            "metadata": metadata
        }),
    )
    .await;
    assert_eq!(second.status(), StatusCode::OK);

    let runs = sqlx::query_scalar::<_, Value>(
        r#"
        select input_payload
        from flow_runs
        where compatibility_mode = 'anthropic-messages-v1'
        order by created_at asc, id asc
        "#,
    )
    .fetch_all(state.store.pool())
    .await
    .unwrap();
    assert_eq!(runs.len(), 2);
    let history = runs[1]["node-start"]["history"]
        .as_array()
        .expect("second run should receive rehydrated history");
    assert_eq!(history.len(), 2);
    assert_eq!(
        history[0],
        json!({
            "role": "user",
            "content": "Describe uploads/agent-flow-preview-debug.png"
        })
    );
    assert_eq!(history[1]["role"], json!("assistant"));
    assert!(
        history[1]["content"]
            .as_str()
            .is_some_and(|value| !value.is_empty()),
        "{history:?}"
    );

    let third = post_json_with_headers(
        &app,
        "/v1/messages",
        ("x-api-key", token.clone()),
        vec![("x-claude-code-session-id", session_id)],
        json!({
            "model": "anthropic/custom-model:latest",
            "max_tokens": 64,
            "messages": [
                {"role": "user", "content": "Keep going"}
            ],
            "metadata": metadata
        }),
    )
    .await;
    assert_eq!(third.status(), StatusCode::OK);

    let runs = sqlx::query_scalar::<_, Value>(
        r#"
        select input_payload
        from flow_runs
        where compatibility_mode = 'anthropic-messages-v1'
        order by created_at asc, id asc
        "#,
    )
    .fetch_all(state.store.pool())
    .await
    .unwrap();
    assert_eq!(runs.len(), 3);
    let third_history = runs[2]["node-start"]["history"]
        .as_array()
        .expect("third run should receive unique prior turns");
    assert_eq!(
        third_history
            .iter()
            .map(|message| message["role"].as_str().unwrap_or_default())
            .collect::<Vec<_>>(),
        vec!["user", "assistant", "user", "assistant"]
    );
    assert_eq!(
        third_history[0]["content"],
        json!("Describe uploads/agent-flow-preview-debug.png")
    );
    assert_eq!(
        third_history[2]["content"],
        json!("Find the corresponding code")
    );
}

#[tokio::test]
async fn anthropic_count_tokens_returns_usage_without_creating_run() {
    let (app, state) = test_app_with_state().await;
    let token = setup_published_app(&app, "Anthropic Count Tokens Compatible Route App").await;
    let before = flow_run_count(state.as_ref()).await;

    let response = post_json(
        &app,
        "/v1/messages/count_tokens",
        ("x-api-key", token),
        json!({
            "model": "anthropic/custom-model:latest",
            "messages": [
                {"role": "user", "content": "Count this prompt"}
            ],
            "tools": [{
                "name": "lookup_order",
                "description": "Find an order by id",
                "input_schema": {
                    "type": "object",
                    "properties": {
                        "order_id": {"type": "string"}
                    }
                }
            }]
        }),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let payload = response_json(response).await;
    assert!(
        payload["input_tokens"].as_u64().unwrap_or_default() > 0,
        "{payload}"
    );
    assert_eq!(flow_run_count(state.as_ref()).await, before);
}

#[tokio::test]
async fn anthropic_messages_routes_hidden_system_reminder_tool_result_to_callback_resume() {
    let (app, state) = test_app_with_state().await;
    let token = setup_published_app(&app, "Anthropic Hidden Reminder Tool Result App").await;
    let before = flow_run_count(state.as_ref()).await;
    let callback_task_id = uuid::Uuid::from_u128(0xffffffffffffffffffffffffffffffff);
    let tool_use_id = encode_anthropic_callback_tool_use_id(callback_task_id, "toolu_read");

    let response = post_json(
        &app,
        "/v1/messages",
        ("x-api-key", token),
        json!({
            "model": "anthropic/custom-model:latest",
            "max_tokens": 64,
            "messages": [
                {
                    "role": "assistant",
                    "content": [{
                        "type": "tool_use",
                        "id": tool_use_id,
                        "name": "Read",
                        "input": {}
                    }]
                },
                {
                    "role": "user",
                    "content": [
                        {
                            "type": "tool_result",
                            "tool_use_id": tool_use_id,
                            "content": "Found 3 files"
                        },
                        {
                            "type": "text",
                            "text": "<system-reminder>Claude Code internal reminder</system-reminder>"
                        }
                    ]
                }
            ]
        }),
    )
    .await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let payload = response_json(response).await;
    assert_eq!(payload["error"]["type"], json!("callback_task"));
    assert_eq!(flow_run_count(state.as_ref()).await, before);
}

#[tokio::test]
async fn anthropic_messages_routes_latest_message_only_tool_result_to_callback_resume() {
    let (app, state) = test_app_with_state().await;
    let token = setup_published_app(&app, "Anthropic Latest Tool Result App").await;
    let before = flow_run_count(state.as_ref()).await;
    let callback_task_id = uuid::Uuid::from_u128(0xffffffffffffffffffffffffffffffff);
    let tool_use_id = encode_anthropic_callback_tool_use_id(callback_task_id, "toolu_read");

    let response = post_json(
        &app,
        "/v1/messages",
        ("x-api-key", token),
        json!({
            "model": "anthropic/custom-model:latest",
            "max_tokens": 64,
            "messages": [
                {
                    "role": "user",
                    "content": [{
                        "type": "tool_result",
                        "tool_use_id": tool_use_id,
                        "content": "Found 3 files"
                    }]
                }
            ]
        }),
    )
    .await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let payload = response_json(response).await;
    assert_eq!(payload["error"]["type"], json!("callback_task"));
    assert_eq!(flow_run_count(state.as_ref()).await, before);
}

#[tokio::test]
async fn anthropic_messages_rejects_orphan_tool_result_without_creating_run() {
    let (app, state) = test_app_with_state().await;
    let token = setup_published_app(&app, "Anthropic Orphan Tool Result App").await;
    let before = flow_run_count(state.as_ref()).await;

    let response = post_json(
        &app,
        "/v1/messages",
        ("x-api-key", token),
        json!({
            "model": "anthropic/custom-model:latest",
            "max_tokens": 64,
            "messages": [
                {
                    "role": "assistant",
                    "content": [{
                        "type": "tool_use",
                        "id": "toolu_read",
                        "name": "Read",
                        "input": {}
                    }]
                },
                {
                    "role": "user",
                    "content": [{
                        "type": "tool_result",
                        "tool_use_id": "toolu_read",
                        "content": "plain Anthropic tool result"
                    }]
                }
            ]
        }),
    )
    .await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let payload = response_json(response).await;
    assert_eq!(payload["error"]["type"], json!("tool_result_only_orphan"));
    assert_eq!(flow_run_count(state.as_ref()).await, before);
}

#[tokio::test]
async fn anthropic_messages_matches_plain_tool_result_to_same_conversation_pending_callback() {
    let (app, state) = test_app_with_state().await;
    let token = setup_published_app(&app, "Anthropic Plain Tool Result Resume App").await;
    let before = flow_run_count(state.as_ref()).await;
    let session_id = "claude-code-session-plain-tool".to_string();
    let metadata = json!({
        "expand_id": "claude-code-user"
    });

    let first = post_json_with_headers(
        &app,
        "/v1/messages",
        ("x-api-key", token.clone()),
        vec![("x-claude-code-session-id", session_id.clone())],
        json!({
            "model": "anthropic/custom-model:latest",
            "max_tokens": 64,
            "messages": [
                {"role": "user", "content": "uploads/image-1.png 这部分代码在哪里？"}
            ],
            "metadata": metadata
        }),
    )
    .await;
    assert_eq!(first.status(), StatusCode::OK);
    let first_payload = response_json(first).await;
    let run_id = Uuid::parse_str(
        first_payload["id"]
            .as_str()
            .expect("anthropic response id")
            .strip_prefix("msg_")
            .expect("anthropic response id should include msg_ prefix"),
    )
    .unwrap();
    seed_pending_anthropic_llm_callback(state.as_ref(), run_id, "toolu_read").await;

    let response = post_json_with_headers(
        &app,
        "/v1/messages",
        ("x-api-key", token),
        vec![("x-claude-code-session-id", session_id)],
        json!({
            "model": "anthropic/custom-model:latest",
            "max_tokens": 64,
            "messages": [
                {
                    "role": "assistant",
                    "content": [{
                        "type": "tool_use",
                        "id": "toolu_read",
                        "name": "Grep",
                        "input": {"pattern": "image-1.png"}
                    }]
                },
                {
                    "role": "user",
                    "content": [
                        {
                            "type": "tool_result",
                            "tool_use_id": "toolu_read",
                            "content": "No files found"
                        },
                        {
                            "type": "text",
                            "text": "<system-reminder>Claude Code internal reminder</system-reminder>"
                        }
                    ]
                }
            ],
            "metadata": metadata
        }),
    )
    .await;

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(flow_run_count(state.as_ref()).await, before + 1);
    let event_types = sqlx::query_scalar::<_, String>(
        "select event_type from flow_run_events where flow_run_id = $1 order by sequence asc",
    )
    .bind(run_id)
    .fetch_all(state.store.pool())
    .await
    .unwrap();
    assert!(
        event_types.contains(&"public_run_resume_requested".to_string()),
        "plain tool_result should route to callback resume, not a new run: {event_types:?}"
    );
}

#[tokio::test]
async fn anthropic_probe_message_uses_published_native_run() {
    let (app, state) = test_app_with_state().await;
    let token = setup_published_app(&app, "Anthropic Probe Compatible Route App").await;
    let before = flow_run_count(state.as_ref()).await;

    let response = post_json(
        &app,
        "/v1/messages",
        ("x-api-key", token),
        json!({
            "model": "anthropic/custom-model:latest",
            "max_tokens": 1,
            "messages": [
                {"role": "user", "content": "test"}
            ],
            "metadata": {
                "user_id": "{\"device_id\":\"probe-device\",\"account_uuid\":\"\",\"session_id\":\"probe-session\"}"
            }
        }),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let payload = response_json(response).await;
    assert_eq!(payload["type"], json!("message"));
    assert_eq!(flow_run_count(state.as_ref()).await, before + 1);
}

#[tokio::test]
async fn anthropic_probe_message_requires_active_publication() {
    let (app, state) = test_app_with_state().await;
    let token =
        setup_unpublished_app_key(&app, "Anthropic Unpublished Probe Compatible Route App").await;

    let response = post_json(
        &app,
        "/v1/messages",
        ("x-api-key", token),
        json!({
            "model": "anthropic/custom-model:latest",
            "max_tokens": 1,
            "messages": [
                {"role": "user", "content": "test"}
            ]
        }),
    )
    .await;

    assert_eq!(response.status(), StatusCode::CONFLICT);
    let payload = response_json(response).await;
    assert_eq!(payload["error"]["type"], json!("application_not_published"));
    assert_eq!(flow_run_count(state.as_ref()).await, 0);
}

#[tokio::test]
async fn anthropic_structured_title_message_requires_active_publication() {
    let (app, state) = test_app_with_state().await;
    let token = setup_unpublished_app_key(
        &app,
        "Anthropic Unpublished Structured Compatible Route App",
    )
    .await;

    let response = post_json(
        &app,
        "/v1/messages",
        ("x-api-key", token),
        json!({
            "model": "anthropic/custom-model:latest",
            "max_tokens": 64,
            "stream": true,
            "messages": [
                {"role": "user", "content": "帮我找找这个代码位置"}
            ],
            "output_config": {
                "format": {
                    "type": "json_schema",
                    "schema": {
                        "type": "object",
                        "properties": {
                            "title": { "type": "string" }
                        },
                        "required": ["title"],
                        "additionalProperties": false
                    }
                }
            }
        }),
    )
    .await;

    assert_eq!(response.status(), StatusCode::CONFLICT);
    let payload = response_json(response).await;
    assert_eq!(payload["error"]["type"], json!("application_not_published"));
    assert_eq!(flow_run_count(state.as_ref()).await, 0);
}
