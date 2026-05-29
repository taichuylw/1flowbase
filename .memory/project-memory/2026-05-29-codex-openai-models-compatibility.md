---
memory_type: project
topic: codex-openai-models-compatibility
summary: Codex 接入 1flowbase OpenAI-compatible API 时，`/v1/models?client_version=...` 返回 Codex `ModelsResponse` 形状，普通 `/v1/models` 保持 OpenAI-compatible `object/data` 形状。
keywords:
  - codex
  - openai compatible
  - /v1/models
  - client_version
  - auto compaction
  - application public api
match_when:
  - 修改应用公开 API 的 OpenAI-compatible models 列表
  - 排查 Codex 自动压缩、模型元数据或 Responses SSE usage
  - 设计 OpenAI-compatible 与 Codex 专用元数据的边界
created_at: 2026-05-29 00
updated_at: 2026-05-29 00
last_verified_at: 2026-05-29 00
decision_policy: verify_before_decision
scope:
  - api/apps/api-server/src/routes/application_public_api/openai.rs
  - api/apps/api-server/src/routes/application_public_api/compat_sse.rs
  - api/crates/control-plane/src/application_public_api/compat/openai.rs
  - https://github.com/taichuy/1flowbase/issues/526
---

# Codex OpenAI Models Compatibility

## 时间

`2026-05-29 00`

## 谁在做什么

用户确认 #526 采用 Codex 专用模型元数据分支：Codex 调用 `/v1/models?client_version=...` 时，1flowbase 返回 Codex `ModelsResponse { models: [...] }` 形状；普通 `/v1/models` 仍返回 OpenAI-compatible `{ object: "list", data: [...] }`。

## 为什么这样做

Codex 会自动刷新 `/models` 并携带 `client_version` 查询参数。它需要 `ModelInfo` 中的 `context_window`、`max_context_window`、`auto_compact_token_limit` 等字段来计算上下文窗口和自动压缩阈值；普通 OpenAI-compatible 客户端仍预期标准 OpenAI 模型列表形状。

## 为什么要做

如果只返回 OpenAI-compatible `data` 列表，Codex 无法拿到模型上下文窗口和自动压缩阈值，接入 1flowbase 应用 API 时不会按预期触发客户端自动压缩。

## 截止日期

随 #526 当前实现落地，无额外截止日期。

## 决策背后动机

- 保持后端作为协议真值来源，不在客户端做兼容拼接。
- 不新增独立接口，使用 Codex 已有自动行为中的 `client_version` 作为稳定分支信号。
- `config.model_list` 中的模型描述负责提供 `context_window` 和 `auto_compact_token_limit`；兼容层只做外部协议投影。
- Responses SSE 的 `response.completed.response.usage` 也需要保留，供 Codex 更新 token 状态。

## 关联文档

- GitHub issue: `https://github.com/taichuy/1flowbase/issues/526`
