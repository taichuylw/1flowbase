# Examples

## Vague Feature Request

User asks: "Improve the settings page workflow."

Response shape:

```md
现状
- The request changes product workflow but does not define the primary user action or success criteria.
- Settings already has project rules about shared surfaces and compact operational UI.

方向
- First frame the goal, affected settings areas, primary user path, and success criteria.

风险收益
- Benefit: avoids redesigning unrelated settings surfaces.
- Risk: if we skip framing, UI work may change navigation or state ownership without approval.

建议
- I recommend a short discussion brief first, then hand off to frontend-development after approval.
```

## Cross-Domain Architecture Request

User asks: "Centralize defaults and fix frontend/backend drift."

Response shape:

```md
现状
- This touches frontend fallback, backend defaults, runtime contracts, persisted data, and possible migration behavior.
- These are not one concept even if they are all called defaults.

方向
- Build a domain matrix before designing APIs or migrations.
- Compare conservative / balanced / aggressive options.

风险收益
- Benefit: protects user-owned historical data and runtime contracts.
- Risk: broader cleanup may introduce silent behavior changes.

建议
- I recommend the balanced option only after the matrix proves source of truth and historical impact.
```
