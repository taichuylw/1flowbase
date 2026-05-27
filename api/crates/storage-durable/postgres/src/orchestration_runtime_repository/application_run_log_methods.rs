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
                api_key_name_snapshot,
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
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10,
                (select name from api_keys where id = $10),
                $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22
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
            select count(*)::bigint
            from flow_runs
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
            with selected_runs as (
                select
                    id,
                    run_mode,
                    status,
                    target_node_id,
                    title,
                    input_payload,
                    external_user,
                    api_key_id,
                    publication_version_id,
                    external_conversation_id,
                    external_trace_id,
                    compatibility_mode,
                    idempotency_key,
                    (
                        select users.account
                        from users
                        where users.id = flow_runs.created_by
                    ) as authorized_account,
                    started_at,
                    finished_at,
                    created_at,
                    updated_at
                from flow_runs
                where application_id = $1
                  and ($2::timestamptz is null or created_at >= $2)
                order by {}
                limit $3 offset $4
            )
            select
                selected_runs.id,
                selected_runs.run_mode,
                selected_runs.status,
                selected_runs.target_node_id,
                selected_runs.title,
                selected_runs.input_payload,
                selected_runs.external_user,
                selected_runs.authorized_account,
                selected_runs.api_key_id,
                selected_runs.publication_version_id,
                selected_runs.external_conversation_id,
                selected_runs.external_trace_id,
                selected_runs.compatibility_mode,
                selected_runs.idempotency_key,
                node_statistics.total_tokens,
                coalesce(node_statistics.unique_node_count, 0)::bigint as unique_node_count,
                coalesce(callback_statistics.tool_callback_count, 0)::bigint as tool_callback_count,
                selected_runs.started_at,
                selected_runs.finished_at,
                selected_runs.created_at,
                selected_runs.updated_at
            from selected_runs
            left join lateral (
                select
                    (
                        sum(
                            case
                                when node_runs.metrics_payload->'usage' ? 'total_tokens'
                                    then (node_runs.metrics_payload->'usage'->>'total_tokens')::bigint
                                when node_runs.metrics_payload ? 'usage'
                                    and (
                                        node_runs.metrics_payload->'usage' ? 'input_tokens'
                                        or node_runs.metrics_payload->'usage' ? 'output_tokens'
                                        or node_runs.metrics_payload->'usage' ? 'reasoning_tokens'
                                    )
                                    then coalesce((node_runs.metrics_payload->'usage'->>'input_tokens')::bigint, 0)
                                        + coalesce((node_runs.metrics_payload->'usage'->>'output_tokens')::bigint, 0)
                                        + coalesce((node_runs.metrics_payload->'usage'->>'reasoning_tokens')::bigint, 0)
                                else null
                            end
                        )
                    )::bigint as total_tokens,
                    count(distinct node_runs.node_id)::bigint as unique_node_count
                from node_runs
                where node_runs.flow_run_id = selected_runs.id
            ) node_statistics on true
            left join lateral (
                select coalesce(
                    sum(
                        case
                            when flow_run_callback_tasks.callback_kind = 'llm_tool_calls'
                                and jsonb_typeof(flow_run_callback_tasks.request_payload->'tool_calls') = 'array'
                                then jsonb_array_length(flow_run_callback_tasks.request_payload->'tool_calls')
                            else 0
                        end
                    ),
                    0
                )::bigint as tool_callback_count
                from flow_run_callback_tasks
                where flow_run_callback_tasks.flow_run_id = selected_runs.id
            ) callback_statistics on true
            order by {}
            "#,
            order_by, order_by
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
            avg_tokens_per_run: row.get("avg_tokens_per_run"),
            token_recorded_count: row.get("token_recorded_count"),
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
                count(*)::bigint as run_count,
                coalesce(sum(coalesce(total_tokens, 0)), 0)::bigint as total_tokens
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

        Ok(rows
            .into_iter()
            .map(|row| control_plane::ports::ApplicationRunMonitoringTokenTrendPoint {
                bucket_start: row.get("bucket_start"),
                run_count: row.get("run_count"),
                total_tokens: row.get("total_tokens"),
            })
            .collect())
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
struct ApplicationRunLogStatistics {
    total_tokens: Option<i64>,
    unique_node_count: i64,
    tool_callback_count: i64,
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
        )
        {select_sql}
        "#
    )
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
