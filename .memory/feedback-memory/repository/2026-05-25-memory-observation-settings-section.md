---
memory_type: feedback
feedback_category: repository
topic: memory-observation-settings-section
summary: 内存观察是 settings 一级入口，不再作为基础设施 Provider 配置页内的二级 tab。
keywords:
  - settings
  - memory-observation
  - host-infrastructure
  - navigation
  - tabs
created_at: 2026-05-25 11
updated_at: 2026-05-25 11
last_verified_at: 2026-05-25 11
decision_policy: direct_reference
scope:
  - web/app/src/features/settings
  - web/app/src/app/router.tsx
---

# Memory Observation Settings Section

## 规则

内存观察应挂在 `/settings` 左侧一级导航，路径使用 `/settings/memory-observation`。基础设施页只承载 Provider 配置，不再用 `Provider 配置 / 内存观察` 这种页面内二级 tabs。

## 原因

用户确认内存观察是独立观察工作台，不应该藏在基础设施 Provider 配置下面；基础设施页和内存观察页的用户意图不同，拆成 settings 一级入口更清晰。

## 适用场景

- 修改 settings 导航、基础设施页或内存观察入口。
- 调整 host infrastructure provider 配置与易失层观察的页面层级。
