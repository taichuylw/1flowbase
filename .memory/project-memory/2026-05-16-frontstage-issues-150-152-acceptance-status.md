---
memory_type: project
topic: frontstage-issues-150-155-acceptance-status
summary: 2026-05-16 01 对 #149 前端子任务继续验收后确认：#152 复核仍可判验收通过；#150、#151、#154、#155 仍不能签收，主因是 `FrontStagePage` 测试套件仍有失败且 frontstage 页面 style-boundary 场景映射缺失；#149 整体状态应统一记为 `1.开发中`。
keywords:
  - frontstage
  - acceptance
  - issue-149
  - issue-150
  - issue-151
  - issue-152
  - issue-154
  - issue-155
  - qa
  - style-boundary
  - page-debug
match_when:
  - 验收前台路由、设计模式入口骨架或登录态权限切换时
  - 判断 #149 / #150 / #151 / #152 / #154 / #155 是否已通过验收时
created_at: 2026-05-16 01
updated_at: 2026-05-16 01
last_verified_at: 2026-05-16 01
decision_policy: verify_before_decision
scope:
  - web/app/src/app/router.tsx
  - web/app/src/routes/route-config.ts
  - web/app/src/features/frontstage/pages/FrontStagePage.tsx
  - web/app/src/features/frontstage/_tests/FrontStagePage.test.tsx
  - web/app/src/style-boundary/scenario-manifest.json
---

# Frontstage issues 150-155 acceptance status

## 时间

`2026-05-16 01`

## 谁在做什么

用户安排对 JS 扩展平台中 #149「前台路由、页面设计模式与区块编排管理」的前端子任务做继续验收，只允许输出纠正建议，不改业务代码。本轮在已有 #150、#151、#152 结论基础上，继续核对 #154、#155，并重新执行最小测试与 style-boundary 检查，把仍然有效的结论和新增失败点合并进同一条项目记忆。

## 为什么这样做

用户要求按子 issue 逐项给出验收判断，并在后续轮次中优先复用已验收记忆，避免重复从零核对已经通过的子项。本轮因此直接把哪些项可继续跳过、哪些项必须回退重做写清楚。

## 为什么要做

当前仓库里 #149 相关 issue 与评论同时存在「开发完成」表述，但并不等于已经达到仓库的 QA 门禁。需要把“开发完成”和“验收通过”明确分开，避免后续模型继续在错误状态上串行推进。

## 截止日期

`2026-05-16 02`

## 决策背后动机

本轮验收得到以下边界：

- `#152` 建议状态改为 `4.验收通过`。
  证据：
  - `pnpm --dir web/app test -- src/routes/_tests/route-config.test.ts src/app-shell/_tests/navigation.test.tsx src/features/frontstage/_tests/FrontStagePage.test.tsx` 中 `route-config` 与 `navigation` 用例通过。
  - `node scripts/node/check-style-boundary.js file web/app/src/app/router.tsx` 通过。
  - 本轮复核时 `web/app/src/routes/route-config.ts` 仍保持 `permissionKey: null` + `guard: 'session-required'`，与“内部登录用户可浏览前台”的规则一致。

- `#151` 建议状态改为 `5.验收不通过`。
  证据：
  - 同一条前端测试命令下，`web/app/src/features/frontstage/_tests/FrontStagePage.test.tsx` 失败。
  - 失败点在第 `50` 行：`screen.getByText('页面 page-1')` 无法命中当前渲染结构。
  - 当前仓库不能把这项标成 `3.测试完成` 或 `4.验收通过`。

- `#150` 暂不签收，建议状态改为 `5.验收不通过`，原因不是路由行为错误，而是缺少仓库要求的页面样式门禁映射。
  证据：
  - `/frontstage/workspace-1/page-1` 的桌面和移动端快照可正常打开，页面与导航骨架工作正常。
  - 但 `node scripts/node/check-style-boundary.js file web/app/src/features/frontstage/pages/FrontStagePage.tsx` 返回“未声明页面/组件场景映射”。
  - `node scripts/node/check-style-boundary.js file web/app/src/routes/route-config.ts` 也没有场景映射。
  - 在本仓库规则下，导航/路由/页面改动缺少 style-boundary 场景，不能直接签收为验收通过。

- `#154` 建议状态改为 `5.验收不通过`。
  证据：
  - `FrontStagePage` 的设计态工具栏按钮行为已存在，`src/features/frontstage/_tests/FrontStagePage.test.tsx` 第 `54-68` 行对应测试可以通过。
  - 但同一测试文件仍有失败用例，当前 feature 目标测试没有全绿，不能把“工具栏骨架”单独抬到 `3.测试完成` 或 `4.验收通过`。
  - `FrontStagePage.tsx` 仍缺少 style-boundary 场景映射，命中仓库前端验收硬门禁。

- `#155` 建议状态改为 `5.验收不通过`。
  证据：
  - `FrontStagePage.tsx` 已出现左右布局骨架与“JS Block 试运行”入口，`src/features/frontstage/_tests/FrontStagePage.test.tsx` 第 `70-79` 行布局骨架断言可以通过。
  - 但当前 `web/app/src/features/frontstage/` 目录下仍只有页面组件和测试两个文件，没有页面树 API、默认首页真值、codeRef 或后端 DTO 接入，离 #149 定义的后端真值优先仍有明显距离。
  - 同样受限于前述测试红灯与 style-boundary 缺失，不能签收。

- `#149` 整体仍保持 `1.开发中`。
  原因：
  - 先前记忆里把编号写成了 `2.开发中`，与用户本轮固定的 `1-5` 状态枚举不一致；当前已统一修正为 `1.开发中`。
  - 当前只完成了路由骨架、空态、权限切换和静态布局壳层的前置子任务。
  - 页面树、后端真值、布局持久化、schema storage 编排、block initializer、codeRef/code storage 等主体范围都未完成。

## 关联文档

- `docs/plans/2026-05-15-js-extension-platform-architecture.md`
- `web/AGENTS.md`
- `web/app/src/features/frontstage/pages/FrontStagePage.tsx`
- `web/app/src/features/frontstage/_tests/FrontStagePage.test.tsx`
- `tmp/page-debug/2026-05-15T17-19-55-554Z/`
- `tmp/page-debug/2026-05-15T17-19-55-602Z/`
- `tmp/page-debug/frontstage-mobile-2026-05-15.png`
