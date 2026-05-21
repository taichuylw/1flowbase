---
memory_type: feedback
feedback_category: repository
topic: llm-tool-callback-provider-boundary
summary: LLM tool callback 的运行时/Native 形状是唯一真值；供应商 wire shape 只能由 provider 插件源码适配，plugin host 不替插件改写 tool_calls。
keywords:
  - llm tool callback
  - tool_calls
  - ProviderInvocationInput
  - provider host
  - native truth
  - openai compatible
created_at: 2026-05-21 17
updated_at: 2026-05-21 17
last_verified_at: 2026-05-21 17
decision_policy: direct_reference
scope:
  - api/crates/orchestration-runtime
  - api/crates/plugin-framework/src/provider_contract.rs
  - api/apps/plugin-runner/src/provider_host.rs
  - api/apps/api-server/src/provider_runtime.rs
---

# LLM Tool Callback Provider Boundary

## 规则

处理 LLM tool callback、assistant `tool_calls` 历史或 tool result continuation 时，运行时与 1flowbase Native API 保持中立 `ProviderToolCall` 语义作为唯一真值。`plugin-runner` / provider host 只把 1flowbase native truth 交给 provider 插件，不替插件改写成某个供应商 wire shape。外部供应商协议需要的字段形状，例如 OpenAI-compatible 的 `type:function` + `function.arguments` 字符串，由对应 provider 插件源码自己适配。

## 原因

用户纠正过：把 OpenAI 形状补在 `api-server`、运行时输入或 `plugin-runner` 通用 host 里，只解决了一个协议的表象，还会让兼容协议污染 Native 真值。真正问题是工具回调机制跨越运行时真值和供应商线协议时缺少清晰边界。

## 适用场景

- 修改 `ProviderInvocationInput.messages[].tool_calls`、`ProviderToolCall`、LLM 节点输出或 continuation history。
- 修复供应商返回 `missing field type`、`function.arguments` 类型不匹配等协议反序列化错误。
- 新增非 OpenAI provider 协议适配时，优先在 provider 插件源码处理，不改 `plugin-runner` 通用 host、运行时 checkpoint、Native public API 或 api-server 通用 runtime port。
