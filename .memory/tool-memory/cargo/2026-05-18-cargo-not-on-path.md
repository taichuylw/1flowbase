---
memory_type: tool
tool: cargo
failure_category: path_missing
decision_policy: reference_on_failure
created_at: 2026-05-18 21
updated_at: 2026-05-20 09
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

## Stable Codex shell fix

On `2026-05-20 09`, bridge cargo into `/home/taichu/.local/bin`, which is already on the Codex non-interactive PATH:

```bash
ln -sfn /home/taichu/.cargo/bin/rustup /home/taichu/.local/bin/cargo
```

Verified direct commands from the Codex shell:

```bash
cargo --version
cargo test -p control-plane resource_crud_tests -- --nocapture
```

The targeted `control-plane` test command passed with 5 tests.
