---
memory_type: project
topic: js-extension-platform-acceptance-snapshot
summary: 2026-05-16 02 的 JS 扩展平台验收快照显示：#142 总体与 #143-#149 仍处于 `1.开发中`，当前只有 #149 的子任务 #152 可判 `4.验收通过`；最新 frontstage 提交已补到页面树本地增删和 pageId 同步，但其余 frontstage 子任务仍受目标测试红灯、style-boundary 缺失和后端真值未落地阻塞；后端 frontstage 真值、JS dependency pack、Code node runner、block runtime、isolation profile 与 native trusted block 仍未见实现闭环。
keywords:
  - js-extension-platform
  - acceptance
  - issue-142
  - issue-143
  - issue-144
  - issue-145
  - issue-146
  - issue-147
  - issue-148
  - issue-149
match_when:
  - 需要继续验收 JS 扩展平台相关实现时
  - 需要判断 #142 到 #149 当前应该推进哪一项时
created_at: 2026-05-16 01
updated_at: 2026-05-16 02
last_verified_at: 2026-05-16 02
decision_policy: verify_before_decision
scope:
  - docs/plans/2026-05-15-js-extension-platform-architecture.md
  - docs/plans/2026-05-15-code-node-isolation-architecture.md
  - api/crates/plugin-framework/src/host_contract.rs
  - api/crates/plugin-framework/src/manifest_v1.rs
  - api/crates/orchestration-runtime/src/execution_engine.rs
  - web/app/src/features/frontstage/pages/FrontStagePage.tsx
---

# JS extension platform acceptance snapshot

## 时间

`2026-05-16 02`

## 谁在做什么

用户安排对 JS 扩展平台总计划 `#142` 及其子计划 `#143` 到 `#149` 做阶段性验收。本轮要求只输出纠正指导，不参与改代码；若已有子项在项目记忆中被确认通过，则跳过重复验收，直接检查下一项。

## 为什么这样做

GitHub issue 评论里已经出现多条“开发完成”同步，但当前主分支的真实代码、测试和门禁证据并没有支撑整个平台进入“测试完成”或“验收通过”。需要把“已提交骨架”与“已通过验收”拆开，减少后续模型误判。

## 为什么要做

JS 扩展平台跨前后端、插件系统、运行时和页面编排；如果不先把当前完成度固定下来，后续容易在没有后端真值、没有运行时闭环和没有 QA 证据的前提下继续堆前端壳层，造成验收口径漂移。

## 截止日期

`2026-05-16 02`

## 决策背后动机

- `#142` 总体状态建议保持 `1.开发中`。
  证据：
  - 总体验收项要求的 dependency pack、Code node import 校验、发布快照、frontend block runtime、`ctx.data` CRUD、runner 扩展预留都没有形成仓库级闭环。
  - 当前主分支最近提交仍全部集中在 `frontstage` 前端骨架，没有任何对应 `#143-#148` 的后端或运行时实现提交。

- `#143` 建议保持 `1.开发中`。
  证据：
  - `api/crates/plugin-framework/src/manifest_v1.rs` 仍只允许现有 `slot_codes`，没有 `js_dependency_pack`。
  - `validate_slot_codes` 只接受 `node_contribution`、现有 provider/data-source/file processor 等槽位，没有 dependency pack 注册入口。

- `#144` 建议保持 `1.开发中`。
  证据：
  - 仓库中未见 application dependency selection、dependency snapshot 或 import alias 校验的实现痕迹。
  - 本轮仓库扫描没有发现与应用级 dependency 启用、发布快照固化或 Code import 校验相匹配的控制面/运行时实现提交。

- `#145` 建议保持 `1.开发中`。
  证据：
  - `api/crates/orchestration-runtime/src/execution_engine.rs` 第 `364` 行仍对未支持节点返回 `unsupported debug node type`。
  - 仓库中未见 `CompiledCodeRuntime`、`CodeInvoker` 或 `zod` import 闭环实现。

- `#146` 建议保持 `1.开发中`。
  证据：
  - 仓库搜索未发现 `frontend_block` manifest、`@1flowbase/antd-facade`、`@1flowbase/block-sdk`、worker runtime 或 `ctx.data` 受控 CRUD 桥接实现。
  - 当前 `web/app/src/features/frontstage/` 虽已新增本地页面树交互，但仍只有前台壳层页面与测试文件，没有区块 runtime、facade、host renderer 或 schema primitive 实现。

- `#147` 建议保持 `1.开发中`。
  证据：
  - `api/crates/plugin-framework/src/host_contract.rs` 的 `RuntimeSlotCode` 仍无 `code_executor`。
  - 仓库搜索未发现 `NodeIsolationProfile` 或 resolved isolation profile 进入 Code 节点执行链路。

- `#148` 建议保持 `1.开发中`。
  证据：
  - 仓库搜索未发现 `ui_block.javascript.native`、independent React root、portal containment 或 native trusted block 运行时实现。

- `#149` 建议保持 `1.开发中`。
  证据：
  - `web/app/src/features/frontstage/pages/FrontStagePage.tsx` 已从静态空态推进到本地 `pageTree` / `selectedPageId` 状态管理，但页面树、默认首页和删除逻辑仍是前端内存态，不是后端 DTO 真值。
  - `api` 目录下搜索 `frontstage_pages` 与 `frontstage_block_codes` 无结果，说明后端 migration、service、route、repository 还未开始落地。
  - 已有项目记忆确认只有子任务 `#152` 可判 `4.验收通过`，`#150/#151/#154/#155` 仍受目标测试红灯、style-boundary 缺失和后端真值缺口阻塞。
