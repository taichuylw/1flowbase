---
memory_type: feedback
feedback_category: repository
topic: L3 执行任务层是 AI 实现控制边界
summary: Issue 层级中 L3 是必要执行控制层；L2 workstream 进入实现前必须拆 L3，避免 AI 过度抽象、范围扩张或耗时失控。
keywords:
  - issue hierarchy
  - level:l3
  - execution boundary
  - problem-framing
  - ai control
match_when:
  - 拆分 parent / child issue
  - 将 issue 推进到 implementation
  - 优化 problem-framing 或 issue lifecycle 规则
created_at: 2026-05-21 00
updated_at: 2026-05-21 00
last_verified_at: 2026-05-21 00
decision_policy: direct_reference
scope:
  - .agents/skills/problem-framing
  - .memory/feedback-memory/repository
---

# L3 执行任务层是 AI 实现控制边界

## 时间

`2026-05-21 00`

## 规则

- L3 execution task 是必要层级，不是可选细化。
- L2 workstream 不能直接进入实现；实现前必须拆到 L3，或把该 issue 改标为 L3。
- L3 必须写清单一目标、主要文件/模块、验证证据和停止条件。

## 原因

如果不提前规划 L3，AI 容易把 L2 workstream 当成大包任务，产生过度抽象、范围扩张、不可控实现或耗时变长。

## 适用场景

- G3 / G4 issue 拆 parent / child。
- 将 `phase:ready` issue 推进到 `phase:implementation`。
- 判断某个 issue 是否足够小，可以交给 agent 直接实现。
