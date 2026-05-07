# Agent Flow Runtime Contract Refactor 03 Variable Linker Debug Cache Snapshot Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. Update the index plan after each completed task.

**Goal:** Make Variable Picker, Variable Cache, and durable debug snapshot share the same public-output definition while keeping resolved inputs in trace/audit only.

**Architecture:** The frontend linker reads authoring contracts and public outputs, not runtime cache samples. The backend snapshot endpoint returns an isolated acceleration cache keyed by workspace, actor, draft, document hash, schema version, debug session, and latest run scope.

**Tech Stack:** TypeScript, React 19, TanStack Query, Vitest, Rust 2021, Axum, SQLx/PostgreSQL.

---

## Files

- Modify: `web/app/src/features/agent-flow/lib/selector-options.ts`
- Modify: `web/app/src/features/agent-flow/lib/start-node-variables.ts`
- Modify: `web/app/src/features/agent-flow/lib/variable-labels.ts`
- Modify: `web/app/src/features/agent-flow/lib/debug-console/variable-groups.ts`
- Modify: `web/app/src/features/agent-flow/hooks/runtime/useAgentFlowDebugSession.ts`
- Modify: `web/app/src/features/agent-flow/api/runtime.ts`
- Modify: `api/apps/api-server/src/routes/applications/application_runtime.rs`
- Modify: `api/crates/control-plane/src/ports/runtime.rs`
- Modify: `api/crates/storage-durable/postgres/src/orchestration_runtime_repository/**`
- Test: `web/app/src/features/agent-flow/_tests/debug-console/**`
- Test: `api/apps/api-server/src/_tests/application/application_runtime_routes.rs`

## Tasks

### Task 1: Rebuild frontend variable cache source rules

- [ ] Remove `node_run.input_payload` and non-Start `flow_run.input_payload` from frontend variable cache reconstruction.
- [ ] Keep resolved inputs visible in Trace item detail.
- [ ] Keep Run Context as a separate display group.
- [ ] Preserve Start public input variables as the only input-like values allowed in Variable Cache.

### Task 2: Use stable variable identity

- [ ] Make display identity `node.alias/key`.
- [ ] Keep selector identity as `node.id + key`.
- [ ] Treat output title as helper text only.
- [ ] Update tests that assert title-based variable identity.

### Task 3: Object-level Variables tab

- [ ] Stop recursively flattening output objects into separate variable identities.
- [ ] Show one object-level entry per public output key.
- [ ] Keep optional value inspector expansion for display only.
- [ ] Ensure Variable Picker deep expansion comes from output schema, not cached values.

### Task 4: Backend debug snapshot isolation

- [ ] Extend snapshot request/response or storage model with workspace, actor, draft, document hash, flow schema version, snapshot schema version, debug session, and latest run scope.
- [ ] Build snapshot from Start public inputs and succeeded or waiting-success node public outputs only.
- [ ] Do not read `node_run.input_payload`, `metrics_payload`, `error_payload`, or `debug_payload` into variable cache.
- [ ] Return `snapshot_completeness` for partial/running snapshots.
- [ ] Return source `node_run_id` evidence for values that overwrite older values.

### Task 5: Verification

Run:

```bash
pnpm --dir web/app test -- variable-groups use-agent-flow-debug-session node-debug-preview-input selector-options
cargo test -p api-server debug_variable_snapshot -- --test-threads=1
```

Expected:

- Non-Start `input_payload` remains visible in trace detail but never appears in Variable Cache.
- Snapshot does not cross workspace, actor, draft, document hash, schema, debug session, or run scope.
- Variables tab and Variable Picker agree on what is a variable.

## Stop Conditions

- Product requires input fields to be downstream-referenceable variables.
- Variable Picker must infer fields from runtime samples.
- Snapshot must merge multiple drafts or multiple document hashes.
