---
memory_type: project
topic: anthropic-callback-replay-implemented
summary: Claude Code Anthropic /v1/messages duplicate completed callback resumes are handled as idempotent replays in the api-server Anthropic projection layer, while native callback completion remains strict.
keywords:
  - anthropic
  - claude code
  - callback_task_not_pending
  - tool_result replay
  - idempotent callback replay
  - /v1/messages
  - /api/v1/agent/runs
created_at: 2026-05-31 09
updated_at: 2026-05-31 09
last_verified_at: 2026-05-31 09
decision_policy: verify_before_decision
scope:
  - api/apps/api-server/src/routes/application_public_api/anthropic.rs
  - api/crates/control-plane/src/application_public_api
  - /v1/messages
  - /api/v1/agent/runs
---

# Anthropic Callback Replay Implemented

2026-05-31，Claude Code 对接 1flowbase 的 Anthropic-compatible `POST /v1/messages` 时，会在客户端重试或 replay 场景中再次提交同一个已完成 callback 的 `tool_result`。本次排查确认真实 run 已经成功，报错来自协议投影层把已完成 callback 的重复提交继续交给 native callback 状态机，触发 `callback_task_not_pending`。

处理边界：`/api/v1/agent/runs` 仍是唯一 native truth，runtime / repository 的 callback 完成状态机保持严格；只在 `api-server` 的 Anthropic projection layer 里识别“已完成 callback + `response_payload` 完全一致”的重复 resume，并投影返回已存储的 `NativeRunResult`。如果同一 callback 的 payload 不一致，或 callback 已取消，仍返回 `409 callback_task_not_pending`。

验证证据：`cargo fmt --check`、`cargo test -p api-server routes::application_public_api::anthropic::tests --`、`cargo test -p control-plane application_public_api::anthropic_compat --`、`cargo test -p api-server _tests::application_public_api::anthropic_routes --` 全部通过；临时 `127.0.0.1:7802` smoke 验证同 payload 重放返回 `200 text/event-stream`，不同 payload 仍返回 `409 callback_task_not_pending`。
