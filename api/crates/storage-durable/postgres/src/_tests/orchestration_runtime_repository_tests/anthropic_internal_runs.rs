use super::*;

#[tokio::test]
async fn claude_code_builtin_agent_run_is_hidden_from_business_run_logs() {
    let pool = connect(&isolated_database_url().await).await.unwrap();
    run_migrations(&pool).await.unwrap();
    let store = PgControlPlaneStore::new(pool);
    let seeded = seed_runtime_base(&store).await;
    let compiled = seed_compiled_plan(&store, &seeded).await;
    let api_key_id = seed_application_api_key(&store, &seeded).await;
    let started_at = datetime!(2026-06-12 18:02:05 UTC);

    let agent_run = <PgControlPlaneStore as OrchestrationRuntimeRepository>::create_flow_run(
        &store,
        &CreateFlowRunInput {
            actor_user_id: seeded.actor_user_id,
            application_id: seeded.application_id,
            flow_id: seeded.flow_id,
            flow_draft_id: seeded.draft_id,
            compiled_plan_id: compiled.id,
            debug_session_id: "claude-code-builtin-agent".to_string(),
            flow_schema_version: compiled.schema_version.clone(),
            document_hash: compiled.document_hash.clone(),
            run_mode: FlowRunMode::PublishedApiRun,
            target_node_id: None,
            title: "Find UI navigation code".to_string(),
            status: FlowRunStatus::Running,
            input_payload: json!({
                "node-start": {
                    "query": "Search the codebase under /home/taichu/git/1flowbase/web for UI code.",
                    "system": "You are Claude Code, Anthropic's official CLI for Claude.\n\nYou are a file search specialist for Claude Code, Anthropic's official CLI for Claude.\n\nNotes:\n- Agent threads always have their cwd reset between bash calls, as a result please only use absolute file paths.\n- Do NOT Write report/summary/findings/analysis .md files. Return findings directly as your final assistant message - the parent agent reads your text output, not files you create."
                }
            }),
            started_at,
            api_key_id: Some(api_key_id),
            publication_version_id: Some(Uuid::now_v7()),
            external_user: Some("claude-code-user".to_string()),
            external_conversation_id: Some("claude-code-session".to_string()),
            external_trace_id: None,
            compatibility_mode: Some("anthropic-messages-v1".to_string()),
            idempotency_key: None,
        },
    )
    .await
    .unwrap();

    let raw_summary_count: i64 = sqlx::query_scalar(
        "select count(*)::bigint from application_run_log_summaries where flow_run_id = $1",
    )
    .bind(agent_run.id)
    .fetch_one(store.pool())
    .await
    .unwrap();
    assert_eq!(raw_summary_count, 0);

    <PgControlPlaneStore as OrchestrationRuntimeRepository>::update_flow_run(
        &store,
        &UpdateFlowRunInput {
            flow_run_id: agent_run.id,
            status: FlowRunStatus::Succeeded,
            output_payload: json!({ "answer": "web/app/src/app-shell/Navigation.tsx" }),
            error_payload: None,
            finished_at: Some(started_at + Duration::seconds(20)),
        },
    )
    .await
    .unwrap();

    let projected_message_count: i64 = sqlx::query_scalar(
        "select count(*)::bigint from application_conversation_messages where flow_run_id = $1",
    )
    .bind(agent_run.id)
    .fetch_one(store.pool())
    .await
    .unwrap();
    assert_eq!(projected_message_count, 0);

    let logs =
        <PgControlPlaneStore as OrchestrationRuntimeRepository>::list_application_run_logs_page(
            &store,
            seeded.application_id,
            ListApplicationRunsPageInput {
                page: 1,
                page_size: 20,
                created_after: None,
                sort_by: Some("created_at".to_string()),
                sort_order: Some("desc".to_string()),
            },
        )
        .await
        .unwrap();
    assert_eq!(logs.total, 0);
    assert!(logs.items.is_empty());

    sqlx::query(
        r#"
        insert into application_run_log_summaries (
            flow_run_id,
            scope_id,
            application_id,
            run_mode,
            status,
            target_node_id,
            title,
            input_payload,
            external_user,
            authorized_account,
            api_key_id,
            api_key_name_snapshot,
            publication_version_id,
            external_conversation_id,
            external_trace_id,
            compatibility_mode,
            idempotency_key,
            total_tokens,
            input_tokens,
            output_tokens,
            input_cache_hit_tokens,
            unique_node_count,
            tool_callback_count,
            started_at,
            finished_at,
            created_at,
            updated_at
        )
        select
            flow_runs.id,
            applications.workspace_id,
            flow_runs.application_id,
            flow_runs.run_mode,
            flow_runs.status,
            flow_runs.target_node_id,
            flow_runs.title,
            '{}'::jsonb,
            flow_runs.external_user,
            (
                select users.account
                from users
                where users.id = flow_runs.created_by
            ),
            flow_runs.api_key_id,
            null,
            flow_runs.publication_version_id,
            flow_runs.external_conversation_id,
            flow_runs.external_trace_id,
            flow_runs.compatibility_mode,
            flow_runs.idempotency_key,
            0,
            0,
            0,
            0,
            0,
            0,
            flow_runs.started_at,
            flow_runs.finished_at,
            flow_runs.created_at,
            flow_runs.updated_at
        from flow_runs
        join applications on applications.id = flow_runs.application_id
        where flow_runs.id = $1
        "#,
    )
    .bind(agent_run.id)
    .execute(store.pool())
    .await
    .unwrap();

    let logs =
        <PgControlPlaneStore as OrchestrationRuntimeRepository>::list_application_run_logs_page(
            &store,
            seeded.application_id,
            ListApplicationRunsPageInput {
                page: 1,
                page_size: 20,
                created_after: None,
                sort_by: Some("created_at".to_string()),
                sort_order: Some("desc".to_string()),
            },
        )
        .await
        .unwrap();
    assert_eq!(logs.total, 0);
    assert!(logs.items.is_empty());

    let report =
        <PgControlPlaneStore as OrchestrationRuntimeRepository>::get_application_run_monitoring_report(
            &store,
            seeded.application_id,
            GetApplicationRunMonitoringReportInput {
                started_from: Some(started_at - Duration::minutes(1)),
                started_to: Some(started_at + Duration::minutes(10)),
                bucket: "hour".to_string(),
                slow_run_threshold_ms: 30_000,
            },
        )
        .await
        .unwrap();
    assert_eq!(report.overview.total_count, 0);

    let conversation_runs =
        <PgControlPlaneStore as OrchestrationRuntimeRepository>::list_application_conversation_runs_page(
            &store,
            seeded.application_id,
            ListApplicationConversationRunsPageInput {
                external_conversation_id: "claude-code-session".to_string(),
                around_run_id: Some(agent_run.id),
                before_run_id: None,
                after_run_id: None,
                limit: 20,
            },
        )
        .await
        .unwrap();
    assert!(conversation_runs.items.is_empty());
}
