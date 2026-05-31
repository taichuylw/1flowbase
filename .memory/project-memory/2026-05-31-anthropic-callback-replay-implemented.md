---
memory_type: project
topic: anthropic-callback-replay-implemented
summary: Claude Code Anthropic /v1/messages duplicate completed callback resumes are handled as idempotent replays, and streaming tool_use input is emitted through input_json_delta so Claude Code receives real tool parameters.
keywords:
  - anthropic
  - claude code
  - callback_task_not_pending
  - tool_result replay
  - input_json_delta
  - Invalid tool parameters
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

后续同日继续确认：Claude Code 终端出现“新用户输入下面重复显示上一轮答案”，不是后端把新 run 做成旧输出。DB 中 `019e7b92-4b73-70e3-a178-1a101b9647a2` 已生成图片描述，终端却显示上一轮问候；原因是上一轮 `hi` 触发了 Bash tool_use，但 Anthropic SSE 当时只在 `content_block_start` 放完整 `input`，没有按 Anthropic streaming 协议发送 `content_block_delta` / `input_json_delta`。Claude Code 流式解析时拿到空输入 `{}`，于是本地工具校验报 `Invalid tool parameters` / `command is missing`，后续用户输入会 replay 这个旧 tool_result，幂等 replay 又返回上一轮 run 的最终问候，造成 UI 上像“发送内容重复”。

修复边界：`api/apps/api-server/src/routes/application_public_api/compat_sse.rs` 的 Anthropic `waiting_callback` tool_use SSE 改为 `content_block_start input:{}` + `input_json_delta partial_json` + `content_block_stop`，非流式响应仍保留完整 `input`。验证证据：新增红灯测试 `anthropic_waiting_callback_streams_tool_input_json_delta`；修复后 `cargo fmt --check`、`cargo test -p api-server routes::application_public_api::compat_sse::tests --`、`cargo test -p api-server routes::application_public_api::anthropic::tests --`、`cargo test -p api-server _tests::application_public_api::anthropic_routes --`、`cargo test -p control-plane application_public_api::anthropic_compat --` 全部通过；临时 7802 smoke 看到 Bash tool_use SSE 已包含 `input_json_delta` 里的 `command`，模拟 tool_result resume 返回 `200`。
