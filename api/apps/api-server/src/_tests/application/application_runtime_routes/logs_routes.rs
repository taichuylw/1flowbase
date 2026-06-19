use super::*;
use control_plane::ports::{
    AppendRuntimeEventInput, CreateCallbackTaskInput, OrchestrationRuntimeRepository,
    UpdateFlowRunInput,
};
use storage_durable::MainDurableStore;

async fn get_console_json(app: &axum::Router, cookie: &str, uri: String) -> Value {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(uri)
                .header("cookie", cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));

    serde_json::from_slice(&body).unwrap()
}

#[tokio::test]
async fn application_runtime_routes_start_node_preview_and_query_logs() {
    let (state, _) = test_api_state_with_database_url().await;
    let app = crate::app_with_state_and_config(state.clone(), &test_config());
    let (cookie, csrf) = login_and_capture_cookie(&app, "root", "change-me").await;
    let provider_instance_id = create_ready_provider_instance(&app, &cookie, &csrf).await;
    let application_id =
        seed_agent_flow_application(&app, &cookie, &csrf, &provider_instance_id).await;

    let preview = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/console/applications/{application_id}/orchestration/nodes/node-llm/debug-runs"
                ))
                .header("cookie", &cookie)
                .header("x-csrf-token", &csrf)
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "input_payload": {
                            "node-start": { "query": "总结退款政策" }
                        }
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let preview_status = preview.status();
    let preview_body = to_bytes(preview.into_body(), usize::MAX).await.unwrap();
    assert_eq!(
        preview_status,
        StatusCode::CREATED,
        "{}",
        String::from_utf8_lossy(&preview_body)
    );
    let preview_payload: Value = serde_json::from_slice(&preview_body).unwrap();
    let flow_run_id = preview_payload["data"]["flow_run"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    assert_eq!(
        preview_payload["data"]["flow_run"]["run_mode"].as_str(),
        Some("debug_node_preview")
    );
    assert_eq!(
        preview_payload["data"]["node_run"]["node_id"].as_str(),
        Some("node-llm")
    );
    assert_eq!(
        preview_payload["data"]["node_run"]["output_payload"]["text"],
        json!("reply:总结退款政策")
    );
    for hidden_key in [
        "resolved_inputs",
        "rendered_templates",
        "output_contract",
        "metrics_payload",
        "debug_payload",
        "provider_events",
    ] {
        assert!(
            preview_payload["data"]["node_run"]["output_payload"]
                .get(hidden_key)
                .is_none(),
            "{hidden_key} must not leak into node output"
        );
        assert!(
            preview_payload["data"]["flow_run"]["output_payload"]
                .get(hidden_key)
                .is_none(),
            "{hidden_key} must not leak into flow output"
        );
    }
    assert_eq!(
        preview_payload["data"]["events"][0]["event_type"].as_str(),
        Some("node_preview_started")
    );
    let event_types = preview_payload["data"]["events"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|event| event["event_type"].as_str())
        .collect::<Vec<_>>();
    assert!(event_types.contains(&"text_delta"));
    assert!(event_types.contains(&"usage_snapshot"));
    assert!(event_types.contains(&"finish"));
    assert!(event_types.contains(&"node_preview_completed"));

    let list = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/console/applications/{application_id}/logs/runs"
                ))
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(list.status(), StatusCode::OK);
    let list_body = to_bytes(list.into_body(), usize::MAX).await.unwrap();
    let list_payload: Value = serde_json::from_slice(&list_body).unwrap();
    assert_eq!(list_payload["data"]["page"].as_i64(), Some(1));
    assert_eq!(list_payload["data"]["page_size"].as_i64(), Some(20));
    assert_eq!(list_payload["data"]["total"].as_i64(), Some(1));
    assert_eq!(list_payload["data"]["items"].as_array().unwrap().len(), 1);
    assert_eq!(
        list_payload["data"]["items"][0]["id"].as_str(),
        Some(flow_run_id.as_str())
    );
    assert_eq!(
        list_payload["data"]["items"][0]["application_id"].as_str(),
        Some(application_id.as_str())
    );
    assert_eq!(
        list_payload["data"]["items"][0]["application_type"].as_str(),
        Some("agent_flow")
    );
    assert_eq!(
        list_payload["data"]["items"][0]["run_object_kind"].as_str(),
        Some("application_run")
    );
    assert_eq!(
        list_payload["data"]["items"][0]["subject"]["kind"].as_str(),
        Some("agent_flow")
    );
    assert_eq!(
        list_payload["data"]["items"][0]["title"].as_str(),
        Some("总结退款政策")
    );
    assert!(list_payload["data"]["items"][0]["created_at"].is_string());
    assert!(list_payload["data"]["items"][0]["updated_at"].is_string());

    let runtime_activity = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/console/applications/{application_id}/monitoring/runtime-activity"
                ))
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(runtime_activity.status(), StatusCode::OK);
    let runtime_activity_body = to_bytes(runtime_activity.into_body(), usize::MAX)
        .await
        .unwrap();
    let runtime_activity_payload: Value = serde_json::from_slice(&runtime_activity_body).unwrap();
    assert_eq!(
        runtime_activity_payload["data"]["meta"]["storage"].as_str(),
        Some("memory")
    );
    assert_eq!(
        runtime_activity_payload["data"]["meta"]["scope"].as_str(),
        Some("current_instance")
    );
    assert!(
        runtime_activity_payload["data"]["peaks"]["process_peak_concurrency"]
            .as_u64()
            .unwrap()
            >= 1
    );
    assert!(
        runtime_activity_payload["data"]["rolling_minute"]["completed"]
            .as_u64()
            .unwrap()
            >= 1
    );

    let trace_tree = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/console/applications/{application_id}/logs/runs/{flow_run_id}/trace-tree"
                ))
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(trace_tree.status(), StatusCode::OK);
    let trace_tree_body = to_bytes(trace_tree.into_body(), usize::MAX).await.unwrap();
    let trace_tree_payload: Value = serde_json::from_slice(&trace_tree_body).unwrap();
    assert_eq!(
        trace_tree_payload["data"]["flow_run"]["id"].as_str(),
        Some(flow_run_id.as_str())
    );
    assert_eq!(
        trace_tree_payload["data"]["run"]["id"].as_str(),
        Some(flow_run_id.as_str())
    );
    assert_eq!(
        trace_tree_payload["data"]["run"]["application_type"].as_str(),
        Some("agent_flow")
    );
    assert_eq!(
        trace_tree_payload["data"]["run"]["run_object_kind"].as_str(),
        Some("application_run")
    );
    assert_eq!(
        trace_tree_payload["data"]["flow_run"]["title"].as_str(),
        Some("总结退款政策")
    );
    assert_eq!(
        trace_tree_payload["data"]["nodes"][0]["node_alias"].as_str(),
        Some("LLM")
    );
    let cache_entries = state
        .infrastructure
        .cache_store()
        .list_cache_entries("application-logs")
        .await
        .unwrap();
    assert!(
        cache_entries
            .iter()
            .any(|entry| entry.key.contains(":summary-page:")),
        "application log summary-page cache entry missing: {cache_entries:?}"
    );
    let summary_cache_key = cache_entries
        .iter()
        .find(|entry| entry.key.contains(":summary-page:"))
        .expect("summary-page cache entry must exist")
        .key
        .clone();
    let mut stale_page = list_payload["data"].clone();
    stale_page["items"][0]["title"] = json!("stale cache title");
    state
        .infrastructure
        .cache_store()
        .set_json(
            &summary_cache_key,
            stale_page,
            Some(time::Duration::minutes(5)),
        )
        .await
        .unwrap();
    let cached_list = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/console/applications/{application_id}/logs/runs"
                ))
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(cached_list.status(), StatusCode::OK);
    let cached_list_body = to_bytes(cached_list.into_body(), usize::MAX).await.unwrap();
    let cached_list_payload: Value = serde_json::from_slice(&cached_list_body).unwrap();
    assert_eq!(
        cached_list_payload["data"]["items"][0]["title"].as_str(),
        Some("stale cache title")
    );
    let refreshed_list = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/console/applications/{application_id}/logs/runs?cache_mode=refresh"
                ))
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(refreshed_list.status(), StatusCode::OK);
    let refreshed_list_body = to_bytes(refreshed_list.into_body(), usize::MAX)
        .await
        .unwrap();
    let refreshed_list_payload: Value = serde_json::from_slice(&refreshed_list_body).unwrap();
    assert_eq!(
        refreshed_list_payload["data"]["items"][0]["title"].as_str(),
        Some("总结退款政策")
    );
    assert!(
        !cache_entries.iter().any(|entry| {
            entry.key.contains(":run-detail:") && entry.key.contains(flow_run_id.as_str())
        }),
        "application log run-detail must not be cached: {cache_entries:?}"
    );
    let scoped_node_run = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/console/applications/{application_id}/logs/runs/{flow_run_id}/nodes/node-llm"
                ))
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(scoped_node_run.status(), StatusCode::OK);
    let scoped_node_run_body = to_bytes(scoped_node_run.into_body(), usize::MAX)
        .await
        .unwrap();
    let scoped_node_run_payload: Value = serde_json::from_slice(&scoped_node_run_body).unwrap();
    assert_eq!(
        scoped_node_run_payload["data"]["node_run"]["node_id"].as_str(),
        Some("node-llm")
    );
    assert!(scoped_node_run_payload["data"]["events"]
        .as_array()
        .unwrap()
        .iter()
        .all(|event| event["node_run_id"].as_str()
            == Some(
                scoped_node_run_payload["data"]["node_run"]["id"]
                    .as_str()
                    .unwrap()
            )));

    let last_run = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/console/applications/{application_id}/orchestration/nodes/node-llm/last-run"
                ))
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(last_run.status(), StatusCode::OK);
    let last_run_body = to_bytes(last_run.into_body(), usize::MAX).await.unwrap();
    let last_run_payload: Value = serde_json::from_slice(&last_run_body).unwrap();
    assert_eq!(
        last_run_payload["data"]["node_run"]["node_id"].as_str(),
        Some("node-llm")
    );
    assert_eq!(
        last_run_payload["data"]["flow_run"]["id"].as_str(),
        Some(flow_run_id.as_str())
    );
}

#[tokio::test]
async fn application_runtime_routes_log_trace_tree_loads_summary_children_and_content_lazily() {
    let (state, _) = test_api_state_with_database_url().await;
    let app = crate::app_with_state_and_config(state.clone(), &test_config());
    let (cookie, csrf) = login_and_capture_cookie(&app, "root", "change-me").await;
    let provider_instance_id = create_ready_provider_instance(&app, &cookie, &csrf).await;
    let application_id =
        seed_agent_flow_application(&app, &cookie, &csrf, &provider_instance_id).await;

    let preview = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/console/applications/{application_id}/orchestration/nodes/node-llm/debug-runs"
                ))
                .header("cookie", &cookie)
                .header("x-csrf-token", &csrf)
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "input_payload": {
                            "node-start": { "query": "总结退款政策" }
                        }
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let preview_body = to_bytes(preview.into_body(), usize::MAX).await.unwrap();
    let preview_payload: Value = serde_json::from_slice(&preview_body).unwrap();
    let flow_run_id = preview_payload["data"]["flow_run"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let node_run_id =
        Uuid::parse_str(preview_payload["data"]["node_run"]["id"].as_str().unwrap()).unwrap();

    let _callback = <MainDurableStore as OrchestrationRuntimeRepository>::create_callback_task(
        &state.store,
        &CreateCallbackTaskInput {
            flow_run_id: Uuid::parse_str(&flow_run_id).unwrap(),
            node_run_id,
            callback_kind: "llm_tool_calls".to_string(),
            request_payload: json!({
                "tool_calls": [
                    {
                        "id": "call-refund-policy",
                        "name": "refund_policy_lookup"
                    }
                ]
            }),
            external_ref_payload: None,
        },
    )
    .await
    .unwrap();

    let trace_tree = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/console/applications/{application_id}/logs/runs/{flow_run_id}/trace-tree"
                ))
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(trace_tree.status(), StatusCode::OK);
    let trace_tree_body = to_bytes(trace_tree.into_body(), usize::MAX).await.unwrap();
    let trace_tree_payload: Value = serde_json::from_slice(&trace_tree_body).unwrap();
    assert_eq!(
        trace_tree_payload["data"]["flow_run"]["id"].as_str(),
        Some(flow_run_id.as_str())
    );
    let root_nodes = trace_tree_payload["data"]["nodes"].as_array().unwrap();
    assert_eq!(root_nodes.len(), 1);
    assert_eq!(root_nodes[0]["node_id"].as_str(), Some("node-llm"));
    assert_eq!(root_nodes[0]["node_kind"].as_str(), Some("node_run"));
    assert_eq!(root_nodes[0]["has_children"].as_bool(), Some(true));
    assert_eq!(root_nodes[0]["has_content"].as_bool(), Some(true));
    assert!(
        root_nodes[0].get("input_payload").is_none(),
        "trace node summary must not include heavy node input payload"
    );
    assert!(
        root_nodes[0].get("debug_payload").is_none(),
        "trace node summary must not include heavy node debug payload"
    );
    let root_trace_node_id = root_nodes[0]["trace_node_id"].as_str().unwrap();

    let children = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/console/applications/{application_id}/logs/runs/{flow_run_id}/trace-tree/nodes?parent_trace_node_id={root_trace_node_id}"
                ))
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(children.status(), StatusCode::OK);
    let children_body = to_bytes(children.into_body(), usize::MAX).await.unwrap();
    let children_payload: Value = serde_json::from_slice(&children_body).unwrap();
    let child_nodes = children_payload["data"]["items"].as_array().unwrap();
    assert_eq!(
        child_nodes.len(),
        1,
        "llm_tool_calls are exposed through the projection tools group"
    );
    assert_eq!(child_nodes[0]["node_kind"], json!("tool_group"));
    assert_eq!(child_nodes[0]["has_children"], json!(true));
    assert_eq!(child_nodes[0]["has_content"], json!(false));

    let content = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/console/applications/{application_id}/logs/runs/{flow_run_id}/trace-tree/nodes/{root_trace_node_id}/content"
                ))
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(content.status(), StatusCode::OK);
    let content_body = to_bytes(content.into_body(), usize::MAX).await.unwrap();
    let content_payload: Value = serde_json::from_slice(&content_body).unwrap();
    assert!(
        content_payload["data"]["payload"].get("node_run").is_none(),
        "trace node content should advertise detail refs instead of materializing node_run"
    );
    let node_run_detail_payload = load_trace_node_detail_payload_for_kind(
        &app,
        &cookie,
        &application_id,
        &flow_run_id,
        root_trace_node_id,
        "node_run",
    )
    .await
    .expect("root trace node should advertise a node_run detail ref");
    assert_eq!(
        node_run_detail_payload["data"]["payload"]["node_run"]["output_payload"]["text"],
        json!("reply:总结退款政策")
    );
    let events_detail_payload = load_trace_node_detail_payload_for_kind(
        &app,
        &cookie,
        &application_id,
        &flow_run_id,
        root_trace_node_id,
        "events",
    )
    .await
    .expect("root trace node should advertise an events detail ref");
    assert!(events_detail_payload["data"]["payload"]["events"]
        .as_array()
        .unwrap()
        .iter()
        .all(|event| event["node_run_id"] == json!(node_run_id.to_string())));
}

#[tokio::test]
async fn application_runtime_routes_logs_include_public_run_identity_fields() {
    let app = test_app().await;
    let (cookie, csrf) = login_and_capture_cookie(&app, "root", "change-me").await;
    let provider_instance_id = create_ready_provider_instance(&app, &cookie, &csrf).await;
    let application_id =
        seed_agent_flow_application(&app, &cookie, &csrf, &provider_instance_id).await;
    publish_application_public_api(&app, &cookie, &csrf, &application_id).await;
    let token = create_application_public_api_key(&app, &cookie, &csrf, &application_id).await;

    let create = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/agent/v1/runs")
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "query": "请总结退款政策",
                        "title": "公开 API 退款总结",
                        "expand_id": "customer-42",
                        "compatibility_mode": "native-v1",
                        "response_mode": "queued"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(create.status(), StatusCode::CREATED);
    let create_body = to_bytes(create.into_body(), usize::MAX).await.unwrap();
    let create_payload: Value = serde_json::from_slice(&create_body).unwrap();
    let flow_run_id = create_payload["data"]["id"].as_str().unwrap().to_string();

    let list = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/console/applications/{application_id}/logs/runs"
                ))
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(list.status(), StatusCode::OK);
    let list_body = to_bytes(list.into_body(), usize::MAX).await.unwrap();
    let list_payload: Value = serde_json::from_slice(&list_body).unwrap();
    assert_eq!(list_payload["data"]["page"].as_i64(), Some(1));
    assert_eq!(list_payload["data"]["page_size"].as_i64(), Some(20));
    assert_eq!(list_payload["data"]["total"].as_i64(), Some(1));
    assert_eq!(list_payload["data"]["items"].as_array().unwrap().len(), 1);
    assert_eq!(
        list_payload["data"]["items"][0]["id"].as_str(),
        Some(flow_run_id.as_str())
    );
    assert_eq!(
        list_payload["data"]["items"][0]["run_mode"].as_str(),
        Some("published_api_run")
    );
    assert_eq!(
        list_payload["data"]["items"][0]["title"].as_str(),
        Some("公开 API 退款总结")
    );
    assert_eq!(
        list_payload["data"]["items"][0]["expand_id"].as_str(),
        Some("customer-42")
    );
    assert_eq!(
        list_payload["data"]["items"][0]["authorized_account"].as_str(),
        Some("root")
    );
    assert_eq!(
        list_payload["data"]["items"][0]["source"].as_str(),
        Some("public_api")
    );
    assert_eq!(
        list_payload["data"]["items"][0]["compatibility_mode"].as_str(),
        Some("native-v1")
    );
    assert!(!list_payload["data"]["items"][0]
        .as_object()
        .unwrap()
        .contains_key("protocol"));
    assert_eq!(
        list_payload["data"]["items"][0]["correlation"]["external_user"].as_str(),
        Some("customer-42")
    );
    assert!(list_payload["data"]["items"][0]["correlation"]["api_key_id"].is_string());
    assert!(list_payload["data"]["items"][0]["correlation"]["publication_version_id"].is_string());

    let trace_tree = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/console/applications/{application_id}/logs/runs/{flow_run_id}/trace-tree"
                ))
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(trace_tree.status(), StatusCode::OK);
    let trace_tree_body = to_bytes(trace_tree.into_body(), usize::MAX).await.unwrap();
    let trace_tree_payload: Value = serde_json::from_slice(&trace_tree_body).unwrap();
    assert_eq!(
        trace_tree_payload["data"]["flow_run"]["title"].as_str(),
        Some("公开 API 退款总结")
    );
    assert_eq!(
        trace_tree_payload["data"]["flow_run"]["expand_id"].as_str(),
        Some("customer-42")
    );
    assert_eq!(
        trace_tree_payload["data"]["flow_run"]["authorized_account"].as_str(),
        Some("root")
    );
    assert_eq!(
        trace_tree_payload["data"]["run"]["source"].as_str(),
        Some("public_api")
    );
    assert_eq!(
        trace_tree_payload["data"]["run"]["compatibility_mode"].as_str(),
        Some("native-v1")
    );
    assert!(!trace_tree_payload["data"]["run"]
        .as_object()
        .unwrap()
        .contains_key("protocol"));
    assert_eq!(
        trace_tree_payload["data"]["run"]["correlation"]["external_user"].as_str(),
        Some("customer-42")
    );
    assert!(trace_tree_payload["data"]["run"]["correlation"]["api_key_id"].is_string());
    assert!(trace_tree_payload["data"]["run"]["correlation"]["publication_version_id"].is_string());
}

#[tokio::test]
async fn application_runtime_routes_logs_report_run_statistics() {
    let (state, database_url) = test_api_state_with_database_url().await;
    let app = crate::app_with_state_and_config(state.clone(), &test_config());
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
    assert_eq!(start.status(), StatusCode::CREATED);
    let start_body = to_bytes(start.into_body(), usize::MAX).await.unwrap();
    let start_payload: Value = serde_json::from_slice(&start_body).unwrap();
    let flow_run_id_string = start_payload["data"]["flow_run"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let flow_run_id = Uuid::parse_str(&flow_run_id_string).unwrap();
    wait_for_run_detail(
        &app,
        &cookie,
        &application_id,
        &flow_run_id_string,
        &["succeeded", "failed", "cancelled"],
    )
    .await;
    let pool = sqlx::PgPool::connect(&database_url).await.unwrap();

    sqlx::query("delete from application_run_log_summaries where flow_run_id = $1")
        .bind(flow_run_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("delete from flow_run_callback_tasks where flow_run_id = $1")
        .bind(flow_run_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("delete from flow_run_checkpoints where flow_run_id = $1")
        .bind(flow_run_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("delete from flow_run_events where flow_run_id = $1")
        .bind(flow_run_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("delete from node_runs where flow_run_id = $1")
        .bind(flow_run_id)
        .execute(&pool)
        .await
        .unwrap();

    let llm_run_id = Uuid::now_v7();
    for (node_run_id, node_id, node_type, node_alias, metrics_payload) in [
        (
            llm_run_id,
            "node-llm",
            "llm",
            "LLM",
            json!({ "usage": { "total_tokens": 12 } }),
        ),
        (
            Uuid::now_v7(),
            "node-llm",
            "llm",
            "LLM",
            json!({ "usage": { "total_tokens": 8 } }),
        ),
        (
            Uuid::now_v7(),
            "node-summary",
            "llm",
            "Summary",
            json!({ "usage": { "input_tokens": 10, "output_tokens": 20 } }),
        ),
        (Uuid::now_v7(), "node-answer", "answer", "Answer", json!({})),
    ] {
        sqlx::query(
            r#"
            insert into node_runs (
                id,
                scope_id,
                flow_run_id,
                node_id,
                node_type,
                node_alias,
                status,
                input_payload,
                output_payload,
                error_payload,
                metrics_payload,
                started_at,
                finished_at
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
                $4,
                $5,
                'succeeded',
                '{}'::jsonb,
                '{}'::jsonb,
                null,
                $6,
                now(),
                now()
            )
            "#,
        )
        .bind(node_run_id)
        .bind(flow_run_id)
        .bind(node_id)
        .bind(node_type)
        .bind(node_alias)
        .bind(metrics_payload)
        .execute(&pool)
        .await
        .unwrap();
    }

    let tool_calls = (0..20)
        .map(|index| {
            json!({
                "id": format!("call-{index}"),
                "name": "lookup_policy"
            })
        })
        .collect::<Vec<_>>();
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
            '{}'::jsonb,
            '{}'::jsonb,
            now()
        )
        "#,
    )
    .bind(Uuid::now_v7())
    .bind(flow_run_id)
    .bind(llm_run_id)
    .bind(json!({ "tool_calls": tool_calls }))
    .execute(&pool)
    .await
    .unwrap();

    let flow_run = <MainDurableStore as OrchestrationRuntimeRepository>::get_flow_run(
        &state.store,
        Uuid::parse_str(&application_id).unwrap(),
        flow_run_id,
    )
    .await
    .unwrap()
    .unwrap();
    <MainDurableStore as OrchestrationRuntimeRepository>::update_flow_run(
        &state.store,
        &UpdateFlowRunInput {
            flow_run_id,
            status: flow_run.status,
            output_payload: flow_run.output_payload,
            error_payload: flow_run.error_payload,
            finished_at: flow_run.finished_at,
        },
    )
    .await
    .unwrap();

    let expected_statistics = json!({
        "total_tokens": 50,
        "input_tokens": 10,
        "output_tokens": 20,
        "input_cache_hit_tokens": null,
        "unique_node_count": 3,
        "tool_callback_count": 20
    });
    let list = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/console/applications/{application_id}/logs/runs"
                ))
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(list.status(), StatusCode::OK);
    let list_body = to_bytes(list.into_body(), usize::MAX).await.unwrap();
    let list_payload: Value = serde_json::from_slice(&list_body).unwrap();
    assert_eq!(
        list_payload["data"]["items"][0]["statistics"],
        expected_statistics
    );

    let trace_tree = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/console/applications/{application_id}/logs/runs/{flow_run_id}/trace-tree"
                ))
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(trace_tree.status(), StatusCode::OK);
    let trace_tree_body = to_bytes(trace_tree.into_body(), usize::MAX).await.unwrap();
    let trace_tree_payload: Value = serde_json::from_slice(&trace_tree_body).unwrap();
    assert_eq!(
        trace_tree_payload["data"]["statistics"],
        expected_statistics
    );
}

#[tokio::test]
async fn application_runtime_routes_trace_tree_content_exposes_visible_internal_llm_route_trace() {
    let (state, database_url) = test_api_state_with_database_url().await;
    let app = crate::app_with_state_and_config(state.clone(), &test_config());
    let (cookie, csrf) = login_and_capture_cookie(&app, "root", "change-me").await;
    let provider_instance_id = create_ready_provider_instance(&app, &cookie, &csrf).await;
    let application_id =
        seed_agent_flow_application(&app, &cookie, &csrf, &provider_instance_id).await;

    let preview = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/console/applications/{application_id}/orchestration/nodes/node-llm/debug-runs"
                ))
                .header("cookie", &cookie)
                .header("x-csrf-token", &csrf)
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "input_payload": {
                            "node-start": { "query": "uploads/image-1.png 帮我看看导航栏代码" }
                        }
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let preview_status = preview.status();
    let preview_body = to_bytes(preview.into_body(), usize::MAX).await.unwrap();
    assert_eq!(
        preview_status,
        StatusCode::CREATED,
        "{}",
        String::from_utf8_lossy(&preview_body)
    );
    let preview_payload: Value = serde_json::from_slice(&preview_body).unwrap();
    let flow_run_id = preview_payload["data"]["flow_run"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let flow_run_uuid = Uuid::parse_str(&flow_run_id).unwrap();
    let node_run_id =
        Uuid::parse_str(preview_payload["data"]["node_run"]["id"].as_str().unwrap()).unwrap();
    let pool = sqlx::PgPool::connect(&database_url).await.unwrap();
    let debug_payload = json!({});
    let output_payload = json!({
        "text": "很好，图片分析出来了！这是一个 1flowbase 的顶部导航栏。",
        "usage": {
            "total_tokens": 128
        }
    });
    sqlx::query("update node_runs set debug_payload = $2, output_payload = $3 where id = $1")
        .bind(node_run_id)
        .bind(&debug_payload)
        .bind(&output_payload)
        .execute(&pool)
        .await
        .unwrap();
    <MainDurableStore as OrchestrationRuntimeRepository>::append_runtime_events(
        &state.store,
        &[
            AppendRuntimeEventInput {
                flow_run_id: flow_run_uuid,
                node_run_id: Some(node_run_id),
                span_id: None,
                parent_span_id: None,
                event_type: "visible_internal_llm_tool_started".to_string(),
                layer: domain::RuntimeEventLayer::AgentTransition,
                source: domain::RuntimeEventSource::Host,
                trust_level: domain::RuntimeTrustLevel::HostFact,
                item_id: None,
                ledger_ref: None,
                payload: json!({
                    "event_type": "visible_internal_llm_tool_started",
                    "tool_call_id": "call-image",
                    "tool_name": "image_llm",
                    "main_node_id": "node-llm",
                    "target_node_id": "node-llm-1",
                    "arguments": {
                        "task": "描述图片里的导航栏",
                        "media": [
                            {
                                "kind": "image",
                                "path": "uploads/image-1.png",
                                "source": "workspace_path"
                            }
                        ]
                    }
                }),
                visibility: domain::RuntimeEventVisibility::Workspace,
                durability: domain::RuntimeEventDurability::Durable,
            },
            AppendRuntimeEventInput {
                flow_run_id: flow_run_uuid,
                node_run_id: Some(node_run_id),
                span_id: None,
                parent_span_id: None,
                event_type: "visible_internal_llm_tool_completed".to_string(),
                layer: domain::RuntimeEventLayer::AgentTransition,
                source: domain::RuntimeEventSource::Host,
                trust_level: domain::RuntimeTrustLevel::HostFact,
                item_id: None,
                ledger_ref: None,
                payload: json!({
                    "event_type": "visible_internal_llm_tool_completed",
                    "tool_call_id": "call-image",
                    "tool_name": "image_llm",
                    "main_node_id": "node-llm",
                    "target_node_id": "node-llm-1",
                    "node_id": "node-llm-1",
                    "provider_route": {
                        "model": "mimo-v2.5",
                        "protocol": "anthropic_messages",
                        "provider_code": "anthropic",
                        "provider_instance_id": "provider-mimo"
                    }
                }),
                visibility: domain::RuntimeEventVisibility::Workspace,
                durability: domain::RuntimeEventDurability::Durable,
            },
        ],
    )
    .await
    .unwrap();
    let before_artifact_count = sqlx::query_scalar::<_, i64>(
        "select count(*) from runtime_debug_artifacts where flow_run_id = $1",
    )
    .bind(flow_run_uuid)
    .fetch_one(&pool)
    .await
    .unwrap();

    let trace_tree = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/console/applications/{application_id}/logs/runs/{flow_run_id}/trace-tree"
                ))
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(trace_tree.status(), StatusCode::OK);
    let trace_tree_body = to_bytes(trace_tree.into_body(), usize::MAX).await.unwrap();
    let trace_tree_payload: Value = serde_json::from_slice(&trace_tree_body).unwrap();
    let root_trace_node_id = trace_tree_payload["data"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|node| node["node_id"] == json!("node-llm"))
        .expect("trace tree should include the llm node")["trace_node_id"]
        .as_str()
        .unwrap();
    assert!(
        trace_tree_payload["data"]["nodes"][0]
            .get("debug_payload")
            .is_none(),
        "trace node summary must not include heavy debug payload"
    );

    let content_payload = get_console_json(
        &app,
        &cookie,
        format!(
            "/api/console/applications/{application_id}/logs/runs/{flow_run_id}/trace-tree/nodes/{root_trace_node_id}/content"
        ),
    )
    .await;
    assert!(
        content_payload["data"]["payload"].get("node_run").is_none(),
        "trace node content should advertise detail refs instead of materializing node_run"
    );
    let node_run_detail_ref_id = content_payload["data"]["detail_refs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|detail_ref| detail_ref["detail_kind"] == json!("node_run"))
        .expect("node_run detail ref should be present")["detail_ref_id"]
        .as_str()
        .unwrap();
    let node_run_detail_payload = get_console_json(
        &app,
        &cookie,
        format!(
            "/api/console/applications/{application_id}/logs/runs/{flow_run_id}/trace-tree/nodes/{root_trace_node_id}/details/{node_run_detail_ref_id}"
        ),
    )
    .await;
    assert!(
        node_run_detail_payload["data"]["payload"]["node_run"]["debug_payload"]
            .get("visible_internal_llm_tool_trace")
            .is_none(),
        "visible internal tool trace should be represented by lazy tool children"
    );

    let root_children_payload = get_console_json(
        &app,
        &cookie,
        format!(
            "/api/console/applications/{application_id}/logs/runs/{flow_run_id}/trace-tree/nodes?parent_trace_node_id={root_trace_node_id}"
        ),
    )
    .await;
    let tool_group_id = root_children_payload["data"]["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|node| node["node_kind"] == json!("tool_group"))
        .expect("visible internal route trace should synthesize a tools group")["trace_node_id"]
        .as_str()
        .unwrap();
    let tool_children_payload = get_console_json(
        &app,
        &cookie,
        format!(
            "/api/console/applications/{application_id}/logs/runs/{flow_run_id}/trace-tree/nodes?parent_trace_node_id={tool_group_id}"
        ),
    )
    .await;
    let tool_trace_node_id = tool_children_payload["data"]["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|node| node["node_alias"] == json!("image_llm"))
        .expect("visible internal route trace should expose the tool callback node")
        ["trace_node_id"]
        .as_str()
        .unwrap();
    let tool_content_payload = get_console_json(
        &app,
        &cookie,
        format!(
            "/api/console/applications/{application_id}/logs/runs/{flow_run_id}/trace-tree/nodes/{tool_trace_node_id}/content"
        ),
    )
    .await;
    let trace = &tool_content_payload["data"]["payload"]["route_trace"];
    assert_eq!(trace["kind"], json!("visible_internal_llm_tool_trace"));
    assert_eq!(trace["tool_name"], json!("image_llm"));
    assert_eq!(trace["status"], json!("succeeded"));
    assert_eq!(trace["route_model"], json!("mimo-v2.5"));
    assert_eq!(trace["returned_to_main"], json!(true));
    assert_eq!(trace["main_resume"], json!(true));
    assert_eq!(trace["route_output_summary"]["preview"], json!("null"));
    assert_eq!(
        trace["final_output_summary"]["preview"],
        json!("很好，图片分析出来了！这是一个 1flowbase 的顶部导航栏。")
    );
    assert!(trace.get("artifact_ref").is_none());

    let scoped_node = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/console/applications/{application_id}/logs/runs/{flow_run_id}/nodes/node-llm"
                ))
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(scoped_node.status(), StatusCode::OK);
    let scoped_node_body = to_bytes(scoped_node.into_body(), usize::MAX).await.unwrap();
    let scoped_node_payload: Value = serde_json::from_slice(&scoped_node_body).unwrap();
    assert_eq!(
        scoped_node_payload["data"]["node_run"]["debug_payload"]["visible_internal_llm_tool_trace"]
            [0]["route_model"],
        json!("mimo-v2.5")
    );

    let after_artifact_count = sqlx::query_scalar::<_, i64>(
        "select count(*) from runtime_debug_artifacts where flow_run_id = $1",
    )
    .bind(flow_run_uuid)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(after_artifact_count, before_artifact_count);
}

#[tokio::test]
async fn application_runtime_routes_trace_tree_stitches_prior_claude_code_tool_runs_lazily() {
    // Mirrors #882 samples:
    // Run A 019ebed1-9f5b-77d0-810c-7227d1b5f7e3,
    // Run B 019ebed3-47ad-7921-8b74-a7def920d9db,
    // Run C 019ebed3-d60a-71d0-9b81-0234a0b89147.
    let (state, database_url) = test_api_state_with_database_url().await;
    let app = crate::app_with_state_and_config(state.clone(), &test_config());
    let (cookie, csrf) = login_and_capture_cookie(&app, "root", "change-me").await;
    let provider_instance_id = create_ready_provider_instance(&app, &cookie, &csrf).await;
    let application_id =
        seed_agent_flow_application(&app, &cookie, &csrf, &provider_instance_id).await;

    let run_a_id = start_full_debug_run(
        &app,
        &cookie,
        &csrf,
        &application_id,
        "来 uploads\\test-01.png 找一下这幅图相关代码",
    )
    .await;
    let run_b_id = start_full_debug_run(
        &app,
        &cookie,
        &csrf,
        &application_id,
        "No matches found\n-rw-r--r-- 1 Lw 197121 uploads/test-01.png\nTook a screenshot of the current page's viewport.",
    )
    .await;
    let run_c_id = start_full_debug_run(
        &app,
        &cookie,
        &csrf,
        &application_id,
        "File does not exist. Note: your current working directory is E:\\code\\taichuCode\\1flowbase.\nFound 4 files\nweb\\app\\src\\routes\\route-config.ts\nweb\\app\\src\\app-shell\\Navigation.tsx",
    )
    .await;

    let run_a_uuid = Uuid::parse_str(&run_a_id).unwrap();
    let run_b_uuid = Uuid::parse_str(&run_b_id).unwrap();
    let run_c_uuid = Uuid::parse_str(&run_c_id).unwrap();
    wait_for_run_detail(
        &app,
        &cookie,
        &application_id,
        &run_a_id,
        &["succeeded", "failed", "cancelled"],
    )
    .await;
    wait_for_run_detail(
        &app,
        &cookie,
        &application_id,
        &run_b_id,
        &["succeeded", "failed", "cancelled"],
    )
    .await;
    wait_for_run_detail(
        &app,
        &cookie,
        &application_id,
        &run_c_id,
        &["succeeded", "failed", "cancelled"],
    )
    .await;

    let pool = sqlx::PgPool::connect(&database_url).await.unwrap();
    let external_user = "claude-code-user-fixture";
    let external_conversation_id = "claude-code-session-fixture";
    for run_id in [run_a_uuid, run_b_uuid, run_c_uuid] {
        sqlx::query(
            r#"
            update flow_runs
            set external_user = $2,
                external_conversation_id = $3,
                compatibility_mode = 'anthropic-messages-v1'
            where id = $1
            "#,
        )
        .bind(run_id)
        .bind(external_user)
        .bind(external_conversation_id)
        .execute(&pool)
        .await
        .unwrap();
    }
    for run_id in [run_a_uuid, run_b_uuid] {
        <MainDurableStore as OrchestrationRuntimeRepository>::update_flow_run(
            &state.store,
            &UpdateFlowRunInput {
                flow_run_id: run_id,
                status: domain::FlowRunStatus::Cancelled,
                output_payload: json!({}),
                error_payload: None,
                finished_at: Some(time::OffsetDateTime::now_utc()),
            },
        )
        .await
        .unwrap();
    }

    let run_a_llm_node_run_id = latest_llm_node_run_id(&pool, run_a_uuid).await;
    let run_b_llm_node_run_id = latest_llm_node_run_id(&pool, run_b_uuid).await;
    let _run_a_callback_task =
        <MainDurableStore as OrchestrationRuntimeRepository>::create_callback_task(
            &state.store,
            &CreateCallbackTaskInput {
                flow_run_id: run_a_uuid,
                node_run_id: run_a_llm_node_run_id,
                callback_kind: "llm_tool_calls".to_string(),
                request_payload: json!({
                    "tool_calls": [
                        { "id": "call_image", "name": "image_llm" },
                        { "id": "call_glob", "name": "Glob" }
                    ]
                }),
                external_ref_payload: None,
            },
        )
        .await
        .unwrap();
    let _run_b_callback_task =
        <MainDurableStore as OrchestrationRuntimeRepository>::create_callback_task(
            &state.store,
            &CreateCallbackTaskInput {
                flow_run_id: run_b_uuid,
                node_run_id: run_b_llm_node_run_id,
                callback_kind: "llm_tool_calls".to_string(),
                request_payload: json!({
                    "tool_calls": [
                        { "id": "call_grep", "name": "Grep" },
                        { "id": "call_read", "name": "Read" }
                    ]
                }),
                external_ref_payload: None,
            },
        )
        .await
        .unwrap();
    <MainDurableStore as OrchestrationRuntimeRepository>::append_runtime_events(
        &state.store,
        &[
            AppendRuntimeEventInput {
                flow_run_id: run_a_uuid,
                node_run_id: Some(run_a_llm_node_run_id),
                span_id: None,
                parent_span_id: None,
                event_type: "visible_internal_llm_tool_started".to_string(),
                layer: domain::RuntimeEventLayer::AgentTransition,
                source: domain::RuntimeEventSource::Host,
                trust_level: domain::RuntimeTrustLevel::HostFact,
                item_id: None,
                ledger_ref: None,
                payload: json!({
                    "tool_call_id": "call_image",
                    "tool_name": "image_llm",
                    "main_node_id": "node-llm",
                    "target_node_id": "node-llm-image",
                    "arguments": {
                        "media": [
                            {
                                "kind": "image",
                                "path": "uploads/test-01.png",
                                "source": "workspace_path"
                            }
                        ]
                    }
                }),
                visibility: domain::RuntimeEventVisibility::Workspace,
                durability: domain::RuntimeEventDurability::Durable,
            },
            AppendRuntimeEventInput {
                flow_run_id: run_a_uuid,
                node_run_id: Some(run_a_llm_node_run_id),
                span_id: None,
                parent_span_id: None,
                event_type: "visible_internal_llm_tool_completed".to_string(),
                layer: domain::RuntimeEventLayer::AgentTransition,
                source: domain::RuntimeEventSource::Host,
                trust_level: domain::RuntimeTrustLevel::HostFact,
                item_id: None,
                ledger_ref: None,
                payload: json!({
                    "tool_call_id": "call_image",
                    "tool_name": "image_llm",
                    "main_node_id": "node-llm",
                    "target_node_id": "node-llm-image",
                    "provider_route": {
                        "model": "mimo-v2.5",
                        "protocol": "anthropic_messages"
                    },
                    "content": "图片是 1flowbase 顶部导航栏"
                }),
                visibility: domain::RuntimeEventVisibility::Workspace,
                durability: domain::RuntimeEventDurability::Durable,
            },
        ],
    )
    .await
    .unwrap();

    let trace_tree = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/console/applications/{application_id}/logs/runs/{run_c_id}/trace-tree"
                ))
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(trace_tree.status(), StatusCode::OK);
    let trace_tree_body = to_bytes(trace_tree.into_body(), usize::MAX).await.unwrap();
    let trace_tree_payload: Value = serde_json::from_slice(&trace_tree_body).unwrap();
    let data = &trace_tree_payload["data"];

    assert_eq!(data["flow_run"]["id"], json!(run_c_id));
    assert!(
        data.get("stitched_trace").is_none(),
        "lazy trace tree should expose stitched nodes as expandable summaries, not full stitched detail"
    );
    let root_nodes = data["nodes"].as_array().unwrap();
    assert!(root_nodes
        .iter()
        .any(|node| node["flow_run_id"] == json!(run_c_id)));
    assert!(root_nodes
        .iter()
        .all(|node| node["flow_run_id"] != json!(run_a_id)));
    assert!(root_nodes
        .iter()
        .all(|node| node["flow_run_id"] != json!(run_b_id)));
    assert!(root_nodes
        .iter()
        .all(|node| node.get("input_payload").is_none()));
    assert!(root_nodes
        .iter()
        .all(|node| node.get("debug_payload").is_none()));

    let stitched_group = root_nodes
        .iter()
        .find(|node| node["node_kind"] == json!("stitched_context"))
        .expect("stitched context group should be present at root");
    let stitched_group_id = stitched_group["trace_node_id"].as_str().unwrap();
    assert_eq!(stitched_group["has_children"].as_bool(), Some(true));

    let stitched_children = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/console/applications/{application_id}/logs/runs/{run_c_id}/trace-tree/nodes?parent_trace_node_id={stitched_group_id}"
                ))
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(stitched_children.status(), StatusCode::OK);
    let stitched_children_body = to_bytes(stitched_children.into_body(), usize::MAX)
        .await
        .unwrap();
    let stitched_children_payload: Value = serde_json::from_slice(&stitched_children_body).unwrap();
    let stitched_runs = stitched_children_payload["data"]["items"]
        .as_array()
        .unwrap();
    assert_eq!(stitched_runs.len(), 2);
    assert!(stitched_runs
        .iter()
        .all(|node| node["node_kind"] == json!("stitched_run")));
    let run_a_trace_node_id = stitched_runs[0]["trace_node_id"].as_str().unwrap();

    let run_a_node_content = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/console/applications/{application_id}/logs/runs/{run_c_id}/trace-tree/nodes/{run_a_trace_node_id}/content"
                ))
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(run_a_node_content.status(), StatusCode::OK);
    let run_a_node_content_body = to_bytes(run_a_node_content.into_body(), usize::MAX)
        .await
        .unwrap();
    let run_a_node_content_payload: Value =
        serde_json::from_slice(&run_a_node_content_body).unwrap();
    assert_eq!(
        run_a_node_content_payload["data"]["payload"]["id"],
        json!(run_a_id)
    );
}

async fn start_full_debug_run(
    app: &axum::Router,
    cookie: &str,
    csrf: &str,
    application_id: &str,
    query: &str,
) -> String {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/console/applications/{application_id}/orchestration/debug-runs"
                ))
                .header("cookie", cookie)
                .header("x-csrf-token", csrf)
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "input_payload": {
                            "node-start": {
                                "query": query
                            }
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
    payload["data"]["flow_run"]["id"]
        .as_str()
        .unwrap()
        .to_string()
}

async fn latest_llm_node_run_id(pool: &sqlx::PgPool, flow_run_id: Uuid) -> Uuid {
    sqlx::query_scalar(
        r#"
        select id
        from node_runs
        where flow_run_id = $1
          and node_type = 'llm'
        order by started_at desc, id desc
        limit 1
        "#,
    )
    .bind(flow_run_id)
    .fetch_one(pool)
    .await
    .unwrap()
}

#[tokio::test]
async fn application_runtime_routes_logs_are_paginated_and_newest_first() {
    let app = test_app().await;
    let (cookie, csrf) = login_and_capture_cookie(&app, "root", "change-me").await;
    let provider_instance_id = create_ready_provider_instance(&app, &cookie, &csrf).await;
    let application_id =
        seed_agent_flow_application(&app, &cookie, &csrf, &provider_instance_id).await;
    let mut flow_run_ids = Vec::new();

    for index in 0..25 {
        let create = app
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
                                "node-start": {
                                    "query": format!("run-{index:02}")
                                }
                            }
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(create.status(), StatusCode::CREATED);
        let create_body = to_bytes(create.into_body(), usize::MAX).await.unwrap();
        let create_payload: Value = serde_json::from_slice(&create_body).unwrap();
        flow_run_ids.push(
            create_payload["data"]["flow_run"]["id"]
                .as_str()
                .unwrap()
                .to_string(),
        );
    }

    for flow_run_id in &flow_run_ids {
        wait_for_run_detail(
            &app,
            &cookie,
            &application_id,
            flow_run_id,
            &["succeeded", "failed", "cancelled"],
        )
        .await;
    }

    let list = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/console/applications/{application_id}/logs/runs?page=1&page_size=20"
                ))
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(list.status(), StatusCode::OK);
    let list_body = to_bytes(list.into_body(), usize::MAX).await.unwrap();
    let list_payload: Value = serde_json::from_slice(&list_body).unwrap();
    let items = list_payload["data"]["items"].as_array().unwrap();

    assert_eq!(list_payload["data"]["page"].as_i64(), Some(1));
    assert_eq!(list_payload["data"]["page_size"].as_i64(), Some(20));
    assert_eq!(list_payload["data"]["total"].as_i64(), Some(25));
    assert_eq!(items.len(), 20);
    assert_eq!(items[0]["id"].as_str(), Some(flow_run_ids[24].as_str()));
    assert_eq!(items[19]["id"].as_str(), Some(flow_run_ids[5].as_str()));
}
