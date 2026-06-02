use super::*;

#[tokio::test]
async fn creates_flow_run_shell_and_attaches_compiled_plan() {
    let pool = connect(&isolated_database_url().await).await.unwrap();
    run_migrations(&pool).await.unwrap();
    let store = PgControlPlaneStore::new(pool);
    let seeded = seed_runtime_base(&store).await;

    let shell = <PgControlPlaneStore as OrchestrationRuntimeRepository>::create_flow_run_shell(
        &store,
        &CreateFlowRunShellInput {
            actor_user_id: seeded.actor_user_id,
            application_id: seeded.application_id,
            flow_id: seeded.flow_id,
            flow_draft_id: seeded.draft_id,
            debug_session_id: "test-debug-session".to_string(),
            flow_schema_version: "1flowbase.flow/v2".to_string(),
            document_hash: "test-document-hash".to_string(),
            run_mode: FlowRunMode::DebugFlowRun,
            target_node_id: None,
            title: "hello".to_string(),
            status: FlowRunStatus::Queued,
            input_payload: json!({ "node-start": { "query": "hello" } }),
            started_at: OffsetDateTime::now_utc(),
            api_key_id: None,
            publication_version_id: None,
            external_user: None,
            external_conversation_id: None,
            external_trace_id: None,
            compatibility_mode: None,
            idempotency_key: None,
        },
    )
    .await
    .unwrap();

    assert_eq!(shell.compiled_plan_id, None);
    assert_eq!(shell.status, domain::FlowRunStatus::Queued);

    let compiled = <PgControlPlaneStore as OrchestrationRuntimeRepository>::upsert_compiled_plan(
        &store,
        &UpsertCompiledPlanInput {
            actor_user_id: seeded.actor_user_id,
            flow_id: seeded.flow_id,
            flow_draft_id: seeded.draft_id,
            schema_version: "1flowbase.flow/v2".to_string(),
            document_hash: "test-document-hash".to_string(),
            document_updated_at: seeded.draft_updated_at,
            plan: json!({ "nodes": {}, "topological_order": [] }),
        },
    )
    .await
    .unwrap();

    let attached =
        <PgControlPlaneStore as OrchestrationRuntimeRepository>::attach_compiled_plan_to_flow_run(
            &store,
            &AttachCompiledPlanToFlowRunInput {
                flow_run_id: shell.id,
                compiled_plan_id: compiled.id,
                flow_schema_version: compiled.schema_version.clone(),
                document_hash: "test-document-hash".to_string(),
                status: FlowRunStatus::Running,
            },
        )
        .await
        .unwrap();

    assert_eq!(attached.compiled_plan_id, Some(compiled.id));
    assert_eq!(attached.status, FlowRunStatus::Running);
}

#[tokio::test]
async fn compiled_plan_rows_are_immutable_per_compile_and_attach_checks_document_scope() {
    let pool = connect(&isolated_database_url().await).await.unwrap();
    run_migrations(&pool).await.unwrap();
    let store = PgControlPlaneStore::new(pool);
    let seeded = seed_runtime_base(&store).await;

    let first = <PgControlPlaneStore as OrchestrationRuntimeRepository>::upsert_compiled_plan(
        &store,
        &UpsertCompiledPlanInput {
            actor_user_id: seeded.actor_user_id,
            flow_id: seeded.flow_id,
            flow_draft_id: seeded.draft_id,
            schema_version: "1flowbase.flow/v2".to_string(),
            document_hash: "document-hash-a".to_string(),
            document_updated_at: seeded.draft_updated_at,
            plan: json!({ "marker": "a", "nodes": {}, "topological_order": [] }),
        },
    )
    .await
    .unwrap();
    let second = <PgControlPlaneStore as OrchestrationRuntimeRepository>::upsert_compiled_plan(
        &store,
        &UpsertCompiledPlanInput {
            actor_user_id: seeded.actor_user_id,
            flow_id: seeded.flow_id,
            flow_draft_id: seeded.draft_id,
            schema_version: "1flowbase.flow/v2".to_string(),
            document_hash: "document-hash-b".to_string(),
            document_updated_at: seeded.draft_updated_at + time::Duration::seconds(1),
            plan: json!({ "marker": "b", "nodes": {}, "topological_order": [] }),
        },
    )
    .await
    .unwrap();

    assert_ne!(first.id, second.id);
    let stored_first = <PgControlPlaneStore as OrchestrationRuntimeRepository>::get_compiled_plan(
        &store, first.id,
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(stored_first.document_hash, "document-hash-a");
    assert_eq!(stored_first.plan["marker"], "a");

    let shell = <PgControlPlaneStore as OrchestrationRuntimeRepository>::create_flow_run_shell(
        &store,
        &CreateFlowRunShellInput {
            actor_user_id: seeded.actor_user_id,
            application_id: seeded.application_id,
            flow_id: seeded.flow_id,
            flow_draft_id: seeded.draft_id,
            debug_session_id: "test-debug-session".to_string(),
            flow_schema_version: "1flowbase.flow/v2".to_string(),
            document_hash: "document-hash-a".to_string(),
            run_mode: FlowRunMode::DebugFlowRun,
            target_node_id: None,
            title: "hello".to_string(),
            status: FlowRunStatus::Queued,
            input_payload: json!({ "node-start": { "query": "hello" } }),
            started_at: OffsetDateTime::now_utc(),
            api_key_id: None,
            publication_version_id: None,
            external_user: None,
            external_conversation_id: None,
            external_trace_id: None,
            compatibility_mode: None,
            idempotency_key: None,
        },
    )
    .await
    .unwrap();

    let err =
        <PgControlPlaneStore as OrchestrationRuntimeRepository>::attach_compiled_plan_to_flow_run(
            &store,
            &AttachCompiledPlanToFlowRunInput {
                flow_run_id: shell.id,
                compiled_plan_id: second.id,
                flow_schema_version: second.schema_version.clone(),
                document_hash: second.document_hash.clone(),
                status: FlowRunStatus::Running,
            },
        )
        .await
        .unwrap_err();

    assert!(err
        .to_string()
        .contains("flow run compiled plan cannot be attached"));
}

#[tokio::test]
async fn creates_flow_run_shell_and_attaches_compiled_plan_rejects_already_attached_shell() {
    let pool = connect(&isolated_database_url().await).await.unwrap();
    run_migrations(&pool).await.unwrap();
    let store = PgControlPlaneStore::new(pool);
    let seeded = seed_runtime_base(&store).await;

    let shell = <PgControlPlaneStore as OrchestrationRuntimeRepository>::create_flow_run_shell(
        &store,
        &CreateFlowRunShellInput {
            actor_user_id: seeded.actor_user_id,
            application_id: seeded.application_id,
            flow_id: seeded.flow_id,
            flow_draft_id: seeded.draft_id,
            debug_session_id: "test-debug-session".to_string(),
            flow_schema_version: "1flowbase.flow/v2".to_string(),
            document_hash: "test-document-hash".to_string(),
            run_mode: FlowRunMode::DebugFlowRun,
            target_node_id: None,
            title: "hello".to_string(),
            status: FlowRunStatus::Queued,
            input_payload: json!({ "node-start": { "query": "hello" } }),
            started_at: OffsetDateTime::now_utc(),
            api_key_id: None,
            publication_version_id: None,
            external_user: None,
            external_conversation_id: None,
            external_trace_id: None,
            compatibility_mode: None,
            idempotency_key: None,
        },
    )
    .await
    .unwrap();
    let compiled = seed_compiled_plan(&store, &seeded).await;

    <PgControlPlaneStore as OrchestrationRuntimeRepository>::attach_compiled_plan_to_flow_run(
        &store,
        &AttachCompiledPlanToFlowRunInput {
            flow_run_id: shell.id,
            compiled_plan_id: compiled.id,
            flow_schema_version: compiled.schema_version.clone(),
            document_hash: "test-document-hash".to_string(),
            status: FlowRunStatus::Running,
        },
    )
    .await
    .unwrap();

    let err =
        <PgControlPlaneStore as OrchestrationRuntimeRepository>::attach_compiled_plan_to_flow_run(
            &store,
            &AttachCompiledPlanToFlowRunInput {
                flow_run_id: shell.id,
                compiled_plan_id: compiled.id,
                flow_schema_version: compiled.schema_version.clone(),
                document_hash: "test-document-hash".to_string(),
                status: FlowRunStatus::Running,
            },
        )
        .await
        .unwrap_err();

    assert!(err
        .to_string()
        .contains("flow run compiled plan cannot be attached"));
    let stored = <PgControlPlaneStore as OrchestrationRuntimeRepository>::get_flow_run(
        &store,
        seeded.application_id,
        shell.id,
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(stored.compiled_plan_id, Some(compiled.id));
    assert_eq!(stored.status, FlowRunStatus::Running);
}

#[tokio::test]
async fn creates_flow_run_shell_and_attaches_compiled_plan_rejects_mismatched_compiled_plan() {
    let pool = connect(&isolated_database_url().await).await.unwrap();
    run_migrations(&pool).await.unwrap();
    let store = PgControlPlaneStore::new(pool);
    let seeded = seed_runtime_base(&store).await;
    let other_seeded = seed_runtime_base_with_workspace_name(&store, "Runtime Other").await;

    let shell = <PgControlPlaneStore as OrchestrationRuntimeRepository>::create_flow_run_shell(
        &store,
        &CreateFlowRunShellInput {
            actor_user_id: seeded.actor_user_id,
            application_id: seeded.application_id,
            flow_id: seeded.flow_id,
            flow_draft_id: seeded.draft_id,
            debug_session_id: "test-debug-session".to_string(),
            flow_schema_version: "1flowbase.flow/v2".to_string(),
            document_hash: "test-document-hash".to_string(),
            run_mode: FlowRunMode::DebugFlowRun,
            target_node_id: None,
            title: "hello".to_string(),
            status: FlowRunStatus::Queued,
            input_payload: json!({ "node-start": { "query": "hello" } }),
            started_at: OffsetDateTime::now_utc(),
            api_key_id: None,
            publication_version_id: None,
            external_user: None,
            external_conversation_id: None,
            external_trace_id: None,
            compatibility_mode: None,
            idempotency_key: None,
        },
    )
    .await
    .unwrap();
    let other_compiled = seed_compiled_plan(&store, &other_seeded).await;

    let err =
        <PgControlPlaneStore as OrchestrationRuntimeRepository>::attach_compiled_plan_to_flow_run(
            &store,
            &AttachCompiledPlanToFlowRunInput {
                flow_run_id: shell.id,
                compiled_plan_id: other_compiled.id,
                flow_schema_version: other_compiled.schema_version.clone(),
                document_hash: "test-document-hash".to_string(),
                status: FlowRunStatus::Running,
            },
        )
        .await
        .unwrap_err();

    assert!(err
        .to_string()
        .contains("flow run compiled plan cannot be attached"));
    let stored = <PgControlPlaneStore as OrchestrationRuntimeRepository>::get_flow_run(
        &store,
        seeded.application_id,
        shell.id,
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(stored.compiled_plan_id, None);
    assert_eq!(stored.status, FlowRunStatus::Queued);
}

#[tokio::test]
async fn update_flow_run_if_status_does_not_overwrite_cancelled_run() {
    let pool = connect(&isolated_database_url().await).await.unwrap();
    run_migrations(&pool).await.unwrap();
    let store = PgControlPlaneStore::new(pool);
    let seeded = seed_runtime_base(&store).await;
    let compiled = seed_compiled_plan(&store, &seeded).await;
    let run = seed_flow_run_with_mode(
        &store,
        &seeded,
        &compiled,
        OffsetDateTime::now_utc(),
        FlowRunMode::DebugFlowRun,
        None,
    )
    .await;

    let cancelled = <PgControlPlaneStore as OrchestrationRuntimeRepository>::update_flow_run(
        &store,
        &UpdateFlowRunInput {
            flow_run_id: run.id,
            status: FlowRunStatus::Cancelled,
            output_payload: json!({}),
            error_payload: None,
            finished_at: Some(OffsetDateTime::now_utc()),
        },
    )
    .await
    .unwrap();
    assert_eq!(cancelled.status, FlowRunStatus::Cancelled);

    let updated =
        <PgControlPlaneStore as OrchestrationRuntimeRepository>::update_flow_run_if_status(
            &store,
            &UpdateFlowRunInput {
                flow_run_id: run.id,
                status: FlowRunStatus::Succeeded,
                output_payload: json!({ "answer": "done" }),
                error_payload: None,
                finished_at: Some(OffsetDateTime::now_utc()),
            },
            FlowRunStatus::Running,
        )
        .await
        .unwrap();

    assert!(updated.is_none());
    let stored = <PgControlPlaneStore as OrchestrationRuntimeRepository>::get_flow_run(
        &store,
        seeded.application_id,
        run.id,
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(stored.status, FlowRunStatus::Cancelled);
    assert_eq!(stored.output_payload, json!({}));
}

#[tokio::test]
async fn update_flow_run_if_status_returns_not_found_for_missing_run() {
    let pool = connect(&isolated_database_url().await).await.unwrap();
    run_migrations(&pool).await.unwrap();
    let store = PgControlPlaneStore::new(pool);

    let error = <PgControlPlaneStore as OrchestrationRuntimeRepository>::update_flow_run_if_status(
        &store,
        &UpdateFlowRunInput {
            flow_run_id: Uuid::now_v7(),
            status: FlowRunStatus::Succeeded,
            output_payload: json!({ "answer": "done" }),
            error_payload: None,
            finished_at: Some(OffsetDateTime::now_utc()),
        },
        FlowRunStatus::Running,
    )
    .await
    .unwrap_err();

    assert!(matches!(
        error.downcast_ref::<ControlPlaneError>(),
        Some(ControlPlaneError::NotFound("flow_run"))
    ));
}
