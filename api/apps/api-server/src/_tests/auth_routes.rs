use crate::_tests::support::{
    create_member, create_role, login_and_capture_cookie, replace_member_roles,
    replace_role_permissions, test_app, test_app_with_database_url,
};
use axum::{
    body::{to_bytes, Body},
    http::{header, Request, StatusCode},
};
use serde_json::json;
use sqlx::PgPool;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
use tower::ServiceExt;

#[tokio::test]
async fn console_user_api_key_create_list_and_revoke_hides_plaintext_after_creation() {
    let app = test_app().await;
    let (cookie, csrf) = login_and_capture_cookie(&app, "root", "change-me").await;

    let create = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/console/user-api-keys")
                .header("cookie", &cookie)
                .header("x-csrf-token", &csrf)
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "name": "script access",
                        "expiration_policy": "30d"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(create.status(), StatusCode::CREATED);
    let created_payload: serde_json::Value =
        serde_json::from_slice(&to_bytes(create.into_body(), usize::MAX).await.unwrap()).unwrap();
    let api_key_id = created_payload["data"]["id"].as_str().unwrap().to_string();
    let token = created_payload["data"]["token"].as_str().unwrap();
    assert!(token.starts_with("pat_"));
    assert_eq!(created_payload["data"]["key_kind"], json!("user_api_key"));
    assert_eq!(
        created_payload["data"]["token_hash"],
        serde_json::Value::Null
    );
    assert!(created_payload["data"]["expires_at"].is_string());

    let list = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/console/user-api-keys")
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(list.status(), StatusCode::OK);
    let list_payload: serde_json::Value =
        serde_json::from_slice(&to_bytes(list.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(list_payload["data"]["items"][0]["id"], json!(api_key_id));
    assert_eq!(
        list_payload["data"]["items"][0]["token"],
        serde_json::Value::Null
    );
    assert_eq!(
        list_payload["data"]["items"][0]["token_hash"],
        serde_json::Value::Null
    );
    assert_eq!(list_payload["data"]["items"][0]["enabled"], json!(true));
    assert_eq!(list_payload["data"]["items"][0]["revoked"], json!(false));

    let revoke = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/console/user-api-keys/{api_key_id}/revoke"))
                .header("cookie", &cookie)
                .header("x-csrf-token", &csrf)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(revoke.status(), StatusCode::OK);

    let revoked = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/console/me")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(revoked.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn console_user_api_key_create_maps_expiration_policies() {
    let app = test_app().await;
    let (cookie, csrf) = login_and_capture_cookie(&app, "root", "change-me").await;

    let thirty_days =
        create_user_api_key_payload(&app, &cookie, &csrf, "thirty day pat", "30d").await;
    assert_expiration_days_between(&thirty_days["expires_at"], 29, 31);

    let one_year = create_user_api_key_payload(&app, &cookie, &csrf, "one year pat", "1y").await;
    assert_expiration_days_between(&one_year["expires_at"], 364, 366);

    let three_years =
        create_user_api_key_payload(&app, &cookie, &csrf, "three year pat", "3y").await;
    assert_expiration_days_between(&three_years["expires_at"], 1094, 1096);

    let never = create_user_api_key_payload(&app, &cookie, &csrf, "never pat", "never").await;
    assert_eq!(never["expires_at"], serde_json::Value::Null);
}

#[tokio::test]
async fn console_user_api_key_authenticates_console_get_and_mutation_without_csrf() {
    let app = test_app().await;
    let (cookie, csrf) = login_and_capture_cookie(&app, "root", "change-me").await;
    let token = create_user_api_key(&app, &cookie, &csrf, "console pat").await;

    let me = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/console/me")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(me.status(), StatusCode::OK);

    let update = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri("/api/console/me")
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "name": "Root via PAT",
                        "nickname": "PAT Root",
                        "email": "root@example.com",
                        "phone": null,
                        "avatar_url": null,
                        "introduction": "updated without csrf",
                        "preferred_locale": null
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(update.status(), StatusCode::OK);
}

#[tokio::test]
async fn console_user_api_key_rejects_expired_disabled_key_and_disabled_user() {
    let (app, database_url) = test_app_with_database_url().await;
    let pool = PgPool::connect(&database_url).await.unwrap();
    let (cookie, csrf) = login_and_capture_cookie(&app, "root", "change-me").await;

    let expired_token = create_user_api_key(&app, &cookie, &csrf, "expired pat").await;
    expire_user_api_key(&pool, &expired_token).await;
    assert_eq!(
        get_console_me_with_bearer(&app, &expired_token).await,
        StatusCode::UNAUTHORIZED
    );

    let disabled_key_token = create_user_api_key(&app, &cookie, &csrf, "disabled pat").await;
    disable_user_api_key(&pool, &disabled_key_token).await;
    assert_eq!(
        get_console_me_with_bearer(&app, &disabled_key_token).await,
        StatusCode::UNAUTHORIZED
    );

    let disabled_user_token = create_user_api_key(&app, &cookie, &csrf, "disabled user pat").await;
    sqlx::query("update users set status = 'disabled' where account = 'root'")
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        get_console_me_with_bearer(&app, &disabled_user_token).await,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn console_user_api_key_rejects_dmk_on_console_and_pat_on_application_public_api() {
    let app = test_app().await;
    let (cookie, csrf) = login_and_capture_cookie(&app, "root", "change-me").await;
    let model_id = create_minimal_model(&app, &cookie, &csrf, "auth_route_key_isolation").await;
    let dmk_token = create_data_model_api_key(&app, &cookie, &csrf, &model_id).await;
    let pat_token = create_user_api_key(&app, &cookie, &csrf, "application public isolation").await;

    let console_with_dmk = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/console/me")
                .header("authorization", format!("Bearer {dmk_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(console_with_dmk.status(), StatusCode::UNAUTHORIZED);

    let public_with_pat = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/chat/completions")
                .header("authorization", format!("Bearer {pat_token}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "model": "default",
                        "messages": [{"role": "user", "content": "hello"}]
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(public_with_pat.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn public_auth_sign_in_sets_cookie_and_returns_wrapped_payload() {
    let app = test_app().await;
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/public/auth/providers/password-local/sign-in")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "identifier": "root@example.com",
                        "password": "change-me"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert!(response.headers().get("set-cookie").is_some());

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(payload["data"]["csrf_token"].is_string());
    assert!(payload["data"]["current_workspace_id"].is_string());
    assert!(payload["data"]["effective_display_role"].is_string());
    assert!(payload["meta"].is_null());
}

#[tokio::test]
async fn public_auth_sign_in_handles_cors_preflight() {
    let app = test_app().await;
    let response = app
        .oneshot(
            Request::builder()
                .method("OPTIONS")
                .uri("/api/public/auth/providers/password-local/sign-in")
                .header(header::ORIGIN, "http://127.0.0.1:3100")
                .header(header::ACCESS_CONTROL_REQUEST_METHOD, "POST")
                .header(header::ACCESS_CONTROL_REQUEST_HEADERS, "content-type")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get(header::ACCESS_CONTROL_ALLOW_ORIGIN),
        Some(&header::HeaderValue::from_static("http://127.0.0.1:3100"))
    );
    assert!(response
        .headers()
        .get(header::ACCESS_CONTROL_ALLOW_METHODS)
        .is_some());
}

#[tokio::test]
async fn console_api_key_create_returns_plaintext_token_once() {
    let app = test_app().await;
    let (cookie, csrf) = login_and_capture_cookie(&app, "root", "change-me").await;
    let model_id = create_minimal_model(&app, &cookie, &csrf, "auth_route_api_key_orders").await;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/console/api-keys")
                .header("cookie", cookie)
                .header("x-csrf-token", csrf)
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "name": "console route key",
                        "permissions": [
                            {
                                "data_model_id": model_id,
                                "list": true,
                                "get": true,
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
    assert!(token.starts_with("dmk_"));
    assert_eq!(payload["data"]["name"], json!("console route key"));
    assert_eq!(payload["data"]["permissions"][0]["list"], json!(true));
    assert!(payload["data"]["token_hash"].is_null());
}

#[tokio::test]
async fn console_api_key_create_requires_session() {
    let app = test_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/console/api-keys")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "name": "missing session",
                        "permissions": []
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let payload: serde_json::Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(payload["code"], json!("not_authenticated"));
}

#[tokio::test]
async fn console_api_key_create_requires_state_model_manage_permission() {
    let app = test_app().await;
    let (root_cookie, root_csrf) = login_and_capture_cookie(&app, "root", "change-me").await;
    let member_id = create_member(
        &app,
        &root_cookie,
        &root_csrf,
        "api-key-no-manage",
        "temp-pass",
    )
    .await;
    create_role(&app, &root_cookie, &root_csrf, "api_key_no_manage").await;
    replace_role_permissions(&app, &root_cookie, &root_csrf, "api_key_no_manage", &[]).await;
    replace_member_roles(
        &app,
        &root_cookie,
        &root_csrf,
        &member_id,
        &["api_key_no_manage"],
    )
    .await;
    let (member_cookie, member_csrf) =
        login_and_capture_cookie(&app, "api-key-no-manage", "temp-pass").await;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/console/api-keys")
                .header("cookie", member_cookie)
                .header("x-csrf-token", member_csrf)
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "name": "forbidden key",
                        "permissions": []
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let payload: serde_json::Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(payload["code"], json!("permission_denied"));
}

async fn create_minimal_model(app: &axum::Router, cookie: &str, csrf: &str, code: &str) -> String {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/console/models")
                .header("cookie", cookie)
                .header("x-csrf-token", csrf)
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

    assert_eq!(response.status(), StatusCode::CREATED);
    let payload: serde_json::Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    payload["data"]["id"].as_str().unwrap().to_string()
}

async fn create_user_api_key(app: &axum::Router, cookie: &str, csrf: &str, name: &str) -> String {
    let payload = create_user_api_key_payload(app, cookie, csrf, name, "never").await;

    payload["token"].as_str().unwrap().to_string()
}

async fn create_user_api_key_payload(
    app: &axum::Router,
    cookie: &str,
    csrf: &str,
    name: &str,
    expiration_policy: &str,
) -> serde_json::Value {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/console/user-api-keys")
                .header("cookie", cookie)
                .header("x-csrf-token", csrf)
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "name": name,
                        "expiration_policy": expiration_policy
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let status = response.status();
    let payload: serde_json::Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(status, StatusCode::CREATED, "{payload}");
    payload["data"].clone()
}

fn assert_expiration_days_between(value: &serde_json::Value, min_days: i64, max_days: i64) {
    let expires_at = OffsetDateTime::parse(value.as_str().unwrap(), &Rfc3339).unwrap();
    let now = OffsetDateTime::now_utc();
    let delta = expires_at - now;

    assert!(
        delta >= time::Duration::days(min_days) && delta <= time::Duration::days(max_days),
        "expected expires_at {expires_at} to be between {min_days} and {max_days} days from {now}"
    );
}

async fn get_console_me_with_bearer(app: &axum::Router, token: &str) -> StatusCode {
    app.clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/console/me")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
        .status()
}

fn user_api_key_token_prefix(token: &str) -> &str {
    token
        .rsplit_once('_')
        .map(|(prefix, _)| prefix)
        .expect("test user API key token should contain prefix separator")
}

async fn expire_user_api_key(pool: &PgPool, token: &str) {
    sqlx::query(
        "update api_keys set expires_at = now() - interval '1 second' where token_prefix = $1",
    )
    .bind(user_api_key_token_prefix(token))
    .execute(pool)
    .await
    .unwrap();
}

async fn disable_user_api_key(pool: &PgPool, token: &str) {
    sqlx::query("update api_keys set enabled = false where token_prefix = $1")
        .bind(user_api_key_token_prefix(token))
        .execute(pool)
        .await
        .unwrap();
}

async fn create_data_model_api_key(
    app: &axum::Router,
    cookie: &str,
    csrf: &str,
    model_id: &str,
) -> String {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/console/api-keys")
                .header("cookie", cookie)
                .header("x-csrf-token", csrf)
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "name": "legacy data model key",
                        "permissions": [
                            {
                                "data_model_id": model_id,
                                "list": true,
                                "get": true,
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

    let status = response.status();
    let payload: serde_json::Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(status, StatusCode::CREATED, "{payload}");
    payload["data"]["token"].as_str().unwrap().to_string()
}
