---
memory_type: feedback
feedback_category: repository
topic: quality gate router should run locally before commit
summary: 质量门禁路由判断不能只放 GitHub Actions；Actions 可以验证已提交 diff，但看不到本地未提交工作区。用于决定应补哪些质量证据、该跑哪些门禁切片的轻量 router，应接入本地提交前检查，同时复用到线上 repo-tooling。
keywords:
  - quality-gate
  - gate-router
  - pre-commit
  - repo-tooling
  - local verification
match_when:
  - 设计质量门禁路由
  - 判断门禁规则放线上还是本地
  - 调整提交前检查或 repo-tooling
  - 防止新功能合并后才暴露缺失测试或质量证据
created_at: 2026-06-08 00
updated_at: 2026-06-08 00
decision_policy: direct_reference
scope:
  - scripts/node/verify
  - scripts/node/tooling
  - .github/workflows/verify.yml
  - .github/workflows/quality-gate.yml
  - .agents/skills/qa-evaluation
---

# Quality Gate Router Should Run Locally Before Commit

## 规则

用于判断“本次改动应该补哪些质量证据、该跑哪些门禁切片”的轻量 gate router，不能只依赖 GitHub Actions。它应优先接入本地提交前检查，并在 GitHub Actions 的 `repo-tooling` 中复用同一脚本。

## 原因

GitHub Actions 只能基于已提交的 commit / PR diff 做判断，无法看到开发者本地未提交的工作区。若 gate router 只在线上运行，开发者仍会在 push 或合并后才发现缺少测试、contract、状态机或数据一致性证据。

## 适用场景

- 设计或实现 `quality-gate-router` / `repo-gate-router`。
- 判断新增质量规则应写进 AGENTS、skill、本地 hook 还是 GitHub Actions。
- 处理“新功能合并后门禁才失败”的开发阶段前置控制问题。

## 边界

本地提交前默认只跑轻量路由和轻量证据检查，不默认跑 Rust clippy/test、coverage、container image 等重型门禁；重型门禁仍由 GitHub Actions 或显式本地命令承载。
