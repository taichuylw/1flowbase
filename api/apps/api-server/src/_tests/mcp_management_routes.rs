use crate::_tests::support::{login_and_capture_cookie, test_app};
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use serde_json::{json, Value};
use tower::ServiceExt;

async fn response_json(response: axum::response::Response) -> Value {
    serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap()
}

#[tokio::test]
async fn mcp_management_routes_seed_catalog_and_derive_tool_contract_from_interface_catalog() {
    let app = test_app().await;
    let (root_cookie, root_csrf) = login_and_capture_cookie(&app, "root", "change-me").await;

    let catalog_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/console/mcp/catalog")
                .header("cookie", &root_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(catalog_response.status(), StatusCode::OK);
    let catalog_payload = response_json(catalog_response).await;
    assert_eq!(
        catalog_payload["data"]["default_instance"]["instance_id"].as_str(),
        Some("default_system")
    );
    assert_eq!(
        catalog_payload["data"]["meta_tool_config"]["list_default_limit"].as_i64(),
        Some(50)
    );

    let interface_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/console/mcp/interface-capabilities")
                .header("cookie", &root_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(interface_response.status(), StatusCode::OK);
    let interface_payload = response_json(interface_response).await;
    assert!(interface_payload["data"]
        .as_array()
        .unwrap()
        .iter()
        .any(|entry| {
            entry["interface_id"].as_str() == Some("settings.file_storages.list")
                && entry["bindable"].as_bool() == Some(false)
                && entry["disabled_reason"].as_str() == Some("root_only_service_contract")
        }));

    let create_tool_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/console/mcp/tools")
                .header("cookie", &root_cookie)
                .header("x-csrf-token", &root_csrf)
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "tool_id": null,
                        "suggested_group_path": "/system",
                        "name": "Runtime profile",
                        "short_description": "Runtime profile",
                        "usage_description": "Read runtime profile",
                        "full_description": "Read system runtime topology and locale profile.",
                        "interface_id": "settings.system_runtime.get_profile",
                        "parameter_schema": { "type": "object", "properties": { "fake": { "type": "string" } } },
                        "result_schema": { "type": "string" },
                        "input_mapping": {},
                        "output_mapping": {},
                        "permission_code": "file_storage.manage.all",
                        "risk_level": "low",
                        "audit_policy": { "enabled": true },
                        "des_id_required": true,
                        "status": "enabled"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create_tool_response.status(), StatusCode::CREATED);
    let create_tool_payload = response_json(create_tool_response).await;
    let tool_id = create_tool_payload["data"]["tool_id"].as_str().unwrap();
    let first_des_id = create_tool_payload["data"]["des_id"].as_str().unwrap();
    assert_eq!(first_des_id.len(), 8);
    assert_eq!(
        create_tool_payload["data"]["permission_code"].as_str(),
        Some("system_runtime.view.all")
    );
    assert_eq!(
        create_tool_payload["data"]["risk_level"].as_str(),
        Some("high")
    );
    assert!(
        create_tool_payload["data"]["parameter_schema"]["properties"]
            .get("locale")
            .is_some()
    );
    assert!(
        create_tool_payload["data"]["parameter_schema"]["properties"]
            .get("fake")
            .is_none()
    );

    let upsert_group_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/console/mcp/instances/default_system/groups")
                .header("cookie", &root_cookie)
                .header("x-csrf-token", &root_csrf)
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "path": "/system",
                        "display_name": "System",
                        "description_short": "System tools",
                        "enabled": true,
                        "sort_order": 1
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(upsert_group_response.status(), StatusCode::OK);

    let get_tool_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/console/mcp/tools/{tool_id}"))
                .header("cookie", &root_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(get_tool_response.status(), StatusCode::OK);
    let get_tool_payload = response_json(get_tool_response).await;
    assert_eq!(get_tool_payload["data"]["tool_id"].as_str(), Some(tool_id));

    let refresh_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/console/mcp/tools/{tool_id}/description/refresh"
                ))
                .header("cookie", &root_cookie)
                .header("x-csrf-token", &root_csrf)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(refresh_response.status(), StatusCode::OK);
    let refresh_payload = response_json(refresh_response).await;
    assert_ne!(
        refresh_payload["data"]["des_id"].as_str().unwrap(),
        first_des_id
    );

    let directory_export_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/console/mcp/instances/export")
                .header("cookie", &root_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(directory_export_response.status(), StatusCode::OK);
    let directory_export_payload = response_json(directory_export_response).await;
    assert!(directory_export_payload["data"].get("tools").is_none());
    assert!(directory_export_payload["data"]["groups"]
        .as_array()
        .unwrap()
        .iter()
        .any(|group| group["path"].as_str() == Some("/system")));

    let delete_group_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/api/console/mcp/instances/default_system/groups?path=%2Fsystem")
                .header("cookie", &root_cookie)
                .header("x-csrf-token", &root_csrf)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(delete_group_response.status(), StatusCode::NO_CONTENT);

    let delete_tool_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/console/mcp/tools/{tool_id}"))
                .header("cookie", &root_cookie)
                .header("x-csrf-token", &root_csrf)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(delete_tool_response.status(), StatusCode::NO_CONTENT);

    let missing_get_tool_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/console/mcp/tools/{tool_id}"))
                .header("cookie", &root_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(missing_get_tool_response.status(), StatusCode::NOT_FOUND);

    let missing_description_check_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/console/mcp/tools/{tool_id}/description-check"
                ))
                .header("cookie", &root_cookie)
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "des_id": first_des_id
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        missing_description_check_response.status(),
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn mcp_meta_tool_config_updates_validate_and_shape_list_defaults() {
    let app = test_app().await;
    let (root_cookie, root_csrf) = login_and_capture_cookie(&app, "root", "change-me").await;

    let catalog_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/console/mcp/catalog")
                .header("cookie", &root_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(catalog_response.status(), StatusCode::OK);

    for (path, display_name, sort_order) in [
        ("/system", "System", 1),
        ("/system/runtime", "Runtime", 2),
        ("/ops", "Operations", 3),
    ] {
        let upsert_group_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/console/mcp/instances/default_system/groups")
                    .header("cookie", &root_cookie)
                    .header("x-csrf-token", &root_csrf)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "path": path,
                            "display_name": display_name,
                            "description_short": null,
                            "enabled": true,
                            "sort_order": sort_order
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(upsert_group_response.status(), StatusCode::OK);
    }

    let invalid_return_fields_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/console/mcp/meta-tool-config")
                .header("cookie", &root_cookie)
                .header("x-csrf-token", &root_csrf)
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "list_default_limit": 1,
                        "list_max_depth": 1,
                        "list_regex_enabled": true,
                        "list_regex_max_length": 16,
                        "list_return_fields": ["id", "secret"],
                        "get_include_mapping_summary": true,
                        "get_include_interface_summary": true,
                        "call_default_des_id_policy": "required",
                        "call_high_risk_requires_des_id": true,
                        "call_validation_error_format": "field_errors"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        invalid_return_fields_response.status(),
        StatusCode::BAD_REQUEST
    );
    let invalid_return_fields_payload = response_json(invalid_return_fields_response).await;
    assert_eq!(
        invalid_return_fields_payload["code"].as_str(),
        Some("list_return_fields")
    );

    let invalid_des_policy_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/console/mcp/meta-tool-config")
                .header("cookie", &root_cookie)
                .header("x-csrf-token", &root_csrf)
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "list_default_limit": 1,
                        "list_max_depth": 1,
                        "list_regex_enabled": true,
                        "list_regex_max_length": 16,
                        "list_return_fields": ["id", "name"],
                        "get_include_mapping_summary": true,
                        "get_include_interface_summary": true,
                        "call_default_des_id_policy": "frontend_only",
                        "call_high_risk_requires_des_id": true,
                        "call_validation_error_format": "field_errors"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        invalid_des_policy_response.status(),
        StatusCode::BAD_REQUEST
    );
    let invalid_des_policy_payload = response_json(invalid_des_policy_response).await;
    assert_eq!(
        invalid_des_policy_payload["code"].as_str(),
        Some("call_default_des_id_policy")
    );

    let update_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/console/mcp/meta-tool-config")
                .header("cookie", &root_cookie)
                .header("x-csrf-token", &root_csrf)
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "list_default_limit": 1,
                        "list_max_depth": 1,
                        "list_regex_enabled": true,
                        "list_regex_max_length": 16,
                        "list_return_fields": ["id", "name"],
                        "get_include_mapping_summary": true,
                        "get_include_interface_summary": true,
                        "call_default_des_id_policy": "required",
                        "call_high_risk_requires_des_id": true,
                        "call_validation_error_format": "field_errors"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(update_response.status(), StatusCode::OK);
    let update_payload = response_json(update_response).await;
    assert_eq!(
        update_payload["data"]["call_default_des_id_policy"].as_str(),
        Some("required")
    );
    assert_eq!(
        update_payload["data"]["call_validation_error_format"].as_str(),
        Some("field_errors")
    );

    let default_list_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/console/mcp/list")
                .header("cookie", &root_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(default_list_response.status(), StatusCode::OK);
    let default_list_payload = response_json(default_list_response).await;
    let default_items = default_list_payload["data"].as_array().unwrap();
    assert_eq!(default_items.len(), 1);
    assert!(default_items[0].get("id").is_some());
    assert!(default_items[0].get("name").is_some());
    assert!(default_items[0].get("path").is_none());
    assert!(default_items[0].get("children_count").is_none());

    let regex_list_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/console/mcp/list?limit=10&path_regex=%5E%2Fsystem")
                .header("cookie", &root_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(regex_list_response.status(), StatusCode::OK);
    let regex_list_payload = response_json(regex_list_response).await;
    let regex_items = regex_list_payload["data"].as_array().unwrap();
    assert_eq!(regex_items.len(), 1);
    assert_eq!(regex_items[0]["name"].as_str(), Some("System"));

    let long_regex_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/console/mcp/list?path_regex=%5E%2Fsystem%2Fruntime-long")
                .header("cookie", &root_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(long_regex_response.status(), StatusCode::BAD_REQUEST);
    let long_regex_payload = response_json(long_regex_response).await;
    assert_eq!(long_regex_payload["code"].as_str(), Some("path_regex"));
}
