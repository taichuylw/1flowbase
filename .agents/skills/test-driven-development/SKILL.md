---
name: test-driven-development
description: Use when implementing 1flowbase features, bug fixes, refactors, backend APIs, state transitions, permissions, contract changes, or behavior changes that can be covered by automated tests. Use to write the minimum failing test that captures the expected observable result before implementation.
---

# Test Driven Development

## Goal

用最小测试先锁定目标行为，再写实现，避免先实现后补“证明型测试”。

## When to Use

- 新功能、缺陷修复、重构和行为变化
- API、状态流转、UI 交互、数据映射或权限规则变化
- 修复回归时，先写能复现问题的测试

**可跳过但必须说明原因**

- 纯配置、文案、样式 token 或脚手架调整
- 一次性原型、生成代码或测试基础设施暂时无法覆盖
- 用户明确要求跳过 TDD

## Preflight Gate

开始 TDD 前先确认实现入口：

- 1flowbase 功能、缺陷、重构或行为变化：必须已有用户确认的 L3 implementation issue。
- 可接受替代证据：用户在当前任务中明确说跳过 issue、直接实现或无需确认。
- 没有 issue 或跳过证据时，停止；回到 `problem-framing` 创建 / 更新 issue 并等待用户确认。
- 改产品代码前检查 `problem-framing/references/design-rules.md`；命中规则时停止，回到 `problem-framing` 给更小 redesign。

## Workflow

1. 写一个最小失败测试，表达目标行为或复现缺陷。
2. 运行定向测试，确认失败原因符合预期。
3. 写最小实现让测试通过。
4. 绿灯后再重构，重构后保持绿灯。
5. 按变更风险补必要回归：定向测试优先，必要时再跑类型、lint、build 或 smoke。

## Backend API Red Test

后端 API、权限、状态写入或 DTO contract 变化时，红灯测试必须表达可观察结果，而不是只测内部调用次数。

- 优先使用 route integration / service integration 测试覆盖真实中间件、DTO、错误映射和状态结果；纯领域规则再用单元测试。
- 需要认证的 console route 使用项目测试 support 的登录 / session / CSRF fixture；不要为了测试方便绕过 `require_session`、`require_csrf` 或 ACL。
- 测试命名和断言写清 method / path、请求 payload、预期 status、响应字段、错误 shape、scope、状态副作用或审计结果。
- 字段断言使用后端 DTO / 领域语义原名；不要为了前端展示别名写测试。
- 红灯失败原因必须是当前缺失行为或 contract 不匹配；如果失败来自 fixture、认证或环境不稳定，先修测试入口再实现。

## Evidence

交付说明至少覆盖：

- 新增或调整的测试
- 红灯确认方式
- 通过的验证命令
- 后端 API 任务的预期 response / 状态结果如何被测试断言覆盖
- 未验证范围、原因和替代验证

warning 与 coverage 产物统一落到 `tmp/test-governance/`。

## Common Mistakes

- 测试和实现一起写，没看过红灯。
- 方案确认后直接进入实现，没检查 issue gate。
- 实现前没检查 design rules，顺手新增模糊 helper、bool 分支或 pass-through 层。
- 只测 mock 调用次数，不测真实行为。
- 后端接口只测 service 内部逻辑，没有覆盖 route 认证、DTO、status / error shape 或状态副作用。
- 为了通过测试扩大实现范围。
- 跳过 TDD 但没有说明原因和替代验证。
