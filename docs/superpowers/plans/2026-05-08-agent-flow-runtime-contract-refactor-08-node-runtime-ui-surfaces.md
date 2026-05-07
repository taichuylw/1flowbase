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

- [ ] Build node picker items from contract `meta`, `ports`, and `defaults`.
- [ ] Create node documents from contract defaults.
- [ ] Reject unavailable plugin contribution nodes instead of creating uncompilable drafts.

### Task 2: Contract-driven Card and Inspector

- [ ] Render node cards from contract `card` fields.
- [ ] Render inspector forms from contract `panel_schema` and host renderer allowlist.
- [ ] Keep config editing separate from runtime variables.
- [ ] Remove output-title-based identity from UI tests.

### Task 3: Runtime panels

- [ ] Trace Inputs displays `input_payload`.
- [ ] Trace Outputs displays public `output_payload`.
- [ ] Trace Metrics displays usage, duration, route, attempt, finish reason.
- [ ] Trace Error displays `error_payload`.
- [ ] Trace Debug displays refs and artifact metadata.
- [ ] No non-output payload appears in Variable Cache.

### Task 4: Variables tab and preview UX

- [ ] Display Variable Cache object-level entries.
- [ ] Show `node.alias/key` as primary label.
- [ ] Show output title as secondary helper text only.
- [ ] Show truncation and full-load affordance when plan 05 metadata exists.
- [ ] Keep Run Context / Environment / Session separate from Variable Cache.

### Task 5: Verification

Run:

```bash
pnpm --dir web/app test -- agent-flow node-schema-registry variable-groups use-agent-flow-debug-session
```

Expected:

- UI surfaces consume the same contract shape.
- `input_payload`, metrics, error, and debug evidence are inspectable but not variable entries.
- Data Model and plugin nodes display public outputs consistently.

## Stop Conditions

- UI must infer output schema from runtime samples.
- Plugin nodes must inject React panels.
- Variables tab must recursively flatten objects as separate variable identities.
