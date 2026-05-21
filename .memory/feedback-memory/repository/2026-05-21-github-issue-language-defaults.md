---
memory_type: feedback
feedback_category: repository
topic: GitHub issue 默认使用中文正文
summary: 用户发现 issue 标题中文但正文英文；GitHub issue 本身不是 implementation plan，标题和正文默认应使用中文，只有 labels、代码标识符、路径、命令和外部协议字段保持原文。
keywords:
  - github issue
  - issue language
  - problem-framing
  - issue draft
  - user preference
match_when:
  - 创建或更新 GitHub issue
  - 输出 issue draft
  - 调整 problem-framing issue 模板
  - 判断 implementation plan 英文偏好是否适用于 issue
created_at: 2026-05-21 10
updated_at: 2026-05-21 10
last_verified_at: 2026-05-21 10
decision_policy: direct_reference
scope:
  - .agents/skills/problem-framing
  - .memory/user-memory.md
---

# GitHub issue 默认使用中文正文

## 时间

`2026-05-21 10`

## 规则

- GitHub issue 标题和正文默认使用中文。
- labels、代码标识符、API 路径、文件路径、命令、错误码和外部协议字段保持原文。
- 不要把 `implementation plan 默认英文` 套到 issue 正文；`Implementation Handoff` 可以按英文偏好处理。

## 原因

用户在 issue 中看到标题中文、正文英文，判断这不是期望格式。issue 是讨论、决策和验收的工作单元，默认面向当前中文协作上下文；正文英文会增加审阅成本，也容易让 issue 看起来像 implementation plan。

## 适用场景

- `problem-framing` 输出或创建 issue draft。
- 将讨论内容同步到 GitHub issue。
- 区分 issue、设计文档、implementation handoff 和 commit message 的语言规则。
