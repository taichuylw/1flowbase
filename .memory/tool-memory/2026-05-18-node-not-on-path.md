---
tool: node
summary: Codex shell has stable node/npm/pnpm shims in ~/.local/bin; if they disappear, restore them from the installed Node 24 bin directory.
decision_policy: reference_on_failure
last_verified: "2026-05-20 09"
---

# Node/npm/pnpm not on PATH in Codex shell

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

On `2026-05-20 09`, the Codex non-interactive shell also lacked direct `node`, `npm`, and `pnpm` commands even though the interactive desktop terminal had them via bash startup files. The stable fix was to bridge the installed Node 24 commands into `/home/taichu/.local/bin`, which is already on the non-interactive PATH:

```bash
ln -sfn /home/taichu/.nvm/versions/node/v24.15.0/bin/node /home/taichu/.local/bin/node
ln -sfn /home/taichu/.nvm/versions/node/v24.15.0/bin/npm /home/taichu/.local/bin/npm
ln -sfn /home/taichu/.nvm/versions/node/v24.15.0/bin/pnpm /home/taichu/.local/bin/pnpm
```

Verified direct commands from the Codex shell:

```bash
node -v   # v24.15.0
npm -v    # 11.12.1
pnpm -v   # 10.28.2
```
