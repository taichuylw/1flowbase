use super::*;

#[tokio::test]
async fn model_definition_routes_manage_models_and_fields_without_publish() {
    let app = test_app().await;
    let (cookie, csrf) = login_and_capture_cookie(&app, "root", "change-me").await;

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
                        "code": "orders",
                        "title": "Orders"
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
    assert_eq!(created["data"]["status"], json!("published"));
    assert_eq!(
        created["data"]["api_exposure_status"],
        json!("published_not_exposed")
    );
    assert_eq!(created["data"]["runtime_availability"], json!("available"));
    let model_id = created["data"]["id"].as_str().unwrap().to_string();

    let list_main_source_models = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/console/models?data_source_instance_id=main_source")
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(list_main_source_models.status(), StatusCode::OK);
    let list_main_source_payload: serde_json::Value = serde_json::from_slice(
        &to_bytes(list_main_source_models.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    let models = list_main_source_payload["data"].as_array().unwrap();
    let model_codes = models
        .iter()
        .filter_map(|model| model["code"].as_str())
        .collect::<Vec<_>>();
    assert!(model_codes.contains(&"attachments"));
    assert!(model_codes.contains(&"users"));
    assert!(model_codes.contains(&"roles"));
    assert!(models.iter().any(|model| {
        model["id"].as_str() == Some(&model_id)
            && model["source_kind"].as_str() == Some("main_source")
    }));
    assert!(models.iter().all(|model| {
        model["data_source_instance_id"].is_null()
            && model["source_kind"].as_str() == Some("main_source")
    }));

    let field_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/console/models/{model_id}/fields"))
                .header("cookie", &cookie)
                .header("x-csrf-token", &csrf)
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "code": "status",
                        "title": "Status",
                        "field_kind": "enum",
                        "is_required": true,
                        "is_unique": false,
                        "display_interface": "select",
                        "display_options": { "options": ["draft", "paid"] }
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(field_response.status(), StatusCode::CREATED);
    let created_field: serde_json::Value = serde_json::from_slice(
        &to_bytes(field_response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    let field_id = created_field["data"]["id"].as_str().unwrap().to_string();

    let create_runtime_record = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/runtime/models/orders/records")
                .header("cookie", &cookie)
                .header("x-csrf-token", &csrf)
                .header("content-type", "application/json")
                .body(Body::from(json!({ "status": "draft" }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(create_runtime_record.status(), StatusCode::CREATED);

    let update_model_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/api/console/models/{model_id}"))
                .header("cookie", &cookie)
                .header("x-csrf-token", &csrf)
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "title": "Orders V2"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(update_model_response.status(), StatusCode::OK);

    let create_after_model_update = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/runtime/models/orders/records")
                .header("cookie", &cookie)
                .header("x-csrf-token", &csrf)
                .header("content-type", "application/json")
                .body(Body::from(json!({ "status": "paid" }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(create_after_model_update.status(), StatusCode::CREATED);

    let update_field_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/api/console/models/{model_id}/fields/{field_id}"))
                .header("cookie", &cookie)
                .header("x-csrf-token", &csrf)
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "title": "Lifecycle Status",
                        "is_required": true,
                        "is_unique": false,
                        "default_value": "draft",
                        "display_interface": "select",
                        "display_options": { "options": ["draft", "paid"] },
                        "relation_options": {}
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(update_field_response.status(), StatusCode::OK);

    let create_after_field_update = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/runtime/models/orders/records")
                .header("cookie", &cookie)
                .header("x-csrf-token", &csrf)
                .header("content-type", "application/json")
                .body(Body::from(json!({ "status": "draft" }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(create_after_field_update.status(), StatusCode::CREATED);

    let second_field_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/console/models/{model_id}/fields"))
                .header("cookie", &cookie)
                .header("x-csrf-token", &csrf)
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "code": "note",
                        "title": "Note",
                        "field_kind": "text",
                        "is_required": false,
                        "is_unique": false
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(second_field_response.status(), StatusCode::CREATED);
    let second_field: serde_json::Value = serde_json::from_slice(
        &to_bytes(second_field_response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    let second_field_id = second_field["data"]["id"].as_str().unwrap().to_string();

    let delete_field_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!(
                    "/api/console/models/{model_id}/fields/{second_field_id}?confirmed=true"
                ))
                .header("cookie", &cookie)
                .header("x-csrf-token", &csrf)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(delete_field_response.status(), StatusCode::OK);

    let create_after_field_delete = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/runtime/models/orders/records")
                .header("cookie", &cookie)
                .header("x-csrf-token", &csrf)
                .header("content-type", "application/json")
                .body(Body::from(json!({ "status": "paid" }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(create_after_field_delete.status(), StatusCode::CREATED);

    let list_runtime_records = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/runtime/models/orders/records")
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(list_runtime_records.status(), StatusCode::OK);
    let listed_records: serde_json::Value = serde_json::from_slice(
        &to_bytes(list_runtime_records.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(listed_records["data"]["total"], json!(4));
}

#[tokio::test]
async fn model_definition_routes_reject_main_source_external_mapping_keys() {
    let app = test_app().await;
    let (cookie, csrf) = login_and_capture_cookie(&app, "root", "change-me").await;

    let create_with_external_resource_key = app
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
                        "external_resource_key": "contacts",
                        "code": "main_source_with_external_key",
                        "title": "Main Source With External Key"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        create_with_external_resource_key.status(),
        StatusCode::BAD_REQUEST
    );
    let error: serde_json::Value = serde_json::from_slice(
        &to_bytes(create_with_external_resource_key.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(error["code"], json!("external_resource_key"));
    assert_eq!(
        error["message"],
        json!("invalid input: external_resource_key")
    );

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
                        "code": "main_source_field_external_key",
                        "title": "Main Source Field External Key"
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

    let field_with_external_key = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/console/models/{model_id}/fields"))
                .header("cookie", &cookie)
                .header("x-csrf-token", &csrf)
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "code": "email",
                        "title": "Email",
                        "external_field_key": "properties.email",
                        "field_kind": "string"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(field_with_external_key.status(), StatusCode::BAD_REQUEST);
    let error: serde_json::Value = serde_json::from_slice(
        &to_bytes(field_with_external_key.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(error["code"], json!("external_field_key"));
    assert_eq!(error["message"], json!("invalid input: external_field_key"));
}
