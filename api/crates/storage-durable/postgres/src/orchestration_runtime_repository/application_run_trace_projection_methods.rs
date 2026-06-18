impl PgControlPlaneStore {
    async fn replace_application_run_trace_projection(
        &self,
        input: &ReplaceApplicationRunTraceProjectionInput,
    ) -> Result<()> {
        let mut tx = self.pool().begin().await?;

        sqlx::query(
            r#"
            delete from application_run_trace_node_contents
            where trace_node_id in (
                select trace_node_id
                from application_run_trace_nodes
                where flow_run_id = $1
            )
            "#,
        )
        .bind(input.flow_run_id)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            "delete from application_run_trace_nodes where flow_run_id = $1",
        )
        .bind(input.flow_run_id)
        .execute(&mut *tx)
        .await?;

        for node in &input.nodes {
            sqlx::query(
                r#"
                insert into application_run_trace_nodes (
                    trace_node_id,
                    flow_run_id,
                    parent_trace_node_id,
                    stable_locator,
                    node_kind,
                    owner_kind,
                    owner_id,
                    order_key,
                    node_id,
                    node_type,
                    node_alias,
                    status,
                    started_at,
                    finished_at,
                    duration_ms,
                    metrics_payload,
                    has_children,
                    child_count,
                    has_content,
                    content_ref,
                    projection_version,
                    source_watermark
                ) values (
                    $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12,
                    $13, $14, $15, $16, $17, $18, $19, $20, $21, $22
                )
                "#,
            )
            .bind(node.trace_node_id)
            .bind(input.flow_run_id)
            .bind(node.parent_trace_node_id)
            .bind(&node.stable_locator)
            .bind(&node.node_kind)
            .bind(&node.owner_kind)
            .bind(&node.owner_id)
            .bind(&node.order_key)
            .bind(&node.node_id)
            .bind(&node.node_type)
            .bind(&node.node_alias)
            .bind(&node.status)
            .bind(node.started_at)
            .bind(node.finished_at)
            .bind(node.duration_ms)
            .bind(&node.metrics_payload)
            .bind(node.has_children)
            .bind(node.child_count)
            .bind(node.has_content)
            .bind(&node.content_ref)
            .bind(input.projection_version)
            .bind(&input.source_watermark)
            .execute(&mut *tx)
            .await?;
        }

        for content in &input.contents {
            sqlx::query(
                r#"
                insert into application_run_trace_node_contents (
                    trace_node_id,
                    content_kind,
                    payload,
                    source_refs
                ) values ($1, $2, $3, $4)
                "#,
            )
            .bind(content.trace_node_id)
            .bind(&content.content_kind)
            .bind(&content.payload)
            .bind(&content.source_refs)
            .execute(&mut *tx)
            .await?;
        }

        upsert_application_run_trace_projection_status_in_tx(
            &mut tx,
            &UpsertApplicationRunTraceProjectionStatusInput {
                flow_run_id: input.flow_run_id,
                projection_version: input.projection_version,
                status: domain::ApplicationRunTraceProjectionStatus::Succeeded,
                source_watermark: input.source_watermark.clone(),
                attempt_count: 1,
                last_attempt_at: Some(OffsetDateTime::now_utc()),
                last_success_at: Some(OffsetDateTime::now_utc()),
                diagnostic: None,
            },
        )
        .await?;

        tx.commit().await?;
        Ok(())
    }

    async fn upsert_application_run_trace_projection_status(
        &self,
        input: &UpsertApplicationRunTraceProjectionStatusInput,
    ) -> Result<()> {
        let mut tx = self.pool().begin().await?;
        upsert_application_run_trace_projection_status_in_tx(&mut tx, input).await?;
        tx.commit().await?;
        Ok(())
    }

    async fn get_application_run_trace_projection_status(
        &self,
        flow_run_id: Uuid,
        projection_version: i32,
    ) -> Result<Option<domain::ApplicationRunTraceProjectionStatusRecord>> {
        let row = sqlx::query(
            r#"
            select
                flow_run_id,
                projection_version,
                status,
                source_watermark,
                attempt_count,
                last_attempt_at,
                last_success_at,
                last_error_code,
                last_error_stage,
                last_error_source_kind,
                last_error_source_locator,
                last_error_message,
                last_error_ref,
                retriable,
                created_at,
                updated_at
            from application_run_trace_projection_statuses
            where flow_run_id = $1
              and projection_version = $2
            "#,
        )
        .bind(flow_run_id)
        .bind(projection_version)
        .fetch_optional(self.pool())
        .await?;

        row.map(map_application_run_trace_projection_status_record)
            .transpose()
    }

    async fn list_application_run_trace_roots(
        &self,
        flow_run_id: Uuid,
    ) -> Result<Vec<domain::ApplicationRunTraceNodeRecord>> {
        let sql = trace_node_select_sql(
            "where flow_run_id = $1 and parent_trace_node_id is null order by order_key asc, trace_node_id asc",
        );
        let rows = sqlx::query(&sql)
        .bind(flow_run_id)
        .fetch_all(self.pool())
        .await?;

        rows.into_iter()
            .map(map_application_run_trace_node_record)
            .collect()
    }

    async fn list_application_run_trace_nodes_for_statistics(
        &self,
        flow_run_id: Uuid,
    ) -> Result<Vec<domain::ApplicationRunTraceNodeRecord>> {
        let sql = trace_node_select_sql(
            "where flow_run_id = $1 order by order_key asc, trace_node_id asc",
        );
        let rows = sqlx::query(&sql)
            .bind(flow_run_id)
            .fetch_all(self.pool())
            .await?;

        rows.into_iter()
            .map(map_application_run_trace_node_record)
            .collect()
    }

    async fn list_application_run_trace_children(
        &self,
        flow_run_id: Uuid,
        parent_trace_node_id: Uuid,
    ) -> Result<Vec<domain::ApplicationRunTraceNodeRecord>> {
        let sql = trace_node_select_sql(
            "where flow_run_id = $1 and parent_trace_node_id = $2 order by order_key asc, trace_node_id asc",
        );
        let rows = sqlx::query(&sql)
        .bind(flow_run_id)
        .bind(parent_trace_node_id)
        .fetch_all(self.pool())
        .await?;

        rows.into_iter()
            .map(map_application_run_trace_node_record)
            .collect()
    }

    async fn get_application_run_trace_node(
        &self,
        flow_run_id: Uuid,
        trace_node_id: Uuid,
    ) -> Result<Option<domain::ApplicationRunTraceNodeRecord>> {
        let sql = trace_node_select_sql("where flow_run_id = $1 and trace_node_id = $2");
        let row = sqlx::query(&sql)
            .bind(flow_run_id)
            .bind(trace_node_id)
            .fetch_optional(self.pool())
            .await?;

        row.map(map_application_run_trace_node_record).transpose()
    }

    async fn get_application_run_trace_node_by_locator(
        &self,
        flow_run_id: Uuid,
        stable_locator: &str,
    ) -> Result<Option<domain::ApplicationRunTraceNodeRecord>> {
        let sql = trace_node_select_sql("where flow_run_id = $1 and stable_locator = $2");
        let row = sqlx::query(&sql)
        .bind(flow_run_id)
        .bind(stable_locator)
        .fetch_optional(self.pool())
        .await?;

        row.map(map_application_run_trace_node_record).transpose()
    }

    async fn get_application_run_trace_node_content(
        &self,
        flow_run_id: Uuid,
        trace_node_id: Uuid,
    ) -> Result<Option<domain::ApplicationRunTraceNodeContentRecord>> {
        let row = sqlx::query(
            r#"
            select
                contents.trace_node_id,
                contents.content_kind,
                contents.payload,
                contents.source_refs,
                contents.created_at,
                contents.updated_at
            from application_run_trace_node_contents contents
            join application_run_trace_nodes nodes
              on nodes.trace_node_id = contents.trace_node_id
            where nodes.flow_run_id = $1
              and contents.trace_node_id = $2
            "#,
        )
        .bind(flow_run_id)
        .bind(trace_node_id)
        .fetch_optional(self.pool())
        .await?;

        row.map(map_application_run_trace_node_content_record)
            .transpose()
    }
}

async fn upsert_application_run_trace_projection_status_in_tx(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    input: &UpsertApplicationRunTraceProjectionStatusInput,
) -> Result<()> {
    let diagnostic = input.diagnostic.clone();

    sqlx::query(
        r#"
        insert into application_run_trace_projection_statuses (
            flow_run_id,
            projection_version,
            status,
            source_watermark,
            attempt_count,
            last_attempt_at,
            last_success_at,
            last_error_code,
            last_error_stage,
            last_error_source_kind,
            last_error_source_locator,
            last_error_message,
            last_error_ref,
            retriable
        ) values (
            $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14
        )
        on conflict (flow_run_id, projection_version) do update
        set status = excluded.status,
            source_watermark = excluded.source_watermark,
            attempt_count = excluded.attempt_count,
            last_attempt_at = excluded.last_attempt_at,
            last_success_at = excluded.last_success_at,
            last_error_code = excluded.last_error_code,
            last_error_stage = excluded.last_error_stage,
            last_error_source_kind = excluded.last_error_source_kind,
            last_error_source_locator = excluded.last_error_source_locator,
            last_error_message = excluded.last_error_message,
            last_error_ref = excluded.last_error_ref,
            retriable = excluded.retriable,
            updated_at = now()
        "#,
    )
    .bind(input.flow_run_id)
    .bind(input.projection_version)
    .bind(input.status.as_str())
    .bind(&input.source_watermark)
    .bind(input.attempt_count)
    .bind(input.last_attempt_at)
    .bind(input.last_success_at)
    .bind(diagnostic.as_ref().and_then(|value| value.last_error_code.as_deref()))
    .bind(diagnostic.as_ref().and_then(|value| value.last_error_stage.as_deref()))
    .bind(
        diagnostic
            .as_ref()
            .and_then(|value| value.last_error_source_kind.as_deref()),
    )
    .bind(
        diagnostic
            .as_ref()
            .and_then(|value| value.last_error_source_locator.as_deref()),
    )
    .bind(
        diagnostic
            .as_ref()
            .and_then(|value| value.last_error_message.as_deref()),
    )
    .bind(diagnostic.as_ref().and_then(|value| value.last_error_ref.as_deref()))
    .bind(diagnostic.as_ref().is_some_and(|value| value.retriable))
    .execute(&mut **tx)
    .await?;

    Ok(())
}

fn trace_node_select_sql(predicate: &str) -> String {
    format!(
        r#"
        select
            trace_node_id,
            flow_run_id,
            parent_trace_node_id,
            stable_locator,
            node_kind,
            owner_kind,
            owner_id,
            order_key,
            node_id,
            node_type,
            node_alias,
            status,
            started_at,
            finished_at,
            duration_ms,
            metrics_payload,
            has_children,
            child_count,
            has_content,
            content_ref,
            projection_version,
            source_watermark,
            created_at,
            updated_at
        from application_run_trace_nodes
        {predicate}
        "#
    )
}

fn map_application_run_trace_projection_status_record(
    row: sqlx::postgres::PgRow,
) -> Result<domain::ApplicationRunTraceProjectionStatusRecord> {
    let status = match row.get::<String, _>("status").as_str() {
        "pending" => domain::ApplicationRunTraceProjectionStatus::Pending,
        "running" => domain::ApplicationRunTraceProjectionStatus::Running,
        "succeeded" => domain::ApplicationRunTraceProjectionStatus::Succeeded,
        "failed" => domain::ApplicationRunTraceProjectionStatus::Failed,
        "stale" => domain::ApplicationRunTraceProjectionStatus::Stale,
        "partial" => domain::ApplicationRunTraceProjectionStatus::Partial,
        value => return Err(anyhow!("unknown trace projection status: {value}")),
    };

    Ok(domain::ApplicationRunTraceProjectionStatusRecord {
        flow_run_id: row.get("flow_run_id"),
        projection_version: row.get("projection_version"),
        status,
        source_watermark: row.get("source_watermark"),
        attempt_count: row.get("attempt_count"),
        last_attempt_at: row.get("last_attempt_at"),
        last_success_at: row.get("last_success_at"),
        last_error_code: row.get("last_error_code"),
        last_error_stage: row.get("last_error_stage"),
        last_error_source_kind: row.get("last_error_source_kind"),
        last_error_source_locator: row.get("last_error_source_locator"),
        last_error_message: row.get("last_error_message"),
        last_error_ref: row.get("last_error_ref"),
        retriable: row.get("retriable"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}

fn map_application_run_trace_node_record(
    row: sqlx::postgres::PgRow,
) -> Result<domain::ApplicationRunTraceNodeRecord> {
    Ok(domain::ApplicationRunTraceNodeRecord {
        trace_node_id: row.get("trace_node_id"),
        flow_run_id: row.get("flow_run_id"),
        parent_trace_node_id: row.get("parent_trace_node_id"),
        stable_locator: row.get("stable_locator"),
        node_kind: row.get("node_kind"),
        owner_kind: row.get("owner_kind"),
        owner_id: row.get("owner_id"),
        order_key: row.get("order_key"),
        node_id: row.get("node_id"),
        node_type: row.get("node_type"),
        node_alias: row.get("node_alias"),
        status: row.get("status"),
        started_at: row.get("started_at"),
        finished_at: row.get("finished_at"),
        duration_ms: row.get("duration_ms"),
        metrics_payload: row.get("metrics_payload"),
        has_children: row.get("has_children"),
        child_count: row.get("child_count"),
        has_content: row.get("has_content"),
        content_ref: row.get("content_ref"),
        projection_version: row.get("projection_version"),
        source_watermark: row.get("source_watermark"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}

fn map_application_run_trace_node_content_record(
    row: sqlx::postgres::PgRow,
) -> Result<domain::ApplicationRunTraceNodeContentRecord> {
    Ok(domain::ApplicationRunTraceNodeContentRecord {
        trace_node_id: row.get("trace_node_id"),
        content_kind: row.get("content_kind"),
        payload: row.get("payload"),
        source_refs: row.get("source_refs"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}
