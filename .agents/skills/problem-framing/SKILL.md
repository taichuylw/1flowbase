---
name: problem-framing
description: Use before implementation for non-trivial 1flowbase work when requirements are ambiguous, architecture-affecting, cross frontend/backend, or touch contracts, defaults, migrations, historical data, permissions, state ownership, user-owned content, product workflow, issue shaping, ADR drafting, or implementation planning. Frame facts, assumptions, scope, invariants, options, risks, stop conditions, and user decision points before coding.
---

# Problem Framing

## Overview

本 Skill 是 1flowbase 的动工前规划闸门。它只负责把需求、证据、边界和拍板点收敛清楚，不负责直接实现。

## Iron Law

未完成事实整理、范围收敛和用户拍板前，不进入代码实现、迁移设计或大规模重构计划。

## Entry Gate

Use this Skill before `frontend-development`, `backend-development`, or `test-driven-development` when the request is not a narrow, already-decided change.

Skip only when the request is a pure local fix, copy/style token tweak, mechanical rename, or the user explicitly says to start directly with current assumptions.

## Scope Boundary

Allowed:

- Clarify goal, scope, success criteria, assumptions, unknowns, invariants, failure modes, and human decisions.
- Inspect only the code, docs, issues, tests, and logs needed to confirm direct facts.
- Produce a discussion brief, decision matrix, three-option comparison, red-team notes, issue draft, ADR draft, or implementation handoff.

Forbidden:

- Do not edit product code, migrations, tests, schemas, or runtime behavior while this Skill is active.
- Do not invent new abstractions, compatibility layers, rollback systems, provenance, migrations, or repo-wide refactors unless they are explicitly one of the options for user approval.
- Do not turn a narrow request into a roadmap, platform redesign, or cleanup campaign.

## Convergence Budget

- Read current request, nearest relevant AGENTS / README / docs, and directly implicated code paths only.
- Trace one layer of adjacent blast radius; stop before second-order roadmap work.
- Ask at most 3 blocking questions, all in one batch.
- For any non-trivial decision with multiple viable directions or data / contract / architecture risk, offer exactly 3 options: conservative, balanced, aggressive.
- Keep recommendations tied to evidence. Mark unsupported claims as assumptions.

## Workflow

1. Collect facts: separate confirmed facts, assumptions, unknowns, invariants, failure modes, and human decisions.
2. Split concepts: identify mixed concepts before naming APIs, services, enums, directories, or migrations.
3. Build a matrix when the task touches defaults, contracts, schema, state, permissions, migration, history, or user content. Use `references/domain-matrix.md`.
4. Present options when more than one direction is valid or the task has data / contract / architecture risk. Use exactly conservative / balanced / aggressive; do not collapse to a single best answer before user approval.
5. Red-team the recommended option before asking the user to approve it. Use `references/options-and-red-team.md`.
6. Stop at a decision artifact. Use `references/artifacts.md` for brief, issue, ADR, or implementation handoff formats.

## User Decision Format

When asking the user to choose or approve, use this format:

- `现状`: What is confirmed, what is uncertain, and why the decision matters.
- `方向`: The viable direction or option.
- `风险收益`: Concrete upside, downside, hidden cost, and failure mode.
- `建议`: Give a clear recommendation first, then list what the user must approve.

For three-option decisions, repeat the four-part format for each option and finish with one recommended option.

## Stop Conditions

Stop and wait for user approval when any condition is met:

- A discussion brief, issue draft, ADR draft, or implementation handoff is ready.
- Three options plus a clear recommendation have been presented.
- Blocking decisions have been reduced to at most 3 questions.
- Evidence is insufficient to distinguish options safely.
- The user rejects or changes a core assumption.

## Handoff Rules

After approval, switch to the relevant implementation Skill:

- Frontend surface, interaction, UI structure: `frontend-development`.
- Backend API, state, write path, domain boundary: `backend-development`.
- Testable behavior change: `test-driven-development`.
- Self-check, regression, delivery evidence: `qa-evaluation`.

Implementation must follow the approved artifact. Do not expand scope during implementation; unresolved decisions go back through this Skill.
