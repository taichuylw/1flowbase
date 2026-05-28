---
tool: cargo
failure_category: cross_compile_dependency
decision_policy: reference_on_failure
first_seen: 2026-05-28 13
last_verified: 2026-05-28 13
---

# Provider Release Musl Reqwest Rustls

`1flowbase-official-plugins` 的 Rust model provider 如果使用 `reqwest = { features = ["json", "stream"] }` 默认 native TLS，GitHub Actions 的 `provider-release` / `provider-ci` 在 `x86_64-unknown-linux-musl` 和 `aarch64-unknown-linux-musl` cross build 会因为 `openssl-sys` 找不到 cross OpenSSL/pkg-config 而失败。

已验证解法：新 provider 的 `Cargo.toml` 对齐现有 provider 写法：

```toml
reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls", "stream"] }
```

修改后重新生成该 provider 自己的 `Cargo.lock`，确认 lock 里不再出现 `openssl`、`native-tls`、`hyper-tls`、`tokio-native-tls`，再推送触发 release。
