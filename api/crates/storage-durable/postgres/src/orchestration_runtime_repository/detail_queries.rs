use anyhow::Result;
use uuid::Uuid;

use crate::repositories::PgControlPlaneStore;

use super::record_mappers::{
    map_callback_task_record, map_checkpoint_record, map_flow_run_record, map_node_run_record,
    map_run_event_record, map_runtime_event_record,
};

pub(super) async fn fetch_flow_run_for_application(
    store: &PgControlPlaneStore,
    application_id: Uuid,
    flow_run_id: Uuid,
) -> Result<Option<domain::FlowRunRecord>> {
    let row = sqlx::query(
        r#"
        select
            id,
            application_id,
            flow_id,
            flow_draft_id,
            compiled_plan_id,
            debug_session_id,
            flow_schema_version,
            document_hash,
            run_mode,
            target_node_id,
            title,
            status,
            input_payload,
            output_payload,
            error_payload,
            created_by,
            (
                select users.account
                from users
                where users.id = flow_runs.created_by
            ) as authorized_account,
            api_key_id,
            publication_version_id,
            external_user,
            external_conversation_id,
            external_trace_id,
            compatibility_mode,
            idempotency_key,
            started_at,
            finished_at,
            created_at,
            updated_at
        from flow_runs
        where application_id = $1
          and id = $2
          and (
              flow_runs.import_job_id is null
              or exists (
                  select 1
                  from run_archive_import_jobs import_jobs
                  where import_jobs.id = flow_runs.import_job_id
                    and import_jobs.status = 'succeeded'
              )
          )
        "#,
    )
    .bind(application_id)
    .bind(flow_run_id)
    .fetch_optional(store.pool())
    .await?;

    row.map(map_flow_run_record).transpose()
}

pub(super) async fn fetch_node_run(
    store: &PgControlPlaneStore,
    node_run_id: Uuid,
) -> Result<Option<domain::NodeRunRecord>> {
    let row = sqlx::query(
        r#"
        select
            id,
            flow_run_id,
            node_id,
            node_type,
            node_alias,
            status,
            input_payload,
            output_payload,
            error_payload,
            metrics_payload,
            debug_payload,
            started_at,
            finished_at
        from node_runs
        where id = $1
        "#,
    )
    .bind(node_run_id)
    .fetch_optional(store.pool())
    .await?;

    row.map(map_node_run_record).transpose()
}

pub(super) async fn list_node_runs_for_flow_run(
    store: &PgControlPlaneStore,
    flow_run_id: Uuid,
) -> Result<Vec<domain::NodeRunRecord>> {
    let rows = sqlx::query(
        r#"
        select
            id,
            flow_run_id,
            node_id,
            node_type,
            node_alias,
            status,
            input_payload,
            output_payload,
            error_payload,
            metrics_payload,
            debug_payload,
            started_at,
            finished_at
        from node_runs
        where flow_run_id = $1
        order by started_at asc, id asc
        "#,
    )
    .bind(flow_run_id)
    .fetch_all(store.pool())
    .await?;

    rows.into_iter().map(map_node_run_record).collect()
}

pub(super) async fn list_checkpoints_for_flow_run(
    store: &PgControlPlaneStore,
    flow_run_id: Uuid,
) -> Result<Vec<domain::CheckpointRecord>> {
    let rows = sqlx::query(
        r#"
        select
            id,
            flow_run_id,
            node_run_id,
            status,
            reason,
            locator_payload,
            variable_snapshot,
            external_ref_payload,
            created_at
        from flow_run_checkpoints
        where flow_run_id = $1
        order by created_at asc, id asc
        "#,
    )
    .bind(flow_run_id)
    .fetch_all(store.pool())
    .await?;

    Ok(rows.into_iter().map(map_checkpoint_record).collect())
}

pub(super) async fn list_checkpoints_for_node_run(
    store: &PgControlPlaneStore,
    node_run_id: Uuid,
) -> Result<Vec<domain::CheckpointRecord>> {
    let rows = sqlx::query(
        r#"
        select
            id,
            flow_run_id,
            node_run_id,
            status,
            reason,
            locator_payload,
            variable_snapshot,
            external_ref_payload,
            created_at
        from flow_run_checkpoints
        where node_run_id = $1
        order by created_at asc, id asc
        "#,
    )
    .bind(node_run_id)
    .fetch_all(store.pool())
    .await?;

    Ok(rows.into_iter().map(map_checkpoint_record).collect())
}

pub(super) async fn list_events_for_flow_run(
    store: &PgControlPlaneStore,
    flow_run_id: Uuid,
) -> Result<Vec<domain::RunEventRecord>> {
    let rows = sqlx::query(
        r#"
        select
            id,
            flow_run_id,
            node_run_id,
            sequence,
            event_type,
            payload,
            created_at
        from flow_run_events
        where flow_run_id = $1
        order by sequence asc, id asc
        "#,
    )
    .bind(flow_run_id)
    .fetch_all(store.pool())
    .await?;

    Ok(rows.into_iter().map(map_run_event_record).collect())
}

pub(super) async fn list_callback_tasks_for_flow_run(
    store: &PgControlPlaneStore,
    flow_run_id: Uuid,
) -> Result<Vec<domain::CallbackTaskRecord>> {
    let rows = sqlx::query(
        r#"
        select
            id,
            flow_run_id,
            node_run_id,
            callback_kind,
            status,
            request_payload,
            response_payload,
            external_ref_payload,
            created_at,
            completed_at
        from flow_run_callback_tasks
        where flow_run_id = $1
        order by created_at asc, id asc
        "#,
    )
    .bind(flow_run_id)
    .fetch_all(store.pool())
    .await?;

    rows.into_iter().map(map_callback_task_record).collect()
}

pub(super) async fn list_stitched_trace_source_runs_for_flow_run(
    store: &PgControlPlaneStore,
    current_run: &domain::FlowRunRecord,
) -> Result<Vec<domain::FlowRunRecord>> {
    let Some(external_conversation_id) = current_run
        .external_conversation_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Ok(Vec::new());
    };
    let Some(external_user) = current_run
        .external_user
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Ok(Vec::new());
    };

    let rows = sqlx::query(
        r#"
        select
            prior.id,
            prior.application_id,
            prior.flow_id,
            prior.flow_draft_id,
            prior.compiled_plan_id,
            prior.debug_session_id,
            prior.flow_schema_version,
            prior.document_hash,
            prior.run_mode,
            prior.target_node_id,
            prior.title,
            prior.status,
            prior.input_payload,
            prior.output_payload,
            prior.error_payload,
            prior.created_by,
            (
                select users.account
                from users
                where users.id = prior.created_by
            ) as authorized_account,
            prior.api_key_id,
            prior.publication_version_id,
            prior.external_user,
            prior.external_conversation_id,
            prior.external_trace_id,
            prior.compatibility_mode,
            prior.idempotency_key,
            prior.started_at,
            prior.finished_at,
            prior.created_at,
            prior.updated_at
        from flow_runs prior
        where prior.application_id = $1
          and prior.external_conversation_id = $2
          and prior.id <> $3
          and prior.started_at < $4
          and prior.external_user = $5
          and prior.api_key_id is not distinct from $6
          and prior.compatibility_mode is not distinct from $7
          and prior.status in ('cancelled', 'waiting_callback')
          and (
              prior.import_job_id is null
              or exists (
                  select 1
                  from run_archive_import_jobs prior_import_jobs
                  where prior_import_jobs.id = prior.import_job_id
                    and prior_import_jobs.status = 'succeeded'
              )
          )
          and not exists (
              select 1
              from flow_runs boundary
              where boundary.application_id = prior.application_id
                and boundary.external_conversation_id = prior.external_conversation_id
                and boundary.external_user = prior.external_user
                and boundary.api_key_id is not distinct from prior.api_key_id
                and boundary.compatibility_mode is not distinct from prior.compatibility_mode
                and boundary.id <> $3
                and boundary.started_at > prior.started_at
                and boundary.started_at < $4
                and boundary.status in ('succeeded', 'failed')
                and (
                    boundary.import_job_id is null
                    or exists (
                        select 1
                        from run_archive_import_jobs boundary_import_jobs
                        where boundary_import_jobs.id = boundary.import_job_id
                          and boundary_import_jobs.status = 'succeeded'
                    )
                )
          )
        order by prior.started_at asc, prior.id asc
        limit 12
        "#,
    )
    .bind(current_run.application_id)
    .bind(external_conversation_id)
    .bind(current_run.id)
    .bind(current_run.started_at)
    .bind(external_user)
    .bind(current_run.api_key_id)
    .bind(current_run.compatibility_mode.as_deref())
    .fetch_all(store.pool())
    .await?;

    rows.into_iter().map(map_flow_run_record).collect()
}

pub(super) async fn list_runtime_events_for_flow_run(
    store: &PgControlPlaneStore,
    flow_run_id: Uuid,
) -> Result<Vec<domain::RuntimeEventRecord>> {
    let rows = sqlx::query(
        r#"
        select
            id,
            flow_run_id,
            node_run_id,
            span_id,
            parent_span_id,
            sequence,
            event_type,
            layer,
            source,
            trust_level,
            item_id,
            ledger_ref,
            payload,
            visibility,
            durability,
            created_at
        from runtime_events
        where flow_run_id = $1
        order by sequence asc, id asc
        "#,
    )
    .bind(flow_run_id)
    .fetch_all(store.pool())
    .await?;

    rows.into_iter().map(map_runtime_event_record).collect()
}

pub(super) async fn list_stitched_trace_for_flow_run(
    store: &PgControlPlaneStore,
    flow_run: &domain::FlowRunRecord,
) -> Result<Vec<domain::ApplicationRunStitchedTrace>> {
    let source_runs = list_stitched_trace_source_runs_for_flow_run(store, flow_run).await?;
    let mut stitched_trace = Vec::with_capacity(source_runs.len());

    for source_flow_run in source_runs {
        stitched_trace.push(domain::ApplicationRunStitchedTrace {
            node_runs: list_node_runs_for_flow_run(store, source_flow_run.id).await?,
            callback_tasks: list_callback_tasks_for_flow_run(store, source_flow_run.id).await?,
            events: list_events_for_flow_run(store, source_flow_run.id).await?,
            runtime_events: list_runtime_events_for_flow_run(store, source_flow_run.id).await?,
            source_flow_run,
        });
    }

    Ok(stitched_trace)
}

pub(super) async fn list_events_for_node_context(
    store: &PgControlPlaneStore,
    flow_run_id: Uuid,
    node_run_id: Uuid,
) -> Result<Vec<domain::RunEventRecord>> {
    let rows = sqlx::query(
        r#"
        select
            id,
            flow_run_id,
            node_run_id,
            sequence,
            event_type,
            payload,
            created_at
        from flow_run_events
        where flow_run_id = $1
          and (node_run_id is null or node_run_id = $2)
        order by sequence asc, id asc
        "#,
    )
    .bind(flow_run_id)
    .bind(node_run_id)
    .fetch_all(store.pool())
    .await?;

    Ok(rows.into_iter().map(map_run_event_record).collect())
}
