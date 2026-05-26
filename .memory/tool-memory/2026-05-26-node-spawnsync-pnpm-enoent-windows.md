---
memory_type: tool
topic: Node spawnSync pnpm ENOENT on Windows Codex shell
summary: On this Windows workspace, Node scripts that spawn bare `pnpm` can fail with `spawnSync pnpm ENOENT`; run the equivalent command with `pnpm.CMD` when the script exposes the underlying command.
keywords:
  - node
  - pnpm
  - spawnSync
  - ENOENT
  - Windows
match_when:
  - A repository Node verification script fails with `spawnSync pnpm ENOENT`.
  - `where.exe pnpm.CMD` succeeds but a Node child process cannot spawn bare `pnpm`.
created_at: 2026-05-26 07
updated_at: 2026-05-26 07
last_verified_at: 2026-05-26 07
decision_policy: reference_on_failure
scope:
  - scripts/node/test-contracts.js
  - pnpm.CMD
  - Windows PowerShell
---

# Node spawnSync pnpm ENOENT on Windows Codex shell

## Time

`2026-05-26 07`

## Failure

Running:

```powershell
node scripts/node/test-contracts.js
```

failed with:

```text
[1flowbase-test-contracts] spawnSync pnpm ENOENT
```

## Trigger

The script builds a managed command whose executable is the bare string `pnpm`.
In this Windows environment, `pnpm.CMD` is available and works from PowerShell,
but the Node child-process spawn path did not resolve bare `pnpm`.

## Fix

When the script's underlying command is visible, run the equivalent command with
`pnpm.CMD`. For the contracts gate this was:

```powershell
pnpm.CMD --dir web/app exec vitest run src/features/settings/api/_tests/settings-api.test.ts src/style-boundary/_tests/registry.test.tsx src/features/agent-flow/_tests/llm-model-provider-field.test.tsx
```

## Verification

The equivalent `pnpm.CMD` command passed on `2026-05-26 07` with 3 test files
and 33 tests passing.
