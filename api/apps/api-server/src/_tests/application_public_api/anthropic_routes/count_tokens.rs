use super::*;

#[tokio::test]
async fn anthropic_count_tokens_returns_usage_without_creating_run() {
    let (app, state) = test_app_with_state().await;
    let token = setup_published_app(&app, "Anthropic Count Tokens Compatible Route App").await;
    let before = flow_run_count(state.as_ref()).await;

    let response = post_json(
        &app,
        "/v1/messages/count_tokens",
        ("x-api-key", token),
        json!({
            "model": "anthropic/custom-model:latest",
            "messages": [
                {"role": "user", "content": "Count this prompt"}
            ],
            "tools": [{
                "name": "lookup_order",
                "description": "Find an order by id",
                "input_schema": {
                    "type": "object",
                    "properties": {
                        "order_id": {"type": "string"}
                    }
                }
            }]
        }),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let payload = response_json(response).await;
    assert!(
        payload["input_tokens"].as_u64().unwrap_or_default() > 0,
        "{payload}"
    );
    assert_eq!(flow_run_count(state.as_ref()).await, before);
}
