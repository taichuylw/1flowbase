# Agent Flow Runtime Contract Refactor 07 Data Model Side Effects Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. Update the index plan after each completed task.

**Goal:** Make Data Model workflow nodes output-stable, permission-safe, and side-effect-auditable, with same-run replay idempotency for writes.

**Architecture:** Read nodes remain normal public-output producers. Write nodes require a debug side-effect policy and persist a side-effect receipt tied to `run_id`, `node_id`, action, and resolved payload hash so checkpoint replay in the same run cannot duplicate writes.

**Tech Stack:** Rust 2021, runtime-core Data Model runtime, control-plane, SQLx/PostgreSQL, TypeScript node definitions.

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

- [ ] Ensure list outputs `records` and `total`.
- [ ] Ensure get/create/update output `record`.
- [ ] Ensure delete outputs `deleted_id` and `affected_count`.
- [ ] Update frontend node definition tests and backend runtime tests.

### Task 2: Add side-effect policy

- [ ] Add debug run policy values: `disabled`, `confirm_each_run`, `allow_with_idempotency`.
- [ ] Make write nodes fail with `DATA_MODEL_SIDE_EFFECT_DISABLED` when policy is disabled.
- [ ] Make `confirm_each_run` enter a waiting confirmation state with actor, node id, run id, payload hash, and expiry.
- [ ] Keep read nodes unaffected.

### Task 3: Add same-run idempotency receipt

- [ ] Generate key from `workspace_id + application_id + draft_id + run_id + node_id + action + resolved payload hash`.
- [ ] Add receipt fields: action, model code, record id or deleted id, affected count, idempotency key, actor, scope id, node run id, created time, status.
- [ ] Add unique index on `(workspace_id, idempotency_key)`.
- [ ] On same-run checkpoint replay, return the recorded result instead of writing again.
- [ ] Do not deduplicate across different debug runs.

### Task 4: Add audit/outbox failure semantics

- [ ] Treat write + receipt + audit/outbox as one owner-controlled action.
- [ ] If receipt or audit/outbox fails after the write, mark the node as not fully successful and expose formal error/debug evidence.
- [ ] Do not let node UI or plugins synthesize missing receipts.

### Task 5: Verification

Run:

```bash
cargo test -p control-plane orchestration_runtime_data_model -- --test-threads=1
cargo test -p storage-durable data_model_side_effect_receipts -- --test-threads=1
pnpm --dir web/app test -- node-schema-registry
```

Expected:

- Delete returns `deleted_id` and `affected_count`.
- Same `run_id` checkpoint replay does not duplicate create/update/delete.
- Cross debug run execution gets a new idempotency key.

## Stop Conditions

- Product requires cross-debug-run business idempotency.
- Data Model write must succeed even when receipt/audit/outbox persistence fails.
- Write nodes can run in debug without explicit side-effect policy.
