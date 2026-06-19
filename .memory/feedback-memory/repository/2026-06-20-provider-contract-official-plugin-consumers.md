---
memory_type: feedback
feedback_category: repository
topic: Provider contract changes must update official plugin consumers
summary: 增加 provider contract 字段或 runtime invocation payload 时，不能只改首个验证 provider；需要同步检查 `../1flowbase-official-plugins/runtime-extensions/model-providers/*` 的本地输入结构、header/request builder 和默认策略测试。
created_at: 2026-06-20 06
updated_at: 2026-06-20 06
decision_policy: direct_reference
scope:
  - api/crates/plugin-framework
  - runtime-extensions/model-providers
---

# Provider Contract Changes Must Update Official Plugin Consumers

## Rule

当主仓 `ProviderInvocationInput`、provider contract 或 runtime invocation payload 增加字段时，必须同步检查 official plugins 下所有 model provider 的本地 DTO 和请求构造逻辑。

## Reason

这些 provider crate 会各自复制输入结构；只更新首个验证 provider 会导致其他 provider 隐式忽略新字段，甚至把新字段落入 `extra`，让 contract 看起来已完成但消费者没有显式策略。

## Applies When

- 修改 `api/crates/plugin-framework/src/provider_contract.rs`。
- 修改 provider invocation payload、runtime-to-provider stdio input 或客户端协议 envelope。
- 给某个 provider 增加协议 allowlist / denylist / header policy。

## Expected Handling

同步检查 `../1flowbase-official-plugins/runtime-extensions/model-providers/*`，至少给未声明 provider 添加默认保守策略测试，确保未知字段不盲透传、入口 auth 不覆盖 provider config。
