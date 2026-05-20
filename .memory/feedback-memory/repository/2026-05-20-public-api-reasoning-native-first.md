---
memory_type: feedback
feedback_category: repository
topic: public-api-reasoning-native-first
summary: 应用接入 API 的思考过程输出以 1flowbase Native runtime event 为真值层，OpenAI / Anthropic 兼容接口必须从原生事件映射，不允许只补单一兼容协议。
keywords:
  - application public api
  - application api docs
  - native api
  - openai compatible
  - anthropic compatible
  - reasoning_delta
  - thinking_delta
  - heartbeat
  - model list
created_at: 2026-05-20 19
updated_at: 2026-05-20 19
last_verified_at: 2026-05-20 19
decision_policy: direct_reference
scope:
  - api/apps/api-server/src/routes/application_public_api
  - api/apps/api-server/src/application_public_docs.rs
  - api/crates/control-plane/src/orchestration_runtime
---

# Public API Reasoning Native First

## 规则

修改应用接入 API 的思考过程、流式输出或兼容协议映射时，先确认 Native runtime event stream 是否表达了真实语义；OpenAI / Anthropic 兼容接口只能作为 Native 事件的协议投影同步维护。

## 原因

用户纠正过：思考过程不是 OpenAI 专属能力，应用接入 API 应该以 1flowbase 原生接口为基础，再分别映射 OpenAI 和 Anthropic。如果只补一个兼容接口，会造成不同接入方式行为不一致。

## 适用场景

- 修改 `/api/1flowbase/runs` Native SSE。
- 修改 `/v1/chat/completions` OpenAI-compatible SSE。
- 修改 `/v1/messages` Anthropic-compatible SSE。
- 修改应用 API 页面 / OpenAPI 文档目录，必须同步公开运行时已经支持的 Native 与兼容端点，例如模型列表、心跳和流式事件契约。
- 调整 `reasoning_delta`、`text_delta`、`thinking_delta`、`reasoning_content` 等流式事件映射。
