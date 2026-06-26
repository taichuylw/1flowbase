use super::*;

#[tokio::test]
async fn runtime_model_routes_api_key_can_list_granted_records() {
    let app = test_app().await;
    let (cookie, csrf) = login_and_capture_cookie(&app, "root", "change-me").await;
    let model_code = "api_key_list_orders";
    let model_id =
        create_model_with_status(&app, &cookie, &csrf, model_code, Some("published")).await;
    create_text_field(&app, &cookie, &csrf, &model_id, "title").await;
    create_runtime_record(&app, &cookie, &csrf, model_code, "visible").await;

    let token = create_api_key(
        &app,
        &cookie,
        &csrf,
        "runtime list key",
        json!([
            {
                "data_model_id": model_id,
                "list": true,
                "get": false,
                "create": false,
                "update": false,
                "delete": false
            }
        ]),
    )
    .await;

    let (status, payload) = list_records_with_api_key(&app, model_code, &token).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(payload["data"]["total"], json!(1));
    assert_eq!(payload["data"]["items"][0]["title"], json!("visible"));
}

#[tokio::test]
async fn runtime_model_routes_user_api_key_uses_bound_user_role_permissions() {
    let app = test_app().await;
    let (cookie, csrf) = login_and_capture_cookie(&app, "root", "change-me").await;
    let model_code = "pat_user_role_orders";
    let model_id =
        create_model_with_status(&app, &cookie, &csrf, model_code, Some("published")).await;
    create_text_field(&app, &cookie, &csrf, &model_id, "title").await;
    create_runtime_record(&app, &cookie, &csrf, model_code, "pat-visible").await;

    let token = create_user_api_key(&app, &cookie, &csrf, "runtime pat").await;

    let (status, payload) = list_records_with_api_key(&app, model_code, &token).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(payload["data"]["total"], json!(1));
    assert_eq!(payload["data"]["items"][0]["title"], json!("pat-visible"));
}

#[tokio::test]
async fn runtime_model_routes_api_key_cannot_call_ungranted_data_model() {
    let app = test_app().await;
    let (cookie, csrf) = login_and_capture_cookie(&app, "root", "change-me").await;
    let granted_model_id = create_model_with_status(
        &app,
        &cookie,
        &csrf,
        "api_key_granted_orders",
        Some("published"),
    )
    .await;
    create_text_field(&app, &cookie, &csrf, &granted_model_id, "title").await;
    let denied_model_code = "api_key_ungranted_orders";
    let denied_model_id =
        create_model_with_status(&app, &cookie, &csrf, denied_model_code, Some("published")).await;
    create_text_field(&app, &cookie, &csrf, &denied_model_id, "title").await;

    let token = create_api_key(
        &app,
        &cookie,
        &csrf,
        "runtime ungranted key",
        json!([
            {
                "data_model_id": granted_model_id,
                "list": true,
                "get": false,
                "create": false,
                "update": false,
                "delete": false
            }
        ]),
    )
    .await;

    let (status, payload) = list_records_with_api_key(&app, denied_model_code, &token).await;

    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(payload["code"], json!("api_key_action_not_allowed"));
}

#[tokio::test]
async fn runtime_model_routes_api_key_cannot_call_disabled_action() {
    let app = test_app().await;
    let (cookie, csrf) = login_and_capture_cookie(&app, "root", "change-me").await;
    let model_code = "api_key_disabled_action_orders";
    let model_id =
        create_model_with_status(&app, &cookie, &csrf, model_code, Some("published")).await;
    create_text_field(&app, &cookie, &csrf, &model_id, "title").await;

    let token = create_api_key(
        &app,
        &cookie,
        &csrf,
        "runtime disabled action key",
        json!([
            {
                "data_model_id": model_id,
                "list": true,
                "get": false,
                "create": false,
                "update": false,
                "delete": false
            }
        ]),
    )
    .await;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/runtime/models/{model_code}/records"))
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(json!({ "title": "blocked" }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let payload: serde_json::Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(payload["code"], json!("api_key_action_not_allowed"));
}

#[tokio::test]
async fn runtime_model_routes_audit_api_key_denied_and_write_results() {
    let (app, database_url) = test_app_with_database_url().await;
    let (cookie, csrf) = login_and_capture_cookie(&app, "root", "change-me").await;
    let model_code = "api_key_audit_orders";
    let model_id =
        create_model_with_status(&app, &cookie, &csrf, model_code, Some("published")).await;
    create_text_field(&app, &cookie, &csrf, &model_id, "title").await;

    let read_only_token = create_api_key(
        &app,
        &cookie,
        &csrf,
        "runtime audit denied key",
        json!([
            {
                "data_model_id": model_id,
                "list": true,
                "get": false,
                "create": false,
                "update": false,
                "delete": false
            }
        ]),
    )
    .await;
    let denied = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/runtime/models/{model_code}/records"))
                .header("authorization", format!("Bearer {read_only_token}"))
                .header("content-type", "application/json")
                .body(Body::from(json!({ "title": "denied" }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);
    assert_eq!(
        audit_event_count(&database_url, "state_model.api_key_runtime_access_denied").await,
        1
    );

    let write_token = create_api_key(
        &app,
        &cookie,
        &csrf,
        "runtime audit write key",
        json!([
            {
                "data_model_id": model_id,
                "list": false,
                "get": false,
                "create": true,
                "update": false,
                "delete": false
            }
        ]),
    )
    .await;
    let success = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/runtime/models/{model_code}/records"))
                .header("authorization", format!("Bearer {write_token}"))
                .header("content-type", "application/json")
                .body(Body::from(json!({ "title": "success" }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(success.status(), StatusCode::CREATED);
    assert_eq!(
        audit_event_count(&database_url, "state_model.api_key_runtime_write_succeeded").await,
        1
    );

    drop_runtime_table(&database_url, &model_id).await;
    let failure = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/runtime/models/{model_code}/records"))
                .header("authorization", format!("Bearer {write_token}"))
                .header("content-type", "application/json")
                .body(Body::from(json!({ "title": "failure" }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(failure.status(), StatusCode::CONFLICT);
    assert_eq!(
        audit_event_count(&database_url, "state_model.api_key_runtime_write_failed").await,
        1
    );
}

#[tokio::test]
async fn runtime_model_routes_api_key_cannot_bypass_owner_scope() {
    let (app, database_url) = test_app_with_database_url().await;
    let (root_cookie, root_csrf) = login_and_capture_cookie(&app, "root", "change-me").await;
    let model_code = "api_key_owner_orders";
    let model_id = create_model_with_status(
        &app,
        &root_cookie,
        &root_csrf,
        model_code,
        Some("published"),
    )
    .await;
    create_text_field(&app, &root_cookie, &root_csrf, &model_id, "title").await;

    let member_id = create_member(
        &app,
        &root_cookie,
        &root_csrf,
        "api-key-owner-member",
        "temp-pass",
    )
    .await;
    replace_member_roles(&app, &root_cookie, &root_csrf, &member_id, &["admin"]).await;
    let (member_cookie, member_csrf) =
        login_and_capture_cookie(&app, "api-key-owner-member", "temp-pass").await;

    set_model_grant_permission_profile(&database_url, &model_id, "owner").await;
    create_runtime_record(&app, &root_cookie, &root_csrf, model_code, "root-owner").await;
    create_runtime_record(
        &app,
        &member_cookie,
        &member_csrf,
        model_code,
        "member-owner",
    )
    .await;

    let token = create_api_key(
        &app,
        &root_cookie,
        &root_csrf,
        "runtime owner key",
        json!([
            {
                "data_model_id": model_id,
                "list": true,
                "get": false,
                "create": false,
                "update": false,
                "delete": false
            }
        ]),
    )
    .await;

    let (status, payload) = list_records_with_api_key(&app, model_code, &token).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(payload["data"]["total"], json!(1));
    assert_eq!(payload["data"]["items"][0]["title"], json!("root-owner"));
}

#[tokio::test]
async fn runtime_model_routes_api_key_uses_system_scope_grant() {
    let app = test_app().await;
    let (cookie, csrf) = login_and_capture_cookie(&app, "root", "change-me").await;
    let model_code = "api_key_system_orders";
    let model_id = create_system_model(&app, &cookie, &csrf, model_code).await;
    create_text_field(&app, &cookie, &csrf, &model_id, "title").await;
    create_runtime_record(&app, &cookie, &csrf, model_code, "system-visible").await;

    let response = app
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
                        "name": "runtime system key",
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
    assert_eq!(response.status(), StatusCode::CREATED);
    let payload: serde_json::Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    let token = payload["data"]["token"].as_str().unwrap();

    let (status, payload) = list_records_with_api_key(&app, model_code, token).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(payload["data"]["total"], json!(1));
    assert_eq!(
        payload["data"]["items"][0]["title"],
        json!("system-visible")
    );
}

#[tokio::test]
async fn runtime_model_routes_audit_api_key_engine_level_acl_denials() {
    let (app, database_url) = test_app_with_database_url().await;
    let (cookie, csrf) = login_and_capture_cookie(&app, "root", "change-me").await;
    let model_code = "api_key_system_all_acl_orders";
    let model_id = create_system_model(&app, &cookie, &csrf, model_code).await;
    create_text_field(&app, &cookie, &csrf, &model_id, "title").await;
    set_model_grant_permission_profile(&database_url, &model_id, "system_all").await;

    let response = app
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
                        "name": "runtime system all denied key",
                        "scope_kind": "system",
                        "scope_id": domain::SYSTEM_SCOPE_ID,
                        "permissions": [
                            {
                                "data_model_id": model_id,
                                "list": true,
                                "get": false,
                                "create": true,
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
    assert_eq!(response.status(), StatusCode::CREATED);
    let payload: serde_json::Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    let token = payload["data"]["token"].as_str().unwrap();

    let (list_status, list_payload) = list_records_with_api_key(&app, model_code, token).await;
    assert_eq!(list_status, StatusCode::FORBIDDEN);
    assert_eq!(
        list_payload["code"],
        json!("system_all_requires_system_actor")
    );
    assert_eq!(
        audit_event_count(&database_url, "state_model.api_key_runtime_access_denied").await,
        1
    );

    let create_denied = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/runtime/models/{model_code}/records"))
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(json!({ "title": "denied" }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create_denied.status(), StatusCode::FORBIDDEN);
    let create_payload: serde_json::Value = serde_json::from_slice(
        &to_bytes(create_denied.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        create_payload["code"],
        json!("system_all_requires_system_actor")
    );
    assert_eq!(
        audit_event_count(&database_url, "state_model.api_key_runtime_access_denied").await,
        2
    );
    assert_eq!(
        audit_event_count(&database_url, "state_model.api_key_runtime_write_failed").await,
        1
    );
    assert_eq!(
        audit_event_count(&database_url, "state_model.api_key_runtime_write_succeeded").await,
        0
    );
}
