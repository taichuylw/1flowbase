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

`grade:*` 表示风险与规划强度，不表示父子结构。父子结构使用 `level:*`。

| Grade | Name | Use When | Required Artifact | Close Condition |
| --- | --- | --- | --- | --- |
| G0 | Direct Task | 纯查询、机械精确改动、用户明确要求直接开始 | 无 issue；在最终回复写明跳过原因 | 命令或精确改动完成 |
| G1 | Simple Requirement | 单一页面、接口、局部 bugfix、轻量流程调整，无数据 / contract / migration 风险 | 2-3 个轻量做法 + Issue Draft | 定向验证通过，用户确认 |
| G2 | Standard Change | 涉及一个子系统的功能、缺陷、重构或行为变化，需要测试 | Discussion Brief + Issue Draft + Implementation Handoff | 测试 / QA 证据通过，用户验收 |
| G3 | Cross-Domain Decision | 跨 frontend/backend、状态入口、schema、权限、runtime contract 或多模块影响 | 三方案 + Issue Draft + Implementation Handoff | 子 issue 验证通过，总 issue 用户验收 |
| G4 | Architecture / Data Risk | 影响历史数据、migration、用户内容、核心 contract、ADR 或不可逆决策 | Domain Matrix + Red Team + ADR Draft + Issue Draft | ADR 批准、preview/rollback 证据、用户验收 |

## Issue Hierarchy Levels

`level:*` 表示 issue 在工作树中的结构位置。每一层可以有多个 sibling issue；它们的下一层 issue 必须把上一层 issue 写作 Parent，并加 `child-issue`。

| Level | Name | Use When | Owns | Child Level |
| --- | --- | --- | --- | --- |
| L0 | Initiative / Umbrella Issue | 项目级总问题，横跨多个决策、epic 或 workstream | 战略目标、范围边界、总验收 | L1 |
| L1 | Decision Issue / ADR | 架构决策、contract、source of truth、不可逆方向 | 已批准决策、ADR、约束和停止条件 | L2 |
| L2 | Epic / Workstream Issue | 按 backend / frontend / QA / migration 等工作流拆分 | 子系统目标、交付边界、验收证据 | L3 |
| L3 | Execution Task Issue | 单个可执行开发、测试、修复或文档任务 | 具体代码或验证任务 | None |

Rules:

- `level:*` 和 `grade:*` 必须同时判断：`level` 管结构位置，`grade` 管风险与规划强度。
- L0 / L1 / L2 可以有多个 child issue；L3 不再拆 child，除非先升级为 L2。
- Child issue 必须在正文写 `Parent issue: #<number>`，并继承必要 `area:*`、`risk:*`、`contract` / `migration` 等语义标签。
- Parent issue 必须维护 child issue 列表；child 完成不代表 parent 完成。
- L1 决策 issue 不直接承载大段实现；批准后拆 L2 / L3。
- 没有 L0 时，L1 可以临时作为最高层 parent，但不得改称 L0。

## Required Labels

每个需要创建的 issue 必须至少包含这些标签：

```text
type:<feature|bug|refactor|design|docs|chore|qa|spike>
area:<frontend|backend|api|schema-ui|runtime|workflow|settings|infra|docs|test>
level:<l0|l1|l2|l3>
grade:<g1|g2|g3|g4>
priority:<p0|p1|p2|p3>
risk:<low|medium|high>
size:<xs|s|m|l>
phase:<discussion|ready|implementation|qa|user-acceptance|closed>
```

## Issue Title

issue 标题必须使用：

```text
[状态]标题
```

状态必须和 `phase:*` 标签同步：

| Phase Label | Title Status | Example |
| --- | --- | --- |
| `phase:discussion` | `[讨论]` | `[讨论]给列表增加更新时间排序` |
| `phase:ready` | `[待开发]` | `[待开发]给列表增加更新时间排序` |
| `phase:implementation` | `[开发中]` | `[开发中]给列表增加更新时间排序` |
| `phase:qa` | `[验收中]` | `[验收中]给列表增加更新时间排序` |
| `phase:user-acceptance` | `[待确认]` | `[待确认]给列表增加更新时间排序` |
| `phase:closed` | `[已完成]` | `[已完成]给列表增加更新时间排序` |

标题只放状态和可读标题；分级、类型、影响面、父子关系放 labels 和正文，不塞进标题。

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
- `level:*` 描述 issue 在父子工作树中的结构位置；和 `grade:*` 独立。
- `grade:*` 必须和 Issue Grades 对齐。
- `priority:*` 表示业务或交付紧急度，不代表技术难度。
- `risk:*` 表示错误后果；涉及用户数据、migration、contract 默认不低于 `risk:high`。
- `size:*` 表示 review 和实现规模；超过 `size:m` 时优先拆子 issue。
- `phase:*` 随流程更新，只保留当前阶段。
- 更新 `phase:*` 时，同步更新标题前缀 `[状态]`。

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
- 拆分时优先按 L0 -> L1 -> L2 -> L3 建树；每一层可以有多个 sibling issue。
- 上一层 issue 对应下一层 child issues：L0 只挂 L1，L1 只挂 L2，L2 只挂 L3；不要跨层挂载，除非中间层确实没有必要并在正文说明。
- 子 issue 必须写清 parent issue，并继承相关 `area:*`、`risk:*` 和 `grade:*`。
- 子 issue 完成不代表总 issue 完成；总 issue 关闭条件永远是用户验收通过。
