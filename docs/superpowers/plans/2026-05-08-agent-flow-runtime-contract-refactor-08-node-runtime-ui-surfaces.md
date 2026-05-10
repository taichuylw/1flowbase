# Agent Flow Runtime Contract Refactor 08 Node Runtime UI Surfaces Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. Update the index plan after each completed task.

**Goal:** Move Agent Flow Node Picker, Factory, Card, Inspector, Detail Panel, Trace, and Variables surfaces to the shared Node Runtime UI Contract.

**Architecture:** Frontend surfaces read one contract shape instead of local special cases. The Variables tab shows public output objects, Trace detail shows resolved inputs/outputs/metrics/errors/debug evidence, and selectors keep stable `node.id + key` identity while display uses `node.alias/key`.

**Tech Stack:** React 19, TypeScript, Ant Design 5, TanStack Query, Vitest.

---

## Files

- Modify: `web/app/src/features/agent-flow/lib/node-definitions/**`
- Modify: `web/app/src/features/agent-flow/lib/document/node-factory.ts`
- Modify: `web/app/src/features/agent-flow/components/nodes/AgentFlowNodeCard.tsx`
- Modify: `web/app/src/features/agent-flow/components/detail/**`
- Modify: `web/app/src/features/agent-flow/components/editor/AgentFlowCanvasFrame.tsx`
- Modify: `web/app/src/features/agent-flow/lib/debug-console/**`
- Modify: `web/app/src/features/agent-flow/schema/**`
- Test: `web/app/src/features/agent-flow/_tests/**`

## Tasks

### Task 1: Contract-driven Node Picker and Factory

- [x] Build node picker items from contract `meta`, `ports`, and `defaults`.
- [x] Create node documents from contract defaults.
- [x] Reject unavailable plugin contribution nodes instead of creating uncompilable drafts.

### Task 2: Contract-driven Card and Inspector

- [x] Render node cards from contract `card` fields.
- [x] Render inspector forms from contract `panel_schema` and host renderer allowlist.
- [x] Keep config editing separate from runtime variables.
- [x] Remove output-title-based identity from UI tests.

### Task 3: Runtime panels

- [x] Trace Inputs displays `input_payload`.
- [x] Trace Outputs displays public `output_payload`.
- [x] Trace Metrics displays usage, duration, route, attempt, finish reason.
- [x] Trace Error displays `error_payload`.
- [x] Trace Debug displays refs and artifact metadata.
- [x] No non-output payload appears in Variable Cache.

### Task 4: Variables tab and preview UX

- [x] Display Variable Cache object-level entries.
- [x] Show `node.alias/key` as primary label.
- [x] Show output title as secondary helper text only.
- [x] Show truncation and full-load affordance when plan 05 metadata exists.
- [x] Keep Run Context / Environment / Session separate from Variable Cache.

### Task 5: Verification

Run:

```bash
pnpm --dir web/app test -- agent-flow node-schema-registry variable-groups use-agent-flow-debug-session
```

Expected:

- UI surfaces consume the same contract shape.
- `input_payload`, metrics, error, and debug evidence are inspectable but not variable entries.
- Data Model and plugin nodes display public outputs consistently.

Status:

- [x] Passed `pnpm --dir web/app test -- agent-flow node-schema-registry variable-groups use-agent-flow-debug-session` on 2026-05-08. Result: 41 files, 267 tests passed.
- [x] Passed `cargo fmt --all -- --check` from `api/` on 2026-05-08.
- [x] Passed `cargo test -p control-plane live_debug_persists_llm_debug_payload_without_polluting_public_outputs -- --test-threads=1` on 2026-05-08. Result: 1 passed.
- [ ] `pnpm --dir web/app build` remains blocked by pre-existing `app-shell/_tests` type errors unrelated to Agent Flow: missing `ConsoleSessionActor.id`, null string assignment, and unsafe `.props` access in settings chrome menu tests.

Implementation Notes:

- Added complete builtin `NodeRuntimeUiContract` coverage for picker metadata, factory defaults, card metadata, panel sections, runtime displays, and Data Model fixed output contracts.
- Node picker now derives builtin options from contract metadata and ports; factory rejects disabled plugin contributions before creating drafts.
- Last Run and Debug Trace now render input, public output, metrics, error, and debug payloads as separate JSON blocks with artifact full-load support.
- Variable Cache, live preview cache, and run-detail Node Outputs keep object-level public output entries, use `node.alias/key` as the primary label, and show contract or document output title as secondary helper text.
- Frontend debug stream now accepts optional `debug_payload`; backend node-finished stream events include `debug_payload` for live Trace Debug.
- Independent read-only QA initially found the live/durable Variable Cache still displayed whole-node cache objects; this was fixed by mapping preview cache with document node metadata.

## Stop Conditions

- UI must infer output schema from runtime samples.
- Plugin nodes must inject React panels.
- Variables tab must recursively flatten objects as separate variable identities.
