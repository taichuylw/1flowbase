use crate::_tests::support::{
    create_member, create_role, login_and_capture_cookie, replace_member_roles,
    replace_role_permissions, test_api_state_with_database_url, test_config,
};
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use control_plane::ports::{
    RuntimeEventDurability, RuntimeEventPayload, RuntimeEventSource, RuntimeEventStreamPolicy,
};
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

async fn response_json(response: axum::response::Response) -> Value {
    serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap()
}

async fn audit_event_count(database_url: &str, event_code: &str) -> i64 {
    let store = storage_durable::build_main_durable_postgres(database_url)
        .await
        .unwrap()
        .store;
    sqlx::query_scalar("select count(*) from audit_logs where event_code = $1")
        .bind(event_code)
        .fetch_one(store.pool())
        .await
        .unwrap()
}

async fn latest_audit_payload(database_url: &str, event_code: &str) -> Value {
    let store = storage_durable::build_main_durable_postgres(database_url)
        .await
        .unwrap()
        .store;
    sqlx::query_scalar(
        "select payload from audit_logs where event_code = $1 order by created_at desc limit 1",
    )
    .bind(event_code)
    .fetch_one(store.pool())
    .await
    .unwrap()
}

fn contract_summary<'a>(payload: &'a Value, contract_code: &str) -> &'a Value {
    payload["data"]["contracts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|contract| contract["contract_code"] == contract_code)
        .unwrap_or_else(|| panic!("missing memory contract summary for {contract_code}"))
}

#[tokio::test]
async fn host_infrastructure_memory_routes_list_categories_and_reveal_with_audit() {
    let (state, database_url) = test_api_state_with_database_url().await;
    state
        .infrastructure
        .cache_store()
        .set_json(
            "application-logs:run:1",
            json!({ "flow_run": { "status": "succeeded" } }),
            Some(time::Duration::seconds(60)),
        )
        .await
        .unwrap();
    state
        .infrastructure
        .rate_limit_store()
        .consume("login:root", 5, time::Duration::minutes(1))
        .await
        .unwrap();
    state
        .infrastructure
        .distributed_lock()
        .acquire("workflow:compile", "worker-1", time::Duration::minutes(5))
        .await
        .unwrap();
    state
        .infrastructure
        .task_queue()
        .enqueue("runtime", json!({ "job": "sync" }), Some("sync-1"))
        .await
        .unwrap();
    state
        .infrastructure
        .event_bus()
        .publish("runtime-events", json!({ "kind": "queued" }))
        .await
        .unwrap();
    let run_id = Uuid::now_v7();
    let runtime_event_stream = state.infrastructure.runtime_event_stream().unwrap();
    runtime_event_stream
        .open_run(run_id, RuntimeEventStreamPolicy::debug_default())
        .await
        .unwrap();
    runtime_event_stream
        .append(
            run_id,
            RuntimeEventPayload {
                event_type: "text_delta".to_string(),
                source: RuntimeEventSource::Runtime,
                durability: RuntimeEventDurability::Ephemeral,
                persist_required: false,
                trace_visible: true,
                payload: json!({ "delta": "hello", "delta_index": 1 }),
            },
        )
        .await
        .unwrap();

    let app = crate::app_with_state_and_config(state, &test_config());
    let (cookie, csrf) = login_and_capture_cookie(&app, "root", "change-me").await;

    let overview_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/console/settings/host-infrastructure/memory")
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(overview_response.status(), StatusCode::OK);
    let overview_payload = response_json(overview_response).await;
    assert_eq!(overview_payload["data"]["can_manage"], true);
    for contract_code in [
        "session-store",
        "cache-store",
        "rate-limit-store",
        "distributed-lock",
        "task-queue",
        "event-bus",
        "runtime-event-stream",
    ] {
        let summary = contract_summary(&overview_payload, contract_code);
        assert_eq!(summary["provider_code"], "local");
        assert_eq!(summary["supported"], true);
        assert!(summary["entry_count"].as_u64().unwrap() >= 1);
    }

    let entries_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/console/settings/host-infrastructure/memory/contracts/session-store/entries")
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(entries_response.status(), StatusCode::OK);
    let entries_payload = response_json(entries_response).await;
    let first_session = &entries_payload["data"]["entries"][0];
    assert_eq!(first_session["contract_code"], "session-store");
    assert_eq!(first_session["sensitive"], true);
    assert!(first_session.as_object().unwrap().get("value").is_none());
    let session_key = first_session["key"].as_str().unwrap();

    let reveal_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/console/settings/host-infrastructure/memory/contracts/session-store/entries/reveal")
                .header("cookie", &cookie)
                .header("x-csrf-token", &csrf)
                .header("content-type", "application/json")
                .body(Body::from(json!({ "key": session_key }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(reveal_response.status(), StatusCode::OK);
    let reveal_payload = response_json(reveal_response).await;
    assert_eq!(reveal_payload["data"]["value"]["session_id"], session_key);
    assert_eq!(
        audit_event_count(&database_url, "host_infrastructure.memory_value_revealed").await,
        1
    );
    let audit_payload =
        latest_audit_payload(&database_url, "host_infrastructure.memory_value_revealed").await;
    assert_eq!(audit_payload["contract_code"], "session-store");
    assert_eq!(audit_payload["key"], session_key);
    assert!(audit_payload.as_object().unwrap().get("value").is_none());
}

#[tokio::test]
async fn host_infrastructure_memory_routes_keep_viewer_metadata_only() {
    let (state, _database_url) = test_api_state_with_database_url().await;
    state
        .infrastructure
        .cache_store()
        .set_json("application-logs:run:1", json!({ "secret": true }), None)
        .await
        .unwrap();
    let app = crate::app_with_state_and_config(state, &test_config());
    let (root_cookie, root_csrf) = login_and_capture_cookie(&app, "root", "change-me").await;
    create_role(&app, &root_cookie, &root_csrf, "plugin_viewer").await;
    replace_role_permissions(
        &app,
        &root_cookie,
        &root_csrf,
        "plugin_viewer",
        &["plugin_config.view.all"],
    )
    .await;
    let member_id =
        create_member(&app, &root_cookie, &root_csrf, "memory-viewer", "change-me").await;
    replace_member_roles(
        &app,
        &root_cookie,
        &root_csrf,
        &member_id,
        &["plugin_viewer"],
    )
    .await;
    let (viewer_cookie, viewer_csrf) =
        login_and_capture_cookie(&app, "memory-viewer", "change-me").await;

    let overview_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/console/settings/host-infrastructure/memory")
                .header("cookie", &viewer_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(overview_response.status(), StatusCode::OK);
    let overview_payload = response_json(overview_response).await;
    assert_eq!(overview_payload["data"]["can_manage"], false);

    let entries_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/console/settings/host-infrastructure/memory/contracts/cache-store/entries")
                .header("cookie", &viewer_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(entries_response.status(), StatusCode::OK);
    let entries_payload = response_json(entries_response).await;
    let cache_key = entries_payload["data"]["entries"][0]["key"]
        .as_str()
        .unwrap();

    let reveal_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/console/settings/host-infrastructure/memory/contracts/cache-store/entries/reveal")
                .header("cookie", &viewer_cookie)
                .header("x-csrf-token", &viewer_csrf)
                .header("content-type", "application/json")
                .body(Body::from(json!({ "key": cache_key }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(reveal_response.status(), StatusCode::FORBIDDEN);
}
