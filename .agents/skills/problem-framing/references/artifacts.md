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
## Observed Facts

## Suspected Problems

## Non-negotiable Invariants

## Draft Hypotheses

## Open Decisions

## Bad Solutions

## Acceptance Evidence
```

Rules:

- `Observed Facts` 必须带证据来源。
- `Draft Hypotheses` 必须保持可被挑战，不能写成已决设计。
- `Open Decisions` 必须是用户需要拍板的真实决策，不是实现杂项。

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

- 除非用户明确要求，handoff 不是完整 implementation plan。
- 每个实现任务都必须能追溯到已批准范围或验收证据。
- 实现中发现新决策时，回到 `problem-framing`。
