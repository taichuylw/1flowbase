---
memory_type: project
topic: openai-chat-calltask-history-normalization
summary: 2026-05-28 修复 OpenAI Chat 外部 callback id 泄露到 native history 的问题；Chat 外部仍使用 calltask_<callback_task_uuid>_<original_call_id> 定位回调，但进入 native history 前必须还原为 provider 原始 call_id。
keywords:
  - application public api
  - openai chat completions
  - openai responses
  - tool calling
  - callback task
  - call_id
  - calltask
created_at: 2026-05-28 15
updated_at: 2026-05-28 15
last_verified_at: 2026-05-28 15
decision_policy: verify_before_decision
scope:
  - api/crates/control-plane/src/application_public_api/compat/openai.rs
  - api/crates/control-plane/src/application_public_api/callback_tool_ids.rs
  - api/apps/api-server/src/routes/application_public_api/openai.rs
  - api/apps/api-server/src/routes/application_public_api/tool_callback_ids.rs
  - https://github.com/taichuy/1flowbase/issues/505
---

# OpenAI Chat calltask History Normalization

## 时间

`2026-05-28 15`

## 谁在做什么

用户确认 #505 采用 B 方案：OpenAI Chat 外部协议层继续使用 `calltask_<callback_task_uuid>_<original_call_id>`，但 Chat 历史进入 native `/api/v1/agent/runs` 前必须把可识别的外部 callback id 还原成 provider 原始 `call_id`。

## 为什么这样做

`calltask_...` 是 1flowbase 为无状态 Chat 工具回调定位 `flow_run_callback_tasks.id` 的外部投影 id，不是 provider 原始工具调用 id。它进入内部 LLM history 后，会被 OpenAI Responses provider 当作 `function_call_output.call_id` 发送给上游，可能超过 OpenAI 64 字符限制并触发 `Invalid input[].call_id`。

## 已落地边界

- `api-server` 仍负责 OpenAI Chat / Responses 外部投影和 callback resume。
- callback id 编解码下沉到 `control-plane::application_public_api::callback_tool_ids`，供 Chat 转 native history 时复用。
- `map_chat_completion_request` 只规范化历史中的 `assistant.tool_calls[].id` 和 `tool.tool_call_id`。
- 尾部 `role=tool` callback resume 仍用 encoded id 定位 callback task，并解码出 provider 原始 id。
- 无法识别的 id 原样保留，不猜测改写。

## 验证

- `cargo test -p control-plane application_public_api::compat::openai::tests -- --nocapture`
- `cargo test -p api-server openai_ -- --nocapture`
- `cargo clippy -p control-plane -p api-server --all-targets -- -D warnings`
