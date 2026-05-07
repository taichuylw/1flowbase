# Agent Flow Runtime Contract Refactor 04 RuntimeEventStream Replay Debug Events Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. Update the index plan after each completed task.

**Goal:** Make RuntimeEventStream reconnectable and idempotent while keeping LLM text/reasoning deltas out of variable pool.

**Architecture:** Expose a full event envelope to clients and persist durable debug events with cursor evidence. The frontend applies stream deltas by `event_id/sequence/delta_index` so reconnects cannot duplicate text or reasoning content.

**Tech Stack:** Rust 2021, Axum SSE, Tokio, Serde, TypeScript fetch streaming, Vitest.

---

## Files

- Modify: `api/crates/control-plane/src/ports/infrastructure.rs`
- Modify: `api/crates/control-plane/src/orchestration_runtime/debug_stream_events.rs`
- Modify: `api/crates/control-plane/src/orchestration_runtime/debug_event_persister.rs`
- Modify: `api/apps/api-server/src/routes/applications/debug_run_stream.rs`
- Modify: `api/apps/api-server/src/routes/applications/application_runtime.rs`
- Modify: `web/packages/api-client/src/console-application-runtime.ts`
- Modify: `web/app/src/features/agent-flow/lib/debug-console/stream-events.ts`
- Modify: `web/app/src/features/agent-flow/lib/debug-console/run-detail-mapper.ts`
- Modify: `web/app/src/features/agent-flow/hooks/runtime/useAgentFlowDebugSession.ts`
- Test: `api/apps/api-server/src/_tests/runtime_event_stream_tests.rs`
- Test: `web/app/src/features/agent-flow/_tests/debug-console/use-agent-flow-debug-session-stream.test.tsx`

## Tasks

### Task 1: Define client-visible event envelope

- [ ] Add shared DTO fields: `event_id`, `run_id`, `node_run_id`, `event_type`, `sequence`, `created_at`, `payload`.
- [ ] Add delta fields: `delta_index`, `content_type`, and `text`.
- [ ] Keep terminal events explicit: `flow_finished`, `flow_failed`, `flow_cancelled`, `waiting_human`, `waiting_callback`.

### Task 2: Add cursor replay contract

- [ ] Accept `last_event_id` or `from_sequence` on stream subscribe.
- [ ] Set SSE `id` to `event_id` or a documented cursor that round-trips to sequence.
- [ ] Emit `replay_expired` with a typed payload when local replay cannot satisfy the cursor.
- [ ] Ensure durable-required events do not rely only on live buffer retention.

### Task 3: Durable debug event read model

- [ ] Persist text/reasoning deltas with `node_run_id`, event type, sequence range, content type, truncation/ref fields, and artifact refs.
- [ ] Keep coalesced read model available for run detail.
- [ ] Do not use durable debug events as variable pool source.

### Task 4: Frontend idempotent stream apply

- [ ] Parse event envelope in `api-client`.
- [ ] Track applied `event_id` or `(run_id, sequence)` per running message.
- [ ] Append text/reasoning deltas only once.
- [ ] Key trace rows by `node_run_id` first, falling back to `node_id` only for legacy events during development.
- [ ] Keep reasoning in trace/debug display, not public output.

### Task 5: Verification

Run:

```bash
cargo test -p api-server runtime_event_stream -- --test-threads=1
cargo test -p control-plane orchestration_runtime_runtime_events -- --test-threads=1
pnpm --dir web/app test -- use-agent-flow-debug-session-stream run-detail-mapper
```

Expected:

- Replayed events do not duplicate assistant text.
- `reasoning_delta` is visible in debug display but not public output.
- Terminal events close the stream or enter an explicit waiting state.

## Stop Conditions

- Stream reconnect must replay from durable storage before the durable read model exists.
- UI needs provider raw events as public variables.
- RuntimeEventStream provider must be changed to Redis/NATS/Kafka in this same child plan.
