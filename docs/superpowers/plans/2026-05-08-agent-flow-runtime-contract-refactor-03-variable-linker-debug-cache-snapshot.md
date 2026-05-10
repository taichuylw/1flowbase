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

- [x] Remove `node_run.input_payload` and non-Start `flow_run.input_payload` from frontend variable cache reconstruction.
- [x] Keep resolved inputs visible in Trace item detail.
- [x] Keep Run Context as a separate display group.
- [x] Preserve Start public input variables as the only input-like values allowed in Variable Cache.

Evidence:

- RED: `pnpm --dir web/app test -- variable-groups` failed with `expected '' to be '请总结退款政策'` before the Run Detail Start-input display fix.
- GREEN: `pnpm --dir web/app test -- variable-groups` passed, 1 file / 2 tests.
- Targeted: `pnpm --dir web/app test -- variable-groups use-agent-flow-debug-session node-debug-preview-input` passed, 4 files / 25 tests.
- Targeted stream: `pnpm --dir web/app test -- use-agent-flow-debug-session-stream` passed, 1 file / 11 tests.
- Spec review: PASS for removing resolved inputs from Variable Cache while preserving trace detail.
- Code quality review: initial FAIL found Run Detail Start-input display regression; re-review PASS after `detail.flow_run.input_payload[nodeId][key]` is used only for Start-field whitelisted display values.

### Task 2: Use stable variable identity

- [x] Make display identity `node.alias/key`.
- [x] Keep selector identity as `node.id + key`.
- [x] Treat output title as helper text only.
- [x] Update tests that assert title-based variable identity.

Evidence:

- Targeted: `pnpm --dir web/app test -- start-node-variables templated-text-field-focus-layout node-debug-preview-input` passed, 3 files / 21 tests.
- Spec review: PASS; selector display uses `node.alias/key`, selector value remains `[node.id, key]`, and `output.title` is not consumed by selector identity/display identity.
- Code quality review: initial FAIL found missing `outputLabel === key` assertions for multiple LLM outputs and unrelated negative assertions; re-review PASS after complete identity assertions and cleanup.

### Task 3: Object-level Variables tab

- [x] Stop recursively flattening output objects into separate variable identities.
- [x] Show one object-level entry per public output key.
- [x] Keep optional value inspector expansion for display only.
- [x] Ensure Variable Picker deep expansion comes from output schema, not cached values.

Evidence:

- RED: `pnpm --dir web/app test -- variable-groups` failed with `expected [ 'LLM/record.id', …(3) ] to deeply equal [ 'LLM/record', 'LLM/records', …(1) ]` before replacing recursive output flattening.
- GREEN: `pnpm --dir web/app test -- variable-groups` passed, 1 file / 3 tests.
- RED: `pnpm --dir web/app test -- variable-groups` failed with alias-based keys (`LLM.record`) before stable node-id keys were restored for variable item keys.
- GREEN: `pnpm --dir web/app test -- debug-variables-pane` passed, 1 file / 1 test, after duplicate variable paths were made selectable across groups via internal selection keys.
- Targeted: `pnpm --dir web/app test -- variable-groups debug-variables-pane use-agent-flow-debug-session node-debug-preview-input selector-options` passed, 5 files / 27 tests. The filter did not match a separate selector-options test file in this tree.
- Selector review: PASS; `selector-options.ts` still builds picker options from `getNodeVariableOutputs(node)` and does not read cached values.
- Code quality review: initial FAIL found alias-based item keys and duplicate group selection collisions; re-review PASS after key/label separation and scoped UI selection keys.

### Task 4: Backend debug snapshot isolation

- [x] Extend snapshot request/response or storage model with workspace, actor, draft, document hash, flow schema version, snapshot schema version, debug session, and latest run scope.
- [x] Build snapshot from Start public inputs and succeeded or waiting-success node public outputs only.
- [x] Do not read `node_run.input_payload`, `metrics_payload`, `error_payload`, or `debug_payload` into variable cache.
- [x] Return `snapshot_completeness` for partial/running snapshots.
- [x] Return source `node_run_id` evidence for values that overwrite older values.

Evidence:

- Backend: `cargo test -p api-server debug_variable_snapshot -- --test-threads=1` passed, 7 tests.
- Runtime: `cargo test -p control-plane orchestration_runtime -- --test-threads=1` passed, 75 tests.
- Storage: `cargo test -p storage-postgres orchestration_runtime_repository -- --test-threads=1` passed, 16 tests.
- Frontend session: `pnpm --dir web/app test -- use-agent-flow-debug-session node-last-run-runtime` passed, 3 files / 19 tests.
- Formatting: `cargo fmt -p api-server -p control-plane -p storage-postgres -- --check` passed.
- Whitespace: `git diff --check` passed.
- Spec review: PASS; snapshot matches only immutable `flow_runs` actor/session/draft/schema/document hash fields and covers same-draft compiled-plan upsert restoration isolation.
- Code quality re-review: initial FAIL found mutable compiled-plan row risk for flow debug attach/continue; re-review PASS after compiled plans became insert-only immutable rows with `document_hash`, attach validates compiled/run schema and hash, and storage regression covers old row preservation plus mismatch rejection.

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

Evidence:

- `pnpm --dir web/app test -- variable-groups use-agent-flow-debug-session node-debug-preview-input selector-options` passed, 4 files / 26 tests. The filter did not match a separate selector-options test file in this tree.
- `cargo test -p api-server debug_variable_snapshot -- --test-threads=1` passed, 7 tests.

## Stop Conditions

- Product requires input fields to be downstream-referenceable variables.
- Variable Picker must infer fields from runtime samples.
- Snapshot must merge multiple drafts or multiple document hashes.
