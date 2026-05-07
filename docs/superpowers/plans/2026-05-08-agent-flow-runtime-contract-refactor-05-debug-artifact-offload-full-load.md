# Agent Flow Runtime Contract Refactor 05 Debug Artifact Offload Full Load Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. Update the index plan after each completed task.

**Goal:** Add durable artifact refs, truncation previews, full-load APIs, and retention/GC state for large runtime payloads and debug evidence.

**Architecture:** Inline payloads stay small and inspectable. Large input, output, metrics, debug, raw provider response, provider event, and draft variable snapshot values are offloaded to object storage with preview metadata and an authenticated full-load path.

**Tech Stack:** Rust 2021, SQLx/PostgreSQL, object storage port, Axum, TypeScript API client, React 19.

---

## Files

- Create: `api/crates/control-plane/src/orchestration_runtime/debug_artifacts.rs`
- Modify: `api/crates/control-plane/src/ports/runtime.rs`
- Modify: `api/crates/control-plane/src/ports/file_management.rs` or existing object storage port
- Create migration: `api/crates/storage-durable/postgres/migrations/*_create_runtime_debug_artifacts.sql`
- Modify: `api/crates/storage-durable/postgres/src/orchestration_runtime_repository/**`
- Modify: `api/apps/api-server/src/routes/applications/application_runtime.rs`
- Modify: `web/packages/api-client/src/console-application-runtime.ts`
- Modify: `web/app/src/features/agent-flow/lib/debug-console/variable-groups.ts`
- Modify: `web/app/src/features/agent-flow/components/debug-console/**`
- Test: `api/crates/storage-durable/postgres/src/_tests/orchestration_runtime_repository_tests.rs`
- Test: `api/apps/api-server/src/_tests/application/application_runtime_routes.rs`

## Tasks

### Task 1: Add artifact storage model

- [ ] Add table fields for artifact id, workspace id, application id, flow run id, node run id, artifact kind, content type, original size, preview size, storage ref, retention state, and created time.
- [ ] Add repository methods to create artifact refs and load full artifact content by authorized scope.
- [ ] Add GC state values: `active`, `pending_delete`, `deleted`.

### Task 2: Add truncation and preview builder

- [ ] Add inline byte budgets for `input_payload`, `output_payload`, `metrics_payload`, `debug_payload`, provider raw events, and draft snapshot values.
- [ ] Return preview objects with `is_truncated`, `original_size_bytes`, `preview_size_bytes`, `content_type`, and `artifact_ref`.
- [ ] Treat offload failure as a runtime error/debug condition, not as a complete public output.

### Task 3: Add full-load API

- [ ] Add authenticated endpoint for artifact full-load under application runtime routes.
- [ ] Verify actor can see the application and workspace before reading artifact content.
- [ ] Return content type and full JSON/text payload without involving cache-store.

### Task 4: Wire frontend previews

- [ ] Show truncation state in Trace and Variable Cache.
- [ ] Load full value on explicit user expansion.
- [ ] Do not let Variable Picker infer schema fields from full-loaded values.

### Task 5: Verification

Run:

```bash
cargo test -p storage-durable runtime_debug_artifacts -- --test-threads=1
cargo test -p api-server runtime_debug_artifacts -- --test-threads=1
pnpm --dir web/app test -- variable-groups agent-flow
```

Expected:

- Large payloads are stored as preview/ref.
- Full-load requires application visibility.
- Truncated public output is not presented as complete data.

## Stop Conditions

- Object storage port is unavailable and cannot be added in this slice.
- Product requires cache-store lookup for full content.
- Variable Picker must inspect offloaded runtime values.
