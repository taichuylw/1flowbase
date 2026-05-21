---
memory_type: feedback
feedback_category: repository
topic: 设计规则应成为需求讨论和实现前置闸门
summary: 用户认为当前 AI 常有过度设计问题；设计规则必须接入 problem-framing、TDD、frontend-development 和 backend-development，而不是作为实现阶段旁路提醒。
keywords:
  - design rules
  - overengineering
  - problem-framing
  - implementation gate
  - issue gate
match_when:
  - 需求讨论后准备创建 issue
  - 实现前准备新增抽象、公共接口、bool 参数、helper 或 pass-through 层
  - 优化 workflow skill 或实现 skill
created_at: 2026-05-21 09
updated_at: 2026-05-21 09
last_verified_at: 2026-05-21 09
decision_policy: direct_reference
scope:
  - .agents/skills/problem-framing
  - .agents/skills/test-driven-development
  - .agents/skills/frontend-development
  - .agents/skills/backend-development
---

# 设计规则应成为需求讨论和实现前置闸门

## 时间

`2026-05-21 09`

## 规则

- 过度设计要在需求讨论阶段先拦，不要等写代码时再提醒。
- `problem-framing` 负责在方案进入 issue gate 前检查设计规则。
- `test-driven-development`、`frontend-development`、`backend-development` 负责在改产品代码前再次检查设计规则。
- 设计规则命中时，先回到 `problem-framing` 给更小 redesign，再继续 issue 或实现。

## 原因

用户认为当前 AI 容易把小改动扩成抽象、兼容层、通用 helper、manager、bool 分支或 pass-through 层。设计规则若只作为独立 skill 或实现后复查，容易被绕过；放进 workflow gate 才能约束需求讨论、issue 和实现三段。

## 适用场景

- 需求方案看起来“合理”，但实现会新增抽象或公共接口。
- 准备新增 `data/info/result/handler/manager/process/utils/helper/do_*/*_impl` 这类模糊命名。
- 准备新增 bool 参数处理特殊 case。
- 重复 defensive check 已出现多次。
- 新增只转发参数的层。
