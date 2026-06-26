use control_plane::application_public_api::run_service::native_result_from_flow_run;
use serde_json::json;
use time::OffsetDateTime;
use uuid::Uuid;

fn uuid(value: u128) -> Uuid {
    Uuid::from_u128(value)
}

fn failed_published_flow_run(error_payload: serde_json::Value) -> domain::FlowRunRecord {
    let now = OffsetDateTime::now_utc();
    domain::FlowRunRecord {
        id: uuid(1),
        application_id: uuid(2),
        flow_id: uuid(3),
        draft_id: uuid(4),
        compiled_plan_id: None,
        debug_session_id: String::new(),
        flow_schema_version: "1flowbase.flow/v2".to_string(),
        document_hash: "hash".to_string(),
        run_mode: domain::FlowRunMode::PublishedApiRun,
        target_node_id: None,
        title: "Published run".to_string(),
        status: domain::FlowRunStatus::Failed,
        input_payload: json!({}),
        output_payload: json!({}),
        error_payload: Some(error_payload),
        created_by: uuid(5),
        authorized_account: None,
        api_key_id: Some(uuid(6)),
        publication_version_id: Some(uuid(7)),
        external_user: None,
        external_conversation_id: None,
        external_trace_id: None,
        compatibility_mode: None,
        idempotency_key: None,
        started_at: now,
        finished_at: Some(now),
        created_at: now,
        updated_at: now,
    }
}

#[test]
fn native_result_omits_provider_upstream_raw_details_from_public_error() {
    let flow_run = failed_published_flow_run(json!({
        "error_code": "provider_upstream_error",
        "message": "400 Bad Request: missing instructions",
        "provider_summary": "[REDACTED]",
        "provider_details": {
            "status": 400,
            "content_type": "application/json",
            "headers": {
                "x-request-id": "req_123"
            },
            "raw_body": "{\"error\":{\"message\":\"missing instructions\"}}\n"
        }
    }));

    let result = native_result_from_flow_run(&flow_run, json!({}));
    let error = result
        .error
        .expect("failed native run should expose an error");

    assert_eq!(error.message, "400 Bad Request: missing instructions");
    assert_eq!(error.details["error_code"], "provider_upstream_error");
    assert!(error.details.get("provider_summary").is_none());
    assert!(error.details.get("provider_details").is_none());
}

#[test]
fn native_result_sanitizes_legacy_provider_upstream_raw_body_from_public_error() {
    let raw_body = "plain upstream failure body with request payload";
    let flow_run = failed_published_flow_run(json!({
        "error_code": "provider_upstream_error",
        "message": format!("400 Bad Request: {raw_body}"),
        "provider_summary": raw_body,
        "provider_details": {
            "status": 400,
            "content_type": "text/plain",
            "headers": {
                "x-request-id": "req_123"
            },
            "raw_body": raw_body
        }
    }));

    let result = native_result_from_flow_run(&flow_run, json!({}));
    let error = result
        .error
        .expect("failed native run should expose an error");

    assert_eq!(error.message, "provider upstream request failed");
    assert_eq!(error.details["error_code"], "provider_upstream_error");
    assert!(error.details.get("provider_summary").is_none());
    assert!(error.details.get("provider_details").is_none());
    assert!(!error.message.contains(raw_body));
    assert!(!error.details.to_string().contains(raw_body));
}
