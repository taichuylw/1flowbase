use super::*;
use control_plane::ports::{OrchestrationRuntimeRepository, UpdateFlowRunInput};
use storage_durable::MainDurableStore;

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

    let detail = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/console/applications/{application_id}/logs/runs/{flow_run_id}"
                ))
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(detail.status(), StatusCode::OK);
    let detail_body = to_bytes(detail.into_body(), usize::MAX).await.unwrap();
    let detail_payload: Value = serde_json::from_slice(&detail_body).unwrap();
    assert_eq!(
        detail_payload["data"]["flow_run"]["id"].as_str(),
        Some(flow_run_id.as_str())
    );
    assert_eq!(
        detail_payload["data"]["run"]["id"].as_str(),
        Some(flow_run_id.as_str())
    );
    assert_eq!(
        detail_payload["data"]["run"]["application_type"].as_str(),
        Some("agent_flow")
    );
    assert_eq!(
        detail_payload["data"]["run"]["run_object_kind"].as_str(),
        Some("application_run")
    );
    assert_eq!(
        detail_payload["data"]["detail"]["kind"].as_str(),
        Some("agent_flow")
    );
    assert_eq!(
        detail_payload["data"]["detail"]["flow_run"]["id"].as_str(),
        Some(flow_run_id.as_str())
    );
    assert_eq!(
        detail_payload["data"]["flow_run"]["title"].as_str(),
        Some("总结退款政策")
    );
    assert_eq!(
        detail_payload["data"]["node_runs"][0]["node_alias"].as_str(),
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

    let detail = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/console/applications/{application_id}/logs/runs/{flow_run_id}"
                ))
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(detail.status(), StatusCode::OK);
    let detail_body = to_bytes(detail.into_body(), usize::MAX).await.unwrap();
    let detail_payload: Value = serde_json::from_slice(&detail_body).unwrap();
    assert_eq!(
        detail_payload["data"]["flow_run"]["title"].as_str(),
        Some("公开 API 退款总结")
    );
    assert_eq!(
        detail_payload["data"]["flow_run"]["expand_id"].as_str(),
        Some("customer-42")
    );
    assert_eq!(
        detail_payload["data"]["flow_run"]["authorized_account"].as_str(),
        Some("root")
    );
    assert_eq!(
        detail_payload["data"]["run"]["source"].as_str(),
        Some("public_api")
    );
    assert_eq!(
        detail_payload["data"]["run"]["compatibility_mode"].as_str(),
        Some("native-v1")
    );
    assert!(!detail_payload["data"]["run"]
        .as_object()
        .unwrap()
        .contains_key("protocol"));
    assert_eq!(
        detail_payload["data"]["run"]["correlation"]["external_user"].as_str(),
        Some("customer-42")
    );
    assert!(detail_payload["data"]["run"]["correlation"]["api_key_id"].is_string());
    assert!(detail_payload["data"]["run"]["correlation"]["publication_version_id"].is_string());
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

    let detail = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/console/applications/{application_id}/logs/runs/{flow_run_id}"
                ))
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(detail.status(), StatusCode::OK);
    let detail_body = to_bytes(detail.into_body(), usize::MAX).await.unwrap();
    let detail_payload: Value = serde_json::from_slice(&detail_body).unwrap();
    assert_eq!(detail_payload["data"]["statistics"], expected_statistics);
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
