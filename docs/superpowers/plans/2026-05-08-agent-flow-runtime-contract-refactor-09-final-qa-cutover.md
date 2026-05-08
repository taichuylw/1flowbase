# Agent Flow Runtime Contract Refactor 09 Final QA And Cutover Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. Update the index plan after each completed task.

**Goal:** Verify the full runtime contract refactor, remove development-only legacy paths, update generated contracts, and prepare the branch for delivery.

**Architecture:** Run targeted frontend/backend checks first, then use `qa-evaluation` to audit the final behavior against the source spec and the child-plan acceptance matrix. Heavy Rust consistency gates run in GitHub Actions by pushing the current branch unless the user explicitly requests local execution.

**Tech Stack:** Rust 2021, Cargo tests, Node scripts, Vitest, OpenAPI generation, qa-evaluation skill.

---

## Files

- Modify: `docs/superpowers/plans/2026-05-08-agent-flow-runtime-contract-refactor-index.md`
- Modify: active child plans as tasks complete
- Modify: generated OpenAPI/API client files if API shapes changed
- Create: `tmp/test-governance/*` for warning or coverage artifacts only when commands produce them

## Tasks

### Task 1: Source spec coverage audit

- [x] Use `qa-evaluation`.
- [x] Check source spec sections for persistence, cache, variable display, LLM streaming, plugin integration, and Data Model nodes.
- [x] Mark gaps against child plans 01-08.
- [x] Fix missing implementation or document an explicit stop condition before delivery.

### Task 2: Frontend verification

- [x] Run targeted Agent Flow tests:

```bash
pnpm --dir web/app test -- agent-flow
```

- [x] Run flow schema tests:

```bash
pnpm --dir web/packages/flow-schema test
```

- [x] If API client changed, run API client tests:

```bash
pnpm --dir web/packages/api-client test
```

### Task 3: Backend targeted verification

- [x] Run targeted runtime tests:

```bash
cargo test -p orchestration-runtime llm -- --test-threads=1
cargo test -p orchestration-runtime data_model -- --test-threads=1
cargo test -p control-plane orchestration_runtime -- --test-threads=1
cargo test -p api-server application_runtime -- --test-threads=1
```

- [x] Do not run local heavy Rust consistency gates unless explicitly requested.

### Task 4: OpenAPI and generated client

- [x] Regenerate or verify OpenAPI if runtime API shapes changed.
- [x] Regenerate API client if OpenAPI changed.
- [x] Run:

```bash
node scripts/node/verify-openapi.js
```

### Task 5: Legacy path cleanup

- [x] Remove schema v1 compatibility paths introduced only as temporary scaffolding during child plans.
- [x] Remove title-based variable identity tests.
- [x] Remove cache reconstruction from non-Start `input_payload`.
- [x] Remove LLM public `usage`, `reasoning_content`, route, attempts, error, debug, and `__*` expectations.

### Task 6: Final status update

- [x] Update all completed child plan checkboxes.
- [x] Update the index acceptance mapping if any file or verification command moved.
- [x] Record skipped heavy local gates and the GitHub Actions expectation in the final delivery note.

## Completion Notes

- Status: completed on 2026-05-08.
- QA gaps fixed during cutover:
  - Runtime resume payloads are scoped to the waiting node and validated against that node's compiled public outputs.
  - Schema v2 is now the accepted runtime baseline; legacy `1flowbase.flow/v1` documents are rejected.
  - Start nodes reject legacy public `outputs`; Start public input keys no longer come from stale `outputs`.
  - LLM runtime ignores legacy `user_prompt` / `system_prompt` bindings and requires `prompt_messages`.
  - Variable Picker no longer exposes legacy LLM `usage` / `reasoning_content` public outputs.
  - Plugin manifest infra denylist includes storage and rate-limit host infra contracts.
  - Debug artifact preview assertions now use the full-load endpoint before checking original payloads.
  - OpenAPI includes the runtime debug artifact full-load route and is covered by `scripts/node/verify-openapi.js`.
- Generated API client regeneration was not required after OpenAPI verification; API client tests passed against the current client.
- Heavy local Rust consistency gates were intentionally not run; run them in GitHub Actions after pushing unless explicitly requested locally.

## Verification Evidence

- `cargo fmt --all -- --check`
- `cargo test -p orchestration-runtime compile_ -- --test-threads=1`
- `cargo test -p orchestration-runtime resume_flow_debug_run -- --test-threads=1`
- `cargo test -p orchestration-runtime llm_runtime_fails_before_provider_when_prompt_messages_are_empty -- --test-threads=1`
- `cargo test -p orchestration-runtime llm -- --test-threads=1`
- `cargo test -p orchestration-runtime data_model -- --test-threads=1`
- `cargo test -p control-plane orchestration_runtime -- --test-threads=1`
- `cargo test -p api-server start_public_input_keys_ignore_legacy_start_outputs -- --test-threads=1`
- `cargo test -p api-server application_runtime -- --test-threads=1`
- `cargo test -p plugin-framework plugin_manifest_v1_rejects -- --test-threads=1`
- `cargo test -p domain default_flow_document_uses_v2_prompt_messages_contract -- --test-threads=1`
- `pnpm --dir web/app test -- start-node-variables validate-document node-schema-registry`
- `pnpm --dir web/app test -- agent-flow`
- `node scripts/node/run-frontend-vitest.js run src/routes/_tests/application-shell-routing.test.tsx`
- `pnpm --dir web/packages/flow-schema test`
- `pnpm --dir web/packages/api-client test`
- `node scripts/node/test-scripts.js verify-openapi`
- `node scripts/node/verify-openapi.js`

## Verification Evidence Expected

- Variable Picker and Variables tab agree on public outputs.
- Resolved inputs are visible in trace/audit but not Variable Cache.
- LLM streaming can replay without duplicate deltas.
- Debug artifacts show preview/ref/full-load behavior.
- Plugin contribution v2 rejects invalid renderers and invalid public outputs.
- Data Model write same-run replay does not duplicate side effects.

## Stop Conditions

- Any child plan remains partially complete without an explicit follow-up plan.
- Generated API client and backend OpenAPI disagree.
- QA finds a public-output contamination path.
- GitHub Actions heavy gate fails for a runtime/data consistency regression.
