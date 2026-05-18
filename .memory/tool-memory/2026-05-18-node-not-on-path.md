---
tool: node
summary: Codex shell may not have node on PATH; project scripts can run by prepending the installed Node 24 bin directory.
decision_policy: reference_on_failure
last_verified: "2026-05-18 23"
---

# Node not on PATH in Codex shell

On `2026-05-18 23`, running project Node scripts from the Codex shell failed with:

```text
/bin/bash: line 1: node: command not found
```

The repository frontend requires Node `>=24.0.0` via `web/package.json`. Use the installed Node 24 runtime by prepending its bin directory:

```bash
PATH=/home/taichu/.nvm/versions/node/v24.15.0/bin:$PATH node scripts/node/test-scripts.js github-quality-gate
PATH=/home/taichu/.nvm/versions/node/v24.15.0/bin:$PATH node scripts/node/tooling.js check-rust-backend
```

This was verified by successfully running `node scripts/node/test-scripts.js github-quality-gate` with 28 passing tests.
