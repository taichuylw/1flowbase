---
memory_type: feedback
feedback_category: repository
topic: 后端开发 skill 与质量门禁的目标边界
summary: 调整 TDD、QA 或后端相关 skill 时，核心目标是让 AI 在后端开发时理解预期产出并完成测试校验，不是先扩成泛 debug 工具。
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
  - .agents/skills/test-driven-development
  - .agents/skills/qa-evaluation
  - scripts/node
---

# 后端开发结果预期与验证闭环

## 时间

`2026-06-18 15`

## 规则

调整后端开发、TDD、QA 或接口取证相关 skill 时，先围绕“让 AI 在开发后端时感知预期开发结果，并能用测试和真实接口证据校验产出”收敛方案。

不要把问题优先扩展成泛化 `debug skill`。若需要新增工具，也应服务于后端开发闭环中的预期、认证态、请求、返回值、测试与证据保存，而不是成为无边界的调试工具箱。

## 原因

用户明确纠正：这次改动的目的不是为了调试本身，而是为了让 AI 在后端开发完成后知道应该验证什么、如何拿到认证上下文、如何看到真实接口返回，以及如何据此判断开发结果是否符合预期。

## 适用场景

- 修改 `test-driven-development`、`qa-evaluation`、`backend-development` 或相关 reference。
- 设计后端 API 验收、接口 response evidence、认证态获取、CSRF / cookie 使用规则。
- 讨论是否新增 `debug` skill、API 调试脚本或开发后验收工具。
