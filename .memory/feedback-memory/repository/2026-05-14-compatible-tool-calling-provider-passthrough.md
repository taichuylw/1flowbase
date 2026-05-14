---
memory_type: feedback
feedback_category: repository
topic: compatible-tool-calling-provider-passthrough
summary: 公开 OpenAI / Anthropic 兼容 API 的工具调用链路中，本地 agent 负责执行工具；1flowbase 需要把 tools、tool_calls、tool result history 透传给模型供应商，并把供应商返回的 tool_calls 映射回兼容 API。
keywords:
  - application public api
  - openai compatible
  - anthropic compatible
  - tool calling
  - provider passthrough
created_at: 2026-05-14 23
updated_at: 2026-05-14 23
last_verified_at: 2026-05-14 23
decision_policy: direct_reference
scope:
  - api/crates/orchestration-runtime
  - api/crates/plugin-framework
  - api/crates/control-plane/src/application_public_api
  - api/apps/api-server/src/routes/application_public_api
  - ../1flowbase-official-plugins/runtime-extensions/model-providers
---

# Compatible Tool Calling Provider Passthrough

## 规则

当用户讨论公开兼容 API 的 tool calling 时，默认链路是：

- 本地 agent / SDK 调用 1flowbase 兼容 API，并传入 `tools`。
- 1flowbase 把工具定义和对话历史传给已发布 flow/runtime，再由模型 provider 插件传给上游大模型供应商。
- 上游供应商返回 `tool_calls` 后，1flowbase 映射回 OpenAI / Anthropic 兼容响应。
- 本地 agent 执行工具，并在下一轮把 assistant `tool_calls` 与 tool result history 回传给 1flowbase。
- 1flowbase 再把这些历史字段传回模型供应商，完成下一轮推理。

## 原因

用户纠正过：兼容 API 场景下“调用工具”不是 1flowbase 本地自动执行工具，而是本地 agent 执行工具后回传给 API 大模型链路。1flowbase 在这里的职责是协议映射和 provider passthrough。

## 适用场景

- 修改 `/v1/chat/completions`、`/v1/messages` 或 Native application public API。
- 修改 `ProviderInvocationInput`、runtime LLM 节点、provider 插件 message/body 构造。
- 解释工具调用兼容性、调试 agent SDK 二轮工具调用失败。
