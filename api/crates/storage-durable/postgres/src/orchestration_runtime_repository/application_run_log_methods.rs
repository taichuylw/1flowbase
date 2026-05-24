impl PgControlPlaneStore {
    async fn upsert_application_run_log_summary_for_flow_run(
        &self,
        flow_run: &domain::FlowRunRecord,
    ) -> Result<()> {
        if !is_terminal_application_run_log_status(flow_run.status) {
            return Ok(());
        }

        let Some(detail) = self
            .get_application_run_detail(flow_run.application_id, flow_run.id)
            .await?
        else {
            return Ok(());
        };
        let statistics = application_run_log_statistics(&detail);
        let flow_run = detail.flow_run;
        let display_title = control_plane::flow_run_title::display_flow_run_title(
            &flow_run.title,
            &flow_run.input_payload,
        );

        sqlx::query(
            r#"
            insert into application_run_log_summaries (
                flow_run_id,
                application_id,
                run_mode,
                status,
                target_node_id,
                title,
                input_payload,
                external_user,
                authorized_account,
                api_key_id,
                publication_version_id,
                external_conversation_id,
                external_trace_id,
                compatibility_mode,
                idempotency_key,
                total_tokens,
                unique_node_count,
                tool_callback_count,
                started_at,
                finished_at,
                created_at,
                updated_at
            ) values (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11,
                $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22
            )
            on conflict (flow_run_id) do update
            set application_id = excluded.application_id,
                run_mode = excluded.run_mode,
                status = excluded.status,
                target_node_id = excluded.target_node_id,
                title = excluded.title,
                input_payload = excluded.input_payload,
                external_user = excluded.external_user,
                authorized_account = excluded.authorized_account,
                api_key_id = excluded.api_key_id,
                publication_version_id = excluded.publication_version_id,
                external_conversation_id = excluded.external_conversation_id,
                external_trace_id = excluded.external_trace_id,
                compatibility_mode = excluded.compatibility_mode,
                idempotency_key = excluded.idempotency_key,
                total_tokens = excluded.total_tokens,
                unique_node_count = excluded.unique_node_count,
                tool_callback_count = excluded.tool_callback_count,
                started_at = excluded.started_at,
                finished_at = excluded.finished_at,
                created_at = excluded.created_at,
                updated_at = excluded.updated_at,
                log_updated_at = now()
            "#,
        )
        .bind(flow_run.id)
        .bind(flow_run.application_id)
        .bind(flow_run.run_mode.as_str())
        .bind(flow_run.status.as_str())
        .bind(&flow_run.target_node_id)
        .bind(&display_title)
        .bind(serde_json::json!({}))
        .bind(&flow_run.external_user)
        .bind(&flow_run.authorized_account)
        .bind(flow_run.api_key_id)
        .bind(flow_run.publication_version_id)
        .bind(&flow_run.external_conversation_id)
        .bind(&flow_run.external_trace_id)
        .bind(&flow_run.compatibility_mode)
        .bind(&flow_run.idempotency_key)
        .bind(statistics.total_tokens)
        .bind(statistics.unique_node_count)
        .bind(statistics.tool_callback_count)
        .bind(flow_run.started_at)
        .bind(flow_run.finished_at)
        .bind(flow_run.created_at)
        .bind(flow_run.updated_at)
        .execute(self.pool())
        .await?;

        Ok(())
    }

    async fn list_application_run_logs_page(
        &self,
        application_id: Uuid,
        input: ListApplicationRunsPageInput,
    ) -> Result<control_plane::ports::ApplicationRunLogSummaryPage> {
        let page = input.page.max(1);
        let page_size = input.page_size.clamp(1, 100);
        let offset = (page - 1) * page_size;
        let created_after = input.created_after;
        let order_by = Self::application_runs_page_order_by(
            input.sort_by.as_deref(),
            input.sort_order.as_deref(),
        );

        let total = sqlx::query_scalar::<_, i64>(
            r#"
            with rows as (
                select flow_run_id as id
                from application_run_log_summaries
                where application_id = $1
                  and ($2::timestamptz is null or created_at >= $2)
                union all
                select flow_runs.id
                from flow_runs
                where application_id = $1
                  and ($2::timestamptz is null or created_at >= $2)
                  and not exists (
                      select 1
                      from application_run_log_summaries logs
                      where logs.flow_run_id = flow_runs.id
                  )
            )
            select count(*) from rows
            "#,
        )
        .bind(application_id)
        .bind(created_after)
        .fetch_one(self.pool())
        .await?;

        let rows = sqlx::query(&format!(
            r#"
            with node_token_values as (
                select
                    flow_run_id,
                    node_id,
                    case
                        when metrics_payload #>> '{{usage,total_tokens}}' ~ '^-?[0-9]+$'
                            then (metrics_payload #>> '{{usage,total_tokens}}')::bigint
                        when metrics_payload #>> '{{usage,input_tokens}}' ~ '^-?[0-9]+$'
                            or metrics_payload #>> '{{usage,output_tokens}}' ~ '^-?[0-9]+$'
                            or metrics_payload #>> '{{usage,reasoning_tokens}}' ~ '^-?[0-9]+$'
                            then
                                coalesce(
                                    case
                                        when metrics_payload #>> '{{usage,input_tokens}}' ~ '^-?[0-9]+$'
                                            then (metrics_payload #>> '{{usage,input_tokens}}')::bigint
                                    end,
                                    0
                                )
                                + coalesce(
                                    case
                                        when metrics_payload #>> '{{usage,output_tokens}}' ~ '^-?[0-9]+$'
                                            then (metrics_payload #>> '{{usage,output_tokens}}')::bigint
                                    end,
                                    0
                                )
                                + coalesce(
                                    case
                                        when metrics_payload #>> '{{usage,reasoning_tokens}}' ~ '^-?[0-9]+$'
                                            then (metrics_payload #>> '{{usage,reasoning_tokens}}')::bigint
                                    end,
                                    0
                                )
                        else null
                    end as total_tokens
                from node_runs
            ),
            node_stats as (
                select
                    flow_run_id,
                    sum(total_tokens)::bigint as total_tokens,
                    count(distinct node_id)::bigint as unique_node_count
                from node_token_values
                group by flow_run_id
            ),
            callback_stats as (
                select
                    flow_run_id,
                    coalesce(
                        sum(
                            case
                                when callback_kind = 'llm_tool_calls'
                                    and jsonb_typeof(request_payload -> 'tool_calls') = 'array'
                                    then jsonb_array_length(request_payload -> 'tool_calls')
                                else 0
                            end
                        ),
                        0
                    )::bigint as tool_callback_count
                from flow_run_callback_tasks
                group by flow_run_id
            ),
            rows as (
                select
                    flow_run_id as id,
                    run_mode,
                    status,
                    target_node_id,
                    title,
                    input_payload,
                    external_user,
                    authorized_account,
                    api_key_id,
                    publication_version_id,
                    external_conversation_id,
                    external_trace_id,
                    compatibility_mode,
                    idempotency_key,
                    total_tokens,
                    unique_node_count,
                    tool_callback_count,
                    started_at,
                    finished_at,
                    created_at,
                    updated_at
                from application_run_log_summaries
                where application_id = $1
                  and ($2::timestamptz is null or created_at >= $2)
                union all
                select
                    flow_runs.id,
                    flow_runs.run_mode,
                    flow_runs.status,
                    flow_runs.target_node_id,
                    flow_runs.title,
                    flow_runs.input_payload,
                    flow_runs.external_user,
                    (
                        select users.account
                        from users
                        where users.id = flow_runs.created_by
                    ) as authorized_account,
                    flow_runs.api_key_id,
                    flow_runs.publication_version_id,
                    flow_runs.external_conversation_id,
                    flow_runs.external_trace_id,
                    flow_runs.compatibility_mode,
                    flow_runs.idempotency_key,
                    node_stats.total_tokens,
                    coalesce(node_stats.unique_node_count, 0)::bigint as unique_node_count,
                    coalesce(callback_stats.tool_callback_count, 0)::bigint as tool_callback_count,
                    flow_runs.started_at,
                    flow_runs.finished_at,
                    flow_runs.created_at,
                    flow_runs.updated_at
                from flow_runs
                left join node_stats on node_stats.flow_run_id = flow_runs.id
                left join callback_stats on callback_stats.flow_run_id = flow_runs.id
                where flow_runs.application_id = $1
                  and ($2::timestamptz is null or flow_runs.created_at >= $2)
                  and not exists (
                      select 1
                      from application_run_log_summaries logs
                      where logs.flow_run_id = flow_runs.id
                  )
            )
            select
                id,
                run_mode,
                status,
                target_node_id,
                title,
                input_payload,
                external_user,
                authorized_account,
                api_key_id,
                publication_version_id,
                external_conversation_id,
                external_trace_id,
                compatibility_mode,
                idempotency_key,
                total_tokens,
                unique_node_count,
                tool_callback_count,
                started_at,
                finished_at,
                created_at,
                updated_at
            from rows
            order by {}
            limit $3 offset $4
            "#,
            order_by
        ))
        .bind(application_id)
        .bind(created_after)
        .bind(page_size)
        .bind(offset)
        .fetch_all(self.pool())
        .await?;

        Ok(control_plane::ports::ApplicationRunLogSummaryPage {
            items: rows
                .into_iter()
                .map(map_application_run_log_summary)
                .collect::<Result<Vec<_>>>()?,
            total,
            page,
            page_size,
        })
    }
}

#[derive(Debug, Clone, Copy)]
struct ApplicationRunLogStatistics {
    total_tokens: Option<i64>,
    unique_node_count: i64,
    tool_callback_count: i64,
}

fn is_terminal_application_run_log_status(status: domain::FlowRunStatus) -> bool {
    matches!(
        status,
        domain::FlowRunStatus::Succeeded
            | domain::FlowRunStatus::Failed
            | domain::FlowRunStatus::Cancelled
    )
}

fn application_run_log_statistics(
    detail: &domain::ApplicationRunDetail,
) -> ApplicationRunLogStatistics {
    let mut unique_node_ids = std::collections::HashSet::new();
    let mut total_tokens = None;

    for node_run in &detail.node_runs {
        unique_node_ids.insert(node_run.node_id.as_str());

        if let Some(node_tokens) = metrics_payload_total_tokens(&node_run.metrics_payload) {
            total_tokens = Some(total_tokens.unwrap_or(0) + node_tokens);
        }
    }

    ApplicationRunLogStatistics {
        total_tokens,
        unique_node_count: unique_node_ids.len() as i64,
        tool_callback_count: detail
            .callback_tasks
            .iter()
            .map(callback_task_tool_callback_count)
            .sum(),
    }
}

fn metrics_payload_total_tokens(metrics_payload: &serde_json::Value) -> Option<i64> {
    metrics_payload.get("usage").and_then(usage_total_tokens)
}

fn usage_total_tokens(usage: &serde_json::Value) -> Option<i64> {
    if let Some(total_tokens) = usage.get("total_tokens").and_then(usage_token_value) {
        return Some(total_tokens);
    }

    let segments = ["input_tokens", "output_tokens", "reasoning_tokens"];
    let mut total = 0_i64;
    let mut has_segment = false;

    for segment in segments {
        if let Some(tokens) = usage.get(segment).and_then(usage_token_value) {
            total += tokens;
            has_segment = true;
        }
    }

    has_segment.then_some(total)
}

fn usage_token_value(value: &serde_json::Value) -> Option<i64> {
    value
        .as_i64()
        .or_else(|| value.as_u64().and_then(|value| i64::try_from(value).ok()))
}

fn callback_task_tool_callback_count(task: &domain::CallbackTaskRecord) -> i64 {
    if task.callback_kind != "llm_tool_calls" {
        return 0;
    }

    task.request_payload
        .get("tool_calls")
        .and_then(serde_json::Value::as_array)
        .map(|tool_calls| tool_calls.len() as i64)
        .unwrap_or(0)
}
