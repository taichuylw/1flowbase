---
name: problem-framing
description: 1flowbase 需求类请求动工前使用：普通功能、缺陷、交互、重构、规则、文档、架构或跨 frontend/backend 需求，默认先给 2-3 个轻量做法、明确推荐并等待用户确认；需要落地开发计划时默认走 L0 Umbrella 到 L1 ADR 到 L2 Epic 到 L3 Task 四层规划。涉及 contract、defaults、migration、历史数据、权限、状态归属、用户内容、产品流程、issue shaping、issue 层级/分级标签、ADR drafting 或 implementation planning 时升级为完整规划。先收敛目标、范围、成功标准、方案、风险、终止条件和用户拍板点，再进入实现。
---

# Problem Framing

## Overview

本 Skill 是 1flowbase 的动工前规划闸门。它只负责把需求、证据、边界和拍板点收敛清楚，不负责直接实现。需要落地开发计划时，默认按 L0 Umbrella -> L1 ADR -> L2 Epic -> L3 Task 走完整四层；只有 L3 是进入实现的最小受控单元。

## Iron Law

需求类请求未完成对齐和用户确认前，不进入代码实现。当前阶段未完成前，不输出下一阶段产物；方案确认只授权进入 issue 草案 / 审核，不等于授权实现；除非用户明确说跳过 issue 或直接实现，否则没有已确认 L3 issue 不进入实现。影响数据、contract、架构或用户内容的请求，未完成事实整理、范围收敛和用户拍板前，不进入迁移设计或大规模重构计划。需要开发计划时，不跳过 L0 事实、L1 决策、L2 工作流和 L3 执行边界；实现阶段不得用 L3 修改 L1 已定架构边界。

## Entry Gate

拿到需求类请求时，先使用本 Skill，再进入 `frontend-development`、`backend-development` 或 `test-driven-development`。

用户确认方案后，先创建或更新 L3 implementation issue，并等待用户确认 issue 内容。只有存在已确认 issue，或用户明确说跳过 issue / 直接实现，才允许切换到实现 Skill。

只有纯查询、机械精确改动，或用户明确要求直接开始 / 无需确认时，才跳过本 Skill。

## Phase Order Gate

按当前阶段输出，不提前写下一阶段内容：

- `alignment`: 只输出事实、假设、边界、2-3 个方向、风险收益和建议；不写 issue draft、L3 拆分或实现口径。
- `decision`: 只输出三方案、推荐、red-team 和需要用户批准的点；不把推荐写成已批准决策。
- `issue gate`: 只输出 issue draft / 标签 / 验收证据 / 停止条件；不写测试、代码计划或实现步骤。
- `implementation handoff`: 只在 issue 已确认后输出最小实现边界；不扩大 issue 范围。
- `qa/user acceptance`: 只输出证据、风险和待用户验收事项；不自动关闭总 issue 或写验收通过。

到达阶段边界就停止并等待用户确认。用户提醒“不要跳顺序”时，先退回当前阶段并只补当前阶段缺口。

## Scope Boundary

Allowed:

- 收敛目标、范围、成功标准、假设、未知点、不变量、失败模式和需要用户拍板的问题。
- 只检查确认直接事实所需的代码、文档、issue、测试和日志。
- 产出简短对齐、讨论 brief、决策矩阵、三方案对比、red-team 评审、L0/L1/L2/L3 issue 草案、issue 分级标签、ADR 草案或实现交接稿。

Forbidden:

- 本 Skill 生效期间，不修改产品代码、migration、测试、schema 或运行时行为。
- 不新增抽象、兼容层、回滚系统、provenance、migration 或仓库级重构，除非它们被明确列为等待用户批准的方案之一。
- 不把狭窄请求扩展成路线图、平台重设计或清理专项。

## Convergence Budget

- 只读取当前请求、最近相关的 AGENTS / README / docs，以及直接受影响的代码路径。
- 只追一层相邻影响面；进入二阶路线图工作前停止。
- 阻塞问题最多 3 个，并一次性集中提出。
- 普通需求必须至少给出简短对齐：现状、2-3 个轻量方向、风险收益、明确建议，并等待用户确认。
- 需要落地开发计划时，默认产出 L0 -> L1 -> L2 -> L3 四层；纯查询、机械精确改动或用户明确跳过规划除外。
- 任何存在多方向选择，或涉及数据 / contract / 架构风险的决策，都必须给出 3 个方案：conservative、balanced、aggressive。
- 推荐必须绑定证据；无证据支撑的判断标为假设。

## Design Rules Gate

方案进入 issue gate 或实现 handoff 前，若会新增抽象、公共接口、bool/flag 参数、通用 helper/manager/utils、重复校验、pass-through 层或非显然注释，先读取 `references/design-rules.md`。命中规则时停止，先给更小 redesign，不创建实现 issue、不进入实现。

## Workflow

1. 整理事实：分离已确认事实、假设、未知点、不变量、失败模式和需要用户决策的问题。
2. 先做简短对齐：普通需求按“现状、方向、风险收益、建议”输出 2-3 个轻量做法，明确推荐其中一个，并等待用户确认。
3. 检查阶段顺序：使用本文件 `Phase Order Gate` 判断当前只允许输出什么；到阶段边界就停。
4. 检查设计规则：方案可能引入抽象、接口、flag、helper、重复校验或 pass-through 时，读取 `references/design-rules.md`；违反时先输出更小 redesign。
5. 搭四层计划：需要落地开发时，使用 `references/issue-lifecycle.md` 默认建立 L0 Umbrella -> L1 ADR -> L2 Epic -> L3 Task；每一层可以有多个 issue，下一层只关联直接 parent。
6. 拆分概念：在命名 API、service、enum、目录或 migration 前，先识别被混用的概念。
7. 建立矩阵：任务涉及 defaults、contract、schema、state、permissions、migration、history 或 user content 时，使用 `references/domain-matrix.md`。
8. 输出方案：存在多个有效方向，或任务涉及数据 / contract / 架构风险时，必须使用 conservative / balanced / aggressive 三方案；用户批准前不要压缩成单一最佳答案。
9. 管理 issue：需要落地开发时，按 L0/L1/L2/L3 分级、打标签、明确阶段、直接 parent 和关闭条件；用户未确认 issue 前停止。
10. 反方评审：向用户请求批准前，先 red-team 推荐方案，使用 `references/options-and-red-team.md`。
11. 停在决策产物：使用 `references/artifacts.md` 输出 brief、issue、ADR 或 implementation handoff。

## User Decision Format

需要用户选择或批准时，使用这个格式：

- `现状`: 已确认什么、还有什么不确定、为什么这个决策重要。
- `方向`: 可执行的方向或方案。
- `风险收益`: 明确收益、代价、隐藏成本和失败模式。
- `建议`: 先给清晰推荐，再列出用户必须批准的点。

三方案决策中，每个方案都重复这四段，最后给出唯一推荐方案。

## Stop Conditions

命中任一条件就停止，并等待用户批准：

- discussion brief、issue draft、ADR draft 或 implementation handoff 已经可供审阅。
- 普通需求的简短对齐已经可供用户确认。
- 当前阶段允许的产物已经输出，下一阶段尚未得到用户确认。
- 用户已确认方案，但 L3 issue 草案尚未创建或尚未确认。
- 方案触发 `references/design-rules.md`，需要先给更小 redesign。
- 三个方案和一个清晰推荐已经给出。
- 阻塞决策已经收敛到最多 3 个问题。
- 证据不足，无法安全区分方案。
- 用户否定或修改了核心假设。

## Handoff Rules

用户批准方案后，先完成 issue gate：输出或更新 L3 issue，包含目标、范围、验收证据、预算、停止条件、标签和 parent；等待用户确认 issue 内容。

issue 已确认后，再切换到相关实现 Skill：

- 前端界面、交互、UI 结构：`frontend-development`。
- 后端 API、状态、写路径、领域边界：`backend-development`。
- 可测试行为变化：`test-driven-development`。
- 自检、回归、交付证据：`qa-evaluation`。

实现必须遵守已批准的产物。实现阶段不得扩大范围；新出现的未决问题必须回到本 Skill。
