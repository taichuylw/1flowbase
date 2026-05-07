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

- [ ] Use `qa-evaluation`.
- [ ] Check source spec sections for persistence, cache, variable display, LLM streaming, plugin integration, and Data Model nodes.
- [ ] Mark gaps against child plans 01-08.
- [ ] Fix missing implementation or document an explicit stop condition before delivery.

### Task 2: Frontend verification

- [ ] Run targeted Agent Flow tests:

```bash
pnpm --dir web/app test -- agent-flow
```

- [ ] Run flow schema tests:

```bash
pnpm --dir web/packages/flow-schema test
```

- [ ] If API client changed, run API client tests:

```bash
pnpm --dir web/packages/api-client test
```

### Task 3: Backend targeted verification

- [ ] Run targeted runtime tests:

```bash
cargo test -p orchestration-runtime llm -- --test-threads=1
cargo test -p orchestration-runtime data_model -- --test-threads=1
cargo test -p control-plane orchestration_runtime -- --test-threads=1
cargo test -p api-server application_runtime -- --test-threads=1
```

- [ ] Do not run local heavy Rust consistency gates unless explicitly requested.

### Task 4: OpenAPI and generated client

- [ ] Regenerate or verify OpenAPI if runtime API shapes changed.
- [ ] Regenerate API client if OpenAPI changed.
- [ ] Run:

```bash
node scripts/node/verify-openapi.js
```

### Task 5: Legacy path cleanup

- [ ] Remove schema v1 compatibility paths introduced only as temporary scaffolding during child plans.
- [ ] Remove title-based variable identity tests.
- [ ] Remove cache reconstruction from non-Start `input_payload`.
- [ ] Remove LLM public `usage`, `reasoning_content`, route, attempts, error, debug, and `__*` expectations.

### Task 6: Final status update

- [ ] Update all completed child plan checkboxes.
- [ ] Update the index acceptance mapping if any file or verification command moved.
- [ ] Record skipped heavy local gates and the GitHub Actions expectation in the final delivery note.

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
