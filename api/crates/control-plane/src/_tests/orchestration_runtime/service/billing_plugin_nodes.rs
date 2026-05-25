use super::*;

#[tokio::test]
async fn start_flow_debug_run_records_gateway_billing_audit_trace() {
    let service = OrchestrationRuntimeService::for_tests();
    let seeded = service
        .seed_application_with_plugin_node_flow("Capability Agent")
        .await;

    let started = service
        .start_flow_debug_run(StartFlowDebugRunCommand {
            actor_user_id: seeded.actor_user_id,
            application_id: seeded.application_id,
            input_payload: serde_json::json!({
                "node-start": { "query": "world" }
            }),
            document_snapshot: None,
            debug_session_id: None,
        })
        .await
        .unwrap();

    let billing_event = started
        .events
        .iter()
        .find(|event| event.event_type == "gateway_billing_session_reserved")
        .expect("gateway billing event should be recorded before continuation");

    assert_eq!(
        billing_event.payload["billing_session"]["status"].as_str(),
        Some("reserved")
    );
    assert_eq!(
        billing_event.payload["cost_ledger"]["cost_status"].as_str(),
        Some("pending_usage")
    );
    assert_eq!(
        billing_event.payload["credit_ledger"]["transaction_type"].as_str(),
        Some("reserve")
    );
    assert_eq!(
        billing_event.payload["route_trace"]["trust_level"].as_str(),
        Some("host_fact")
    );
    assert_eq!(
        billing_event.payload["audit_hashes"]
            .as_array()
            .map(|hashes| hashes.len()),
        Some(3)
    );
}

#[tokio::test]
async fn continue_flow_debug_run_executes_plugin_node_through_capability_runtime() {
    let service = OrchestrationRuntimeService::for_tests();
    let seeded = service
        .seed_application_with_plugin_node_flow("Capability Agent")
        .await;

    let started = service
        .start_flow_debug_run(StartFlowDebugRunCommand {
            actor_user_id: seeded.actor_user_id,
            application_id: seeded.application_id,
            input_payload: serde_json::json!({
                "node-start": { "query": "world" }
            }),
            document_snapshot: None,
            debug_session_id: None,
        })
        .await
        .unwrap();
    let detail = service
        .continue_flow_debug_run(ContinueFlowDebugRunCommand {
            application_id: seeded.application_id,
            flow_run_id: started.flow_run.id,
            workspace_id: Uuid::nil(),
        })
        .await
        .unwrap();

    assert_eq!(detail.flow_run.status, domain::FlowRunStatus::Succeeded);
    assert_eq!(detail.node_runs[1].node_type, "plugin_node");
    assert_eq!(detail.node_runs[1].output_payload["answer"], "world");
}
