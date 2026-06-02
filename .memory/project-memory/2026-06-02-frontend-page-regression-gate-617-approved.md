---
memory_type: project
topic: 前端 page-regression 质量门禁 #617
summary: 用户确认 #617 采用脚本型 page-regression 门禁，截图、Playwright、浏览器 smoke 和视觉 diff 不作为本期必要验收项。
keywords:
  - issue-617
  - page-regression
  - slow-ui
  - quality-gate
  - frontend
match_when:
  - 实现或评审 GitHub issue #617
  - 调整前端 fast/full/page-regression 测试分层
  - 讨论 slow-ui、legacy-ui、截图或浏览器 smoke 是否进入默认 verify
created_at: 2026-06-02 14
updated_at: 2026-06-02 14
last_verified_at: 2026-06-02 14
decision_policy: verify_before_decision
scope:
  - web/
  - scripts/node/
  - .github/workflows/
  - tmp/test-governance/
---

# 前端 page-regression 质量门禁 #617 已确认

## 时间

`2026-06-02 14`

## 谁在做什么

用户确认 GitHub issue #617 的执行边界：把当前 fast 外的页面级 UI 测试沉淀为脚本型 `page-regression` 门禁，并更新 issue 标题、正文和阶段标签。

## 为什么这样做

当前 `test/test:fast` 会排除一批页面级 UI 测试，默认 verify 存在页面回归、受保护路由、复杂交互和布局断裂滞后暴露的风险。用户认为开发人员开发时会校验页面效果，门禁主要负责不让既有页面行为测试被破坏。

## 为什么要做

需要让 CI/default verify 能阻断 page-regression 测试失败，同时避免把截图、Playwright 或视觉基线引入为本期硬要求，减少噪音和维护成本。

## 截止日期

无明确截止日期；#617 已更新为 `phase:ready`，可进入实现。

## 决策背后动机

门禁重心是“脚本可跑、失败阻断、日志能定位、治理产物可追溯”，而不是证明视觉完美。若后续需要截图、真实浏览器 smoke 或视觉 diff，应停止并另开或升级 issue 决策。

## 关联文档

- GitHub issue: https://github.com/taichuy/1flowbase/issues/617
