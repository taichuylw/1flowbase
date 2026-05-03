---
memory_type: project
topic: Quality Gate Action 与手动 Issue 报告方向已确认
summary: 用户确认质量门禁要封装成仓库内专用 GitHub Action，CI/CD 可复用；只有手动触发 workflow 调用该 Action 且开启 publish_issue 时，才为本次运行创建一个全新的 Issue。
keywords:
  - quality-gate
  - github-actions
  - ci
  - cd
  - issue-report
  - manual-workflow
match_when:
  - 需要实现或调整 GitHub Actions 质量门禁封装
  - 需要判断 Issue 报告何时创建、是否复用旧 Issue
  - 需要编写 quality gate action 或 manual quality gate workflow
created_at: 2026-05-03 23
updated_at: 2026-05-03 23
last_verified_at: 2026-05-03 23
decision_policy: verify_before_decision
scope:
  - .github/actions/quality-gate
  - .github/workflows
  - scripts/node
  - docs/superpowers/specs/1flowbase/2026-05-03-quality-gate-action-design.md
---

# Quality Gate Action 与手动 Issue 报告方向已确认

## 时间

`2026-05-03 23`

## 谁在做什么

用户与 AI 已确认后续要为 1flowbase 做一个仓库内专用 Quality Gate Action。该 Action 统一封装质量检测执行、报告生成和可选 Issue 发布。

## 为什么这样做

仓库已经有 `scripts/node/verify-*` 质量入口。新设计不重写门禁逻辑，而是把 GitHub Actions 的复用边界收敛到 `.github/actions/quality-gate`，让 CI、CD 和手动验收都调用同一个 Action。

## 为什么要做

目标是让质量门禁在 CI/CD 中可复用，同时避免自动流水线污染 Issues。Issue 报告只作为手动验收或手动发布记录使用。

## 已确认决策

- Quality Gate Action 是专用 Action，负责跑质量检测。
- CI/CD workflow 可以调用该 Action。
- 手动 workflow 也调用同一个 Action。
- Issue 报告由 Action 支持，但必须通过 `publish_issue` 显式开启。
- 自动 CI/CD 默认 `publish_issue: "false"`。
- 手动触发 workflow 默认可传 `publish_issue: "true"`。
- 每次手动触发并开启 Issue 发布时，都创建一个全新的 Issue。
- 不维护固定 Issue，不追加历史 comment。

## 截止日期

未设置固定截止日期。后续实现以设计文档 `docs/superpowers/specs/1flowbase/2026-05-03-quality-gate-action-design.md` 为准。

## 决策背后动机

用户希望质量检测本身成为可复用的 GitHub Action，而不是把 CI、CD、Issue 发布逻辑散落在多个 workflow 中。同时用户明确不希望 Issue 报告自动生成，也不希望多次运行堆在同一个 Issue 中导致维护困难。
