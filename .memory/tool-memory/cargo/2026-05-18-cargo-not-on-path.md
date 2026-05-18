---
memory_type: tool
tool: cargo
failure_category: path_missing
decision_policy: reference_on_failure
created_at: 2026-05-18 21
updated_at: 2026-05-18 21
---

# Cargo not on PATH in Codex shell

## Failure

On `2026-05-18 21`, running `cargo test ...` in the Codex shell failed with:

```bash
/bin/bash: line 1: cargo: command not found
```

## Verified workaround

Use the installed cargo binary directly:

```bash
/home/taichu/.cargo/bin/cargo test ...
/home/taichu/.cargo/bin/cargo fmt --all --check
```

The direct binary path successfully ran targeted `orchestration-runtime` tests and fmt checks from `api/`.
