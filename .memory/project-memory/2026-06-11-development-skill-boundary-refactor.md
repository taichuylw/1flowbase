---
created_at: 2026-06-11 19
memory_type: project
decision_policy: verify_before_decision
scope: development skill boundary refactor
source_issue: "#858"
commit: 525fc58a
---

# Development Skill Boundary Refactor

用户在 2026-06-11 确认对 `frontend-development` 和 `backend-development` 走平衡瘦身：不拆新 skill，把 development skill 还原成实现期边界守门员。

已确认职责边界：

- `problem-framing` 负责需求对齐、方案选择、issue shaping、ADR 和未决产品 / 架构拍板。
- `frontend-development` / `backend-development` 负责已确认范围内的实现入口、核心不变量、按场景加载 reference、回退条件和交付出口。
- `qa-evaluation` 负责自检、验收、回归、质量报告和证据结论。
- 涉及可测试行为变化时，仍先联动 `test-driven-development`；不能 TDD 时交付说明写明替代验证。

本轮已创建 GitHub issue #858，并提交 `525fc58a Refine development skill boundaries` 到 `dev`。Issue 已进入 `phase:user-acceptance` 等待用户确认。
