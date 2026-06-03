## 关联 Issue

<!-- 完全解决时使用 `Closes #123` / `Fixes #123` / `Resolves #123`；部分实现或辅助改动使用 `Related to #123`。 -->

- Related to #

## 变更摘要

-

## 范围边界

已做：
-

未做：
-

## 验收证据

<!-- 粘贴实际命令、退出码摘要、截图或 tmp/test-governance 产物路径；未运行时写明原因。 -->

-

## 风险和回滚

-

## Checklist

- [ ] 已关联确认过的 L3 issue，或这是 G0 / 机械精确改动并在说明中写明跳过原因。
- [ ] PR 范围没有超出关联 issue 的目标、验收证据和停止条件。
- [ ] 如果修改了 issue 状态，标题前缀 `[状态]` 与 `phase:*` 标签同步。
- [ ] 涉及前后端接口字段时，字段名沿用后端 DTO / 领域语义，没有为展示文案另起接口字段别名。
- [ ] 涉及旧字段兼容时，兼容代码最近行包含 `@field-contract-compat source=... alias=... remove_by=yyyy-mm-dd`，并有测试和废弃计划。
- [ ] 涉及 i18n 时，已运行或说明 `i18n-hygiene` 结果。
- [ ] 涉及自检、验收、回归或交付时，已有 `qa-evaluation` 证据。
