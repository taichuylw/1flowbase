---
memory_type: feedback
feedback_category: repository
topic: application-run-system-source-distinction
summary: 运行详情里的 system 必须区分外部客户端协议 system / Claude Code system-reminder 与 LLM 节点内置 system；展示兜底不能掩盖写入链路丢失外部 system。
decision_policy: direct_reference
tags:
  - application-runtime
  - anthropic-compatible
  - claude-code
  - system-prompt
---

# Application Run System Source Distinction

用户在 `2026-05-31` 纠正：运行详情中看到的 LLM 节点 `prompt_messages[role=system]` 只是节点内置提示词，不等于 Claude Code / Anthropic 协议传入的真实系统提示词。

适用场景：排查应用日志、运行详情、Anthropic-compatible `/v1/messages`、Claude Code session history 或 system 展示问题。

规则：

- 先判断 system 来源：外部协议 `system` / Claude Code `<system-reminder>`、run-level `node-start.system`、history `role=system`、LLM node `prompt_messages` / `llm_context.effective_system`。
- 运行详情展示可以用 LLM system 做兜底，但不能把它当成外部客户端 system 的证据。
- 如果 run-level input/history 缺失外部 system，优先检查 public API compat mapper 是否在写入前过滤了 system，而不是只改日志展示层。
