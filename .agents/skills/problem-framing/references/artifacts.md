# Planning Artifacts

Use only the artifact needed for the current request. Keep artifacts short enough for a human to approve in one pass.

## Discussion Brief

```md
# Discussion Brief

## 现状
- Confirmed facts:
- Assumptions:
- Unknowns:

## 目标

## 范围
- In scope:
- Out of scope:

## 成功标准

## 不变量

## 风险与失败模式

## 需要拍板
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

- `Observed Facts` require source evidence.
- `Draft Hypotheses` must remain challengeable.
- `Open Decisions` must be explicit user decisions, not implementation chores.

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

- Handoff is not an implementation plan unless the user asks for one.
- Each implementation task must map back to an approved scope item or acceptance evidence.
- If implementation discovers a new decision, return to `problem-framing`.
