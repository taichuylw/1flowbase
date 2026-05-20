---
name: problem-framing
description: 1flowbase 需求类请求动工前使用：普通功能、缺陷、交互、重构、规则、文档、架构或跨 frontend/backend 需求，默认先给 2-3 个轻量做法、明确推荐并等待用户确认；涉及 contract、defaults、migration、历史数据、权限、状态归属、用户内容、产品流程、issue shaping、issue 层级/分级标签、ADR drafting 或 implementation planning 时升级为完整规划。先收敛目标、范围、成功标准、方案、风险、终止条件和用户拍板点，再进入实现。
---

# Problem Framing

## Overview

本 Skill 是 1flowbase 的动工前规划闸门。它只负责把需求、证据、边界和拍板点收敛清楚，不负责直接实现。

## Iron Law

需求类请求未完成对齐和用户确认前，不进入代码实现。影响数据、contract、架构或用户内容的请求，未完成事实整理、范围收敛和用户拍板前，不进入迁移设计或大规模重构计划。

## Entry Gate

拿到需求类请求时，先使用本 Skill，再进入 `frontend-development`、`backend-development` 或 `test-driven-development`。

只有纯查询、机械精确改动，或用户明确要求直接开始 / 无需确认时，才跳过本 Skill。

## Scope Boundary

Allowed:

- 收敛目标、范围、成功标准、假设、未知点、不变量、失败模式和需要用户拍板的问题。
- 只检查确认直接事实所需的代码、文档、issue、测试和日志。
- 产出简短对齐、讨论 brief、决策矩阵、三方案对比、red-team 评审、issue 草案、issue 分级标签、ADR 草案或实现交接稿。

Forbidden:

- 本 Skill 生效期间，不修改产品代码、migration、测试、schema 或运行时行为。
- 不新增抽象、兼容层、回滚系统、provenance、migration 或仓库级重构，除非它们被明确列为等待用户批准的方案之一。
- 不把狭窄请求扩展成路线图、平台重设计或清理专项。

## Convergence Budget

- 只读取当前请求、最近相关的 AGENTS / README / docs，以及直接受影响的代码路径。
- 只追一层相邻影响面；进入二阶路线图工作前停止。
- 阻塞问题最多 3 个，并一次性集中提出。
- 普通需求必须至少给出简短对齐：现状、2-3 个轻量方向、风险收益、明确建议，并等待用户确认。
- 任何存在多方向选择，或涉及数据 / contract / 架构风险的决策，都必须给出 3 个方案：conservative、balanced、aggressive。
- 推荐必须绑定证据；无证据支撑的判断标为假设。

## Workflow

1. 整理事实：分离已确认事实、假设、未知点、不变量、失败模式和需要用户决策的问题。
2. 先做简短对齐：普通需求按“现状、方向、风险收益、建议”输出 2-3 个轻量做法，明确推荐其中一个，并等待用户确认。
3. 拆分概念：在命名 API、service、enum、目录或 migration 前，先识别被混用的概念。
4. 建立矩阵：任务涉及 defaults、contract、schema、state、permissions、migration、history 或 user content 时，使用 `references/domain-matrix.md`。
5. 输出方案：存在多个有效方向，或任务涉及数据 / contract / 架构风险时，必须使用 conservative / balanced / aggressive 三方案；用户批准前不要压缩成单一最佳答案。
6. 管理 issue：需要落地开发时，使用 `references/issue-lifecycle.md` 分级、打标签、明确阶段和关闭条件。
7. 反方评审：向用户请求批准前，先 red-team 推荐方案，使用 `references/options-and-red-team.md`。
8. 停在决策产物：使用 `references/artifacts.md` 输出 brief、issue、ADR 或 implementation handoff。

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
- 三个方案和一个清晰推荐已经给出。
- 阻塞决策已经收敛到最多 3 个问题。
- 证据不足，无法安全区分方案。
- 用户否定或修改了核心假设。

## Handoff Rules

用户批准后，再切换到相关实现 Skill：

- 前端界面、交互、UI 结构：`frontend-development`。
- 后端 API、状态、写路径、领域边界：`backend-development`。
- 可测试行为变化：`test-driven-development`。
- 自检、回归、交付证据：`qa-evaluation`。

实现必须遵守已批准的产物。实现阶段不得扩大范围；新出现的未决问题必须回到本 Skill。
