use super::*;

#[tokio::test]
async fn application_runtime_routes_start_debug_run_and_resume_waiting_human() {
    let app = test_app().await;
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

    let start_status = start.status();
    let start_body = to_bytes(start.into_body(), usize::MAX).await.unwrap();
    assert_eq!(
        start_status,
        StatusCode::CREATED,
        "{}",
        String::from_utf8_lossy(&start_body)
    );
    let payload: Value = serde_json::from_slice(&start_body).unwrap();
    let run_id = payload["data"]["flow_run"]["id"].as_str().unwrap();
    assert_eq!(
        payload["data"]["flow_run"]["status"].as_str(),
        Some("running")
    );
    let detail =
        wait_for_run_detail(&app, &cookie, &application_id, run_id, &["waiting_human"]).await;
    let checkpoint_id = detail["checkpoints"][0]["id"].as_str().unwrap();

    let resume = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/console/applications/{application_id}/orchestration/runs/{run_id}/resume"
                ))
                .header("cookie", &cookie)
                .header("x-csrf-token", &csrf)
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "checkpoint_id": checkpoint_id,
                        "input_payload": {
                            "node-human": { "input": "已审核通过" }
                        }
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resume.status(), StatusCode::OK);
}

#[tokio::test]
async fn application_runtime_routes_cancel_waiting_flow_run() {
    let app = test_app().await;
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
    let waiting_detail =
        wait_for_run_detail(&app, &cookie, &application_id, run_id, &["waiting_human"]).await;
    assert_eq!(
        waiting_detail["flow_run"]["status"].as_str(),
        Some("waiting_human")
    );

    let cancel = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/console/applications/{application_id}/orchestration/runs/{run_id}/cancel"
                ))
                .header("cookie", &cookie)
                .header("x-csrf-token", &csrf)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(cancel.status(), StatusCode::OK);
    let cancel_body = to_bytes(cancel.into_body(), usize::MAX).await.unwrap();
    let cancel_payload: Value = serde_json::from_slice(&cancel_body).unwrap();
    assert_eq!(
        cancel_payload["data"]["flow_run"]["status"].as_str(),
        Some("cancelled")
    );
    assert!(cancel_payload["data"]["events"]
        .as_array()
        .unwrap()
        .iter()
        .any(|event| event["event_type"].as_str() == Some("flow_run_cancelled")));
}
