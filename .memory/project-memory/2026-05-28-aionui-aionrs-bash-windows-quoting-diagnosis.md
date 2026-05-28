---
memory_type: project
topic: aionui aionrs Bash Windows/WSL quoting diagnosis
summary: 2026-05-28 排查 flow_run 019e6c6f-8c01-7e60-afd6-a0fae2c346cc 的 Bash 高频失败。1flowbase runtime 事件、callback task 与 OpenAI Chat SSE/Responses 投影代码未发现 command 字符串改写；失败形态更符合 aionui 依赖的 aionrs BashTool 在 Windows 下经 cmd /C 再进入 WSL bash，导致嵌套引号被二次解释。
keywords:
  - aionui
  - aionrs
  - bash
  - windows
  - wsl
  - openai-chat-sse
  - tool-callback
  - quoting
match_when:
  - OpenAI Chat SSE client 是 aionui，Bash 工具调用出现 nested quote、python -c、heredoc 或 pwd 异常
  - 需要判断 1flowbase 协议转换是否改写 tool_calls.arguments.command
  - 需要比较 1flowbase callback 日志与 aionrs BashTool 执行外壳
created_at: 2026-05-28 11
updated_at: 2026-05-28 11
last_verified_at: 2026-05-28 11
decision_policy: verify_before_decision
scope:
  - api/apps/api-server/src/routes/application_public_api/compat_sse.rs
  - api/apps/api-server/src/routes/application_public_api/openai.rs
  - api/crates/orchestration-runtime/src/execution_engine.rs
  - ../aionui/AionCore/Cargo.toml
  - ~/.cargo/git/checkouts/aionrs-c860c928cccf810e/fca69d3/crates/aion-config/src/shell.rs
  - ~/.cargo/git/checkouts/aionrs-c860c928cccf810e/fca69d3/crates/aion-tools/src/bash.rs
---

# aionui aionrs Bash Windows/WSL quoting diagnosis

## 时间

`2026-05-28 11`

## 谁在做什么

用户要求排查会话 `019e6c6f-8c01-7e60-afd6-a0fae2c346cc` 中 Bash 工具调用频繁失败，判断是 1flowbase 中间协议转换损失，还是模型/客户端工具执行问题。

## 为什么这样判断

数据库中 `tool_call_commit` 与 `waiting_callback` 的 `name + arguments` 逐项匹配，`waiting_callback` 与 `flow_run_callback_tasks.request_payload.tool_calls` 也逐项匹配。1flowbase 的 OpenAI Chat SSE 与 Responses 投影只把 `arguments` 从 JSON object 序列化为 OpenAI 规范要求的 JSON string，不解析、重排或重新 quote 内部 `command`。

失败日志里，1flowbase 存储的命令如 `bash -lc "python -c \"print('hi')\""`，执行 stderr 却出现 `/bin/bash -lc \"python -c \\\"print('hi')\\\"\"` 这种带反斜杠的二次转义形态；该额外转义不在 1flowbase 存储层出现。

aionui 的 `AionCore/Cargo.toml` 依赖 `aionrs v0.1.25`。对应 `aion_config::shell::shell_command_builder` 在 Windows 下使用 `cmd /C`，`BashTool` 直接把模型 command 传给 `shell_command(command)`。结合日志中的 Windows 路径 `E:\...` 与 Bash 输出 `/mnt/e/...`，失败更符合 Windows `cmd /C` 外壳再进入 WSL bash 时破坏 POSIX 引号。

## 后续判断

优先不要在 1flowbase 转换层改写 AI 发出的 `command`。若要修复，应在 aionui/aionrs 工具执行层让 Bash 以 argv 方式进入目标 shell，或明确把 BashTool 实现成真正的 bash/wsl bash 执行器，避免通过 `cmd /C` 解释 POSIX shell 命令。
