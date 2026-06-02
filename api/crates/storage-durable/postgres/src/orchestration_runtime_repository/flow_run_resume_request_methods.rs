impl PgControlPlaneStore {
    pub async fn upsert_flow_run_resume_request(
        &self,
        input: &UpsertFlowRunResumeRequestInput,
    ) -> Result<UpsertFlowRunResumeRequestOutput> {
        let inserted = sqlx::query(
            r#"
            insert into flow_run_resume_requests (
                id,
                scope_id,
                flow_run_id,
                callback_task_id,
                status,
                response_payload,
                idempotency_key
            ) values (
                $1,
                (
                    select applications.workspace_id
                    from flow_runs
                    join applications on applications.id = flow_runs.application_id
                    where flow_runs.id = $2
                ),
                $2,
                $3,
                'pending',
                $4,
                $5
            )
            on conflict (callback_task_id) do nothing
            returning
                id,
                flow_run_id,
                callback_task_id,
                status,
                response_payload,
                idempotency_key,
                claimed_by,
                claim_expires_at,
                error_payload,
                created_at,
                updated_at,
                completed_at
            "#,
        )
        .bind(Uuid::now_v7())
        .bind(input.flow_run_id)
        .bind(input.callback_task_id)
        .bind(&input.response_payload)
        .bind(&input.idempotency_key)
        .fetch_optional(self.pool())
        .await?;

        if let Some(row) = inserted {
            return Ok(UpsertFlowRunResumeRequestOutput {
                request: map_flow_run_resume_request_record(&row)?,
                inserted: true,
            });
        }

        let existing = sqlx::query(
            r#"
            select
                id,
                flow_run_id,
                callback_task_id,
                status,
                response_payload,
                idempotency_key,
                claimed_by,
                claim_expires_at,
                error_payload,
                created_at,
                updated_at,
                completed_at
            from flow_run_resume_requests
            where callback_task_id = $1
            "#,
        )
        .bind(input.callback_task_id)
        .fetch_one(self.pool())
        .await?;

        Ok(UpsertFlowRunResumeRequestOutput {
            request: map_flow_run_resume_request_record(&existing)?,
            inserted: false,
        })
    }

    pub async fn claim_next_flow_run_resume_request(
        &self,
        input: &ClaimFlowRunResumeRequestInput,
    ) -> Result<Option<FlowRunResumeRequestClaim>> {
        let row = sqlx::query(
            r#"
            with candidate as (
                select requests.id
                from flow_run_resume_requests requests
                join flow_runs on flow_runs.id = requests.flow_run_id
                join flow_run_callback_tasks tasks on tasks.id = requests.callback_task_id
                where (
                    requests.status = 'pending'
                    or (
                        requests.status = 'claimed'
                        and requests.claim_expires_at is not null
                        and requests.claim_expires_at <= now()
                    )
                )
                  and flow_runs.status = 'waiting_callback'
                  and tasks.status = 'pending'
                order by requests.created_at asc, requests.id asc
                for update skip locked
                limit 1
            )
            update flow_run_resume_requests requests
            set status = 'claimed',
                claimed_by = $1,
                claim_expires_at = $2,
                updated_at = now()
            from candidate
            join flow_runs on flow_runs.id = (
                select flow_run_id
                from flow_run_resume_requests
                where id = candidate.id
            )
            where requests.id = candidate.id
            returning
                requests.id,
                requests.flow_run_id,
                requests.callback_task_id,
                requests.status,
                requests.response_payload,
                requests.idempotency_key,
                requests.claimed_by,
                requests.claim_expires_at,
                requests.error_payload,
                requests.created_at,
                requests.updated_at,
                requests.completed_at,
                flow_runs.application_id,
                flow_runs.created_by as actor_user_id
            "#,
        )
        .bind(&input.worker_id)
        .bind(input.claim_expires_at)
        .fetch_optional(self.pool())
        .await?;

        row.map(|row| {
            Ok(FlowRunResumeRequestClaim {
                request: map_flow_run_resume_request_record(&row)?,
                application_id: row.get("application_id"),
                actor_user_id: row.get("actor_user_id"),
            })
        })
        .transpose()
    }

    pub async fn finish_flow_run_resume_request(
        &self,
        input: &FinishFlowRunResumeRequestInput,
    ) -> Result<domain::FlowRunResumeRequestRecord> {
        let row = sqlx::query(
            r#"
            update flow_run_resume_requests
            set status = case when status = 'claimed' then $2 else status end,
                error_payload = case when status = 'claimed' then $3 else error_payload end,
                claim_expires_at = null,
                completed_at = coalesce(completed_at, $4),
                updated_at = now()
            where id = $1
              and status in ('claimed', 'cancelled')
            returning
                id,
                flow_run_id,
                callback_task_id,
                status,
                response_payload,
                idempotency_key,
                claimed_by,
                claim_expires_at,
                error_payload,
                created_at,
                updated_at,
                completed_at
            "#,
        )
        .bind(input.request_id)
        .bind(input.status.as_str())
        .bind(&input.error_payload)
        .bind(input.completed_at)
        .fetch_optional(self.pool())
        .await?;

        let Some(row) = row else {
            return Err(ControlPlaneError::Conflict("resume_request_not_claimed").into());
        };

        map_flow_run_resume_request_record(&row)
    }

    pub async fn cancel_flow_run_resume_requests_for_run(
        &self,
        flow_run_id: Uuid,
        completed_at: OffsetDateTime,
    ) -> Result<Vec<domain::FlowRunResumeRequestRecord>> {
        let rows = sqlx::query(
            r#"
            update flow_run_resume_requests
            set status = 'cancelled',
                claim_expires_at = null,
                completed_at = $2,
                updated_at = now()
            where flow_run_id = $1
              and status in ('pending', 'claimed')
            returning
                id,
                flow_run_id,
                callback_task_id,
                status,
                response_payload,
                idempotency_key,
                claimed_by,
                claim_expires_at,
                error_payload,
                created_at,
                updated_at,
                completed_at
            "#,
        )
        .bind(flow_run_id)
        .bind(completed_at)
        .fetch_all(self.pool())
        .await?;

        rows.iter().map(map_flow_run_resume_request_record).collect()
    }

    pub async fn summarize_flow_run_resume_requests(
        &self,
    ) -> Result<FlowRunResumeRequestQueueStats> {
        let row = sqlx::query(
            r#"
            select
                count(*) filter (where status = 'pending')::bigint as pending_count,
                count(*) filter (where status = 'claimed')::bigint as claimed_count,
                count(*) filter (where status = 'succeeded')::bigint as succeeded_count,
                count(*) filter (where status = 'failed')::bigint as failed_count,
                count(*) filter (where status = 'cancelled')::bigint as cancelled_count,
                count(*) filter (
                    where status = 'claimed'
                      and claim_expires_at is not null
                      and claim_expires_at <= now()
                )::bigint as expired_claim_count,
                min(created_at) filter (where status = 'pending') as oldest_pending_created_at
            from flow_run_resume_requests
            "#,
        )
        .fetch_one(self.pool())
        .await?;

        Ok(FlowRunResumeRequestQueueStats {
            pending_count: row.get("pending_count"),
            claimed_count: row.get("claimed_count"),
            succeeded_count: row.get("succeeded_count"),
            failed_count: row.get("failed_count"),
            cancelled_count: row.get("cancelled_count"),
            expired_claim_count: row.get("expired_claim_count"),
            oldest_pending_created_at: row.get("oldest_pending_created_at"),
        })
    }
}
