# Agent Flow Runtime Contract Refactor 06 Plugin Contribution V2 Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. Update the index plan after each completed task.

**Goal:** Implement plugin contribution v2 as a locked, host-rendered, public-output-only node contract.

**Architecture:** Plugin contributions declare schema and policy; host code owns renderers, infra, payload filtering, and runtime execution boundaries. Node instances save immutable contribution identity and output schema snapshot so plugin upgrades cannot silently change old node behavior.

**Tech Stack:** Rust 2021, plugin-framework, control-plane plugin repository, TypeScript flow schema, React host renderers.

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

- [ ] Add `plugin_unique_identifier`.
- [ ] Add `package_id`.
- [ ] Add `contribution_checksum`.
- [ ] Add `compiled_contribution_hash`.
- [ ] Add `output_schema_snapshot`.
- [ ] Store these on node instances and compiled plan entries.

### Task 2: Enforce host renderer allowlist

- [ ] Reject unknown field renderer codes.
- [ ] Reject plugin-provided React panel declarations.
- [ ] Treat renderer allowlist as host capability, not plugin capability.
- [ ] Keep panel schema declarative.

### Task 3: Enforce output and policy rules

- [ ] Reject public output keys `metadata`, `usage`, `debug`, `error`, and `__*`.
- [ ] Require side-effect declaration: `none`, `external_read`, `external_write`, or `durable_write`.
- [ ] Keep metrics/error/debug fields out of `output_schema.outputs`.
- [ ] Reject plugin infra contracts for cache-store, distributed-lock, event-bus, task-queue, and object storage.

### Task 4: Runtime stale contribution handling

- [ ] Compile fails if package is missing.
- [ ] Compile fails if checksum or output schema snapshot drifts.
- [ ] Existing nodes require explicit recompile or migration prompt after plugin upgrade.
- [ ] Executor unknown output keys are rejected by the payload builder and recorded as contract errors.

### Task 5: Verification

Run:

```bash
cargo test -p orchestration-runtime plugin -- --test-threads=1
cargo test -p control-plane plugin_management -- --test-threads=1
pnpm --dir web/app test -- node-schema-registry agent-flow
```

Expected:

- Unknown renderers and invalid output keys cannot enter draft documents.
- Plugin metadata and invocation internals do not enter public output.
- Stale contribution state fails compile instead of falling back to current installation shape.

## Stop Conditions

- Plugins must provide executable frontend code.
- CapabilityPlugin or RuntimeExtension must consume host infrastructure directly.
- Old plugin contribution v1 must remain compatible.
