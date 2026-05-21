---
memory_type: feedback
feedback_category: repository
topic: public-api-reasoning-native-first
summary: 应用接入 API 以 1flowbase Native API / runtime event 为唯一真值层，OpenAI Chat Completions、OpenAI Responses、Anthropic 等外层协议只能从原生请求与事件映射，不允许把兼容协议下沉成状态来源。
keywords:
  - application public api
  - application api docs
  - native api
  - openai compatible
  - openai responses
  - anthropic compatible
  - reasoning_delta
  - thinking_delta
  - heartbeat
  - model list
created_at: 2026-05-20 19
updated_at: 2026-05-21 10
last_verified_at: 2026-05-21 10
decision_policy: direct_reference
scope:
  - api/apps/api-server/src/routes/application_public_api
  - api/apps/api-server/src/application_public_docs.rs
  - api/crates/control-plane/src/orchestration_runtime
---

# Public API Reasoning Native First

## 规则

修改应用接入 API 的思考过程、流式输出、会话续接或兼容协议映射时，先确认 1flowbase Native API / runtime event stream 是否表达了真实语义；OpenAI Chat Completions、OpenAI Responses、Anthropic 等兼容接口只能作为 Native 请求与事件的协议投影同步维护。

## 原因

用户纠正过：思考过程和会话能力都不是 OpenAI 专属能力，应用接入 API 应该以 1flowbase 原生接口为基础，再分别映射 OpenAI Chat Completions、OpenAI Responses 和 Anthropic。如果只补一个兼容接口，或把 `previous_response_id` 等外部协议字段当成内部真值，会造成不同接入方式行为不一致并污染状态边界。

## 适用场景

- 修改 `/api/1flowbase/runs` Native SSE。
- 修改 `/v1/chat/completions` OpenAI-compatible SSE。
- 新增或修改 `/v1/responses` OpenAI Responses-compatible 请求、响应或 SSE。
- 修改 `/v1/messages` Anthropic-compatible SSE。
- 修改应用 API 页面 / OpenAPI 文档目录，必须同步公开运行时已经支持的 Native 与兼容端点，例如模型列表、心跳和流式事件契约。
- 调整 `reasoning_delta`、`text_delta`、`thinking_delta`、`reasoning_content` 等流式事件映射。
- 若兼容协议需要 Native 当前缺失的能力，先补 1flowbase Native 真值能力，再在外层协议 adapter 投影；不要为兼容协议单独创建第二套会话、状态或事件语义。
