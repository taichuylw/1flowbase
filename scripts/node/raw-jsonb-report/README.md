# Raw JSONB Boundary Report

## Goal

Keep large runtime JSONB payloads in PostgreSQL while making read boundaries executable:

- `summary`: bounded fields derived from payload or stable metadata. List and overview APIs prefer these.
- `preview`: truncated or artifact-backed display value with explicit size/ref metadata.
- `raw`: complete JSONB truth. Raw reads must use primary-key, run-scope, or detail entrypoints.

## Evidence

Run:

```bash
node scripts/node/tooling.js raw-jsonb-report
```

The command writes:

- `tmp/test-governance/raw-jsonb-report.json`
- `tmp/test-governance/raw-jsonb-report.md`

## Boundary Rules

- List and tree summary entrypoints must not select configured raw payload columns without a run, node, trace-node, or detail scope.
- If a list UI needs payload-derived text, it must use a summary/projection field or an explicit preview contract.
- Detail and run-scope readers may return raw payloads when the scope columns in `config.json` prove the boundary.
- JSONB stays queryable in PostgreSQL; this report only guards application read paths.

## Stop Conditions

- A configured raw field appears in an unbounded list read.
- A raw field has no declared read contract or scope protection.
- A list endpoint needs full raw JSONB; escalate to #1021 or #1017 before widening the contract.
