impl PgControlPlaneStore {
    async fn upsert_application_run_log_summary_for_flow_run(
        &self,
        flow_run: &domain::FlowRunRecord,
    ) -> Result<()> {
        let is_terminal = is_terminal_application_run_log_status(flow_run.status);
        let display_title = control_plane::flow_run_title::display_flow_run_title(
            &flow_run.title,
            &flow_run.input_payload,
        );
        let mut tx = self.pool().begin().await?;

        Self::upsert_application_run_log_summary_projection(&mut tx, flow_run, &display_title)
            .await?;
        tx.commit().await?;

        if is_terminal {
            self.upsert_application_conversation_messages_for_flow_run(flow_run)
                .await?;
        }

        Ok(())
    }

    async fn upsert_application_run_log_summary_projection(
        tx: &mut sqlx::Transaction<'_, Postgres>,
        flow_run: &domain::FlowRunRecord,
        display_title: &str,
    ) -> Result<()> {
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
                input_tokens, output_tokens, input_cache_hit_tokens,
                unique_node_count,
                tool_callback_count,
                started_at,
                finished_at,
                created_at,
                updated_at
            ) values (
                $1, (select workspace_id from applications where id = $2), $2, $3, $4,
                $5, $6, $7, $8,
                coalesce($9, (select users.account from users where users.id = $20)),
                $10, (select name from api_keys where id = $10),
                $11, $12, $13, $14, $15,
                coalesce(
                    (
                        select sum(runtime_usage_ledger.total_tokens)::bigint
                        from runtime_usage_ledger
                        where runtime_usage_ledger.flow_run_id = $1
                    ),
                    (
                        select sum(
                            case
                                when node_runs.metrics_payload #>> '{usage,total_tokens}' ~ '^-?[0-9]+$'
                                then (node_runs.metrics_payload #>> '{usage,total_tokens}')::bigint
                                when node_runs.metrics_payload #>> '{usage,input_tokens}' ~ '^-?[0-9]+$'
                                  or node_runs.metrics_payload #>> '{usage,output_tokens}' ~ '^-?[0-9]+$'
                                  or node_runs.metrics_payload #>> '{usage,reasoning_tokens}' ~ '^-?[0-9]+$'
                                then
                                    coalesce(
                                        case
                                            when node_runs.metrics_payload #>> '{usage,input_tokens}' ~ '^-?[0-9]+$'
                                            then (node_runs.metrics_payload #>> '{usage,input_tokens}')::bigint
                                        end,
                                        0
                                    )
                                    + coalesce(
                                        case
                                            when node_runs.metrics_payload #>> '{usage,output_tokens}' ~ '^-?[0-9]+$'
                                            then (node_runs.metrics_payload #>> '{usage,output_tokens}')::bigint
                                        end,
                                        0
                                    )
                                    + coalesce(
                                        case
                                            when node_runs.metrics_payload #>> '{usage,reasoning_tokens}' ~ '^-?[0-9]+$'
                                            then (node_runs.metrics_payload #>> '{usage,reasoning_tokens}')::bigint
                                        end,
                                        0
                                    )
                            end
                        )::bigint
                        from node_runs
                        where node_runs.flow_run_id = $1
                    )
                ),
                (select sum(runtime_usage_ledger.input_tokens)::bigint
                   from runtime_usage_ledger where runtime_usage_ledger.flow_run_id = $1),
                (select sum(runtime_usage_ledger.output_tokens)::bigint
                   from runtime_usage_ledger where runtime_usage_ledger.flow_run_id = $1),
                (select sum(coalesce(runtime_usage_ledger.input_cache_hit_tokens,
                                      runtime_usage_ledger.cache_read_tokens,
                                      runtime_usage_ledger.cached_input_tokens))::bigint
                   from runtime_usage_ledger where runtime_usage_ledger.flow_run_id = $1),
                coalesce(
                    (
                        select count(distinct node_runs.node_id)::bigint
                        from node_runs
                        where node_runs.flow_run_id = $1
                    ),
                    0
                ),
                coalesce(
                    (
                        select sum(
                            case
                                when jsonb_typeof(flow_run_callback_tasks.request_payload -> 'tool_calls') = 'array'
                                then jsonb_array_length(flow_run_callback_tasks.request_payload -> 'tool_calls')::bigint
                                else 0
                            end
                        )::bigint
                        from flow_run_callback_tasks
                        where flow_run_callback_tasks.flow_run_id = $1
                          and flow_run_callback_tasks.callback_kind = 'llm_tool_calls'
                    ),
                    0
                ),
                $16, $17, $18, $19
            )
            on conflict (flow_run_id) do update
            set application_id = excluded.application_id,
                scope_id = excluded.scope_id,
                run_mode = excluded.run_mode,
                status = excluded.status,
                target_node_id = excluded.target_node_id,
                title = excluded.title,
                input_payload = excluded.input_payload,
                external_user = excluded.external_user,
                authorized_account = excluded.authorized_account,
                api_key_id = excluded.api_key_id,
                api_key_name_snapshot = coalesce(
                    excluded.api_key_name_snapshot,
                    application_run_log_summaries.api_key_name_snapshot
                ),
                publication_version_id = excluded.publication_version_id,
                external_conversation_id = excluded.external_conversation_id,
                external_trace_id = excluded.external_trace_id,
                compatibility_mode = excluded.compatibility_mode,
                idempotency_key = excluded.idempotency_key,
                total_tokens = excluded.total_tokens,
                input_tokens = excluded.input_tokens,
                output_tokens = excluded.output_tokens,
                input_cache_hit_tokens = excluded.input_cache_hit_tokens,
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
        .bind(display_title)
        .bind(serde_json::json!({}))
        .bind(&flow_run.external_user)
        .bind(&flow_run.authorized_account)
        .bind(flow_run.api_key_id)
        .bind(flow_run.publication_version_id)
        .bind(&flow_run.external_conversation_id)
        .bind(&flow_run.external_trace_id)
        .bind(&flow_run.compatibility_mode)
        .bind(&flow_run.idempotency_key)
        .bind(flow_run.started_at)
        .bind(flow_run.finished_at)
        .bind(flow_run.created_at)
        .bind(flow_run.updated_at)
        .bind(flow_run.created_by)
        .execute(&mut **tx)
        .await?;

        Ok(())
    }

    async fn upsert_application_conversation_messages_for_flow_run(
        &self,
        flow_run: &domain::FlowRunRecord,
    ) -> Result<()> {
        let messages = application_conversation_messages_from_flow_run(flow_run);
        if messages.is_empty() {
            return Ok(());
        }
        let conversation_key = application_conversation_key(flow_run);

        let Some(conversation) = sqlx::query(
            r#"
            with existing as (
                select id, scope_id, 0 as source_order
                from application_conversations
                where application_id = $1
                  and external_conversation_id = $2
                  and api_key_id is not distinct from (select id from api_keys where id = $3)
                  and external_user is not distinct from $4
                order by updated_at desc, id desc
                limit 1
            ),
            inserted as (
                insert into application_conversations (
                    id,
                    scope_id,
                    application_id,
                    api_key_id,
                    external_user,
                    external_conversation_id,
                    created_at,
                    updated_at
                )
                select
                    $5,
                    applications.workspace_id,
                    applications.id,
                    (select id from api_keys where id = $3),
                    $4,
                    $2,
                    $6,
                    $7
                from applications
                where applications.id = $1
                  and not exists (select 1 from existing)
                returning id, scope_id, 1 as source_order
            )
            select id, scope_id, source_order from existing
            union all
            select id, scope_id, source_order from inserted
            order by source_order asc
            limit 1
            "#,
        )
        .bind(flow_run.application_id)
        .bind(&conversation_key)
        .bind(flow_run.api_key_id)
        .bind(&flow_run.external_user)
        .bind(Uuid::now_v7())
        .bind(flow_run.started_at)
        .bind(flow_run.updated_at)
        .fetch_optional(self.pool())
        .await?
        else {
            return Ok(());
        };

        let conversation_id: Uuid = conversation.get("id");
        let scope_id: Uuid = conversation.get("scope_id");
        let sequences = messages
            .iter()
            .map(|message| message.sequence)
            .collect::<Vec<_>>();
        let mut tx = self.pool().begin().await?;

        sqlx::query(
            r#"
            delete from application_conversation_messages
            where conversation_id = $1
              and flow_run_id = $2
              and not (sequence = any($3))
            "#,
        )
        .bind(conversation_id)
        .bind(flow_run.id)
        .bind(&sequences)
        .execute(&mut *tx)
        .await?;

        if let Some(title) = messages
            .iter()
            .find(|message| message.role == "user")
            .map(|message| message.content.as_str())
        {
            sqlx::query(
                r#"
                update application_conversations
                set title = coalesce(nullif(title, ''), $2),
                    updated_at = greatest(updated_at, $3)
                where id = $1
                "#,
            )
            .bind(conversation_id)
            .bind(title)
            .bind(flow_run.updated_at)
            .execute(&mut *tx)
            .await?;
        }

        for message in messages {
            sqlx::query(
                r#"
                insert into application_conversation_messages (
                    id,
                    scope_id,
                    conversation_id,
                    application_id,
                    flow_run_id,
                    node_run_id,
                    role,
                    content,
                    sequence,
                    status,
                    started_at,
                    finished_at,
                    created_at,
                    updated_at
                ) values (
                    $1, $2, $3, $4, $5, null, $6, $7, $8, $9, $10, $11, $12, $13
                )
                on conflict (conversation_id, flow_run_id, sequence) do update
                set role = excluded.role,
                    content = excluded.content,
                    status = excluded.status,
                    started_at = excluded.started_at,
                    finished_at = excluded.finished_at,
                    updated_at = excluded.updated_at
                "#,
            )
            .bind(Uuid::now_v7())
            .bind(scope_id)
            .bind(conversation_id)
            .bind(flow_run.application_id)
            .bind(flow_run.id)
            .bind(message.role)
            .bind(&message.content)
            .bind(message.sequence)
            .bind(flow_run.status.as_str())
            .bind(flow_run.started_at)
            .bind(flow_run.finished_at)
            .bind(flow_run.started_at)
            .bind(flow_run.updated_at)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;

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
            select count(*)::bigint
            from application_run_log_summaries
            where application_id = $1
              and ($2::timestamptz is null or created_at >= $2)
            "#,
        )
        .bind(application_id)
        .bind(created_after)
        .fetch_one(self.pool())
        .await?;

        let rows = sqlx::query(&format!(
            r#"
            with selected_logs as (
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
                    input_tokens, output_tokens, input_cache_hit_tokens,
                    unique_node_count,
                    tool_callback_count,
                    started_at,
                    finished_at,
                    created_at,
                    updated_at
                from application_run_log_summaries
                where application_id = $1
                  and ($2::timestamptz is null or created_at >= $2)
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
                input_tokens, output_tokens, input_cache_hit_tokens,
                unique_node_count,
                tool_callback_count,
                started_at,
                finished_at,
                created_at,
                updated_at
            from selected_logs
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

    async fn get_application_run_monitoring_report(
        &self,
        application_id: Uuid,
        input: GetApplicationRunMonitoringReportInput,
    ) -> Result<control_plane::ports::ApplicationRunMonitoringReport> {
        let bucket = normalize_application_run_monitoring_bucket(&input.bucket);
        let started_from = input.started_from;
        let started_to = input.started_to;
        let slow_run_threshold_ms = input.slow_run_threshold_ms.max(0);

        let overview = self
            .application_run_monitoring_overview(application_id, started_from, started_to)
            .await?;
        let duration = self
            .application_run_monitoring_duration(
                application_id,
                started_from,
                started_to,
                slow_run_threshold_ms,
            )
            .await?;
        let tokens = self
            .application_run_monitoring_tokens(application_id, started_from, started_to)
            .await?;
        let tokens_comparison = self
            .application_run_monitoring_tokens_comparison(
                application_id,
                started_from,
                started_to,
                overview.total_count,
                tokens.total_tokens_sum,
                tokens.avg_tokens_per_run,
            )
            .await?;
        let tool_callbacks = self
            .application_run_monitoring_tool_callbacks(application_id, started_from, started_to)
            .await?;
        let nodes = self
            .application_run_monitoring_nodes(application_id, started_from, started_to)
            .await?;
        let concurrency = self
            .application_run_monitoring_concurrency(application_id, started_from, started_to)
            .await?;
        let tokens_trend = self
            .application_run_monitoring_tokens_trend(
                application_id,
                started_from,
                started_to,
                bucket,
            )
            .await?;
        let protocols = self
            .application_run_monitoring_protocols(application_id, started_from, started_to)
            .await?;
        let sources = self
            .application_run_monitoring_sources(application_id, started_from, started_to)
            .await?;
        let authorized_accounts = self
            .application_run_monitoring_authorized_accounts(
                application_id,
                started_from,
                started_to,
            )
            .await?;
        let external_users = self
            .application_run_monitoring_external_users(application_id, started_from, started_to)
            .await?;
        let api_keys = self
            .application_run_monitoring_api_keys(application_id, started_from, started_to)
            .await?;
        let external_conversations = self
            .application_run_monitoring_external_conversations(
                application_id,
                started_from,
                started_to,
            )
            .await?;
        let slowest_runs = self
            .application_run_monitoring_ranked_runs(
                application_id,
                started_from,
                started_to,
                ApplicationRunMonitoringRankKind::Slowest,
            )
            .await?;
        let high_token_runs = self
            .application_run_monitoring_ranked_runs(
                application_id,
                started_from,
                started_to,
                ApplicationRunMonitoringRankKind::HighToken,
            )
            .await?;

        Ok(control_plane::ports::ApplicationRunMonitoringReport {
            overview,
            duration,
            tokens,
            tokens_comparison,
            tool_callbacks,
            nodes,
            concurrency,
            tokens_trend,
            protocols,
            sources,
            authorized_accounts,
            external_users,
            api_keys,
            external_conversations,
            slowest_runs,
            high_token_runs,
        })
    }

    async fn application_run_monitoring_overview(
        &self,
        application_id: Uuid,
        started_from: Option<OffsetDateTime>,
        started_to: Option<OffsetDateTime>,
    ) -> Result<control_plane::ports::ApplicationRunMonitoringOverview> {
        let row = sqlx::query(&application_run_monitoring_logs_query(
            r#"
            select
                count(*)::bigint as total_count,
                count(*) filter (where status = 'succeeded')::bigint as success_count,
                count(*) filter (where status = 'failed')::bigint as failed_count,
                count(*) filter (where status = 'cancelled')::bigint as cancelled_count,
                coalesce(
                    count(*) filter (where status = 'succeeded')::double precision
                    / nullif(count(*)::double precision, 0),
                    0.0
                ) as success_rate,
                coalesce(
                    count(*) filter (where status = 'failed')::double precision
                    / nullif(count(*)::double precision, 0),
                    0.0
                ) as failed_rate
            from monitoring_logs
            "#,
        ))
        .bind(application_id)
        .bind(started_from)
        .bind(started_to)
        .fetch_one(self.pool())
        .await?;

        Ok(control_plane::ports::ApplicationRunMonitoringOverview {
            total_count: row.get("total_count"),
            success_count: row.get("success_count"),
            failed_count: row.get("failed_count"),
            cancelled_count: row.get("cancelled_count"),
            success_rate: row.get("success_rate"),
            failed_rate: row.get("failed_rate"),
            running_count_included: false,
        })
    }

    async fn application_run_monitoring_duration(
        &self,
        application_id: Uuid,
        started_from: Option<OffsetDateTime>,
        started_to: Option<OffsetDateTime>,
        slow_run_threshold_ms: i64,
    ) -> Result<control_plane::ports::ApplicationRunMonitoringDuration> {
        let row = sqlx::query(&application_run_monitoring_logs_query(
            r#"
            , logs as (
                select
                    extract(epoch from (finished_at - started_at)) * 1000.0 as duration_ms
                from monitoring_logs
            ), recorded as (
                select duration_ms
                from logs
                where duration_ms is not null
            )
            select
                (select count(*) from recorded)::bigint as duration_recorded_count,
                coalesce((select avg(duration_ms) from recorded), 0.0)::double precision
                    as avg_duration_ms,
                coalesce(
                    (select percentile_cont(0.5) within group (order by duration_ms) from recorded),
                    0.0
                )::double precision as p50_duration_ms,
                coalesce(
                    (select percentile_cont(0.95) within group (order by duration_ms) from recorded),
                    0.0
                )::double precision as p95_duration_ms,
                coalesce(
                    (select count(*) filter (where duration_ms > $4)::double precision from recorded)
                    / nullif((select count(*)::double precision from logs), 0),
                    0.0
                )::double precision as slow_run_rate
            "#,
        ))
        .bind(application_id)
        .bind(started_from)
        .bind(started_to)
        .bind(slow_run_threshold_ms as f64)
        .fetch_one(self.pool())
        .await?;

        Ok(control_plane::ports::ApplicationRunMonitoringDuration {
            duration_recorded_count: row.get("duration_recorded_count"),
            avg_duration_ms: row.get("avg_duration_ms"),
            p50_duration_ms: row.get("p50_duration_ms"),
            p95_duration_ms: row.get("p95_duration_ms"),
            slow_run_rate: row.get("slow_run_rate"),
        })
    }

    async fn application_run_monitoring_tokens(
        &self,
        application_id: Uuid,
        started_from: Option<OffsetDateTime>,
        started_to: Option<OffsetDateTime>,
    ) -> Result<control_plane::ports::ApplicationRunMonitoringTokens> {
        let row = sqlx::query(&application_run_monitoring_logs_query(
            r#"
            select
                coalesce(sum(coalesce(total_tokens, 0)), 0)::bigint as total_tokens_sum,
                coalesce(sum(coalesce(input_tokens, 0)), 0)::bigint as input_tokens_sum,
                coalesce(sum(coalesce(output_tokens, 0)), 0)::bigint as output_tokens_sum,
                coalesce(sum(coalesce(input_cache_hit_tokens, 0)), 0)::bigint
                    as input_cache_hit_tokens_sum,
                coalesce(avg(total_tokens::double precision), 0.0)::double precision
                    as avg_tokens_per_run,
                count(total_tokens)::bigint as token_recorded_count
            from monitoring_logs
            "#,
        ))
        .bind(application_id)
        .bind(started_from)
        .bind(started_to)
        .fetch_one(self.pool())
        .await?;

        Ok(control_plane::ports::ApplicationRunMonitoringTokens {
            total_tokens_sum: row.get("total_tokens_sum"),
            input_tokens_sum: row.get("input_tokens_sum"),
            output_tokens_sum: row.get("output_tokens_sum"),
            input_cache_hit_tokens_sum: row.get("input_cache_hit_tokens_sum"),
            avg_tokens_per_run: row.get("avg_tokens_per_run"),
            token_recorded_count: row.get("token_recorded_count"),
        })
    }

    async fn application_run_monitoring_tokens_comparison(
        &self,
        application_id: Uuid,
        started_from: Option<OffsetDateTime>,
        started_to: Option<OffsetDateTime>,
        current_run_count: i64,
        current_total_tokens: i64,
        current_avg_tokens_per_run: f64,
    ) -> Result<control_plane::ports::ApplicationRunMonitoringTokensComparison> {
        let Some((previous_from, previous_to)) = previous_monitoring_window(started_from, started_to)
        else {
            return Ok(empty_tokens_comparison());
        };

        let row = sqlx::query(&application_run_monitoring_logs_query(
            r#"
            select
                coalesce(sum(coalesce(total_tokens, 0)), 0)::bigint
                    as previous_total_tokens_sum,
                count(*)::bigint as previous_run_count,
                coalesce(avg(total_tokens::double precision), 0.0)::double precision
                    as previous_avg_tokens_per_run
            from monitoring_logs
            "#,
        ))
        .bind(application_id)
        .bind(Some(previous_from))
        .bind(Some(previous_to))
        .fetch_one(self.pool())
        .await?;

        let previous_total_tokens_sum = row.get("previous_total_tokens_sum");
        let previous_run_count = row.get("previous_run_count");
        let previous_avg_tokens_per_run = row.get("previous_avg_tokens_per_run");

        Ok(control_plane::ports::ApplicationRunMonitoringTokensComparison {
            previous_total_tokens_sum,
            previous_run_count,
            previous_avg_tokens_per_run,
            token_change_rate: change_rate_i64(current_total_tokens, previous_total_tokens_sum),
            run_count_change_rate: change_rate_i64(current_run_count, previous_run_count),
            avg_tokens_per_run_change_rate: change_rate_f64(
                current_avg_tokens_per_run,
                previous_avg_tokens_per_run,
            ),
            traffic_effect: ratio_i64(current_run_count, previous_run_count),
            cost_per_run_effect: ratio_f64(current_avg_tokens_per_run, previous_avg_tokens_per_run),
        })
    }

    async fn application_run_monitoring_tool_callbacks(
        &self,
        application_id: Uuid,
        started_from: Option<OffsetDateTime>,
        started_to: Option<OffsetDateTime>,
    ) -> Result<control_plane::ports::ApplicationRunMonitoringToolCallbacks> {
        let row = sqlx::query(&application_run_monitoring_logs_query(
            r#"
            select
                coalesce(sum(tool_callback_count), 0)::bigint as total_tool_callback_count,
                coalesce(avg(tool_callback_count::double precision), 0.0)::double precision
                    as avg_tool_callback_count,
                count(*) filter (where tool_callback_count > 0)::bigint
                    as runs_with_tool_callback
            from monitoring_logs
            "#,
        ))
        .bind(application_id)
        .bind(started_from)
        .bind(started_to)
        .fetch_one(self.pool())
        .await?;

        Ok(control_plane::ports::ApplicationRunMonitoringToolCallbacks {
            total_tool_callback_count: row.get("total_tool_callback_count"),
            avg_tool_callback_count: row.get("avg_tool_callback_count"),
            runs_with_tool_callback: row.get("runs_with_tool_callback"),
        })
    }

    async fn application_run_monitoring_nodes(
        &self,
        application_id: Uuid,
        started_from: Option<OffsetDateTime>,
        started_to: Option<OffsetDateTime>,
    ) -> Result<control_plane::ports::ApplicationRunMonitoringNodes> {
        let row = sqlx::query(&application_run_monitoring_logs_query(
            r#"
            select
                coalesce(avg(unique_node_count::double precision), 0.0)::double precision
                    as avg_unique_node_count,
                coalesce(max(unique_node_count), 0)::bigint as max_unique_node_count
            from monitoring_logs
            "#,
        ))
        .bind(application_id)
        .bind(started_from)
        .bind(started_to)
        .fetch_one(self.pool())
        .await?;

        Ok(control_plane::ports::ApplicationRunMonitoringNodes {
            avg_unique_node_count: row.get("avg_unique_node_count"),
            max_unique_node_count: row.get("max_unique_node_count"),
        })
    }

    async fn application_run_monitoring_concurrency(
        &self,
        application_id: Uuid,
        started_from: Option<OffsetDateTime>,
        started_to: Option<OffsetDateTime>,
    ) -> Result<control_plane::ports::ApplicationRunMonitoringConcurrency> {
        let peak_concurrency = sqlx::query_scalar::<_, i64>(
            &application_run_monitoring_logs_query(
            r#"
            , logs as (
                select started_at, finished_at
                from monitoring_logs
            ), events as (
                select started_at as ts, 1 as delta from logs
                union all
                select finished_at as ts, -1 as delta from logs where finished_at is not null
            ), scan as (
                select
                    ts,
                    sum(delta) over (order by ts, delta desc rows unbounded preceding)
                        as concurrency
                from events
            )
            select coalesce(max(concurrency), 0)::bigint as peak_concurrency from scan
            "#,
            )
        )
        .bind(application_id)
        .bind(started_from)
        .bind(started_to)
        .fetch_one(self.pool())
        .await?;

        Ok(control_plane::ports::ApplicationRunMonitoringConcurrency { peak_concurrency })
    }

    async fn application_run_monitoring_tokens_trend(
        &self,
        application_id: Uuid,
        started_from: Option<OffsetDateTime>,
        started_to: Option<OffsetDateTime>,
        bucket: &str,
    ) -> Result<Vec<control_plane::ports::ApplicationRunMonitoringTokenTrendPoint>> {
        let rows = sqlx::query(&application_run_monitoring_logs_query(
            r#"
            select
                date_trunc($4, started_at) as bucket_start,
                count(*)::bigint as run_count, coalesce(sum(coalesce(total_tokens, 0)), 0)::bigint as total_tokens,
                coalesce(sum(coalesce(input_tokens, 0)), 0)::bigint as input_tokens,
                coalesce(sum(coalesce(output_tokens, 0)), 0)::bigint as output_tokens,
                coalesce(sum(coalesce(input_cache_hit_tokens, 0)), 0)::bigint as input_cache_hit_tokens
            from monitoring_logs
            group by bucket_start
            order by bucket_start asc
            "#,
        ))
        .bind(application_id)
        .bind(started_from)
        .bind(started_to)
        .bind(bucket)
        .fetch_all(self.pool())
        .await?;
        Ok(rows.into_iter().map(|row| control_plane::ports::ApplicationRunMonitoringTokenTrendPoint {
                bucket_start: row.get("bucket_start"),
                run_count: row.get("run_count"),
                total_tokens: row.get("total_tokens"),
                input_tokens: row.get("input_tokens"),
                output_tokens: row.get("output_tokens"),
                input_cache_hit_tokens: row.get("input_cache_hit_tokens"),
            }).collect())
    }

    async fn application_run_monitoring_protocols(
        &self,
        application_id: Uuid,
        started_from: Option<OffsetDateTime>,
        started_to: Option<OffsetDateTime>,
    ) -> Result<Vec<control_plane::ports::ApplicationRunMonitoringProtocolBreakdown>> {
        let rows = sqlx::query(&application_run_monitoring_logs_query(
            r#"
            , logs as (
                select
                    coalesce(compatibility_mode, 'default') as protocol,
                    status,
                    total_tokens,
                    extract(epoch from (finished_at - started_at)) * 1000.0 as duration_ms
                from monitoring_logs
            )
            select
                protocol,
                count(*)::bigint as request_count,
                coalesce(
                    count(*) filter (where status = 'succeeded')::double precision
                    / nullif(count(*)::double precision, 0),
                    0.0
                ) as success_rate,
                coalesce(avg(duration_ms) filter (where duration_ms is not null), 0.0)
                    ::double precision as avg_duration_ms,
                coalesce(sum(coalesce(total_tokens, 0)), 0)::bigint as total_tokens
            from logs
            group by protocol
            order by request_count desc, protocol asc
            "#,
        ))
        .bind(application_id)
        .bind(started_from)
        .bind(started_to)
        .fetch_all(self.pool())
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| control_plane::ports::ApplicationRunMonitoringProtocolBreakdown {
                protocol: row.get("protocol"),
                request_count: row.get("request_count"),
                success_rate: row.get("success_rate"),
                avg_duration_ms: row.get("avg_duration_ms"),
                total_tokens: row.get("total_tokens"),
            })
            .collect())
    }

    async fn application_run_monitoring_sources(
        &self,
        application_id: Uuid,
        started_from: Option<OffsetDateTime>,
        started_to: Option<OffsetDateTime>,
    ) -> Result<Vec<control_plane::ports::ApplicationRunMonitoringSourceBreakdown>> {
        let rows = sqlx::query(&application_run_monitoring_logs_query(
            r#"
            , logs as (
                select
                    case when api_key_id is null then 'console' else 'public_api' end as source,
                    status,
                    total_tokens
                from monitoring_logs
            )
            select
                source,
                count(*)::bigint as request_count,
                coalesce(
                    count(*) filter (where status = 'succeeded')::double precision
                    / nullif(count(*)::double precision, 0),
                    0.0
                ) as success_rate,
                coalesce(sum(coalesce(total_tokens, 0)), 0)::bigint as total_tokens
            from logs
            group by source
            order by request_count desc, source asc
            "#,
        ))
        .bind(application_id)
        .bind(started_from)
        .bind(started_to)
        .fetch_all(self.pool())
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| control_plane::ports::ApplicationRunMonitoringSourceBreakdown {
                source: row.get("source"),
                request_count: row.get("request_count"),
                success_rate: row.get("success_rate"),
                total_tokens: row.get("total_tokens"),
            })
            .collect())
    }

    async fn application_run_monitoring_authorized_accounts(
        &self,
        application_id: Uuid,
        started_from: Option<OffsetDateTime>,
        started_to: Option<OffsetDateTime>,
    ) -> Result<Vec<control_plane::ports::ApplicationRunMonitoringAuthorizedAccountUsage>> {
        let rows = self
            .application_run_monitoring_nullable_text_usage(
                application_id,
                started_from,
                started_to,
                "authorized_account",
            )
            .await?;

        Ok(rows
            .into_iter()
            .map(|row| control_plane::ports::ApplicationRunMonitoringAuthorizedAccountUsage {
                authorized_account: row.dimension_value,
                request_count: row.request_count,
                total_tokens: row.total_tokens,
                avg_duration_ms: row.avg_duration_ms,
                failed_count: row.failed_count,
            })
            .collect())
    }

    async fn application_run_monitoring_external_users(
        &self,
        application_id: Uuid,
        started_from: Option<OffsetDateTime>,
        started_to: Option<OffsetDateTime>,
    ) -> Result<Vec<control_plane::ports::ApplicationRunMonitoringExternalUserUsage>> {
        let rows = self
            .application_run_monitoring_nullable_text_usage(
                application_id,
                started_from,
                started_to,
                "external_user",
            )
            .await?;

        Ok(rows
            .into_iter()
            .map(|row| control_plane::ports::ApplicationRunMonitoringExternalUserUsage {
                external_user: row.dimension_value,
                request_count: row.request_count,
                total_tokens: row.total_tokens,
                avg_duration_ms: row.avg_duration_ms,
                failed_count: row.failed_count,
            })
            .collect())
    }

    async fn application_run_monitoring_external_conversations(
        &self,
        application_id: Uuid,
        started_from: Option<OffsetDateTime>,
        started_to: Option<OffsetDateTime>,
    ) -> Result<Vec<control_plane::ports::ApplicationRunMonitoringExternalConversationUsage>> {
        let rows = self
            .application_run_monitoring_nullable_text_usage(
                application_id,
                started_from,
                started_to,
                "external_conversation_id",
            )
            .await?;

        Ok(rows
            .into_iter()
            .map(|row| {
                control_plane::ports::ApplicationRunMonitoringExternalConversationUsage {
                    external_conversation_id: row.dimension_value,
                    request_count: row.request_count,
                    total_tokens: row.total_tokens,
                    avg_duration_ms: row.avg_duration_ms,
                    failed_count: row.failed_count,
                }
            })
            .collect())
    }

    async fn application_run_monitoring_nullable_text_usage(
        &self,
        application_id: Uuid,
        started_from: Option<OffsetDateTime>,
        started_to: Option<OffsetDateTime>,
        field: &'static str,
    ) -> Result<Vec<ApplicationRunMonitoringTextUsageRow>> {
        let field = match field {
            "authorized_account" => "authorized_account",
            "external_user" => "external_user",
            "external_conversation_id" => "external_conversation_id",
            _ => return Err(anyhow!("unsupported monitoring usage field: {field}")),
        };
        let rows = sqlx::query(&application_run_monitoring_logs_query(&format!(
            r#"
            select
                {field} as dimension_value,
                count(*)::bigint as request_count,
                coalesce(sum(coalesce(total_tokens, 0)), 0)::bigint as total_tokens,
                coalesce(
                    avg(extract(epoch from (finished_at - started_at)) * 1000.0)
                        filter (where finished_at is not null),
                    0.0
                )::double precision as avg_duration_ms,
                count(*) filter (where status = 'failed')::bigint as failed_count
            from monitoring_logs
            where {field} is not null
            group by {field}
            order by request_count desc, {field} asc
            limit 10
            "#
        )))
        .bind(application_id)
        .bind(started_from)
        .bind(started_to)
        .fetch_all(self.pool())
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| ApplicationRunMonitoringTextUsageRow {
                dimension_value: row.get("dimension_value"),
                request_count: row.get("request_count"),
                total_tokens: row.get("total_tokens"),
                avg_duration_ms: row.get("avg_duration_ms"),
                failed_count: row.get("failed_count"),
            })
            .collect())
    }

    async fn application_run_monitoring_api_keys(
        &self,
        application_id: Uuid,
        started_from: Option<OffsetDateTime>,
        started_to: Option<OffsetDateTime>,
    ) -> Result<Vec<control_plane::ports::ApplicationRunMonitoringApiKeyUsage>> {
        let rows = sqlx::query(&application_run_monitoring_logs_query(
            r#"
            select
                api_key_id,
                max(api_key_name_snapshot) filter (
                    where api_key_name_snapshot is not null
                ) as api_key_name_snapshot,
                count(*)::bigint as request_count,
                coalesce(sum(coalesce(total_tokens, 0)), 0)::bigint as total_tokens,
                coalesce(
                    avg(extract(epoch from (finished_at - started_at)) * 1000.0)
                        filter (where finished_at is not null),
                    0.0
                )::double precision as avg_duration_ms,
                count(*) filter (where status = 'failed')::bigint as failed_count
            from monitoring_logs
            where api_key_id is not null
            group by api_key_id
            order by request_count desc, api_key_id asc
            limit 10
            "#,
        ))
        .bind(application_id)
        .bind(started_from)
        .bind(started_to)
        .fetch_all(self.pool())
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| control_plane::ports::ApplicationRunMonitoringApiKeyUsage {
                api_key_id: row.get("api_key_id"),
                api_key_name_snapshot: row.get("api_key_name_snapshot"),
                request_count: row.get("request_count"),
                total_tokens: row.get("total_tokens"),
                avg_duration_ms: row.get("avg_duration_ms"),
                failed_count: row.get("failed_count"),
            })
            .collect())
    }

    async fn application_run_monitoring_ranked_runs(
        &self,
        application_id: Uuid,
        started_from: Option<OffsetDateTime>,
        started_to: Option<OffsetDateTime>,
        rank_kind: ApplicationRunMonitoringRankKind,
    ) -> Result<Vec<control_plane::ports::ApplicationRunMonitoringRunRank>> {
        let (extra_filter, order_by) = match rank_kind {
            ApplicationRunMonitoringRankKind::Slowest => {
                ("and finished_at is not null", "duration_ms desc nulls last")
            }
            ApplicationRunMonitoringRankKind::HighToken => {
                ("and total_tokens is not null", "total_tokens desc nulls last")
            }
        };
        let rows = sqlx::query(&application_run_monitoring_logs_query(&format!(
            r#"
            select
                flow_run_id,
                title,
                status,
                started_at,
                finished_at,
                case
                    when finished_at is null then null
                    else (extract(epoch from (finished_at - started_at)) * 1000.0)::double precision
                end as duration_ms,
                total_tokens
            from monitoring_logs
            where true
              {extra_filter}
            order by {order_by}, started_at desc, flow_run_id desc
            limit 10
            "#
        )))
        .bind(application_id)
        .bind(started_from)
        .bind(started_to)
        .fetch_all(self.pool())
        .await?;

        rows.into_iter()
            .map(|row| {
                let status: String = row.get("status");

                Ok(control_plane::ports::ApplicationRunMonitoringRunRank {
                    flow_run_id: row.get("flow_run_id"),
                    title: row.get("title"),
                    status:
                        crate::mappers::orchestration_runtime_mapper::parse_flow_run_status(
                            &status,
                        )?,
                    started_at: row.get("started_at"),
                    finished_at: row.get("finished_at"),
                    duration_ms: row.get("duration_ms"),
                    total_tokens: row.get("total_tokens"),
                })
            })
            .collect()
    }
}

#[derive(Debug, Clone, Copy)]
enum ApplicationRunMonitoringRankKind {
    Slowest,
    HighToken,
}

#[derive(Debug)]
struct ApplicationRunMonitoringTextUsageRow {
    dimension_value: Option<String>,
    request_count: i64,
    total_tokens: i64,
    avg_duration_ms: f64,
    failed_count: i64,
}

#[derive(Debug)]
struct ApplicationConversationMessageProjection {
    role: &'static str,
    content: String,
    sequence: i64,
}

const APPLICATION_CONVERSATION_INPUT_KEYS: &[&str] =
    &["query", "question", "prompt", "message", "input", "input_text"];

fn application_conversation_key(flow_run: &domain::FlowRunRecord) -> String {
    flow_run
        .external_conversation_id
        .clone()
        .unwrap_or_else(|| format!("flow-run:{}", flow_run.id))
}

fn application_conversation_messages_from_flow_run(
    flow_run: &domain::FlowRunRecord,
) -> Vec<ApplicationConversationMessageProjection> {
    let mut messages = Vec::new();
    let mut ordinal = 1_i64;

    if let Some(system) = application_conversation_system_text(&flow_run.input_payload) {
        push_application_conversation_message(
            &mut messages,
            flow_run.started_at,
            &mut ordinal,
            "system",
            system,
        );
    }

    if let Some(query) = application_conversation_user_text(&flow_run.input_payload) {
        push_application_conversation_message(
            &mut messages,
            flow_run.started_at,
            &mut ordinal,
            "user",
            query,
        );
    }

    let answer = application_conversation_answer_text(&flow_run.output_payload).or_else(|| {
        flow_run
            .error_payload
            .as_ref()
            .and_then(application_conversation_answer_text)
    });
    if let Some(answer) = answer {
        push_application_conversation_message(
            &mut messages,
            flow_run.started_at,
            &mut ordinal,
            "assistant",
            answer,
        );
    }

    messages
}

fn push_application_conversation_message(
    messages: &mut Vec<ApplicationConversationMessageProjection>,
    started_at: OffsetDateTime,
    ordinal: &mut i64,
    role: &'static str,
    content: String,
) {
    messages.push(ApplicationConversationMessageProjection {
        role,
        content,
        sequence: application_conversation_message_sequence(started_at, *ordinal),
    });
    *ordinal += 1;
}

fn application_conversation_message_sequence(started_at: OffsetDateTime, ordinal: i64) -> i64 {
    started_at
        .unix_timestamp()
        .saturating_mul(1_000_000)
        .saturating_add(ordinal)
}

fn application_conversation_system_text(payload: &serde_json::Value) -> Option<String> {
    let start = application_conversation_start_payload(payload);
    string_field_value(start, "system").or_else(|| string_field_value(payload, "system"))
}

fn application_conversation_user_text(payload: &serde_json::Value) -> Option<String> {
    for source in [payload, application_conversation_start_payload(payload)] {
        for key in APPLICATION_CONVERSATION_INPUT_KEYS {
            if let Some(value) = string_field_value(source, key) {
                return Some(value);
            }
        }
    }

    None
}

fn application_conversation_answer_text(payload: &serde_json::Value) -> Option<String> {
    for key in ["answer", "text", "output", "content", "message"] {
        if let Some(value) = string_field_value(payload, key) {
            return Some(value);
        }
    }

    payload
        .get("error")
        .and_then(|value| value.get("message"))
        .and_then(trimmed_string)
}

fn application_conversation_start_payload(payload: &serde_json::Value) -> &serde_json::Value {
    payload
        .get("node-start")
        .or_else(|| payload.get("start"))
        .unwrap_or(payload)
}

fn string_field_value(value: &serde_json::Value, field: &str) -> Option<String> {
    value.get(field).and_then(conversation_text_value)
}

fn conversation_text_value(value: &serde_json::Value) -> Option<String> {
    trimmed_string(value).or_else(|| {
        value.as_object()?;

        artifact_preview_text(value).or_else(|| {
            value
                .get("text")
                .or_else(|| value.get("content"))
                .and_then(trimmed_string)
        })
    })
}

fn artifact_preview_text(value: &serde_json::Value) -> Option<String> {
    let preview = value.get("preview").and_then(trimmed_string)?;
    decode_artifact_preview_text(&preview).or(Some(preview))
}

fn decode_artifact_preview_text(preview: &str) -> Option<String> {
    if let Ok(decoded) = serde_json::from_str::<String>(preview) {
        return trimmed_text(&decoded);
    }

    let stripped = preview.strip_prefix('"')?;
    let completed = if stripped.ends_with('"') {
        preview.to_owned()
    } else {
        format!("{preview}\"")
    };
    if let Ok(decoded) = serde_json::from_str::<String>(&completed) {
        return trimmed_text(&decoded);
    }

    trimmed_text(stripped.trim_end_matches('"'))
        .map(|value| {
            value
                .replace("\\n", "\n")
                .replace("\\r", "\r")
                .replace("\\t", "\t")
                .replace("\\\"", "\"")
        })
        .and_then(|value| trimmed_text(&value))
}

fn trimmed_string(value: &serde_json::Value) -> Option<String> {
    value
        .as_str()
        .and_then(trimmed_text)
}

fn trimmed_text(value: &str) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_owned())
}

fn normalize_application_run_monitoring_bucket(input: &str) -> &'static str {
    match input {
        "hour" => "hour",
        "week" => "week",
        "month" => "month",
        _ => "day",
    }
}

fn application_run_monitoring_logs_query(select_sql: &str) -> String {
    format!(
        r#"
        with monitoring_logs as (
            select *
            from application_run_log_summaries
            where application_id = $1
              and ($2::timestamptz is null or started_at >= $2)
              and ($3::timestamptz is null or started_at < $3)
              and status in ('succeeded', 'failed', 'cancelled')
        )
        {select_sql}
        "#
    )
}

fn previous_monitoring_window(
    started_from: Option<OffsetDateTime>,
    started_to: Option<OffsetDateTime>,
) -> Option<(OffsetDateTime, OffsetDateTime)> {
    let previous_to = started_from?;
    let current_to = started_to.unwrap_or_else(OffsetDateTime::now_utc);
    let window = current_to - previous_to;
    (window > Duration::ZERO).then(|| (previous_to - window, previous_to))
}

fn empty_tokens_comparison() -> control_plane::ports::ApplicationRunMonitoringTokensComparison {
    control_plane::ports::ApplicationRunMonitoringTokensComparison {
        previous_total_tokens_sum: 0,
        previous_run_count: 0,
        previous_avg_tokens_per_run: 0.0,
        token_change_rate: 0.0,
        run_count_change_rate: 0.0,
        avg_tokens_per_run_change_rate: 0.0,
        traffic_effect: 0.0,
        cost_per_run_effect: 0.0,
    }
}

fn change_rate_i64(current: i64, previous: i64) -> f64 {
    (current - previous) as f64 / previous.max(1) as f64
}

fn change_rate_f64(current: f64, previous: f64) -> f64 {
    (current - previous) / previous.max(1.0)
}

fn ratio_i64(numerator: i64, denominator: i64) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64
    }
}

fn ratio_f64(numerator: f64, denominator: f64) -> f64 {
    if denominator == 0.0 {
        0.0
    } else {
        numerator / denominator
    }
}

fn is_terminal_application_run_log_status(status: domain::FlowRunStatus) -> bool {
    matches!(
        status,
        domain::FlowRunStatus::Succeeded
            | domain::FlowRunStatus::Failed
            | domain::FlowRunStatus::Cancelled
    )
}
