# Agent Flow Runtime Contract Refactor 07 Data Model Side Effects Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. Update the index plan after each completed task.

**Goal:** Make Data Model workflow nodes output-stable, permission-safe, and side-effect-auditable, with same-run replay idempotency for writes.

**Architecture:** Read nodes remain normal public-output producers. Write nodes require a debug side-effect policy and persist a side-effect receipt tied to `run_id`, `node_id`, action, and resolved payload hash so checkpoint replay in the same run cannot duplicate writes.

**Tech Stack:** Rust 2021, runtime-core Data Model runtime, control-plane, SQLx/PostgreSQL, TypeScript node definitions.

**Status:** Completed on 2026-05-08.

---

## Files

- Modify: `web/app/src/features/agent-flow/lib/node-definitions/nodes/data-model/index.ts`
- Modify: `api/crates/control-plane/src/orchestration_runtime/data_model_runtime.rs`
- Modify: `api/crates/control-plane/src/orchestration_runtime/live_debug_run/continuation.rs`
- Modify: `api/crates/control-plane/src/ports/runtime.rs`
- Create migration: `api/crates/storage-durable/postgres/migrations/*_create_data_model_side_effect_receipts.sql`
- Modify: `api/crates/storage-durable/postgres/src/orchestration_runtime_repository/**`
- Test: `api/crates/control-plane/src/_tests/orchestration_runtime/data_model_query.rs`
- Test: `api/crates/control-plane/src/_tests/orchestration_runtime/service.rs`
- Test: `web/app/src/features/agent-flow/_tests/node-schema-registry.test.tsx`

## Tasks

### Task 1: Align fixed Data Model outputs

- [x] Ensure list outputs `records` and `total`.
- [x] Ensure get/create/update output `record`.
- [x] Ensure delete outputs `deleted_id` and `affected_count`.
- [x] Update frontend node definition tests and backend runtime tests.

### Task 2: Add side-effect policy

- [x] Add debug run policy values: `disabled`, `confirm_each_run`, `allow_with_idempotency`.
- [x] Make write nodes fail with `DATA_MODEL_SIDE_EFFECT_DISABLED` when policy is disabled.
- [x] Make `confirm_each_run` enter a waiting confirmation state with actor, node id, run id, payload hash, and expiry.
- [x] Keep read nodes unaffected.

### Task 3: Add same-run idempotency receipt

- [x] Generate key from `workspace_id + application_id + draft_id + run_id + node_id + action + resolved payload hash`.
- [x] Add receipt fields: action, model code, record id or deleted id, affected count, idempotency key, actor, scope id, node run id, created time, status.
- [x] Add unique index on `(workspace_id, idempotency_key)`.
- [x] On same-run checkpoint replay, return the recorded result instead of writing again.
- [x] Do not deduplicate across different debug runs.

### Task 4: Add audit/outbox failure semantics

- [x] Treat write + receipt + audit/outbox as one owner-controlled action.
- [x] If receipt or audit/outbox fails after the write, mark the node as not fully successful and expose formal error/debug evidence.
- [x] Do not let node UI or plugins synthesize missing receipts.

### Task 5: Verification

Run:

```bash
cargo test -p control-plane orchestration_runtime_data_model -- --test-threads=1
cargo test -p storage-postgres data_model_side_effect_receipts -- --test-threads=1
pnpm --dir web/app test -- node-schema-registry
```

Expected:

- Delete returns `deleted_id` and `affected_count`.
- Same `run_id` checkpoint replay does not duplicate create/update/delete.
- Cross debug run execution gets a new idempotency key.

### Verification Evidence

- `cargo test -p control-plane orchestration_runtime_data_model -- --test-threads=1`: 17 passed.
- `cargo test -p storage-postgres data_model_side_effect_receipts -- --test-threads=1`: 1 passed.
- `pnpm --dir web/app test -- node-schema-registry`: 21 passed.
- Extra callback regression: `cargo test -p control-plane complete_callback_task -- --test-threads=1`: 1 passed.
- Extra storage callback regression: `cargo test -p storage-postgres callback_tasks -- --test-threads=1`: 1 passed.
- Extra frontend binding regression: `pnpm --dir web/app test -- node-inspector validate-document node-debug-preview-input`: 44 passed.
- Formatting/static: `cargo fmt --all -- --check` and `git diff --check`.

### Notes

- `confirm_each_run` now stops the debug run and records a callback task; approved callback completion executes the Data Model write, persists/replays the receipt, then resumes downstream nodes.
- Callback completion now requires a pending callback task, verifies the original actor and expiry metadata before completion, and rejects duplicate completion.
- Writes now claim a `pending` receipt before executing the Data Model write; success promotes it to `succeeded`, write failure marks it `failed`, and receipt persistence failure leaves a non-replayable `pending` row so same-run retry cannot duplicate the write silently.
- Same-run replay is covered by seeding the matching receipt before callback completion and asserting the confirmed write returns receipt output with `side_effect_replayed = true`.
- Frontend get/update/delete now expose `bindings.record_id` selector fields to match the compiler and runtime contract; list remains the only Data Model query editor.
- The storage crate is `storage-postgres`; the original plan command used the historical `storage-durable` naming.

## Stop Conditions

- Product requires cross-debug-run business idempotency.
- Data Model write must succeed even when receipt/audit/outbox persistence fails.
- Write nodes can run in debug without explicit side-effect policy.
