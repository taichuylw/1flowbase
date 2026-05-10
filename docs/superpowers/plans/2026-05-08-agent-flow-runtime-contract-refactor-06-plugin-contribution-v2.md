# Agent Flow Runtime Contract Refactor 06 Plugin Contribution V2 Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. Update the index plan after each completed task.

**Goal:** Implement plugin contribution v2 as a locked, host-rendered, public-output-only node contract.

**Architecture:** Plugin contributions declare schema and policy; host code owns renderers, infra, payload filtering, and runtime execution boundaries. Node instances save immutable contribution identity and output schema snapshot so plugin upgrades cannot silently change old node behavior.

**Tech Stack:** Rust 2021, plugin-framework, control-plane plugin repository, TypeScript flow schema, React host renderers.

**Status:** Completed on 2026-05-08.

---

## Files

- Modify: `web/packages/flow-schema/src/index.ts`
- Modify: `web/app/src/features/agent-flow/lib/node-definitions/nodes/plugin/**`
- Modify: `api/crates/plugin-framework/src/**`
- Modify: `api/crates/control-plane/src/plugin_management/**`
- Modify: `api/crates/control-plane/src/orchestration_runtime/compile_context.rs`
- Modify: `api/crates/orchestration-runtime/src/compiler.rs`
- Modify: `api/crates/orchestration-runtime/src/execution_engine.rs`
- Test: `api/crates/orchestration-runtime/src/_tests/compiler_tests.rs`
- Test: `api/crates/control-plane/src/_tests/plugin_management/**`
- Test: `web/app/src/features/agent-flow/_tests/node-schema-registry.test.tsx`

## Tasks

### Task 1: Extend contribution identity

- [x] Add `plugin_unique_identifier`.
- [x] Add `package_id`.
- [x] Add `contribution_checksum`.
- [x] Add `compiled_contribution_hash`.
- [x] Add `output_schema_snapshot`.
- [x] Store these on node instances and compiled plan entries.

### Task 2: Enforce host renderer allowlist

- [x] Reject unknown field renderer codes.
- [x] Reject plugin-provided React panel declarations.
- [x] Treat renderer allowlist as host capability, not plugin capability.
- [x] Keep panel schema declarative.

### Task 3: Enforce output and policy rules

- [x] Reject public output keys `metadata`, `usage`, `debug`, `error`, and `__*`.
- [x] Require side-effect declaration: `none`, `external_read`, `external_write`, or `durable_write`.
- [x] Keep metrics/error/debug fields out of `output_schema.outputs`.
- [x] Reject plugin infra contracts for cache-store, distributed-lock, event-bus, task-queue, and object storage.

### Task 4: Runtime stale contribution handling

- [x] Compile fails if package is missing.
- [x] Compile fails if checksum or output schema snapshot drifts.
- [x] Existing nodes require explicit recompile or migration prompt after plugin upgrade.
- [x] Executor unknown output keys are rejected by the payload builder and recorded as contract errors.

### Task 5: Verification

Run:

```bash
cargo test -p orchestration-runtime plugin -- --test-threads=1
cargo test -p control-plane plugin_management -- --test-threads=1
pnpm --dir web/app test -- node-schema-registry agent-flow
```

Additional targeted verification:

```bash
cargo test -p plugin-framework manifest_v1 -- --test-threads=1
cargo test -p orchestration-runtime plugin -- --test-threads=1
cargo test -p storage-postgres node_contribution -- --test-threads=1
cargo test -p control-plane plugin_management -- --test-threads=1
cargo test -p control-plane orchestration_runtime -- --test-threads=1
cargo test -p api-server node_contribution -- --test-threads=1
cargo test -p plugin-runner capability_runtime -- --test-threads=1
pnpm --dir web/packages/flow-schema test
pnpm --dir web/packages/api-client test -- node-contributions console-node-contributions
pnpm --dir web/app test -- node-contribution-picker validate-document node-picker-popover
cargo fmt -p plugin-framework -p control-plane -p orchestration-runtime -p storage-postgres -p api-server -p domain -p plugin-runner -- --check
git diff --check
```

Evidence:

- `cargo test -p plugin-framework manifest_v1 -- --test-threads=1`: 19 passed.
- `cargo test -p orchestration-runtime plugin -- --test-threads=1`: 8 passed.
- `cargo test -p storage-postgres node_contribution -- --test-threads=1`: 2 passed.
- `cargo test -p control-plane plugin_management -- --test-threads=1`: 21 passed.
- `cargo test -p control-plane orchestration_runtime -- --test-threads=1`: 78 passed.
- `cargo test -p api-server node_contribution -- --test-threads=1`: 1 passed.
- `cargo test -p plugin-runner capability_runtime -- --test-threads=1`: 1 integration test passed.
- `pnpm --dir web/packages/flow-schema test`: 1 file / 35 tests passed.
- `pnpm --dir web/packages/api-client test -- node-contributions console-node-contributions`: 4 files / 25 tests passed.
- `pnpm --dir web/app test -- node-contribution-picker validate-document node-picker-popover`: 3 files / 33 tests passed.
- `pnpm --dir web/app test -- node-schema-registry agent-flow`: 41 files / 262 tests passed.
- `cargo fmt -p plugin-framework -p control-plane -p orchestration-runtime -p storage-postgres -p api-server -p domain -p plugin-runner -- --check`: passed.
- `git diff --check`: passed.
- Independent serial review re-check: PASS after resolving migration fake v2 rows, reserved executor keys, and incomplete output schema validation.

Residual risks:

- Database constraints validate `output_schema_snapshot.outputs` shape and legacy hash rejection, while per-output `key/title/valueType` validation is enforced in manifest parsing and compile-context conversion.
- Existing node contribution registry rows that cannot prove v2 identity/hash/snapshot are deleted by the migration and must be restored through plugin install/sync.
- Full OpenAPI/client generation is deferred to plan 09 cutover.

Expected:

- Unknown renderers and invalid output keys cannot enter draft documents.
- Plugin metadata and invocation internals do not enter public output.
- Stale contribution state fails compile instead of falling back to current installation shape.

## Stop Conditions

- Plugins must provide executable frontend code.
- CapabilityPlugin or RuntimeExtension must consume host infrastructure directly.
- Old plugin contribution v1 must remain compatible.
