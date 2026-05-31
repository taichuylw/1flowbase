use super::*;

fn build_answer_only_document(flow_id: &str, answer_text: &str) -> Value {
    json!({
        "schemaVersion": "1flowbase.flow/v2",
        "meta": { "flowId": flow_id, "name": "Answer Only", "description": "", "tags": [] },
        "graph": {
            "nodes": [
                {
                    "id": "node-start",
                    "type": "start",
                    "alias": "Start",
                    "description": "",
                    "containerId": null,
                    "position": { "x": 0, "y": 0 },
                    "configVersion": 1,
                    "config": {},
                    "bindings": {},
                    "outputs": []
                },
                {
                    "id": "node-answer",
                    "type": "answer",
                    "alias": "Answer",
                    "description": "",
                    "containerId": null,
                    "position": { "x": 240, "y": 0 },
                    "configVersion": 1,
                    "config": {},
                    "bindings": {
                        "answer_template": { "kind": "templated_text", "value": answer_text }
                    },
                    "outputs": [{ "key": "answer", "title": "对话输出", "valueType": "string" }]
                }
            ],
            "edges": [
                { "id": "edge-start-answer", "source": "node-start", "target": "node-answer", "sourceHandle": null, "targetHandle": null, "containerId": null, "points": [] }
            ]
        },
        "editor": { "viewport": { "x": 0, "y": 0, "zoom": 1 }, "annotations": [], "activeContainerPath": [] }
    })
}

async fn seed_answer_only_application(
    app: &axum::Router,
    cookie: &str,
    csrf: &str,
    answer_text: &str,
) -> String {
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
                        "name": "Answer Only",
                        "description": "runtime",
                        "icon": "RobotOutlined",
                        "icon_type": "iconfont",
                        "icon_background": "#E6F7F2"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let payload: Value = serde_json::from_slice(&body).unwrap();
    let application_id = payload["data"]["id"].as_str().unwrap().to_string();

    let state = app
        .clone()
        .oneshot(
            Request::builder()
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
    let state_body = to_bytes(state.into_body(), usize::MAX).await.unwrap();
    let state_payload: Value = serde_json::from_slice(&state_body).unwrap();
    let flow_id = state_payload["data"]["draft"]["document"]["meta"]["flowId"]
        .as_str()
        .unwrap();

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
                        "document": build_answer_only_document(flow_id, answer_text),
                        "change_kind": "logical",
                        "summary": "seed answer-only flow"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(save.status(), StatusCode::OK);
    application_id
}

#[tokio::test]
async fn application_runtime_routes_start_debug_run_persists_gateway_billing_audit() {
    let (app, database_url) = test_app_with_database_url().await;
    let (cookie, csrf) = login_and_capture_cookie(&app, "root", "change-me").await;
    let provider_instance_id = create_ready_provider_instance(&app, &cookie, &csrf).await;
    let application_id =
        seed_agent_flow_application(&app, &cookie, &csrf, &provider_instance_id).await;

    let start = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/console/applications/{application_id}/orchestration/debug-runs"
                ))
                .header("cookie", &cookie)
                .header("x-csrf-token", &csrf)
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "input_payload": {
                            "node-start": { "query": "请总结退款政策" }
                        }
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let start_status = start.status();
    let start_body = to_bytes(start.into_body(), usize::MAX).await.unwrap();
    assert_eq!(
        start_status,
        StatusCode::CREATED,
        "{}",
        String::from_utf8_lossy(&start_body)
    );
    let payload: Value = serde_json::from_slice(&start_body).unwrap();
    let run_id = Uuid::parse_str(payload["data"]["flow_run"]["id"].as_str().unwrap()).unwrap();
    let event_types = payload["data"]["events"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|event| event["event_type"].as_str())
        .collect::<Vec<_>>();
    assert!(event_types.contains(&"gateway_billing_session_reserved"));

    let pool = sqlx::PgPool::connect(&database_url).await.unwrap();
    let (billing_count,): (i64,) =
        sqlx::query_as("select count(*) from billing_sessions where flow_run_id = $1")
            .bind(run_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    let (cost_count,): (i64,) =
        sqlx::query_as("select count(*) from runtime_cost_ledger where flow_run_id = $1")
            .bind(run_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    let (credit_count,): (i64,) =
        sqlx::query_as("select count(*) from runtime_credit_ledger where flow_run_id = $1")
            .bind(run_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    let (audit_count,): (i64,) =
        sqlx::query_as("select count(*) from runtime_audit_hashes where flow_run_id = $1")
            .bind(run_id)
            .fetch_one(&pool)
            .await
            .unwrap();

    assert_eq!(billing_count, 1);
    assert_eq!(cost_count, 1);
    assert_eq!(credit_count, 1);
    assert_eq!(audit_count, 3);
}

#[tokio::test]
async fn application_runtime_routes_runtime_debug_artifact_full_load_returns_original_payload() {
    let app = test_app().await;
    let (cookie, csrf) = login_and_capture_cookie(&app, "root", "change-me").await;
    let provider_instance_id = create_ready_provider_instance(&app, &cookie, &csrf).await;
    let application_id =
        seed_agent_flow_application(&app, &cookie, &csrf, &provider_instance_id).await;
    let other_application_id =
        seed_agent_flow_application(&app, &cookie, &csrf, &provider_instance_id).await;
    let large_query = "退款政策".repeat(900);
    let debug_session_id = "runtime-debug-artifact-session";

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/console/applications/{application_id}/orchestration/debug-runs"
                ))
                .header("cookie", &cookie)
                .header("x-csrf-token", &csrf)
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "debug_session_id": debug_session_id,
                        "input_payload": {
                            "node-start": { "query": large_query }
                        }
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let payload: Value = serde_json::from_slice(&body).unwrap();
    let run_id = payload["data"]["flow_run"]["id"].as_str().unwrap();

    let snapshot_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/console/applications/{application_id}/orchestration/debug-variable-snapshot"
                ))
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(snapshot_response.status(), StatusCode::OK);
    let snapshot_body = to_bytes(snapshot_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let snapshot_payload: Value = serde_json::from_slice(&snapshot_body).unwrap();
    assert!(snapshot_payload["data"]["variable_cache"]["node-start"].is_null());

    let detail = wait_for_run_detail(
        &app,
        &cookie,
        &application_id,
        run_id,
        &["succeeded", "failed", "cancelled"],
    )
    .await;
    let preview = &detail["flow_run"]["input_payload"];

    assert_eq!(preview["__runtime_debug_artifact"], true);
    assert_eq!(preview["is_truncated"], true);
    assert!(preview["preview"].as_str().unwrap().len() < large_query.len());
    let artifact_ref = preview["artifact_ref"].as_str().unwrap();

    let unauthorized_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/console/applications/{application_id}/orchestration/debug-artifacts/{artifact_ref}"
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(unauthorized_response.status(), StatusCode::UNAUTHORIZED);

    let wrong_application_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/console/applications/{other_application_id}/orchestration/debug-artifacts/{artifact_ref}"
                ))
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(wrong_application_response.status(), StatusCode::NOT_FOUND);

    let artifact_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/console/applications/{application_id}/orchestration/debug-artifacts/{artifact_ref}"
                ))
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(artifact_response.status(), StatusCode::OK);
    assert_eq!(
        artifact_response.headers()["content-type"]
            .to_str()
            .unwrap(),
        "application/json"
    );
    let artifact_body = to_bytes(artifact_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let full_payload: Value = serde_json::from_slice(&artifact_body).unwrap();

    assert_eq!(full_payload["node-start"]["query"], large_query);
}

#[tokio::test]
async fn application_runtime_routes_flow_output_offloads_answer_field_without_compressing_sys_env()
{
    let app = test_app().await;
    let (cookie, csrf) = login_and_capture_cookie(&app, "root", "change-me").await;
    let answer_text = format!("answer:{}", "A".repeat(3_000));
    let application_id = seed_answer_only_application(&app, &cookie, &csrf, &answer_text).await;
    let debug_session_id = "runtime-debug-answer-artifact-session";

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/console/applications/{application_id}/orchestration/debug-runs"
                ))
                .header("cookie", &cookie)
                .header("x-csrf-token", &csrf)
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "debug_session_id": debug_session_id,
                        "input_payload": {
                            "node-start": { "query": "ping" }
                        }
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let payload: Value = serde_json::from_slice(&body).unwrap();
    let run_id = payload["data"]["flow_run"]["id"].as_str().unwrap();

    let detail = wait_for_run_detail(
        &app,
        &cookie,
        &application_id,
        run_id,
        &["succeeded", "failed", "cancelled"],
    )
    .await;

    assert_eq!(detail["flow_run"]["status"], json!("succeeded"));
    let flow_output = &detail["flow_run"]["output_payload"];
    assert!(flow_output.get("__runtime_debug_artifact").is_none());
    assert_eq!(flow_output["answer"]["__runtime_debug_artifact"], true);
    assert_eq!(flow_output["answer"]["artifact_scope"], json!("field"));
    assert_eq!(flow_output["answer"]["field_path"], json!(["answer"]));
    assert!(flow_output["answer"]["preview"]
        .as_str()
        .expect("answer preview should be a string")
        .contains("answer:"));
    let answer_artifact_ref = flow_output["answer"]["artifact_ref"]
        .as_str()
        .expect("answer artifact ref should exist");
    let answer_artifact_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/console/applications/{application_id}/orchestration/debug-artifacts/{answer_artifact_ref}"
                ))
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(answer_artifact_response.status(), StatusCode::OK);
    let answer_artifact_body = to_bytes(answer_artifact_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let full_answer: Value = serde_json::from_slice(&answer_artifact_body).unwrap();
    assert_eq!(
        full_answer
            .as_str()
            .expect("answer artifact should be text"),
        answer_text
    );
    assert_eq!(flow_output["sys"]["workflow_run_id"], json!(run_id));
    assert_eq!(flow_output["env"], json!({}));

    let answer_node_output = detail["node_runs"]
        .as_array()
        .expect("node runs should be an array")
        .iter()
        .find(|node_run| node_run["node_id"] == json!("node-answer"))
        .expect("answer node should be present")
        .get("output_payload")
        .expect("answer node should have output payload");
    assert_eq!(
        answer_node_output["answer"]["__runtime_debug_artifact"],
        true
    );
    assert_eq!(
        answer_node_output["answer"]["artifact_scope"],
        json!("field")
    );
    assert_eq!(
        answer_node_output["answer"]["field_path"],
        json!(["answer"])
    );
    assert_eq!(answer_node_output["sys"]["workflow_run_id"], json!(run_id));
    assert_eq!(answer_node_output["env"], json!({}));
}

#[tokio::test]
async fn application_runtime_routes_waiting_run_detail_offloads_large_llm_rounds() {
    let (app, database_url) = test_app_with_database_url().await;
    let (cookie, csrf) = login_and_capture_cookie(&app, "root", "change-me").await;
    let provider_instance_id = create_ready_provider_instance(&app, &cookie, &csrf).await;
    let application_id =
        seed_human_input_application(&app, &cookie, &csrf, &provider_instance_id).await;

    let start = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/console/applications/{application_id}/orchestration/debug-runs"
                ))
                .header("cookie", &cookie)
                .header("x-csrf-token", &csrf)
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "input_payload": {
                            "node-start": { "query": "请总结退款政策" }
                        }
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(start.status(), StatusCode::CREATED);
    let start_body = to_bytes(start.into_body(), usize::MAX).await.unwrap();
    let start_payload: Value = serde_json::from_slice(&start_body).unwrap();
    let run_id = start_payload["data"]["flow_run"]["id"].as_str().unwrap();
    let flow_run_id = Uuid::parse_str(run_id).unwrap();

    let detail =
        wait_for_run_detail(&app, &cookie, &application_id, run_id, &["waiting_human"]).await;
    let llm_node_run = detail["node_runs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|node_run| node_run["node_id"].as_str() == Some("node-llm"))
        .expect("waiting run detail should include the LLM node run");
    let llm_node_run_id = Uuid::parse_str(llm_node_run["id"].as_str().unwrap()).unwrap();
    let large_llm_content = "tool callback evidence ".repeat(300);
    let pool = sqlx::PgPool::connect(&database_url).await.unwrap();
    sqlx::query("update node_runs set debug_payload = $1 where id = $2")
        .bind(json!({
            "llm_rounds": [
                {
                    "round_index": 0,
                    "usage": {
                        "input_tokens": 11,
                        "input_cache_hit_tokens": 5,
                        "output_tokens": 3,
                        "total_tokens": 14
                    },
                    "assistant": {
                        "role": "assistant",
                        "content": large_llm_content,
                        "tool_calls": [
                            {
                                "id": "call_weather",
                                "name": "lookup_weather",
                                "call_usage": {
                                    "input_tokens": 11,
                                    "input_cache_hit_tokens": 5,
                                    "output_tokens": 3,
                                    "total_tokens": 14
                                },
                                "arguments": {
                                    "city": "Shanghai"
                                }
                            }
                        ]
                    },
                    "tool_results": [
                        {
                            "role": "tool",
                            "tool_call_id": "call_weather",
                            "result_context_usage": {
                                "input_tokens": 20,
                                "input_cache_hit_tokens": 8,
                                "output_tokens": 4,
                                "total_tokens": 24
                            },
                            "content": "{\"temperature\":21}"
                        }
                    ]
                },
                {
                    "round_index": 1,
                    "usage": {
                        "input_tokens": 20,
                        "input_cache_hit_tokens": 8,
                        "output_tokens": 4,
                        "total_tokens": 24
                    },
                    "assistant": {
                        "role": "assistant",
                        "content": "weather is clear"
                    },
                    "finish_reason": "stop"
                }
            ]
        }))
        .bind(llm_node_run_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(
        r#"
        insert into flow_run_callback_tasks (
            id,
            scope_id,
            flow_run_id,
            node_run_id,
            callback_kind,
            status,
            request_payload,
            response_payload,
            external_ref_payload,
            completed_at
        ) values (
            $1,
            (
                select applications.workspace_id
                from flow_runs
                join applications on applications.id = flow_runs.application_id
                where flow_runs.id = $2
            ),
            $2,
            $3,
            'llm_tool_calls',
            'completed',
            $4,
            $5,
            $4,
            now()
        )
        "#,
    )
    .bind(Uuid::now_v7())
    .bind(flow_run_id)
    .bind(llm_node_run_id)
    .bind(json!({
        "tool_calls": [
            {
                "id": "call_weather",
                "name": "lookup_weather",
                "arguments": {
                    "city": "Shanghai"
                }
            }
        ]
    }))
    .bind(json!({
        "tool_results": [
            {
                "tool_call_id": "call_weather",
                "content": "{\"temperature\":21}",
                "stdout": "{\"temperature\":21}",
                "adapter_trace_id": "trace-weather-1"
            }
        ]
    }))
    .execute(&pool)
    .await
    .unwrap();

    let detail =
        wait_for_run_detail(&app, &cookie, &application_id, run_id, &["waiting_human"]).await;
    let llm_node_run = detail["node_runs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|node_run| node_run["node_id"].as_str() == Some("node-llm"))
        .expect("waiting run detail should include the LLM node run");
    let llm_rounds = &llm_node_run["debug_payload"]["llm_rounds"];

    assert_eq!(llm_rounds["__runtime_debug_artifact"], true);
    assert_eq!(llm_rounds["artifact_scope"], "field");
    assert_eq!(llm_rounds["field_path"], json!(["llm_rounds"]));
    assert!(llm_rounds["preview"].as_str().unwrap().len() < large_llm_content.len());
    let tool_callbacks = llm_rounds["tool_callbacks"].as_array().unwrap();
    assert_eq!(tool_callbacks.len(), 1);
    assert_eq!(tool_callbacks[0]["id"], "call_weather");
    assert_eq!(tool_callbacks[0]["name"], "lookup_weather");
    assert_eq!(tool_callbacks[0]["callback_status"], "returned");
    assert_eq!(tool_callbacks[0]["execution_status"], "unknown");
    assert_eq!(tool_callbacks[0]["request_round_index"], 0);
    assert_eq!(tool_callbacks[0]["result_round_index"], 0);
    assert_eq!(tool_callbacks[0]["call_usage"]["input_tokens"], 11);
    assert_eq!(tool_callbacks[0]["call_usage"]["output_tokens"], 3);
    assert_eq!(tool_callbacks[0]["call_usage"]["total_tokens"], 14);
    assert_eq!(
        tool_callbacks[0]["result_context_usage"]["input_tokens"],
        20
    );
    assert_eq!(
        tool_callbacks[0]["result_context_usage"]["total_tokens"],
        24
    );
    assert!(tool_callbacks[0].get("token_delta").is_none());
    assert!(tool_callbacks[0].get("result_input_tokens").is_none());
    assert!(tool_callbacks[0].get("token_count_method").is_none());
    let tool_callback_artifact_ref = tool_callbacks[0]["artifact_ref"].as_str().unwrap();

    let full_llm_rounds =
        resolve_runtime_debug_artifact_value(&app, &cookie, &application_id, llm_rounds).await;
    assert!(full_llm_rounds[0]["assistant"]["content"]
        .as_str()
        .unwrap()
        .contains("tool callback evidence"));

    let tool_callback_detail = load_runtime_debug_artifact_by_ref(
        &app,
        &cookie,
        &application_id,
        tool_callback_artifact_ref,
    )
    .await;
    assert_eq!(tool_callback_detail["id"], "call_weather");
    assert_eq!(tool_callback_detail["name"], "lookup_weather");
    assert_eq!(tool_callback_detail["callback_status"], "returned");
    assert_eq!(tool_callback_detail["execution_status"], "unknown");
    assert_eq!(
        tool_callback_detail["request_payload"]["arguments"]["city"],
        "Shanghai"
    );
    assert_eq!(
        tool_callback_detail["callback_payload"]["content"],
        "{\"temperature\":21}"
    );
    assert_eq!(
        tool_callback_detail["callback_payload"]["adapter_trace_id"],
        "trace-weather-1"
    );
    assert_eq!(
        tool_callback_detail["parsed_result"]["content"],
        "{\"temperature\":21}"
    );
    assert_eq!(tool_callback_detail["call_usage"]["input_tokens"], 11);
    assert_eq!(tool_callback_detail["call_usage"]["output_tokens"], 3);
    assert_eq!(tool_callback_detail["call_usage"]["total_tokens"], 14);
    assert_eq!(
        tool_callback_detail["result_context_usage"]["input_tokens"],
        20
    );
    assert_eq!(
        tool_callback_detail["result_context_usage"]["total_tokens"],
        24
    );
    assert!(tool_callback_detail.get("token_delta").is_none());
    assert!(tool_callback_detail.get("result_input_tokens").is_none());
    assert!(tool_callback_detail.get("token_count_method").is_none());
}
