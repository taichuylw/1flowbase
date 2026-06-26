use super::*;

#[tokio::test]
async fn anthropic_messages_accepts_x_api_key_and_preserves_model() {
    let (app, _) = test_app_with_state().await;
    let token = setup_published_app(&app, "Anthropic Compatible Route App").await;

    let response = post_json(&app, "/v1/messages", ("x-api-key", token), anthropic_body()).await;

    assert_eq!(response.status(), StatusCode::OK);
    let payload = response_json(response).await;
    assert_eq!(payload["type"], json!("message"));
    assert_eq!(payload["model"], json!("anthropic/custom-model:latest"));
    assert_eq!(payload["content"][0]["type"], json!("text"));
}

#[tokio::test]
async fn anthropic_messages_accepts_last_user_multimodal_content() {
    let (app, _) = test_app_with_state().await;
    let token = setup_published_app(&app, "Anthropic Multimodal Compatible Route App").await;

    let response = post_json(
        &app,
        "/v1/messages",
        ("x-api-key", token),
        anthropic_multimodal_body(),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let payload = response_json(response).await;
    assert_eq!(payload["type"], json!("message"));
    assert_ne!(
        payload["error"]["message"],
        json!("messages is not supported by this endpoint")
    );
}

#[tokio::test]
async fn anthropic_messages_accepts_agent_tool_definitions() {
    let (app, _) = test_app_with_state().await;
    let token = setup_published_app(&app, "Anthropic Tool Compatible Route App").await;
    let mut body = anthropic_body();
    body["tools"] = json!([
        {
            "name": "lookup_order",
            "description": "Find an order",
            "input_schema": {
                "type": "object",
                "properties": {
                    "order_id": {"type": "string"}
                }
            }
        }
    ]);
    body["tool_choice"] = json!({"type": "auto"});

    let response = post_json(&app, "/v1/messages", ("x-api-key", token), body).await;

    assert_eq!(response.status(), StatusCode::OK);
    let payload = response_json(response).await;
    assert_eq!(payload["type"], json!("message"));
    assert_eq!(payload["model"], json!("anthropic/custom-model:latest"));
}

#[tokio::test]
async fn anthropic_messages_rehydrates_session_history_from_durable_turns() {
    let (app, state) = test_app_with_state().await;
    let token = setup_published_app(&app, "Anthropic Session History Route App").await;
    let session_id = "claude-code-session-1".to_string();
    let metadata = json!({
        "user_id": "{\"account_uuid\":\"account-1\",\"device_id\":\"device-1\"}"
    });

    let first = post_json_with_headers(
        &app,
        "/v1/messages",
        ("x-api-key", token.clone()),
        vec![("x-claude-code-session-id", session_id.clone())],
        json!({
            "model": "anthropic/custom-model:latest",
            "max_tokens": 64,
            "messages": [
                {"role": "user", "content": "Describe uploads/agent-flow-preview-debug.png"}
            ],
            "metadata": metadata
        }),
    )
    .await;
    assert_eq!(first.status(), StatusCode::OK);

    let second = post_json_with_headers(
        &app,
        "/v1/messages",
        ("x-api-key", token.clone()),
        vec![("x-claude-code-session-id", session_id.clone())],
        json!({
            "model": "anthropic/custom-model:latest",
            "max_tokens": 64,
            "messages": [
                {"role": "user", "content": "Find the corresponding code"}
            ],
            "metadata": metadata
        }),
    )
    .await;
    assert_eq!(second.status(), StatusCode::OK);

    let runs = sqlx::query_scalar::<_, Value>(
        r#"
        select input_payload
        from flow_runs
        where compatibility_mode = 'anthropic-messages-v1'
        order by created_at asc, id asc
        "#,
    )
    .fetch_all(state.store.pool())
    .await
    .unwrap();
    assert_eq!(runs.len(), 2);
    let history = runs[1]["node-start"]["history"]
        .as_array()
        .expect("second run should receive rehydrated history");
    assert_eq!(history.len(), 2);
    assert_eq!(
        history[0],
        json!({
            "role": "user",
            "content": "Describe uploads/agent-flow-preview-debug.png"
        })
    );
    assert_eq!(history[1]["role"], json!("assistant"));
    assert!(
        history[1]["content"]
            .as_str()
            .is_some_and(|value| !value.is_empty()),
        "{history:?}"
    );

    let third = post_json_with_headers(
        &app,
        "/v1/messages",
        ("x-api-key", token.clone()),
        vec![("x-claude-code-session-id", session_id)],
        json!({
            "model": "anthropic/custom-model:latest",
            "max_tokens": 64,
            "messages": [
                {"role": "user", "content": "Keep going"}
            ],
            "metadata": metadata
        }),
    )
    .await;
    assert_eq!(third.status(), StatusCode::OK);

    let runs = sqlx::query_scalar::<_, Value>(
        r#"
        select input_payload
        from flow_runs
        where compatibility_mode = 'anthropic-messages-v1'
        order by created_at asc, id asc
        "#,
    )
    .fetch_all(state.store.pool())
    .await
    .unwrap();
    assert_eq!(runs.len(), 3);
    let third_history = runs[2]["node-start"]["history"]
        .as_array()
        .expect("third run should receive unique prior turns");
    assert_eq!(
        third_history
            .iter()
            .map(|message| message["role"].as_str().unwrap_or_default())
            .collect::<Vec<_>>(),
        vec!["user", "assistant", "user", "assistant"]
    );
    assert_eq!(
        third_history[0]["content"],
        json!("Describe uploads/agent-flow-preview-debug.png")
    );
    assert_eq!(
        third_history[2]["content"],
        json!("Find the corresponding code")
    );
}
