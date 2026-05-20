# Domain Matrix

Use this reference when the task touches defaults, contracts, schema, state, permissions, migration, historical data, runtime behavior, or user-owned content.

## Required Columns

| Object / field / behavior | Owner | Source of truth | Persisted? | User editable? | Runtime contract? | Historical data impact | Required evidence | Unacceptable failure mode |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
|  |  |  |  |  |  |  |  |  |

## Rules

- Fill the matrix before designing APIs, services, enums, directory layouts, migrations, or upgrade commands.
- Mark unknowns as `unknown`; do not convert them into design conclusions.
- If a row has user-owned content or historical data impact, require explicit user approval before implementation.
- If source of truth is unclear, stop and ask for a decision instead of adding compatibility code.

## Common Rows

- Frontend display fallback
- Backend default value
- Persisted user setting
- Runtime contract
- Database migration
- Audit / preview / rollback behavior
- Permission or policy decision
- Generated or imported system content
