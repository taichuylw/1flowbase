use super::*;
use control_plane::ports::{UpdateNodeRunInput, UpsertApplicationRunTraceProjectionStatusInput};
use std::io::{Cursor, Read};

async fn get_run_export(
    app: &axum::Router,
    cookie: &str,
    application_id: &str,
    run_id: &str,
) -> (StatusCode, axum::http::HeaderMap, Vec<u8>) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/console/applications/{application_id}/logs/runs/{run_id}/export"
                ))
                .header("cookie", cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap()
        .to_vec();

    (status, headers, body)
}

fn read_zip_entries(body: &[u8]) -> Vec<(String, Vec<u8>)> {
    let mut archive = zip::ZipArchive::new(Cursor::new(body)).unwrap();
    let mut entries = Vec::with_capacity(archive.len());

    for index in 0..archive.len() {
        let mut file = archive.by_index(index).unwrap();
        let mut contents = Vec::new();
        file.read_to_end(&mut contents).unwrap();
        entries.push((file.name().to_string(), contents));
    }

    entries
}

async fn wait_for_node_run_error_code(
    pool: &sqlx::PgPool,
    node_run_id: Uuid,
    expected_code: &str,
) -> Value {
    let mut last_payload = Value::Null;
    for _ in 0..200 {
        let payload = sqlx::query_scalar::<_, Value>(
            r#"
            select coalesce(error_payload, 'null'::jsonb)
            from node_runs
            where id = $1
              and status = 'failed'
            "#,
        )
        .bind(node_run_id)
        .fetch_optional(pool)
        .await
        .unwrap();

        if let Some(payload) = payload {
            if payload.get("code").and_then(Value::as_str) == Some(expected_code) {
                return payload;
            }
            last_payload = payload;
        }

        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }

    panic!(
        "timed out waiting for node run {node_run_id} failed error code {expected_code}, last payload: {last_payload}"
    );
}

#[tokio::test]
async fn application_runtime_routes_logs_export_json_trace_dump_preserves_detail_and_error_payload()
{
    let (state, database_url) = test_api_state_with_database_url().await;
    let app = crate::app_with_state_and_config(state.clone(), &test_config());
    let (cookie, csrf) = login_and_capture_cookie(&app, "root", "change-me").await;
    let provider_instance_id = create_ready_provider_instance(&app, &cookie, &csrf).await;
    let application_id =
        seed_agent_flow_application(&app, &cookie, &csrf, &provider_instance_id).await;
    let run_id =
        start_full_debug_run(&app, &cookie, &csrf, &application_id, "导出失败节点错误").await;
    wait_for_run_detail(
        &app,
        &cookie,
        &application_id,
        &run_id,
        &["succeeded", "failed", "cancelled"],
    )
    .await;

    let pool = sqlx::PgPool::connect(&database_url).await.unwrap();
    let run_uuid = Uuid::parse_str(&run_id).unwrap();
    let llm_node_run_id = latest_llm_node_run_id(&pool, run_uuid).await;
    <MainDurableStore as OrchestrationRuntimeRepository>::update_node_run(
        &state.store,
        &UpdateNodeRunInput {
            node_run_id: llm_node_run_id,
            status: domain::NodeRunStatus::Failed,
            output_payload: json!({ "text": "partial answer before failure" }),
            error_payload: Some(json!({
                "code": "provider_failed",
                "message": "fixture provider failure"
            })),
            metrics_payload: json!({ "usage": { "total_tokens": 12 } }),
            debug_payload: json!({
                "provider": "fixture_provider",
                "request_id": "req-export-failed-node"
            }),
            finished_at: Some(time::OffsetDateTime::now_utc()),
        },
    )
    .await
    .unwrap();
    let expected_error_payload =
        wait_for_node_run_error_code(&pool, llm_node_run_id, "provider_failed").await;

    let (status, headers, body) = get_run_export(&app, &cookie, &application_id, &run_id).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    assert!(
        headers
            .get(axum::http::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value.starts_with("application/json")),
        "export should return a JSON download"
    );
    let disposition = headers
        .get(axum::http::header::CONTENT_DISPOSITION)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();
    assert!(disposition.contains("attachment"));
    assert!(
        disposition.contains(".json"),
        "single-run export filename should end with .json: {disposition}"
    );

    let dump: Value = serde_json::from_slice(&body).unwrap();
    for field in [
        "export_version",
        "exported_at",
        "export_status",
        "export_warnings",
        "run",
        "statistics",
        "detail",
        "flow_run",
        "node_runs",
        "trace_tree",
    ] {
        assert!(
            dump.get(field).is_some(),
            "export dump must include root field {field}"
        );
    }
    assert_eq!(dump["run"]["id"], json!(run_id));
    assert_eq!(dump["flow_run"]["id"], json!(run_id));
    assert_eq!(dump["export_version"], json!(1));
    assert_eq!(dump["export_status"], json!("complete"));
    assert!(dump["exported_at"].is_string());
    assert!(dump["export_warnings"].as_array().is_some());
    assert!(dump["trace_tree"]["projection_status"]["projection_status"].is_string());
    assert!(dump["trace_tree"]["nodes"].as_array().is_some());

    let failed_node = dump["node_runs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|node_run| node_run["id"] == json!(llm_node_run_id.to_string()))
        .expect("failed node run should be exported");
    let failed_node_object = failed_node.as_object().unwrap();
    for field in [
        "input_payload",
        "input_payload_view",
        "debug_payload",
        "output_payload",
        "error_payload",
        "metrics_payload",
    ] {
        assert!(
            failed_node_object.contains_key(field),
            "node_runs[] must preserve NodeRunResponse field {field}"
        );
    }
    assert!(
        !failed_node_object.contains_key("summary"),
        "node_runs[] must not wrap NodeRunResponse in a summary object"
    );
    assert_eq!(
        failed_node["error_payload"], expected_error_payload,
        "failed node error payload should be exported"
    );
}

#[tokio::test]
async fn application_runtime_routes_logs_export_selected_runs_zip_uses_csrf_and_selected_order() {
    let (state, _) = test_api_state_with_database_url().await;
    let app = crate::app_with_state_and_config(state.clone(), &test_config());
    let (cookie, csrf) = login_and_capture_cookie(&app, "root", "change-me").await;
    let provider_instance_id = create_ready_provider_instance(&app, &cookie, &csrf).await;
    let application_id =
        seed_agent_flow_application(&app, &cookie, &csrf, &provider_instance_id).await;

    let first_run_id =
        start_full_debug_run(&app, &cookie, &csrf, &application_id, "first selected").await;
    let second_run_id =
        start_full_debug_run(&app, &cookie, &csrf, &application_id, "second selected").await;
    let unselected_run_id =
        start_full_debug_run(&app, &cookie, &csrf, &application_id, "not selected").await;
    for run_id in [&first_run_id, &second_run_id, &unselected_run_id] {
        wait_for_run_detail(
            &app,
            &cookie,
            &application_id,
            run_id,
            &["succeeded", "failed", "cancelled"],
        )
        .await;
    }

    let body = json!({
        "run_ids": [second_run_id, first_run_id]
    });
    let missing_csrf = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/console/applications/{application_id}/logs/runs/export"
                ))
                .header("cookie", &cookie)
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(missing_csrf.status(), StatusCode::UNAUTHORIZED);

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/console/applications/{application_id}/logs/runs/export"
                ))
                .header("cookie", &cookie)
                .header("x-csrf-token", &csrf)
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "run_ids": [second_run_id, first_run_id]
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap()
        .to_vec();
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    assert!(
        headers
            .get(axum::http::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value.starts_with("application/zip")),
        "selected export should return a zip archive"
    );

    let entries = read_zip_entries(&body);
    assert_eq!(entries.len(), 3, "manifest + two selected run dumps");
    assert_eq!(entries[0].0, "manifest.json");
    assert!(entries[1].0.starts_with("runs/001_"));
    assert!(entries[1].0.ends_with(".json"));
    assert!(entries[2].0.starts_with("runs/002_"));
    assert!(entries[2].0.ends_with(".json"));

    let manifest: Value = serde_json::from_slice(&entries[0].1).unwrap();
    assert_eq!(manifest["export_version"], json!(1));
    assert_eq!(manifest["export_status"], json!("complete"));
    assert_eq!(manifest["run_count"], json!(2));
    assert_eq!(
        manifest["selected_run_ids"],
        json!([second_run_id.as_str(), first_run_id.as_str()])
    );
    assert_eq!(
        manifest["entries"]
            .as_array()
            .unwrap()
            .iter()
            .map(|run| run["run_id"].as_str().unwrap())
            .collect::<Vec<_>>(),
        vec![second_run_id.as_str(), first_run_id.as_str()]
    );
    assert_eq!(manifest["entries"][0]["filename"], json!(entries[1].0));
    assert_eq!(manifest["entries"][1]["filename"], json!(entries[2].0));

    let second_dump: Value = serde_json::from_slice(&entries[1].1).unwrap();
    let first_dump: Value = serde_json::from_slice(&entries[2].1).unwrap();
    assert_eq!(second_dump["run"]["id"], json!(second_run_id));
    assert_eq!(first_dump["run"]["id"], json!(first_run_id));
    assert!(
        !String::from_utf8_lossy(&body).contains(&unselected_run_id),
        "zip archive must not include unselected runs"
    );
}

#[tokio::test]
async fn application_runtime_routes_logs_export_keeps_shape_when_projection_failed() {
    let (state, _) = test_api_state_with_database_url().await;
    let app = crate::app_with_state_and_config(state.clone(), &test_config());
    let (cookie, csrf) = login_and_capture_cookie(&app, "root", "change-me").await;
    let provider_instance_id = create_ready_provider_instance(&app, &cookie, &csrf).await;
    let application_id =
        seed_agent_flow_application(&app, &cookie, &csrf, &provider_instance_id).await;
    let run_id = start_full_debug_run(
        &app,
        &cookie,
        &csrf,
        &application_id,
        "projection failed export",
    )
    .await;
    wait_for_run_detail(
        &app,
        &cookie,
        &application_id,
        &run_id,
        &["succeeded", "failed", "cancelled"],
    )
    .await;

    let application_uuid = Uuid::parse_str(&application_id).unwrap();
    let run_uuid = Uuid::parse_str(&run_id).unwrap();
    let source_watermark =
        <MainDurableStore as OrchestrationRuntimeRepository>::get_application_run_trace_projection_source_watermark(
            &state.store,
            application_uuid,
            run_uuid,
        )
        .await
        .unwrap()
        .unwrap();
    <MainDurableStore as OrchestrationRuntimeRepository>::upsert_application_run_trace_projection_status(
        &state.store,
        &UpsertApplicationRunTraceProjectionStatusInput {
            flow_run_id: run_uuid,
            projection_version: control_plane::orchestration_runtime::trace_projection::APPLICATION_RUN_TRACE_PROJECTION_VERSION,
            status: domain::ApplicationRunTraceProjectionStatus::Failed,
            source_watermark,
            attempt_count: 2,
            last_attempt_at: Some(time::OffsetDateTime::now_utc()),
            last_success_at: None,
            diagnostic: Some(domain::ApplicationRunTraceProjectionDiagnostic {
                last_error_code: Some("fixture_projection_failed".to_string()),
                last_error_stage: Some("test".to_string()),
                last_error_source_kind: Some("trace_projection".to_string()),
                last_error_source_locator: Some(run_id.clone()),
                last_error_message: Some("projection failed in fixture".to_string()),
                last_error_ref: Some("fixture-error-ref".to_string()),
                retriable: true,
            }),
        },
    )
    .await
    .unwrap();

    let (status, _, body) = get_run_export(&app, &cookie, &application_id, &run_id).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let dump: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(dump["run"]["id"], json!(run_id));
    assert!(dump["statistics"].is_object());
    assert!(dump["detail"].is_object());
    assert!(dump["flow_run"].is_object());
    assert!(dump["node_runs"].as_array().is_some());
    assert_eq!(
        dump["trace_tree"]["projection_status"]["projection_status"],
        json!("failed")
    );
    assert_eq!(dump["trace_tree"]["nodes"], json!([]));
}
