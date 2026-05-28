---
memory_type: feedback
feedback_category: repository
topic: compatible-tool-calling-provider-passthrough
summary: 公开 OpenAI / Anthropic 兼容 API 的工具调用链路中，本地 agent 负责执行工具；1flowbase 需要把 tools、tool_calls、tool result history 透传给模型供应商，并把供应商返回的 tool_calls 映射回兼容 API。通用 OpenAI-compatible 中转和协议层默认只透传，不替上游供应商做 provider-specific 字段改写；供应商差异应由独立 provider 插件承载。
keywords:
  - application public api
  - openai compatible
  - anthropic compatible
  - tool calling
  - provider passthrough
created_at: 2026-05-14 23
updated_at: 2026-05-28 11
last_verified_at: 2026-05-28 11
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

通用 OpenAI-compatible 中转和协议层只负责透传标准协议字段，不负责把某个供应商不接受的字段形态改写成另一种上游形态。遇到阿里百炼 / Qwen 这类供应商特定入参、图片或工具调用约束时，默认方向是新建或维护独立供应商插件，而不是在通用 OpenAI-compatible 插件中加入 provider-specific 转换。

## 原因

用户纠正过：兼容 API 场景下“调用工具”不是 1flowbase 本地自动执行工具，而是本地 agent 执行工具后回传给 API 大模型链路。1flowbase 在这里的职责是协议映射和 provider passthrough。

用户在 `2026-05-28 11` 再次澄清：透传是合理边界，中转和协议层不应为阿里百炼做字段修正；阿里百炼差异属于单独插件职责。

## 适用场景

- 修改 `/v1/chat/completions`、`/v1/messages` 或 Native application public API。
- 修改 `ProviderInvocationInput`、runtime LLM 节点、provider 插件 message/body 构造。
- 解释工具调用兼容性、调试 agent SDK 二轮工具调用失败。
