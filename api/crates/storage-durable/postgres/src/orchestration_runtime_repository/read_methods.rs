impl PgControlPlaneStore {
    async fn list_runtime_spans(
        &self,
        flow_run_id: Uuid,
    ) -> Result<Vec<domain::RuntimeSpanRecord>> {
        let rows = sqlx::query(
            r#"
            select
                id,
                flow_run_id,
                node_run_id,
                parent_span_id,
                kind,
                name,
                status,
                capability_id,
                input_ref,
                output_ref,
                error_payload,
                metadata,
                started_at,
                finished_at
            from runtime_spans
            where flow_run_id = $1
            order by started_at asc, id asc
            "#,
        )
        .bind(flow_run_id)
        .fetch_all(self.pool())
        .await?;

        rows.into_iter().map(map_runtime_span_record).collect()
    }

    async fn list_runtime_events(
        &self,
        flow_run_id: Uuid,
        after_sequence: i64,
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
              and sequence > $2
            order by sequence asc, id asc
            "#,
        )
        .bind(flow_run_id)
        .bind(after_sequence)
        .fetch_all(self.pool())
        .await?;

        rows.into_iter().map(map_runtime_event_record).collect()
    }

    async fn list_runtime_event_backfill_page(
        &self,
        flow_run_id: Uuid,
        after_stream_sequence: i64,
        limit: usize,
    ) -> Result<Vec<domain::RuntimeEventRecord>> {
        let limit = i64::try_from(limit.max(1)).unwrap_or(i64::MAX);
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
              and coalesce(
                    case
                      when payload ->> 'sequence_end' ~ '^-?[0-9]+$'
                      then (payload ->> 'sequence_end')::bigint
                    end,
                    case
                      when payload ->> 'stream_sequence' ~ '^-?[0-9]+$'
                      then (payload ->> 'stream_sequence')::bigint
                    end,
                    sequence
                  ) > $2
            order by
                coalesce(
                    case
                      when payload ->> 'sequence_end' ~ '^-?[0-9]+$'
                      then (payload ->> 'sequence_end')::bigint
                    end,
                    case
                      when payload ->> 'stream_sequence' ~ '^-?[0-9]+$'
                      then (payload ->> 'stream_sequence')::bigint
                    end,
                    sequence
                ) asc,
                sequence asc,
                id asc
            limit $3
            "#,
        )
        .bind(flow_run_id)
        .bind(after_stream_sequence)
        .bind(limit)
        .fetch_all(self.pool())
        .await?;

        rows.into_iter().map(map_runtime_event_record).collect()
    }

    async fn list_runtime_items(
        &self,
        flow_run_id: Uuid,
    ) -> Result<Vec<domain::RuntimeItemRecord>> {
        let rows = sqlx::query(
            r#"
            select
                id,
                flow_run_id,
                span_id,
                kind,
                status,
                source_event_id,
                input_ref,
                output_ref,
                usage_ledger_id,
                trust_level,
                created_at,
                updated_at
            from runtime_items
            where flow_run_id = $1
            order by created_at asc, id asc
            "#,
        )
        .bind(flow_run_id)
        .fetch_all(self.pool())
        .await?;

        rows.into_iter().map(map_runtime_item_record).collect()
    }

    async fn list_context_projections(
        &self,
        flow_run_id: Uuid,
    ) -> Result<Vec<domain::ContextProjectionRecord>> {
        let rows = sqlx::query(
            r#"
            select
                id,
                flow_run_id,
                node_run_id,
                llm_turn_span_id,
                projection_kind,
                merge_stage_ref,
                source_transcript_ref,
                source_item_refs,
                compaction_event_id,
                summary_version,
                model_input_ref,
                model_input_hash,
                compacted_summary_ref,
                previous_projection_id,
                token_estimate,
                provider_continuation_metadata,
                created_at
            from runtime_context_projections
            where flow_run_id = $1
            order by created_at asc, id asc
            "#,
        )
        .bind(flow_run_id)
        .fetch_all(self.pool())
        .await?;

        Ok(rows
            .into_iter()
            .map(map_context_projection_record)
            .collect())
    }

    async fn list_usage_ledger(&self, flow_run_id: Uuid) -> Result<Vec<domain::UsageLedgerRecord>> {
        let rows = sqlx::query(
            r#"
            select
                id,
                flow_run_id,
                node_run_id,
                span_id,
                failover_attempt_id,
                provider_instance_id,
                gateway_route_id,
                model_id,
                upstream_model_id,
                upstream_request_id,
                input_tokens,
                cached_input_tokens,
                output_tokens,
                reasoning_output_tokens,
                total_tokens,
                input_cache_hit_tokens,
                input_cache_miss_tokens,
                cache_read_tokens,
                cache_write_tokens,
                price_snapshot,
                cost_snapshot,
                usage_status,
                raw_usage,
                normalized_usage,
                created_at
            from runtime_usage_ledger
            where flow_run_id = $1
            order by created_at asc, id asc
            "#,
        )
        .bind(flow_run_id)
        .fetch_all(self.pool())
        .await?;

        rows.into_iter().map(map_usage_ledger_record).collect()
    }

    async fn list_model_failover_attempt_ledger(
        &self,
        flow_run_id: Uuid,
    ) -> Result<Vec<domain::ModelFailoverAttemptLedgerRecord>> {
        let rows = sqlx::query(
            r#"
            select
                id,
                flow_run_id,
                node_run_id,
                llm_turn_span_id,
                queue_snapshot_id,
                attempt_index,
                provider_instance_id,
                provider_code,
                upstream_model_id,
                protocol,
                request_ref,
                request_hash,
                started_at,
                first_token_at,
                finished_at,
                status,
                failed_after_first_token,
                upstream_request_id,
                error_code,
                error_message_ref,
                usage_ledger_id,
                cost_ledger_id,
                response_ref
            from model_failover_attempt_ledger
            where flow_run_id = $1
            order by attempt_index asc, started_at asc, id asc
            "#,
        )
        .bind(flow_run_id)
        .fetch_all(self.pool())
        .await?;

        Ok(rows
            .into_iter()
            .map(map_model_failover_attempt_ledger_record)
            .collect())
    }

    async fn list_capability_invocations(
        &self,
        flow_run_id: Uuid,
    ) -> Result<Vec<domain::CapabilityInvocationRecord>> {
        let rows = sqlx::query(
            r#"
            select
                id,
                flow_run_id,
                span_id,
                capability_id,
                requested_by_span_id,
                requester_kind,
                arguments_ref,
                authorization_status,
                authorization_reason,
                result_ref,
                normalized_result,
                started_at,
                finished_at,
                error_payload,
                created_at
            from capability_invocations
            where flow_run_id = $1
            order by created_at asc, id asc
            "#,
        )
        .bind(flow_run_id)
        .fetch_all(self.pool())
        .await?;

        Ok(rows
            .into_iter()
            .map(map_capability_invocation_record)
            .collect())
    }

    async fn list_application_runs(
        &self,
        application_id: Uuid,
    ) -> Result<Vec<domain::ApplicationRunSummary>> {
        let rows = sqlx::query(
            r#"
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
            order by created_at desc, id desc
            "#,
        )
        .bind(application_id)
        .fetch_all(self.pool())
        .await?;

        rows.into_iter().map(map_application_run_summary).collect()
    }

    async fn list_application_runs_page(
        &self,
        application_id: Uuid,
        input: control_plane::ports::ListApplicationRunsPageInput,
    ) -> Result<control_plane::ports::ApplicationRunSummaryPage> {
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
            select count(*)
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
            "#,
            order_by
        ))
        .bind(application_id)
        .bind(created_after)
        .bind(page_size)
        .bind(offset)
        .fetch_all(self.pool())
        .await?;

        Ok(control_plane::ports::ApplicationRunSummaryPage {
            items: rows
                .into_iter()
                .map(map_application_run_summary)
                .collect::<Result<Vec<_>>>()?,
            total,
            page,
            page_size,
        })
    }

    async fn list_application_conversation_runs_page(
        &self,
        application_id: Uuid,
        input: control_plane::ports::ListApplicationConversationRunsPageInput,
    ) -> Result<control_plane::ports::ApplicationConversationRunsPage> {
        let limit = input.limit.clamp(1, 50);
        let (start_rn, end_rn, total) = if let Some(anchor_run_id) = input.before_run_id {
            let Some((anchor_rn, total)) = self
                .application_conversation_run_position(
                    application_id,
                    &input.external_conversation_id,
                    anchor_run_id,
                )
                .await?
            else {
                return Ok(empty_application_conversation_runs_page());
            };
            ((anchor_rn - limit).max(1), anchor_rn - 1, total)
        } else if let Some(anchor_run_id) = input.after_run_id {
            let Some((anchor_rn, total)) = self
                .application_conversation_run_position(
                    application_id,
                    &input.external_conversation_id,
                    anchor_run_id,
                )
                .await?
            else {
                return Ok(empty_application_conversation_runs_page());
            };
            (anchor_rn + 1, (anchor_rn + limit).min(total), total)
        } else {
            let Some(anchor_run_id) = input.around_run_id else {
                return Ok(empty_application_conversation_runs_page());
            };
            let Some((anchor_rn, total)) = self
                .application_conversation_run_position(
                    application_id,
                    &input.external_conversation_id,
                    anchor_run_id,
                )
                .await?
            else {
                return Ok(empty_application_conversation_runs_page());
            };
            let latest_start = (total - limit + 1).max(1);
            let centered_start = anchor_rn - (limit / 2);
            let start_rn = centered_start.max(1).min(latest_start);
            (start_rn, (start_rn + limit - 1).min(total), total)
        };

        if start_rn > end_rn || total == 0 {
            return Ok(empty_application_conversation_runs_page());
        }

        let rows = sqlx::query(
            r#"
            with ordered as (
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
                    updated_at,
                    row_number() over (order by created_at asc, id asc) as rn
                from flow_runs
                where application_id = $1
                  and external_conversation_id = $2
                  and not (
                      compatibility_mode = 'anthropic-messages-v1'
                      and (
                          input_payload #>> '{node-start,compatibility,claude_code_control}' is not null
                          or input_payload #>> '{start,compatibility,claude_code_control}' is not null
                          or position('Your task is to create a detailed summary of the conversation so far' in coalesce(
                              input_payload #>> '{node-start,query}',
                              input_payload #>> '{start,query}',
                              input_payload #>> '{query}',
                              ''
                          )) > 0
                          or position('Your task is to create a detailed summary of the RECENT portion of the conversation' in coalesce(
                              input_payload #>> '{node-start,query}',
                              input_payload #>> '{start,query}',
                              input_payload #>> '{query}',
                              ''
                          )) > 0
                          or position('Your task is to create a detailed summary of this conversation. This summary will be placed at the start of a continuing session' in coalesce(
                              input_payload #>> '{node-start,query}',
                              input_payload #>> '{start,query}',
                              input_payload #>> '{query}',
                              ''
                          )) > 0
                          or (
                              position('This session is being continued from a previous conversation that ran out of context.' in coalesce(
                                  input_payload #>> '{node-start,query}',
                                  input_payload #>> '{start,query}',
                                  input_payload #>> '{query}',
                                  ''
                              )) > 0
                              and position('If you need specific details from before compaction' in coalesce(
                                  input_payload #>> '{node-start,query}',
                                  input_payload #>> '{start,query}',
                                  input_payload #>> '{query}',
                                  ''
                              )) > 0
                          )
                      )
                  )
            )
            select *
            from ordered
            where rn between $3 and $4
            order by rn asc
            "#,
        )
        .bind(application_id)
        .bind(&input.external_conversation_id)
        .bind(start_rn)
        .bind(end_rn)
        .fetch_all(self.pool())
        .await?;

        let items = rows
            .into_iter()
            .map(map_flow_run_record)
            .collect::<Result<Vec<_>>>()?;
        let before_cursor = items.first().map(|run| run.id);
        let after_cursor = items.last().map(|run| run.id);

        Ok(control_plane::ports::ApplicationConversationRunsPage {
            items,
            has_before: start_rn > 1,
            has_after: end_rn < total,
            before_cursor,
            after_cursor,
        })
    }

    async fn application_conversation_run_position(
        &self,
        application_id: Uuid,
        external_conversation_id: &str,
        flow_run_id: Uuid,
    ) -> Result<Option<(i64, i64)>> {
        let row = sqlx::query(
            r#"
            with ordered as (
                select
                    id,
                    row_number() over (order by created_at asc, id asc) as rn,
                    count(*) over () as total
                from flow_runs
                where application_id = $1
                  and external_conversation_id = $2
                  and not (
                      compatibility_mode = 'anthropic-messages-v1'
                      and (
                          input_payload #>> '{node-start,compatibility,claude_code_control}' is not null
                          or input_payload #>> '{start,compatibility,claude_code_control}' is not null
                          or position('Your task is to create a detailed summary of the conversation so far' in coalesce(
                              input_payload #>> '{node-start,query}',
                              input_payload #>> '{start,query}',
                              input_payload #>> '{query}',
                              ''
                          )) > 0
                          or position('Your task is to create a detailed summary of the RECENT portion of the conversation' in coalesce(
                              input_payload #>> '{node-start,query}',
                              input_payload #>> '{start,query}',
                              input_payload #>> '{query}',
                              ''
                          )) > 0
                          or position('Your task is to create a detailed summary of this conversation. This summary will be placed at the start of a continuing session' in coalesce(
                              input_payload #>> '{node-start,query}',
                              input_payload #>> '{start,query}',
                              input_payload #>> '{query}',
                              ''
                          )) > 0
                          or (
                              position('This session is being continued from a previous conversation that ran out of context.' in coalesce(
                                  input_payload #>> '{node-start,query}',
                                  input_payload #>> '{start,query}',
                                  input_payload #>> '{query}',
                                  ''
                              )) > 0
                              and position('If you need specific details from before compaction' in coalesce(
                                  input_payload #>> '{node-start,query}',
                                  input_payload #>> '{start,query}',
                                  input_payload #>> '{query}',
                                  ''
                              )) > 0
                          )
                      )
                  )
            )
            select rn, total
            from ordered
            where id = $3
            "#,
        )
        .bind(application_id)
        .bind(external_conversation_id)
        .bind(flow_run_id)
        .fetch_optional(self.pool())
        .await?;

        Ok(row.map(|row| (row.get("rn"), row.get("total"))))
    }

    fn application_runs_page_order_by(sort_by: Option<&str>, sort_order: Option<&str>) -> String {
        let sort_by = sort_by.unwrap_or("created_at").to_ascii_lowercase();
        let sort_order = sort_order.unwrap_or("desc").to_ascii_lowercase();
        let field = match sort_by.as_str() {
            "started_at" => "started_at",
            "finished_at" => "finished_at",
            "updated_at" => "updated_at",
            "created_at" => "created_at",
            _ => "created_at",
        };
        let direction = match sort_order.as_str() {
            "asc" => "asc",
            _ => "desc",
        };

        format!("{field} {direction}, id {direction}")
    }

    async fn get_application_run_detail(
        &self,
        application_id: Uuid,
        flow_run_id: Uuid,
    ) -> Result<Option<domain::ApplicationRunDetail>> {
        let Some(flow_run) =
            fetch_flow_run_for_application(self, application_id, flow_run_id).await?
        else {
            return Ok(None);
        };

        Ok(Some(domain::ApplicationRunDetail {
            node_runs: list_node_runs_for_flow_run(self, flow_run.id).await?,
            checkpoints: list_checkpoints_for_flow_run(self, flow_run.id).await?,
            callback_tasks: list_callback_tasks_for_flow_run(self, flow_run.id).await?,
            events: list_events_for_flow_run(self, flow_run.id).await?,
            stitched_trace: list_stitched_trace_for_flow_run(self, &flow_run).await?,
            flow_run,
        }))
    }

    async fn get_latest_node_run(
        &self,
        application_id: Uuid,
        node_id: &str,
    ) -> Result<Option<domain::NodeLastRun>> {
        let latest = sqlx::query(
            r#"
            select
                nr.id as node_run_id,
                fr.id as flow_run_id
            from node_runs nr
            join flow_runs fr on fr.id = nr.flow_run_id
            where fr.application_id = $1
              and nr.node_id = $2
            order by nr.started_at desc, nr.id desc
            limit 1
            "#,
        )
        .bind(application_id)
        .bind(node_id)
        .fetch_optional(self.pool())
        .await?;

        let Some(latest) = latest else {
            return Ok(None);
        };
        let flow_run_id: Uuid = latest.get("flow_run_id");
        let node_run_id: Uuid = latest.get("node_run_id");
        let flow_run = fetch_flow_run_for_application(self, application_id, flow_run_id)
            .await?
            .expect("joined flow_run must exist");
        let node_run = fetch_node_run(self, node_run_id)
            .await?
            .expect("joined node_run must exist");

        Ok(Some(domain::NodeLastRun {
            checkpoints: list_checkpoints_for_node_run(self, node_run.id).await?,
            events: list_events_for_node_context(self, flow_run.id, node_run.id).await?,
            flow_run,
            node_run,
        }))
    }
}

fn empty_application_conversation_runs_page(
) -> control_plane::ports::ApplicationConversationRunsPage {
    control_plane::ports::ApplicationConversationRunsPage {
        items: Vec::new(),
        has_before: false,
        has_after: false,
        before_cursor: None,
        after_cursor: None,
    }
}
