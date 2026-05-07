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

- [ ] Define `RawNodeExecutionResult` with executor output, metrics facts, error facts, debug facts, and provider events.
- [ ] Define `BuiltNodePayloads` with `output_payload`, `metrics_payload`, `error_payload`, and `debug_payload`.
- [ ] Define `PublicOutputContract` from compiled node outputs.
- [ ] Reject unknown public output keys unless the contract explicitly allows structured expansion.
- [ ] Add tests for unknown key rejection and payload bucket exclusivity.

### Task 2: Split LLM raw result

- [ ] Change LLM execution to produce raw fields instead of a fully public `output_payload`.
- [ ] Keep final answer text as public `text`.
- [ ] Keep optional `structured_output` only when contract declares it.
- [ ] Move `usage`, `finish_reason`, `route`, `attempts`, `provider_instance_id`, `provider_code`, `protocol`, `model`, and event count to `metrics_payload`.
- [ ] Move provider metadata, tool calls, MCP calls, raw response refs, context projection refs, and attempt refs to `debug_payload`.
- [ ] Move provider errors to `error_payload`.
- [ ] Ensure failure after first token does not write normal public output to variable pool.

### Task 3: Use payload builder in both execution paths

- [ ] Use the builder in `execution_engine.rs` for non-stream debug execution.
- [ ] Use the same builder in live debug continuation.
- [ ] Ensure `variable_pool.insert(node_id, output_payload)` only receives built public output.
- [ ] Ensure checkpoint `variable_snapshot` only stores public-only variable pool.

### Task 4: Update observability persistence

- [ ] Persist `debug_payload` on node run or the selected debug artifact path.
- [ ] Keep existing metrics ledgers reading from `metrics_payload`.
- [ ] Ensure `flow_run.output_payload` only stores final business output, not variable cache.

### Task 5: Verification

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
