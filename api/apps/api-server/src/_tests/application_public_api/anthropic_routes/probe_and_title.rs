use super::*;

#[tokio::test]
async fn anthropic_probe_message_uses_published_native_run() {
    let (app, state) = test_app_with_state().await;
    let token = setup_published_app(&app, "Anthropic Probe Compatible Route App").await;
    let before = flow_run_count(state.as_ref()).await;

    let response = post_json(
        &app,
        "/v1/messages",
        ("x-api-key", token),
        json!({
            "model": "anthropic/custom-model:latest",
            "max_tokens": 1,
            "messages": [
                {"role": "user", "content": "test"}
            ],
            "metadata": {
                "user_id": "{\"device_id\":\"probe-device\",\"account_uuid\":\"\",\"session_id\":\"probe-session\"}"
            }
        }),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let payload = response_json(response).await;
    assert_eq!(payload["type"], json!("message"));
    assert_eq!(flow_run_count(state.as_ref()).await, before + 1);
}

#[tokio::test]
async fn anthropic_probe_message_requires_active_publication() {
    let (app, state) = test_app_with_state().await;
    let token =
        setup_unpublished_app_key(&app, "Anthropic Unpublished Probe Compatible Route App").await;

    let response = post_json(
        &app,
        "/v1/messages",
        ("x-api-key", token),
        json!({
            "model": "anthropic/custom-model:latest",
            "max_tokens": 1,
            "messages": [
                {"role": "user", "content": "test"}
            ]
        }),
    )
    .await;

    assert_eq!(response.status(), StatusCode::CONFLICT);
    let payload = response_json(response).await;
    assert_eq!(payload["error"]["type"], json!("application_not_published"));
    assert_eq!(flow_run_count(state.as_ref()).await, 0);
}

#[tokio::test]
async fn anthropic_structured_title_message_requires_active_publication() {
    let (app, state) = test_app_with_state().await;
    let token = setup_unpublished_app_key(
        &app,
        "Anthropic Unpublished Structured Compatible Route App",
    )
    .await;

    let response = post_json(
        &app,
        "/v1/messages",
        ("x-api-key", token),
        json!({
            "model": "anthropic/custom-model:latest",
            "max_tokens": 64,
            "stream": true,
            "messages": [
                {"role": "user", "content": "帮我找找这个代码位置"}
            ],
            "output_config": {
                "format": {
                    "type": "json_schema",
                    "schema": {
                        "type": "object",
                        "properties": {
                            "title": { "type": "string" }
                        },
                        "required": ["title"],
                        "additionalProperties": false
                    }
                }
            }
        }),
    )
    .await;

    assert_eq!(response.status(), StatusCode::CONFLICT);
    let payload = response_json(response).await;
    assert_eq!(payload["error"]["type"], json!("application_not_published"));
    assert_eq!(flow_run_count(state.as_ref()).await, 0);
}
