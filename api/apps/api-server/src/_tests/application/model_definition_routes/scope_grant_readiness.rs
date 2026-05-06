use super::*;

#[tokio::test]
async fn model_definition_routes_show_not_exposed_for_stored_ready_or_no_permission_without_api_key(
) {
    let (app, database_url) = test_app_with_database_url().await;
    let (cookie, csrf) = login_and_capture_cookie(&app, "root", "change-me").await;

    for (code, stored_status) in [
        ("stored_ready_without_key_orders", "api_exposed_ready"),
        (
            "stored_no_permission_without_key_orders",
            "api_exposed_no_permission",
        ),
    ] {
        let create_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/console/models")
                    .header("cookie", &cookie)
                    .header("x-csrf-token", &csrf)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "scope_kind": "workspace",
                            "code": code,
                            "title": code
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(create_response.status(), StatusCode::CREATED);
        let created: serde_json::Value = serde_json::from_slice(
            &to_bytes(create_response.into_body(), usize::MAX)
                .await
                .unwrap(),
        )
        .unwrap();
        let model_id = created["data"]["id"].as_str().unwrap();
        set_stored_api_exposure_status(&database_url, model_id, stored_status).await;

        let get_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/api/console/models/{model_id}"))
                    .header("cookie", &cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(get_response.status(), StatusCode::OK);
        let payload: serde_json::Value = serde_json::from_slice(
            &to_bytes(get_response.into_body(), usize::MAX)
                .await
                .unwrap(),
        )
        .unwrap();
        assert_eq!(
            payload["data"]["api_exposure_status"],
            json!("published_not_exposed")
        );
    }
}

#[tokio::test]
async fn model_definition_scope_grant_routes_audit_and_update_runtime_readiness() {
    let (app, database_url) = test_app_with_database_url().await;
    let (cookie, csrf) = login_and_capture_cookie(&app, "root", "change-me").await;

    let create_model_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/console/models")
                .header("cookie", &cookie)
                .header("x-csrf-token", &csrf)
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "scope_kind": "workspace",
                        "code": "scope_grant_route_orders",
                        "title": "Scope Grant Route Orders"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create_model_response.status(), StatusCode::CREATED);
    let created: serde_json::Value = serde_json::from_slice(
        &to_bytes(create_model_response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    let model_id = created["data"]["id"].as_str().unwrap().to_string();

    let create_grant_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/console/models/{model_id}/scope-grants"))
                .header("cookie", &cookie)
                .header("x-csrf-token", &csrf)
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "scope_kind": "system",
                        "scope_id": domain::SYSTEM_SCOPE_ID,
                        "enabled": true,
                        "permission_profile": "scope_all"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create_grant_response.status(), StatusCode::CREATED);
    let grant_payload: serde_json::Value = serde_json::from_slice(
        &to_bytes(create_grant_response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    let grant_id = grant_payload["data"]["id"].as_str().unwrap().to_string();

    let list_grants_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/console/models/{model_id}/scope-grants"))
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(list_grants_response.status(), StatusCode::OK);
    let list_grants_payload: serde_json::Value = serde_json::from_slice(
        &to_bytes(list_grants_response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    assert!(list_grants_payload["data"]
        .as_array()
        .unwrap()
        .iter()
        .any(|grant| {
            grant["id"].as_str() == Some(&grant_id)
                && grant["data_model_id"].as_str() == Some(&model_id)
                && grant["scope_kind"].as_str() == Some("system")
                && grant["permission_profile"].as_str() == Some("scope_all")
        }));

    let system_key_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/console/api-keys")
                .header("cookie", &cookie)
                .header("x-csrf-token", &csrf)
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "name": "scope grant route system key",
                        "scope_kind": "system",
                        "scope_id": domain::SYSTEM_SCOPE_ID,
                        "permissions": [
                            {
                                "data_model_id": model_id,
                                "list": true,
                                "get": false,
                                "create": false,
                                "update": false,
                                "delete": false
                            }
                        ]
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(system_key_response.status(), StatusCode::CREATED);
    let system_key_payload: serde_json::Value = serde_json::from_slice(
        &to_bytes(system_key_response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    let system_token = system_key_payload["data"]["token"].as_str().unwrap();

    let ready_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/console/models/{model_id}"))
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(ready_response.status(), StatusCode::OK);
    let ready_payload: serde_json::Value = serde_json::from_slice(
        &to_bytes(ready_response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        ready_payload["data"]["api_exposure_status"],
        json!("api_exposed_ready")
    );
    let runtime_ready_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/runtime/models/scope_grant_route_orders/records")
                .header("authorization", format!("Bearer {system_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(runtime_ready_response.status(), StatusCode::OK);

    let update_grant_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!(
                    "/api/console/models/{model_id}/scope-grants/{grant_id}"
                ))
                .header("cookie", &cookie)
                .header("x-csrf-token", &csrf)
                .header("content-type", "application/json")
                .body(Body::from(json!({ "enabled": false }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(update_grant_response.status(), StatusCode::OK);

    let no_permission_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/console/models/{model_id}"))
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(no_permission_response.status(), StatusCode::OK);
    let no_permission_payload: serde_json::Value = serde_json::from_slice(
        &to_bytes(no_permission_response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        no_permission_payload["data"]["api_exposure_status"],
        json!("api_exposed_no_permission")
    );

    let reenable_grant_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!(
                    "/api/console/models/{model_id}/scope-grants/{grant_id}"
                ))
                .header("cookie", &cookie)
                .header("x-csrf-token", &csrf)
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "enabled": true,
                        "permission_profile": "owner"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(reenable_grant_response.status(), StatusCode::OK);

    let delete_grant_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!(
                    "/api/console/models/{model_id}/scope-grants/{grant_id}"
                ))
                .header("cookie", &cookie)
                .header("x-csrf-token", &csrf)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(delete_grant_response.status(), StatusCode::OK);

    let after_delete_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/console/models/{model_id}"))
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(after_delete_response.status(), StatusCode::OK);
    let after_delete_payload: serde_json::Value = serde_json::from_slice(
        &to_bytes(after_delete_response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        after_delete_payload["data"]["api_exposure_status"],
        json!("api_exposed_no_permission")
    );
    let runtime_after_delete_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/runtime/models/scope_grant_route_orders/records")
                .header("authorization", format!("Bearer {system_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let runtime_after_delete_status = runtime_after_delete_response.status();
    let runtime_after_delete_payload: serde_json::Value = serde_json::from_slice(
        &to_bytes(runtime_after_delete_response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        runtime_after_delete_status,
        StatusCode::FORBIDDEN,
        "unexpected runtime delete payload: {runtime_after_delete_payload}"
    );

    assert!(audit_event_count(&database_url, "state_model.scope_grant_created").await >= 2);
    assert_eq!(
        audit_event_count(&database_url, "state_model.scope_grant_updated").await,
        2
    );
    assert_eq!(
        audit_event_count(&database_url, "state_model.scope_grant_deleted").await,
        1
    );
}

#[tokio::test]
async fn model_definition_routes_do_not_mark_system_all_api_key_path_ready() {
    let (app, database_url) = test_app_with_database_url().await;
    let (cookie, csrf) = login_and_capture_cookie(&app, "root", "change-me").await;

    let create_model_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/console/models")
                .header("cookie", &cookie)
                .header("x-csrf-token", &csrf)
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "scope_kind": "system",
                        "code": "system_all_route_orders",
                        "title": "System All Route Orders"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create_model_response.status(), StatusCode::CREATED);
    let created: serde_json::Value = serde_json::from_slice(
        &to_bytes(create_model_response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    let model_id = created["data"]["id"].as_str().unwrap().to_string();

    set_model_grant_permission_profile(&database_url, &model_id, "system_all").await;

    let system_key_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/console/api-keys")
                .header("cookie", &cookie)
                .header("x-csrf-token", &csrf)
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "name": "system all non-root key",
                        "scope_kind": "system",
                        "scope_id": domain::SYSTEM_SCOPE_ID,
                        "permissions": [
                            {
                                "data_model_id": model_id,
                                "list": true,
                                "get": false,
                                "create": false,
                                "update": false,
                                "delete": false
                            }
                        ]
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(system_key_response.status(), StatusCode::CREATED);
    let system_key_payload: serde_json::Value = serde_json::from_slice(
        &to_bytes(system_key_response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    let token = system_key_payload["data"]["token"].as_str().unwrap();

    let ready_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/console/models/{model_id}"))
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(ready_response.status(), StatusCode::OK);
    let ready_payload: serde_json::Value = serde_json::from_slice(
        &to_bytes(ready_response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        ready_payload["data"]["api_exposure_status"],
        json!("api_exposed_no_permission")
    );

    let runtime_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/runtime/models/system_all_route_orders/records")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let runtime_status = runtime_response.status();
    let runtime_payload: serde_json::Value = serde_json::from_slice(
        &to_bytes(runtime_response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        runtime_status,
        StatusCode::FORBIDDEN,
        "unexpected runtime payload: {runtime_payload}"
    );
    assert_eq!(
        runtime_payload["code"],
        json!("system_all_requires_system_actor")
    );
    assert_eq!(
        audit_event_count(&database_url, "state_model.api_key_runtime_access_denied").await,
        1
    );
}
