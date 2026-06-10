use super::*;

#[tokio::test]
async fn opens_flow_debug_run_shell_without_compiling_plan() {
    let service = OrchestrationRuntimeService::for_tests();
    let seeded = service.seed_application_with_flow("Support Agent").await;

    let shell = service
        .open_flow_debug_run_shell(StartFlowDebugRunCommand {
            actor_user_id: seeded.actor_user_id,
            application_id: seeded.application_id,
            input_payload: serde_json::json!({ "node-start": { "query": "hello" } }),
            document_snapshot: None,
            debug_session_id: None,
        })
        .await
        .unwrap();

    assert_eq!(shell.status, domain::FlowRunStatus::Queued);
    assert_eq!(shell.compiled_plan_id, None);
}

#[tokio::test]
async fn prepare_flow_debug_run_rejects_shell_input_mismatch() {
    let service = OrchestrationRuntimeService::for_tests();
    let seeded = service.seed_application_with_flow("Support Agent").await;

    let shell = service
        .open_flow_debug_run_shell(StartFlowDebugRunCommand {
            actor_user_id: seeded.actor_user_id,
            application_id: seeded.application_id,
            input_payload: serde_json::json!({ "node-start": { "query": "input A" } }),
            document_snapshot: None,
            debug_session_id: None,
        })
        .await
        .unwrap();

    let error = service
        .prepare_flow_debug_run_from_shell(PrepareFlowDebugRunCommand {
            actor_user_id: seeded.actor_user_id,
            application_id: seeded.application_id,
            flow_run_id: shell.id,
            input_payload: serde_json::json!({ "node-start": { "query": "input B" } }),
            document_snapshot: None,
            debug_session_id: String::new(),
        })
        .await
        .unwrap_err();

    assert!(error
        .to_string()
        .contains("flow debug run shell does not match prepare command"));

    let detail = service
        .application_run_detail(seeded.application_id, shell.id)
        .await;
    assert_eq!(detail.flow_run.status, domain::FlowRunStatus::Queued);
    assert_eq!(detail.flow_run.compiled_plan_id, None);
    assert!(detail.events.is_empty());
}

#[tokio::test]
async fn concurrent_prepare_flow_debug_run_does_not_fail_attached_shell() {
    let service = OrchestrationRuntimeService::for_tests();
    let seeded = service.seed_application_with_flow("Support Agent").await;

    let input_payload = serde_json::json!({ "node-start": { "query": "hello" } });
    let shell = service
        .open_flow_debug_run_shell(StartFlowDebugRunCommand {
            actor_user_id: seeded.actor_user_id,
            application_id: seeded.application_id,
            input_payload: input_payload.clone(),
            document_snapshot: None,
            debug_session_id: None,
        })
        .await
        .unwrap();

    let first_command = PrepareFlowDebugRunCommand {
        actor_user_id: seeded.actor_user_id,
        application_id: seeded.application_id,
        flow_run_id: shell.id,
        input_payload: input_payload.clone(),
        document_snapshot: None,
        debug_session_id: String::new(),
    };
    let second_command = PrepareFlowDebugRunCommand {
        actor_user_id: seeded.actor_user_id,
        application_id: seeded.application_id,
        flow_run_id: shell.id,
        input_payload,
        document_snapshot: None,
        debug_session_id: String::new(),
    };

    let (first, second) = tokio::join!(
        service.prepare_flow_debug_run_from_shell(first_command),
        service.prepare_flow_debug_run_from_shell(second_command),
    );

    assert_eq!(
        [first.is_ok(), second.is_ok()]
            .into_iter()
            .filter(|succeeded| *succeeded)
            .count(),
        1
    );

    let detail = service
        .application_run_detail(seeded.application_id, shell.id)
        .await;
    assert_eq!(detail.flow_run.status, domain::FlowRunStatus::Running);
    assert!(detail.flow_run.compiled_plan_id.is_some());
}

#[tokio::test]
async fn start_flow_debug_run_marks_shell_failed_when_preparation_fails() {
    let service = OrchestrationRuntimeService::for_tests();
    let seeded = service.seed_application_with_flow("Support Agent").await;

    let error = service
        .start_flow_debug_run(StartFlowDebugRunCommand {
            actor_user_id: seeded.actor_user_id,
            application_id: seeded.application_id,
            input_payload: serde_json::json!({ "node-start": { "query": "hello" } }),
            document_snapshot: Some(serde_json::json!({})),
            debug_session_id: None,
        })
        .await
        .unwrap_err();

    assert!(error.to_string().contains("schemaVersion missing"));

    let runs = service.application_runs(seeded.application_id).await;
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].status, domain::FlowRunStatus::Failed);

    let detail = service
        .application_run_detail(seeded.application_id, runs[0].id)
        .await;
    assert_eq!(detail.flow_run.status, domain::FlowRunStatus::Failed);
    assert!(detail.flow_run.error_payload.is_some());
    assert!(detail
        .events
        .iter()
        .any(|event| event.event_type == "flow_run_failed"));
}

#[tokio::test]
async fn failed_prepare_emits_flow_failed_lifecycle_and_closes_runtime_stream() {
    let service = OrchestrationRuntimeService::for_tests();
    let seeded = service.seed_application_with_flow("Support Agent").await;
    let stream =
        std::sync::Arc::new(crate::_tests::support::RecordingRuntimeEventStream::default());
    let service = service.with_runtime_event_stream(stream.clone());

    let error = service
        .start_flow_debug_run(StartFlowDebugRunCommand {
            actor_user_id: seeded.actor_user_id,
            application_id: seeded.application_id,
            input_payload: serde_json::json!({ "node-start": { "query": "hello" } }),
            document_snapshot: Some(serde_json::json!({})),
            debug_session_id: None,
        })
        .await
        .unwrap_err();

    assert!(error.to_string().contains("schemaVersion missing"));

    let runs = service.application_runs(seeded.application_id).await;
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].status, domain::FlowRunStatus::Failed);

    let event_types = stream
        .events()
        .into_iter()
        .map(|event| event.event_type)
        .collect::<Vec<_>>();
    assert!(event_types
        .iter()
        .any(|event_type| event_type == "flow_failed"));
    assert_eq!(
        stream.close_calls(),
        vec![(runs[0].id, crate::ports::RuntimeEventCloseReason::Failed)]
    );
}
