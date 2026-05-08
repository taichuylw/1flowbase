# Agent Flow Runtime Contract Refactor 02 Payload Builder And LLM Output Contract Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. Update the index plan after each completed task.

**Goal:** Route every node execution result through a shared payload builder so `output_payload`, `metrics_payload`, `error_payload`, `debug_payload`, and variable pool updates are mutually exclusive.

**Architecture:** Add a payload builder in the orchestration runtime crate and use it from non-stream and live debug execution paths. LLM execution returns raw execution facts; the builder filters public outputs and moves usage, route, finish reason, provider metadata, tool calls, raw refs, attempts, and errors to the correct payload bucket.

**Tech Stack:** Rust 2021, `serde_json`, `anyhow`, orchestration-runtime tests, control-plane tests.

---

## Files

- Create: `api/crates/orchestration-runtime/src/payload_builder.rs`
- Modify: `api/crates/orchestration-runtime/src/lib.rs`
- Modify: `api/crates/orchestration-runtime/src/execution_engine.rs`
- Modify: `api/crates/control-plane/src/orchestration_runtime/live_debug_run/continuation.rs`
- Modify: `api/crates/control-plane/src/orchestration_runtime/live_debug_run/observability.rs`
- Modify: `api/crates/control-plane/src/orchestration_runtime/persistence.rs`
- Test: `api/crates/orchestration-runtime/src/_tests/execution_engine_tests.rs`
- Test: `api/crates/control-plane/src/_tests/orchestration_runtime/runtime_observability.rs`

## Tasks

### Task 1: Add payload builder model

- [x] Define `RawNodeExecutionResult` with executor output, metrics facts, error facts, debug facts, and provider events.
- [x] Define `BuiltNodePayloads` with `output_payload`, `metrics_payload`, `error_payload`, and `debug_payload`.
- [x] Define `PublicOutputContract` from compiled node outputs.
- [x] Reject unknown public output keys unless the contract explicitly allows structured expansion.
- [x] Add tests for unknown key rejection and payload bucket exclusivity.

### Task 2: Split LLM raw result

- [x] Change LLM execution to produce raw fields instead of a fully public `output_payload`.
- [x] Keep final answer text as public `text`.
- [x] Keep optional `structured_output` only when contract declares it.
- [x] Move `usage`, `finish_reason`, `route`, `attempts`, `provider_instance_id`, `provider_code`, `protocol`, `model`, and event count to `metrics_payload`.
- [x] Move provider metadata, tool calls, MCP calls, raw response refs, context projection refs, and attempt refs to `debug_payload`.
- [x] Move provider errors to `error_payload`.
- [x] Ensure failure after first token does not write normal public output to variable pool.

### Task 3: Use payload builder in both execution paths

- [x] Use the builder in `execution_engine.rs` for non-stream debug execution.
- [x] Use the same builder in live debug continuation.
- [x] Ensure `variable_pool.insert(node_id, output_payload)` only receives built public output.
- [x] Ensure checkpoint `variable_snapshot` only stores public-only variable pool.

### Task 4: Update observability persistence

- [x] Persist `debug_payload` on node run or the selected debug artifact path.
- [x] Keep existing metrics ledgers reading from `metrics_payload`.
- [x] Ensure `flow_run.output_payload` only stores final business output, not variable cache.

### Task 5: Verification

- [x] `cargo test -p orchestration-runtime llm -- --test-threads=1`
- [x] `cargo test -p orchestration-runtime execution_engine -- --test-threads=1`
- [x] `cargo test -p control-plane orchestration_runtime -- --test-threads=1`
- [x] `cargo test -p api-server application_runtime_routes_start_node_preview_and_query_logs -- --test-threads=1`
- [x] `cargo test -p storage-postgres orchestration_runtime_repository_persists_waiting_human_checkpoint -- --test-threads=1`
- [x] `cargo fmt -p api-server -p control-plane -p orchestration-runtime -p storage-postgres -- --check`
- [x] `git diff --check`

Run:

```bash
cargo test -p orchestration-runtime llm -- --test-threads=1
cargo test -p orchestration-runtime execution_engine -- --test-threads=1
cargo test -p control-plane orchestration_runtime -- --test-threads=1
```

Expected:

- LLM public output contains `text` and declared structured output only.
- Variable pool never contains `usage`, `route`, `attempts`, `finish_reason`, `provider_metadata`, `tool_calls`, `mcp_calls`, `error`, or `__*`.
- Existing usage and attempt ledgers still receive metrics facts.

## Stop Conditions

- A downstream feature requires reasoning or usage as a public variable.
- Node executors need to write directly to `variable_pool`.
- Runtime must silently drop unknown output keys instead of rejecting and recording contract errors.
