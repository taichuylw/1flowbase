---
memory_type: feedback
feedback_category: repository
topic: github-automation-doc-entry
summary: `.github/GITHUB_AUTOMATION.md` 是用户专门调整后的 GitHub 自动化说明入口；测试和门禁应读取该文件，不应要求恢复 `.github/README.md`。
decision_policy: direct_reference
created_at: 2026-05-24 09
updated_at: 2026-05-24 09
scope:
  - .github/GITHUB_AUTOMATION.md
  - scripts/node/github-quality-gate
---

# GitHub Automation Docs Entry

## 规则

GitHub 自动化、质量门禁、React Doctor 和 Issue publishing 的说明文档入口是 `.github/GITHUB_AUTOMATION.md`。

## 原因

用户明确说明这是专门改出的文档入口；后续脚本测试、质量门禁或审计不应把缺少 `.github/README.md` 判定为文档缺失，也不应为了测试恢复旧 README。

## 适用场景

- 修改 `scripts/node/github-quality-gate` 相关测试。
- 审计 GitHub Actions / quality gate 文档契约。
- 搜索 GitHub 自动化说明、latest-only issue publishing、React Doctor frontend gate 文档。
