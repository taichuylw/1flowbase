---
memory_type: tool
topic: cargo-plugin-installed-workspace-test
summary: `cargo test --manifest-path api/plugins/installed/.../Cargo.toml` 会因父级 `api/Cargo.toml` workspace 拦截失败；已验证复制插件目录到 `/tmp` 后再用 `--manifest-path` 可跑插件测试和构建。
keywords:
  - cargo
  - workspace
  - plugin-installed
  - current package believes it is in a workspace
match_when:
  - 在 `api/plugins/installed/*/*/Cargo.toml` 上直接运行 `cargo test --manifest-path` 或 `cargo build --manifest-path` 失败
  - 错误包含 `current package believes it's in a workspace when it's not`
created_at: 2026-05-03 11
updated_at: 2026-05-03 11
last_verified_at: 2026-05-03 11
decision_policy: reference_on_failure
scope:
  - cargo
  - api/plugins/installed
---

# Cargo 插件 installed 目录 workspace 拦截

## 时间

`2026-05-03 11`

## 失败现象

对 `api/plugins/installed/openai_compatible/0.3.19/Cargo.toml` 执行：

```bash
cargo test --manifest-path api/plugins/installed/openai_compatible/0.3.19/Cargo.toml ...
```

Cargo 返回：

```text
error: current package believes it's in a workspace when it's not
```

## 触发条件

插件源码目录位于 `api/` workspace 根目录下，但该插件包既不在 workspace members 中，也没有自己的 `[workspace]` 表。

## 根因

Cargo 会向上发现 `api/Cargo.toml` 的 workspace，并要求子包被纳入 members、exclude，或子包自己声明 `[workspace]`。本地 installed 插件目录是忽略产物，不适合为了验证临时改主 workspace 配置。

## 解法

复制插件目录到 `/tmp` 等 workspace 外路径，再对复制后的 `Cargo.toml` 运行测试或构建：

```bash
rm -rf /tmp/1flowbase-openai-compatible-0.3.19-test
cp -a api/plugins/installed/openai_compatible/0.3.19 /tmp/1flowbase-openai-compatible-0.3.19-test
cargo test --manifest-path /tmp/1flowbase-openai-compatible-0.3.19-test/Cargo.toml <filter> -- --nocapture
```

如需替换本地 installed 插件二进制，可在 `/tmp` 构建 release 后用 `install -m 0755` 覆盖 `api/plugins/installed/.../bin/...`。

## 验证方式

`2026-05-03 11` 已验证复制到 `/tmp` 后，`openai_compatible-provider` 单测可正常编译运行，release binary 可正常构建。

## 复现记录

- `2026-05-03 11`：修复 OpenAI-compatible streaming usage 时命中该 Cargo workspace 错误，使用 `/tmp` 复制目录完成测试与 release 构建。
