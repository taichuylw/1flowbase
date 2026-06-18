---
memory_type: feedback
feedback_category: repository
topic: 后端开发 skill 与质量门禁的目标边界
summary: 调整后端相关 skill 时，把接口预期和验收设计放在 problem-framing，TDD 只负责测试写法，QA 负责接口质量门禁与证据结论，backend-development 只负责按已确认设计实现。
keywords:
  - backend-development
  - test-driven-development
  - qa-evaluation
  - api evidence
  - debug skill
match_when:
  - 调整后端开发、TDD、QA、接口取证或 debug 相关 skill / 门禁时
  - 讨论是否新增 debug skill、API 调试工具或后端验收证据链时
created_at: 2026-06-18 15
updated_at: 2026-06-18 15
last_verified_at: 无
decision_policy: direct_reference
scope:
  - .agents/skills/backend-development
  - .agents/skills/problem-framing
  - .agents/skills/test-driven-development
  - .agents/skills/qa-evaluation
  - scripts/node
---

# 后端接口预期、测试和质量门禁边界

## 时间

`2026-06-18 15`

## 规则

调整后端开发、TDD、QA 或接口取证相关 skill 时，先围绕“让 AI 在开发后端前已经有已确认预期，并能用测试和接口质量门禁校验产出”收敛方案。

职责分配必须清楚：接口预期、状态预期和验收设计归 `problem-framing`；后端测试写法归 `test-driven-development`；项目体检、mock / fixture 接口质量门禁和证据结论归 `qa-evaluation`；`backend-development` 只按已确认设计实现，缺少预期时回到前置 skill。不要把问题优先扩展成泛化 `debug skill`。

## 原因

用户明确纠正：这次改动的目的不是为了调试本身，也不是让 `backend-development` 承担需求或测试设计，而是让 AI 在后端开发前已有清晰预期，开发后通过 TDD 和 QA 门禁判断接口、状态、返回结构和值是否符合预期。

## 适用场景

- 修改 `test-driven-development`、`qa-evaluation`、`backend-development` 或相关 reference。
- 设计后端 API 验收、接口 response evidence、认证态获取、CSRF / cookie 使用规则。
- 讨论是否新增 `debug` skill、API 调试脚本或开发后验收工具。
