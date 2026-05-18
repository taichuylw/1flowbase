# 2026-05-19 Project Quality Gate Evaluation

## Scope

本次评估覆盖当前仓库的废弃逻辑、旧断言、测试重复、代码拆分压力、门禁分层和 GitHub Actions 挂接状态。

重点不是给所有业务路径下“通过”结论，而是把全量审计信号纳入可复跑的质量门禁，并记录当前风险面。

## Evidence

已执行：

- `node scripts/node/tooling.js repo-hygiene --max-findings 800`
- `node scripts/node/tooling.js check-rust-backend`
- `node scripts/node/test-scripts.js repo-hygiene tooling verify-repo`
- `node scripts/node/test-scripts.js verify-backend`
- `node scripts/node/test-scripts.js repo-hygiene tooling verify-repo verify-backend test-backend runtime-gate`
- `node scripts/node/run-frontend-vitest.js run ...` for the changed non-legacy app test files
- `pnpm --dir web --filter @1flowbase/api-client test -- --runInBand`
- `cargo fmt --all --check`
- `node scripts/node/hotspot-review.js --since '30 days ago' --min-touches 5 --line-warning 1200 --line-error 1500`

产物：

- `tmp/test-governance/repo-hygiene.json`
- `tmp/test-governance/rust-backend-static-gate.json`
- `tmp/test-governance/hotspot-review.json`

未在本地执行：

- `node scripts/node/verify-repo.js`
- `node scripts/node/verify-ci.js`
- `node scripts/node/verify-coverage.js all`
- `node scripts/node/verify-backend-consistency.js`

原因：这些属于重型全仓 / 覆盖率 / 数据库一致性门禁，本轮已把新增 hygiene 门禁挂入 `verify-repo`，实际全量执行交给 GitHub Actions 的 `repo + backend-consistency + coverage` 分层作业。

受限验证：

- `web/app` 中 `data-models-page.test.tsx`、`me-page.test.tsx`、`section-shell-routing.test.tsx` 仍受既有 legacy/excluded 测试依赖解析问题影响，直接运行时卡在 `block-renderer -> antd` 解析阶段。
- `@1flowbase/page-runtime` 包测试仍受既有 `@1flowbase/block-renderer/antd-facade` 解析问题影响，本轮只改测试标题，未改变运行逻辑。

## Current Gate State

GitHub Actions 当前有两层入口：

- `.github/workflows/verify.yml`：PR、`main` / `latest` push 自动跑 `repo`、`backend-consistency`、`coverage` 和 React Doctor。
- `.github/workflows/quality-gate.yml`：手动和 nightly 跑同一套 Quality Gate Action，`ci` scope 并行拆成 `repo`、`backend-consistency`、`coverage` 后聚合 Issue 报告。

本轮新增：

- `repo-hygiene` 进入 `node scripts/node/verify-repo.js`。
- 因为 GitHub Actions 的 `repo` scope 调用 `verify-repo`，所以全量 CI / nightly 已自动获得该审计层。
- `repo-hygiene` 报告写入 `tmp/test-governance/repo-hygiene.json`，随 Actions artifact 上传。

## Findings

### P1: Rust 后端静态门禁曾有生产 `unwrap`

证据：`check-rust-backend` 初次失败：

- `api/apps/api-server/src/routes/applications/application_runtime/application_logs.rs:98`
- 规则：`no-production-escape`

处理：已改为带原因的 `expect(...)`，并重新验证通过。

当前状态：

- `check-rust-backend` 通过。
- 剩余 3 条 warning 是既有阻塞 IO 风险提示，未作为本轮阻塞项：
  - `api/crates/plugin-framework/src/artifact_reconcile.rs`
  - `api/crates/runtime-profile/src/fingerprint.rs`

### P2: 废弃 / legacy / compatibility 标记仍集中

初次 `repo-hygiene` 发现：

- `source-debt-marker`: 122
- `weak-test-assertion`: 18
- `duplicate-test-title`: 10

本轮已修复：

- `weak-test-assertion`: 18 -> 0
- `duplicate-test-title`: 10 -> 0

当前 `repo-hygiene` 剩余：

- `source-debt-marker`: 122
- `file-size-pressure`: 25
- `directory-pressure`: 13

主要集中区：

- `api/crates/control-plane/src`: 36
- `web/app/src/features`: 33
- `api/apps/api-server/src`: 18
- `api/crates/orchestration-runtime/src`: 6
- `api/crates/plugin-framework/src`: 5

判断：不少命中是兼容协议或反向测试夹具，不等同于必须删除；但现在至少进入 QA 证据层，后续可以逐步区分“允许的外部兼容协议”和“应清理的历史兼容口”。

### P2: 文件和目录拆分压力已经明显

`repo-hygiene` 当前发现：

- `file-size-pressure`: 25
- `directory-pressure`: 13

超过或接近 1500 行的代表文件：

- `api/crates/control-plane/src/_tests/orchestration_runtime/service.rs`: 2649
- `api/apps/api-server/src/_tests/application/application_runtime_routes.rs`: 1949
- `api/crates/storage-durable/postgres/src/_tests/orchestration_runtime_repository_tests.rs`: 1848
- `api/crates/control-plane/src/orchestration_runtime.rs`: 1762
- `api/apps/api-server/src/routes/applications/application_runtime.rs`: 1662
- `web/app/src/features/frontstage/pages/FrontStagePage.tsx`: 1501

目录压力最高：

- `api/crates/storage-durable/postgres/src/_tests`: 28 files
- `api/apps/api-server/src/_tests`: 27 files
- `api/crates/control-plane/src`: 27 files
- `scripts/node`: 26 files
- `api/crates/control-plane/src/_tests`: 23 files

判断：核心压力在 runtime、application runtime、storage repository tests、frontstage 页面和 agent-flow。后续拆分应按 owner 和状态入口切，不要只按“文件变小”机械拆。

### P2: 旧断言和重复测试标题已清零

初次 `repo-hygiene` 发现：

- `weak-test-assertion`: 18
- `duplicate-test-title`: 10

弱断言集中：

- `web/app/src/features/settings/_tests/data-models-page.test.tsx`: 8
- `web/packages/api-client/src/_tests/console-*.test.ts`: 多处只确认 spy 存在
- `web/app/src/app-shell/_tests/*.test.tsx`: 多处样式规则存在性断言

重复标题代表：

- `transport spy is active`
- `prefers VITE_API_BASE_URL when it is present`
- `rejects dynamic import with a stable import error`
- `returns syntax_invalid for malformed source without throwing`

判断：这些不一定导致错误行为，但会让 CI 失败时很难定位真实场景。建议后续把重复标题补上对象名或 feature 名，把 `toBeTruthy` / `toBeDefined` 改成用户可见行为或精确结构断言。

处理：已在本轮把相关断言和标题清理到 0。剩余 hygiene 风险只保留废弃标记和拆分压力。

### P2: 近 30 天热点仍集中在运行态真值和 UI 工作台

`hotspot-review` 当前发现 416 个热点：

- `runtime-truth-churn`: 251
- `general-hotspot`: 71
- `frontend-ui-churn`: 54
- `quality-gate-churn`: 34
- `file-size-pressure`: 6

最高风险热点：

- `api/crates/control-plane/src/orchestration_runtime.rs`
- `web/app/src/features/frontstage/pages/FrontStagePage.tsx`
- `api/crates/control-plane/src/_tests/orchestration_runtime/service.rs`
- `web/app/src/features/agent-flow/hooks/runtime/useAgentFlowDebugSession.ts`
- `api/apps/api-server/src/routes/applications/application_runtime.rs`
- `web/app/src/features/agent-flow/components/editor/AgentFlowCanvasFrame.tsx`

判断：runtime 真值、debug session、frontstage 和 application runtime 是近期反复变动区。后续改这些区域时，应默认升级到 contract / backend consistency / runtime page evidence，而不是只跑局部单测。

## Gate Layering Decision

当前建议保持四层：

1. `repo-hygiene`：静态工程卫生审计，先 warning 收集历史债，只阻塞 `test.only`。
2. `verify-repo`：repo hygiene + scripts + contracts + frontend full + backend full。
3. `backend-consistency`：数据库和状态一致性目标套件，仍独立于 `repo`。
4. `coverage`：独立重型覆盖率门禁，不并入 `repo`。

GitHub Actions 已按这个分层执行：自动 CI 和 nightly 都会跑 `repo + backend-consistency + coverage`，然后聚合报告和 artifact。

## Next Remediation Queue

优先级建议：

1. 把 `web/app/package.json` 里的 `test:legacy-ui` 拆成正式分层测试或删除旧入口。
2. 给 `source-debt-marker` 建允许清单：外部协议兼容保留，内部 legacy fallback 不保留。
3. 拆 `control-plane` orchestration runtime service 测试和 `application_runtime` route 文件。
4. 拆 `FrontStagePage.tsx` 和同名大测试，先按页面状态、画布 runtime、配置抽屉分 owner。
5. 修 `data-models-page.test.tsx` 中的弱断言和重复定位问题。

## QA Status

当前结论：新增工程卫生门禁已落地，并已进入 GitHub Actions 全量质量链路；本地可证明新增门禁、脚本测试、可运行的相关前端定向测试、API client package 测试和 Rust 静态门禁通过。弱断言和重复测试标题已清零。

未下结论：全量 `verify-ci`、coverage 和 backend consistency 未在本地跑完，需以 GitHub Actions run artifact 为最终合入证据。
