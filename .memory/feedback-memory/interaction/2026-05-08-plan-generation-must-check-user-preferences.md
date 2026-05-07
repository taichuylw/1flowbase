---
memory_type: feedback
feedback_category: interaction
topic: plan-generation-must-check-user-preferences
summary: 生成 implementation plan 前必须先核对用户偏好，尤其是计划语言、1+n 拆分规则、是否使用当前仓库而非 worktree。
keywords:
  - plan
  - user preferences
  - 1+n
  - implementation plan
match_when:
  - 用户要求生成 implementation plan
  - 用户要求落成 1+n 的 plan
  - 计划可能超过 800 行
created_at: 2026-05-08 00
updated_at: 2026-05-08 00
last_verified_at: 2026-05-08 00
decision_policy: direct_reference
scope:
  - docs/superpowers/plans
  - .memory/user-memory.md
---

# Plan Generation Must Check User Preferences

## 时间

`2026-05-08 00`

## 规则

生成 implementation plan 前必须先核对 `.memory/user-memory.md` 中的计划相关偏好；当前用户默认要求 implementation plan 使用英文，超过 800 行时拆成 `1+n`，索引 plan 文件名携带 `index`，子 plan 使用 `01`、`02`、`03` 编号，并且计划执行不使用 `git worktree`。

## 原因

本轮在用户要求“落成 1+n 的 plan”后，AI 先读取了通用 `writing-plans` skill，差点按默认大计划模板推进；用户提醒“用户偏好里面有规则”。后续不能只看通用 skill，需要先用用户偏好覆盖默认流程。

## 适用场景

- 用户要求创建、拆分或执行 `docs/superpowers/plans` 下的 implementation plan。
- 任务规模较大，单个计划可能超过 800 行。
- 通用 superpowers skill 与用户偏好存在差异时。
