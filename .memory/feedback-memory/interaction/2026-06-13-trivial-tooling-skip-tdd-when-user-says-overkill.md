---
memory_type: feedback
feedback_category: interaction
topic: trivial-tooling-skip-tdd-when-user-says-overkill
summary: 小型脚本/工具改动中，用户明确表示 TDD 多余时，跳过 TDD 流程，改用目标脚本测试、dry-run 或轻量验证说明。
keywords:
  - TDD
  - tooling
  - scripts
  - lightweight verification
match_when:
  - 用户明确说不用 TDD 或 TDD 多余
  - 小型脚本、CLI 或工具行为改动
  - 改动可通过目标脚本测试、dry-run 或最小验证覆盖
created_at: 2026-06-13 14
updated_at: 2026-06-13 14
last_verified_at: 2026-06-13 14
decision_policy: direct_reference
scope:
  - collaboration
  - scripts
  - tooling
---

# Trivial Tooling Skip TDD When User Says Overkill

## 时间

`2026-06-13 14`

## 规则

小型脚本、CLI 或工具行为改动中，如果用户明确表示“不用 TDD / TDD 多余”，不要再坚持 TDD 流程；可以维护必要的既有测试，但实现节奏应改为直接实现后做目标脚本测试、dry-run 或轻量验证。

## 原因

用户希望简单工具改动保持轻量，避免把低风险变更流程化过度。质量证据仍然需要，但不必用 TDD 作为前置仪式。

## 适用场景

- 调整 `scripts/node` 下的小型 CLI 或工具脚本。
- 用户已经明确给出实现方向，并点明 TDD 对当前改动多余。
- 验证可以由目标测试、`--dry-run`、`--help` 或 `git diff --check` 覆盖。
