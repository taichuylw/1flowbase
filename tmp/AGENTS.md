# Scope

- 作用域：`tmp/` 下已跟踪的沙盒、演示、mock workspace 和测试产物目录。

## Local Rules

- `tmp/demo` 和 `tmp/mock-ui` 是历史演示 / mock workspace，不计入正式质量资产和测试覆盖盘点。
- 正式质量门禁只以仓库根 `scripts/`、`web/`、`api/`、`.github/` 的入口为准。
- `tmp/test-governance/` 只存放 warning、coverage、QA report 等运行产物，不承载源测试。
- 不在 `tmp/` 内新增正式验收测试；需要沉淀时迁移到最近的 `web/`、`api/` 或 `scripts/` 测试目录。

## Stop Conditions

- 若 `tmp/demo` 或 `tmp/mock-ui` 被重新接入 CI、发布流程或外部文档入口，先明确 owner，再决定是否升级为正式质量资产。
