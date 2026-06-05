use api_server::error_response::ApiError;
use axum::{body::to_bytes, http::StatusCode, response::IntoResponse};

#[tokio::test]
async fn internal_error_response_does_not_expose_source_message() {
    let response =
        ApiError(anyhow::anyhow!("database url postgres://secret@example")).into_response();

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(payload["code"], "internal_error");
    assert!(!payload["message"]
        .as_str()
        .unwrap()
        .contains("postgres://secret@example"));
}
