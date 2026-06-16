---
memory_type: project
topic: state-protocol-quality-gate
summary: ACP / Anthropic-compatible 状态协议回归已被提升为固定质量门禁，真实 Claude Code ACP smoke 是该链路的硬证据之一。
keywords:
  - ACP
  - Claude Code
  - Anthropic-compatible
  - state-protocols
  - quality gate
  - agent_thought_chunk
created_at: 2026-06-16 19
updated_at: 2026-06-16 19
decision_policy: verify_before_decision
scope:
  - scripts/node/verify-state-protocols.js
  - scripts/node/acp-claude-smoke
  - scripts/node/gate-router/core.js
  - scripts/node/github-quality-gate/commands.js
---

# State Protocol Quality Gate

## 谁在做什么

用户确认 #922 暴露的 ACP / Anthropic-compatible reasoning 投影问题不是普通 smoke 缺口，而是重要状态协议回归风险。AI 已把该链路提升为固定质量门禁：`node scripts/node/verify-state-protocols.js`。

## 为什么这样做

之前只验内部 `answer_segments` / SSE fallback，没有通过真实 ACP + Claude Code 观察 `agent_thought_chunk`，导致验收漏掉 `reasoning_delta` 未投影为 ACP thought chunk 的问题。

## 为什么要做

这条门禁覆盖“后端协议投影 -> Anthropic-compatible stream -> Claude Code ACP adapter -> session/update 状态语义”的端到端状态协议链路。后续改 `compat_sse`、Anthropic public API 或 ACP smoke 脚本时，gate-router 会提示跑 `verify-state-protocols`。

## 截止日期

已在 2026-06-16 落地并推送 `dev`。

## 决策背后动机

状态协议类问题不能只靠内部单元测试宣称验收通过；真实外部 agent adapter 的状态事件是验收事实之一。该门禁默认包含脚本单测、`api-server anthropic_` 定向测试、后端 ensure 和真实 Claude Code ACP smoke。没有真实 ACP 证据时，只能标记为未验证，不下确定结论。
