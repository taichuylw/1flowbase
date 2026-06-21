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
