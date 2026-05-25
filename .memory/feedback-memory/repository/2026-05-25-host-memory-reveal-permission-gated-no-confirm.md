---
memory_type: feedback
feedback_category: repository
topic: host-memory-reveal-permission-gated-no-confirm
summary: 内存观察 reveal value 由 manage 权限控制；有权限直接打开，无权限隐藏按钮，不再追加确认弹窗。
keywords:
  - memory-observation
  - host-infrastructure
  - reveal
  - permission
  - audit
created_at: 2026-05-25 11
updated_at: 2026-05-25 11
last_verified_at: 2026-05-25 11
decision_policy: direct_reference
scope:
  - web/app/src/features/settings/components/host-infrastructure
---

# Host Memory Reveal Permission Gate

## 规则

内存观察里的 `Reveal` value 是权限门控动作：有基础设施 manage 权限且 contract 支持 reveal 时，按钮出现并直接打开 value；没有权限时不渲染按钮。不要再在点击后追加确认弹窗。

## 原因

用户确认：权限已经表达能不能看，确认弹窗会打断观察流程；无权限用户不应该看到不可用的敏感 value 入口。

## 适用场景

- 修改 `HostInfrastructureMemoryObservationPanel` 的 reveal value 交互。
- 调整内存观察里 sensitive value 的按钮可见性、权限判断或审计提示。
