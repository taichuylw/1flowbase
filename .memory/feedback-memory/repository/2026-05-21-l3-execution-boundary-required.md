---
memory_type: feedback
feedback_category: repository
topic: L3 执行任务层是 AI 实现控制边界
summary: Issue 层级中 L3 是必要执行控制层；problem-framing 必须按方案对齐 -> 用户确认 -> issue gate -> issue 确认 -> 实现顺序推进，当前阶段未完成前不能提前输出下一阶段 L3 或实现口径。
keywords:
  - issue hierarchy
  - level:l3
  - execution boundary
  - problem-framing
  - issue gate
  - phase order
  - ai control
match_when:
  - 拆分 parent / child issue
  - 将 issue 推进到 implementation
  - 用户确认方案后是否可以直接实现
  - AI 提前建议下一阶段 issue 或实现口径
  - 优化 problem-framing 或 issue lifecycle 规则
created_at: 2026-05-21 00
updated_at: 2026-05-21 10
last_verified_at: 2026-05-21 10
decision_policy: direct_reference
scope:
  - .agents/skills/problem-framing
  - .memory/feedback-memory/repository
---

# L3 执行任务层是 AI 实现控制边界

## 时间

`2026-05-21 00`

## 规则

- 方案确认只授权进入 issue 草案 / 审核，不等于授权实现。
- problem-framing 未完成方案对齐和用户确认前，不提前进入 L3 issue 草案；用户明确提醒“不要跳顺序”时，先退回当前阶段完成对齐。
- 当前阶段未完成前，不提前输出下一阶段产物；例如还在边界确认 / 记忆同步阶段时，不直接写“下一步 L3 issue 就按...”，应先停在当前阶段并询问是否进入 issue gate。
- L3 execution task 是必要层级，不是可选细化。
- 除非用户明确说跳过 issue、直接实现或无需确认，否则没有已确认 L3 issue 不进入实现。
- L2 workstream 不能直接进入实现；实现前必须拆到 L3，或把该 issue 改标为 L3。
- L3 必须写清单一目标、主要文件/模块、验证证据和停止条件。

## 原因

如果不提前规划 L3，AI 容易把 L2 workstream 当成大包任务，产生过度抽象、范围扩张、不可控实现或耗时变长。用户确认技术方案时，AI 也容易误判为实现授权，或提前输出下一阶段 issue / 实现口径，因此需要把 phase order 和 issue gate 写成硬门槛。

## 适用场景

- G3 / G4 issue 拆 parent / child。
- 用户说“方案可以 / 合理 / 好”之后，判断下一步是 issue gate 还是实现。
- 当前还在讨论、记忆同步、边界确认或方案收敛时，禁止顺手建议下一阶段的 L3 / implementation 内容。
- 将 `phase:ready` issue 推进到 `phase:implementation`。
- 判断某个 issue 是否足够小，可以交给 agent 直接实现。
