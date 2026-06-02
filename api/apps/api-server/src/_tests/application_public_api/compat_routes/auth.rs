use super::*;

#[tokio::test]
async fn compatible_routes_require_application_api_key() {
    let app = test_app().await;

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/chat/completions")
                .header("content-type", "application/json")
                .body(Body::from(openai_body(false).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let payload = response_json(response).await;
    assert_eq!(payload["error"]["code"], json!("not_authenticated"));

    let responses = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/responses")
                .header("content-type", "application/json")
                .body(Body::from(responses_body(false).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(responses.status(), StatusCode::UNAUTHORIZED);
    let payload = response_json(responses).await;
    assert_eq!(payload["error"]["code"], json!("not_authenticated"));
}

#[tokio::test]
async fn compatible_routes_return_not_published_for_unpublished_key_application() {
    let app = test_app().await;
    let token = setup_unpublished_app_key(&app, "Compatible Unpublished Route App").await;

    let response = post_json(
        &app,
        "/v1/messages",
        ("x-api-key", token),
        anthropic_body(false),
    )
    .await;

    assert_eq!(response.status(), StatusCode::CONFLICT);
    let payload = response_json(response).await;
    assert_eq!(payload["error"]["type"], json!("application_not_published"));
}
