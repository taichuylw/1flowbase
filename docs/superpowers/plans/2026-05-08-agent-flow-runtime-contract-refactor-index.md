# Agent Flow Runtime Contract Refactor 1+n Implementation Plan Index

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. This is an index plan; execute child plans in order and update checkbox state in both this file and the active child plan after each completed task.

**Goal:** Implement the Agent Flow runtime contract refactor from public-only outputs through debug snapshot, streaming replay, plugin contribution v2, and Data Model side-effect safety.

**Architecture:** The refactor starts by freezing schema v2 and Node Runtime UI Contract, then moves all runtime writes through a shared payload builder before touching debug cache or UI. Streaming, artifact offload, plugin contribution v2, and Data Model writes are separate child plans so each slice has a stable boundary and can be verified independently.

**Tech Stack:** Rust 2021, Axum, Tokio, SQLx/PostgreSQL, Serde, UUID v7, React 19, TypeScript, Ant Design 5, TanStack Query, Vitest.

---

## Source Spec

- `docs/superpowers/specs/1flowbase/2026-05-07-agent-flow-variable-linker-runtime-contract-design.md`

## Planning Rules

- Plan language is English.
- Execute in the current repository; do not create a git worktree for this plan set.
- Keep warning and coverage artifacts under `tmp/test-governance/`.
- Use `qa-evaluation` for QA, acceptance, regression, or delivery review stages.
- Heavy Rust consistency gates should run in GitHub Actions unless the user explicitly asks to run them locally.
- After each completed child-plan task, update both the child plan and this index.
- If using subagents during implementation, run only one independent implementation subagent at a time.

## Existing Local Changes To Preserve

- `docs/superpowers/specs/1flowbase/2026-05-07-agent-flow-variable-linker-runtime-contract-design.md`
- `.memory/project-memory/2026-05-07-agentflow-runtime-contract-spec-supplemented.md`
- `.memory/reference-memory/source-reference.md`

Do not revert or reformat these files unless the active child plan explicitly edits them.

## Child Plans

- [x] **01 Schema And Node Runtime Contract:** `docs/superpowers/plans/2026-05-08-agent-flow-runtime-contract-refactor-01-schema-node-runtime-contract.md`
  - Introduces schema v2.
  - Adds first-class Node Runtime UI Contract types.
  - Removes legacy LLM public outputs from authoring defaults.

- [x] **02 Payload Builder And LLM Output Contract:** `docs/superpowers/plans/2026-05-08-agent-flow-runtime-contract-refactor-02-payload-builder-llm-output-contract.md`
  - Adds shared public output filtering and payload splitting.
  - Makes LLM output public-only.
  - Keeps metrics, error, provider metadata, route, attempts, tool calls, and raw refs outside variable pool.

- [x] **03 Variable Linker, Debug Cache, And Snapshot:** `docs/superpowers/plans/2026-05-08-agent-flow-runtime-contract-refactor-03-variable-linker-debug-cache-snapshot.md`
  - Rebuilds Variable Picker and Debug Variable Cache around public outputs.
  - Keeps resolved `input_payload` in Trace detail / node run audit only.
  - Adds snapshot isolation and stable merge semantics.
  - Completed with targeted frontend/API/control-plane/storage verification and serial spec/code-quality reviews.

- [x] **04 RuntimeEventStream Replay And Durable Debug Events:** `docs/superpowers/plans/2026-05-08-agent-flow-runtime-contract-refactor-04-runtime-event-stream-replay-debug-events.md`
  - Adds event envelope DTOs.
  - Adds cursor replay and frontend idempotent delta consumption.
  - Keeps streaming deltas out of variable pool.
  - Completed with targeted frontend/API/control-plane verification and serial review re-check.

- [x] **05 Debug Artifact Offload And Full Load:** `docs/superpowers/plans/2026-05-08-agent-flow-runtime-contract-refactor-05-debug-artifact-offload-full-load.md`
  - Adds inline budgets.
  - Adds artifact refs, truncation preview, full-load API, and retention/GC state.
  - Prevents truncated payloads from masquerading as complete business output.
  - Completed with targeted storage/API/control-plane/frontend verification and serial review re-check.

- [x] **06 Plugin Contribution V2:** `docs/superpowers/plans/2026-05-08-agent-flow-runtime-contract-refactor-06-plugin-contribution-v2.md`
  - Locks plugin identity, package identity, contribution checksum, renderer allowlist, and output schema snapshot.
  - Rejects unknown renderers, infra contracts, and non-public output keys.
  - Stops runtime from deriving old node behavior from current plugin installation state.
  - Completed with targeted plugin-framework/orchestration/storage/control-plane/API/frontend verification and serial review re-check.

- [x] **07 Data Model Side Effects:** `docs/superpowers/plans/2026-05-08-agent-flow-runtime-contract-refactor-07-data-model-side-effects.md`
  - Adds fixed Data Model output parity.
  - Adds side-effect policy, receipt, audit/outbox semantics, and same-run replay idempotency.
  - Ensures write nodes cannot be silently repeated by checkpoint replay.

- [x] **08 Node Runtime UI Surfaces:** `docs/superpowers/plans/2026-05-08-agent-flow-runtime-contract-refactor-08-node-runtime-ui-surfaces.md`
  - Moves Node Picker, Factory, Card, Inspector, Detail Panel, Trace, and Variables surfaces to the Node Runtime UI Contract.
  - Makes display identity `node.alias/key`.
  - Keeps object-level variable cache display and schema-driven deep field expansion.
  - Completed with targeted frontend/control-plane verification and independent read-only QA re-check.

- [x] **09 Final QA And Cutover:** `docs/superpowers/plans/2026-05-08-agent-flow-runtime-contract-refactor-09-final-qa-cutover.md`
  - Runs targeted verification and QA review.
  - Updates OpenAPI/client generation and plan status.
  - Closes the destructive baseline by removing legacy compatibility paths.
  - Completed with qa-evaluation review, legacy contract cleanup, OpenAPI verification, targeted frontend/backend gates, and heavy Rust consistency gates deferred to GitHub Actions.

## Required Execution Order

1. Run plan 01 first because every later plan depends on schema v2 and contract vocabulary.
2. Run plan 02 second because variable cache, snapshot, and UI must consume payload-builder output, not executor raw output.
3. Run plan 03 third because it depends on public-only output payloads.
4. Run plan 04 after plan 02 so stream final events and payload builder share event/output semantics.
5. Run plan 05 after plans 02-04 so artifact refs can be attached consistently to payloads and durable debug events.
6. Run plan 06 after plan 01 and before plugin executor work expands.
7. Run plan 07 after plan 02 so Data Model outputs also pass through payload builder.
8. Run plan 08 after plans 01-07 so UI renders stable backend and schema contracts.
9. Run plan 09 last.

## Acceptance Mapping

| Requirement | Child Plans |
| --- | --- |
| Schema v2 replaces the old authoring/runtime contract | 01, 09 |
| LLM public output only exposes `text` and optional `structured_output` | 01, 02, 08 |
| `usage`, route, attempts, finish reason, provider metadata, tool calls, and raw refs do not enter public output | 02, 04, 05, 09 |
| `input_payload` is persisted for trace/audit/full-load but not variable cache | 03, 05, 08 |
| Variable Picker and Variables tab share the same public-output definition | 03, 08 |
| Debug snapshot is isolated by workspace, actor, draft, document hash, schema, debug session, and run scope | 03, 09 |
| RuntimeEventStream supports cursor replay and frontend idempotent deltas | 04, 09 |
| Large objects use preview/ref/full-load and retention/GC | 05, 09 |
| Plugin contribution v2 locks immutable identity and output schema snapshot | 06, 08, 09 |
| Data Model delete outputs `deleted_id` and `affected_count` | 07, 08, 09 |
| Data Model write has same-run replay idempotency and side-effect receipt | 07, 09 |

## Global Verification Commands

Targeted frontend:

```bash
pnpm --dir web/app test -- agent-flow
pnpm --dir web/packages/flow-schema test
```

Targeted backend:

```bash
cargo test -p orchestration-runtime llm -- --test-threads=1
cargo test -p orchestration-runtime data_model -- --test-threads=1
cargo test -p control-plane orchestration_runtime -- --test-threads=1
cargo test -p api-server application_runtime -- --test-threads=1
```

OpenAPI and generated client:

```bash
node scripts/node/verify-openapi.js
pnpm --dir web/packages/api-client test
```

Run heavy Rust consistency gates in GitHub Actions by pushing the current branch unless the user explicitly requests local heavy gates.

## Stop Conditions

Pause implementation and return to design if any active child plan requires:

- Cross-debug-run business idempotency for Data Model writes.
- Runtime compatibility with schema v1 documents.
- Plugin-provided React panels.
- RuntimeExtension or CapabilityPlugin direct access to cache-store, lock, queue, event bus, or object storage.
- Variable Picker deriving deep fields from runtime samples or offloaded values.
- Provider reasoning content becoming downstream-referenceable public output.
