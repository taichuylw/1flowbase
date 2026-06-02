create table flow_run_resume_requests (
    id uuid primary key,
    scope_id uuid not null references workspaces(id) on delete cascade,
    flow_run_id uuid not null references flow_runs(id) on delete cascade,
    callback_task_id uuid not null references flow_run_callback_tasks(id) on delete cascade,
    status text not null check (status in ('pending', 'claimed', 'succeeded', 'failed', 'cancelled')),
    response_payload jsonb not null default '{}'::jsonb,
    idempotency_key text not null,
    claimed_by text,
    claim_expires_at timestamptz,
    error_payload jsonb,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    completed_at timestamptz
);

create unique index flow_run_resume_requests_callback_task_unique_idx
    on flow_run_resume_requests (callback_task_id);

create unique index flow_run_resume_requests_idempotency_key_unique_idx
    on flow_run_resume_requests (idempotency_key);

create index flow_run_resume_requests_flow_created_idx
    on flow_run_resume_requests (flow_run_id, created_at desc, id desc);

create index flow_run_resume_requests_claim_idx
    on flow_run_resume_requests (created_at asc, id asc)
    where status in ('pending', 'claimed');
