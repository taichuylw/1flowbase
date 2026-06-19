use super::*;

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
