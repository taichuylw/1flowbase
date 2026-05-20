# Options And Red Team

Use this reference when more than one direction is viable or when the task affects architecture, state, contracts, data history, or user-owned content.

## Three-Option Template

### 方案 A：保守

**现状**
State confirmed facts, unknowns, and why this option exists.

**方向**
Describe the smallest change that protects current behavior.

**风险收益**
Name the upside, downside, hidden maintenance cost, and worst failure mode.

**建议**
Say when to choose this option and whether you recommend it.

### 方案 B：平衡

**现状**
State the facts and uncertainty this option balances.

**方向**
Describe the focused product or architecture improvement.

**风险收益**
Name the upside, downside, hidden maintenance cost, and worst failure mode.

**建议**
Say when to choose this option and whether you recommend it.

### 方案 C：激进

**现状**
State what current constraints this option challenges.

**方向**
Describe the broader redesign or cleanup.

**风险收益**
Name the upside, downside, hidden maintenance cost, and worst failure mode.

**建议**
Say when to choose this option and whether you recommend it.

## Recommendation Block

End with:

```md
我的建议：选择方案 B。

原因：
- It best protects the critical invariant.
- It keeps the implementation reviewable.
- It avoids paying for a broader abstraction before evidence exists.

需要你拍板：
- 选择 A / B / C。
- 哪条风险绝对不能接受。
- 哪条边界本轮不能碰。
```

## Red-Team Pass

Before asking for approval, attack the recommended option:

- Concept confusion: Are two different things being merged under one name?
- User data: Could user-owned content be overwritten, reinterpreted, or silently changed?
- History: Could historical data be migrated without enough evidence?
- Runtime behavior: Could this create a silent behavior change?
- Testability: Can the safety claim be proven by tests or preview evidence?
- Scope: Is this option smuggling in unrelated cleanup or future roadmap work?

If red-team finds a serious issue, revise the option or stop with a decision question.
