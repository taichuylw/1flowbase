---
memory_type: project
topic: anthropic-callback-replay-implemented
summary: Claude Code Anthropic /v1/messages duplicate completed callback resumes are handled as idempotent replays; streaming tool_use input uses input_json_delta; latest-message-only requests rehydrate durable conversation turns.
keywords:
  - anthropic
  - claude code
  - callback_task_not_pending
  - tool_result replay
  - input_json_delta
  - usage
  - context
  - cost
  - Invalid tool parameters
  - idempotent callback replay
  - /v1/messages
  - /api/v1/agent/runs
created_at: 2026-05-31 09
updated_at: 2026-05-31 19
last_verified_at: 2026-05-31 19
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

2026-05-31 继续处理 Claude Code `/context` 和 `/cost`：Claude Code 的这两个值不是从 1flowbase workflow 配置直接读取，而是从 Anthropic stream 写入 transcript 的 assistant `usage` 累积；自定义模型名 `1flowbase` 的 context window 分母在 Claude Code 本地默认回落到 200k，除非客户端用 `[1m]`、`CLAUDE_CODE_MAX_CONTEXT_TOKENS` 或一方 Anthropic model capability 机制。后端可修复的是 usage 分子和 cost：`NativeRunResult` 优先保留 `flow_run.output_payload.usage` / `usage_selector`，没有时从 `ApplicationRunDetail.node_runs[].metrics_payload.usage` 聚合多 LLM 节点 token；Anthropic streaming 在最终 `message_delta.usage` 返回 `input_tokens`、`cache_creation_input_tokens`、`cache_read_input_tokens`、`output_tokens`，因为 live `message_start` 发生在 run 结束前通常仍为 0。验证证据：临时 7802 对 `POST /v1/messages` streaming smoke 返回 final delta `input_tokens:212, output_tokens:469`；DB 同一 run 的两个 LLM node usage 分别为 `56/38` 和 `156/431`，flow output 本身没有 top-level usage。

同日继续处理 Claude Code 只发送当前用户消息导致“像是没有记忆”的问题：Anthropic `POST /v1/messages` 通过 `x-claude-code-session-id` 与 metadata user 绑定到 Native conversation，但客户端只发 latest message 时，原先 `NativeRunRequest.history` 为空，第二轮 run 的 `input_payload.node-start.history` 不会带回上一轮图像识别 / assistant 内容。修复边界在 control-plane conversation binding：若请求显式 history 为空，则按 `(application_id, api_key_id, external_user, external_conversation_id)` 从 durable `application_conversation_messages` 读取最近历史并注入 Native history；如果客户端已带完整 history，则不追加，避免重复。读取 durable 消息时按每个旧 `flow_run_id` 只取该 run 的当前轮 user 和后续 assistant，避免把已注入的旧 history 在第三轮再次读出。验证证据：新增 `anthropic_messages_rehydrates_session_history_from_durable_turns`，覆盖第二轮回填和第三轮不重复；`cargo fmt --check`、`cargo test -p api-server anthropic_ -- --nocapture`、`cargo test -p control-plane application_public_api -- --nocapture`、`cargo test -p storage-postgres application_conversation -- --nocapture` 通过。

同日继续处理 run `019e7c6a-184a-7ed3-b5ad-5b2b43a6ea09` 的重复输出：DB 证据显示该 run 没有 `flow_run_callback_tasks`，不是 callback 状态机失败；它的 `node-start.history` 被回灌了 Claude Code `<system-reminder>`、历史 assistant `<think>` 和文本化 `<tool_call>`，且 `application_conversation_messages` 已把每个 run 输入携带的旧 history 再次投影，导致新消息反复带旧内容。修复边界：conversation history 回灌前清洗 `<system-reminder>`、`<think>`、`<tool_call>`，连续同一 user query 只保留最后一轮；conversation message 投影只写当前 run 的 system/query/answer，不再把输入 history 复制进 durable conversation。验证证据：新增 `conversation_history_rehydration_filters_internal_claude_code_payloads`，并把 `terminal_published_run_projects_application_conversation_messages_once` 改为带输入 history 的红灯场景；`cargo fmt --check`、`cargo test -p control-plane application_public_api -- --nocapture`、`cargo test -p storage-postgres application_conversation -- --nocapture`、`cargo test -p api-server anthropic_ -- --nocapture` 通过。
