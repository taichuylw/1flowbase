# Planning Artifacts

只使用当前请求需要的产物。产物要短到用户能一轮审完并拍板。

## Discussion Brief

```md
# Discussion Brief

## Current State
- 已确认事实：
- 假设：
- 未知点：

## Goal

## Scope
- 范围内：
- 范围外：

## Success Criteria

## Invariants

## Risks And Failure Modes

## Decisions Needed
```

## Issue Draft

```md
## Issue 元数据
- 标题：[状态]标题
- 层级：
- 分级：
- 标签：
- 父 issue：
- 子 issue：

## 已确认事实

## 疑似问题

## 不可协商不变量

## 待验证假设

## 待决策事项

## 不采用方案

## 验收证据

## 执行边界
- 主要文件 / 模块：
- 验证：
- 停止 / 升级条件：

## 生命周期
- 当前阶段：
- 关闭条件：
```

Rules:

- GitHub issue 标题和正文默认中文；labels、代码标识符、API 路径、文件路径和命令保持原文。
- `已确认事实` 必须带证据来源。
- `待验证假设` 必须保持可被挑战，不能写成已决设计。
- `待决策事项` 必须是用户需要拍板的真实决策，不是实现杂项。
- `Issue 元数据` 必须按 `references/issue-lifecycle.md` 填写层级、分级和标签。
- `父 issue` 必须指向上一层 issue；`子 issue` 只列直接下一层 issue。
- 开展开发计划时默认形成 L0 -> L1 -> L2 -> L3；小需求可以压缩展示，但不能省略 L3 执行边界。
- L0 记录事实、冲突和总清单；L1 记录用户批准的决策；L2 记录工作流和依赖顺序；L3 记录单一执行任务。
- L3 issue 必须填写 `执行边界`；L2 issue 不得直接当实现任务使用。
- issue 标题必须使用 `[状态]标题`，并和 `phase:*` 标签同步。

## ADR Draft

```md
# ADR: <title>

## Status
Proposed

## Context

## Decision

## Rationale

## Alternatives Considered

## Rejected Options

## Risks

## Rollback

## Tests / Evidence
```

## Implementation Handoff

```md
# Implementation Handoff

## Approved Direction

## Scope
- In scope:
- Out of scope:

## Constraints

## Files / Areas To Inspect First

## Tests To Add First

## Verification Evidence

## Stop / Escalate If
```

Rules:

- 除非用户明确要求，handoff 不是完整 implementation plan；完整开发计划必须回到 L0/L1/L2/L3。
- Handoff 只能从已批准 L1、已收敛 L2 和明确 L3 生成。
- 每个实现任务都必须能追溯到已批准范围或验收证据。
- 实现中发现新决策时，回到 `problem-framing`。
