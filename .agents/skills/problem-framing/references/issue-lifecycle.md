# Issue Lifecycle

需求进入实现前，先把 issue 当成可检索、可验收、可关闭的工作单元整理清楚。普通需求也要先对齐；只有用户明确说直接开始 / 无需确认，才跳过确认。

## Requirement Alignment

普通需求先输出简短对齐，给出 2-3 个轻量做法，明确推荐其中一个，并等待用户确认：

```md
现状
- 已知事实：
- 不确定点：

方向
- 做法 A：
- 做法 B：
- 做法 C（可选）：
- 不做什么：

风险收益
- 收益：
- 风险：
- 验证方式：

建议
- 我的建议：
- 需要你确认：
```

高风险、多方向或影响数据 / contract / 架构的需求，升级为 conservative / balanced / aggressive 三方案。

## Issue Grades

| Grade | Name | Use When | Required Artifact | Close Condition |
| --- | --- | --- | --- | --- |
| G0 | Direct Task | 纯查询、机械精确改动、用户明确要求直接开始 | 无 issue；在最终回复写明跳过原因 | 命令或精确改动完成 |
| G1 | Simple Requirement | 单一页面、接口、局部 bugfix、轻量流程调整，无数据 / contract / migration 风险 | 2-3 个轻量做法 + Issue Draft | 定向验证通过，用户确认 |
| G2 | Standard Change | 涉及一个子系统的功能、缺陷、重构或行为变化，需要测试 | Discussion Brief + Issue Draft + Implementation Handoff | 测试 / QA 证据通过，用户验收 |
| G3 | Cross-Domain Decision | 跨 frontend/backend、状态入口、schema、权限、runtime contract 或多模块影响 | 三方案 + Issue Draft + Implementation Handoff | 子 issue 验证通过，总 issue 用户验收 |
| G4 | Architecture / Data Risk | 影响历史数据、migration、用户内容、核心 contract、ADR 或不可逆决策 | Domain Matrix + Red Team + ADR Draft + Issue Draft | ADR 批准、preview/rollback 证据、用户验收 |

## Required Labels

每个需要创建的 issue 必须至少包含这些标签：

```text
type:<feature|bug|refactor|design|docs|chore|qa|spike>
area:<frontend|backend|api|schema-ui|runtime|workflow|settings|infra|docs|test>
grade:<g1|g2|g3|g4>
priority:<p0|p1|p2|p3>
risk:<low|medium|high>
size:<xs|s|m|l>
phase:<discussion|ready|implementation|qa|user-acceptance|closed>
```

## Optional Labels

按需添加，不要为了凑标签而添加：

```text
needs-decision
needs-adr
needs-design
needs-frontend
needs-backend
needs-qa
blocked
contract
migration
user-data
breaking-change
security
performance
regression
parent-issue
child-issue
```

## Label Rules

- `type:*` 描述工作性质，只选一个主类型。
- `area:*` 描述主要影响面；跨域 issue 可用多个 `area:*`。
- `grade:*` 必须和 Issue Grades 对齐。
- `priority:*` 表示业务或交付紧急度，不代表技术难度。
- `risk:*` 表示错误后果；涉及用户数据、migration、contract 默认不低于 `risk:high`。
- `size:*` 表示 review 和实现规模；超过 `size:m` 时优先拆子 issue。
- `phase:*` 随流程更新，只保留当前阶段。

## Lifecycle

1. `phase:discussion`：需求对齐，输出简短对齐或三方案，等待用户确认。
2. `phase:ready`：issue 内容、分级、标签、验收证据已确认，可以进入实现。
3. `phase:implementation`：按批准范围实现；发现新决策时回到 `problem-framing`。
4. `phase:qa`：实现完成，进入 `qa-evaluation` 收集证据。
5. `phase:user-acceptance`：交付用户验收；总 issue 不得在此阶段前关闭。
6. `phase:closed`：用户确认通过；子 issue 可在自身验证通过后关闭，总 issue 必须等用户人工验收。

## Split Rules

- 一个 issue 只承载一个可验收目标。
- G3 / G4 默认拆 parent issue + child issues；parent 管决策和验收，child 管具体实现。
- 子 issue 必须写清 parent issue，并继承相关 `area:*`、`risk:*` 和 `grade:*`。
- 子 issue 完成不代表总 issue 完成；总 issue 关闭条件永远是用户验收通过。
