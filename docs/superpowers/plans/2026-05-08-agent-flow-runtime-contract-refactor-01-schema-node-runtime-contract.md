# Agent Flow Runtime Contract Refactor 01 Schema And Node Runtime Contract Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. Update the index plan after each completed task.

**Goal:** Introduce schema v2 and a shared Node Runtime UI Contract so authoring, runtime, plugins, and frontend surfaces stop defining node behavior independently.

**Architecture:** Add explicit schema v2 constants and contract types in `@1flowbase/flow-schema`. Built-in node definitions consume those contracts through a thin adapter so later plans can migrate UI and runtime without guessing output shape from legacy node documents.

**Tech Stack:** TypeScript, `@1flowbase/flow-schema`, React 19, Vitest.

---

## Files

- Modify: `web/packages/flow-schema/src/index.ts`
- Modify: `web/packages/flow-schema/src/_tests` or existing package test location
- Modify: `web/app/src/features/agent-flow/lib/node-definitions/types.ts`
- Modify: `web/app/src/features/agent-flow/lib/node-definitions/index.ts`
- Modify: `web/app/src/features/agent-flow/lib/node-definitions/nodes/**`
- Modify: `web/app/src/features/agent-flow/lib/document/node-factory.ts`
- Test: `web/app/src/features/agent-flow/_tests/node-schema-registry.test.tsx`
- Test: `web/app/src/features/agent-flow/_tests/validate-document.test.ts`

## Tasks

### Task 1: Add schema v2 constants and contract types

- [x] Add `FLOW_SCHEMA_VERSION = '1flowbase.flow/v2'`.
- [x] Add `NODE_CONTRIBUTION_SCHEMA_VERSION = '1flowbase.node-contribution/v2'`.
- [x] Add typed contract sections: `meta`, `defaults`, `ports`, `card`, `panel`, `runtime`, `policies`.
- [x] Add public output key validation helper that rejects `metadata`, `usage`, `debug`, `error`, and `__*`.
- [x] Add tests proving invalid public output keys are rejected.

### Task 2: Remove legacy LLM public outputs

- [x] Replace default LLM outputs with `text` only.
- [x] Add optional `structured_output` only when the LLM node config explicitly enables structured output.
- [x] Update tests that currently expect `reasoning_content` or `usage` as default LLM outputs.
- [x] Keep reasoning and usage available only to runtime/trace contracts in later plans.

### Task 3: Add built-in node contracts

- [x] Define built-in contracts for Start, LLM, Answer, Template Transform, HTTP, Plugin Node placeholder, Human Input, and fixed Data Model nodes.
- [x] Ensure Start public variables derive from `config.input_fields`, `query`, and `files`, not from `node.outputs`.
- [x] Ensure Answer exposes public output `answer` only.
- [x] Ensure Template Transform exposes public output `text` only.
- [x] Ensure Data Model delete declares both `deleted_id` and `affected_count`.

### Task 4: Adapt node factory and validation

- [x] Make `node-factory.ts` create nodes from the contract default document.
- [x] Make document validation reject v1-only or unknown public output fields during development.
- [x] Keep the destructive baseline: no v1 compatibility mapper in this plan.

### Task 5: Verification

- [x] Run scoped app verification: `pnpm --dir web/app test -- node-schema-registry validate-document document-transforms`.
- [x] Run schema package verification: `pnpm --dir web/packages/flow-schema test`.
- [x] Confirm expected outcomes for contract-driven defaults, LLM public outputs, and Data Model delete outputs.

Run:

```bash
pnpm --dir web/app test -- node-schema-registry validate-document document-transforms
pnpm --dir web/packages/flow-schema test
```

Expected:

- Built-in node defaults are contract-driven.
- LLM default outputs no longer include `reasoning_content` or `usage`.
- Data Model delete output includes `affected_count`.

## Stop Conditions

- Runtime schema v1 compatibility is required.
- Built-in nodes need runtime sample values to infer outputs.
- Plugin-provided renderer code is required before host renderer allowlist exists.
