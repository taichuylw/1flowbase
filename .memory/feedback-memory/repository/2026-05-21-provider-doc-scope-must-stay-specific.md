---
memory_type: feedback
feedback_category: repository
topic: provider-doc-scope-must-stay-specific
summary: 用户给出某个模型供应商的官方文档或点名某个 provider 时，修改范围必须限定到该 provider，不能顺手改 Gemini、OpenAI-compatible 或其他插件。
keywords:
  - provider plugin
  - official docs
  - deepseek
  - gemini
  - anthropic
  - openai
created_at: 2026-05-21 18
updated_at: 2026-05-21 18
last_verified_at: 2026-05-21 18
decision_policy: direct_reference
scope:
  - ../1flowbase-official-plugins/runtime-extensions/model-providers
  - api/plugins/installed
---

# Provider Doc Scope Must Stay Specific

## 规则

当用户给出某个供应商的官方文档、API 链接或明确点名某个 provider 插件时，只修改该 provider 的源码、配置和测试；除非用户另行要求，不要把同一轮修复扩展到 Gemini、OpenAI-compatible 或其他 provider。

## 原因

用户明确纠正过：“我给你 DeepSeek 的文档你改 Gemini 干嘛”。跨 provider 顺手套用会制造无关改动，也容易把某个供应商协议误当成通用真值。

## 适用场景

- 用户提供 DeepSeek、Anthropic、OpenAI、Gemini 等单一供应商官方 API 文档。
- 修 provider 插件的 tool calls、json mode、streaming、model list、parameter schema 或 manifest。
- 对比多个供应商协议时，先分清“当前要改谁”，再决定是否新增其他 provider 的独立任务。
