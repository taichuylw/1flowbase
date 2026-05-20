# Examples

## Vague Feature Request

用户说：“优化一下 Settings 页面流程。”

回复形态：

```md
现状
- 这个请求会改变产品流程，但还没有定义主操作和成功标准。
- Settings 已有共享 surface 和紧凑运营型 UI 的项目规则。

方向
- 先收敛目标、受影响的 settings 区域、用户主路径和成功标准。

风险收益
- 收益：避免重设计无关 settings surface。
- 风险：跳过收敛会让 UI 改动未经批准地改变导航或状态归属。

建议
- 建议先产出短 discussion brief，用户确认后再交给 `frontend-development`。
```

## Ordinary Requirement

用户说：“给列表加一个按更新时间排序。”

回复形态：

```md
现状
- 这是一个普通需求，但仍会影响列表默认行为和用户预期。
- 当前还没确认是默认排序、可选排序，还是只在某个页面生效。

方向
- 做法 A：只在目标列表增加可选排序。
- 做法 B：把更新时间作为目标列表默认排序。
- 做法 C：抽成通用列表排序能力，供同类列表复用。

风险收益
- 收益：实现范围小，验证清晰。
- 风险：如果不确认作用范围，可能误改全局列表行为。

建议
- 建议选做法 A：只在目标列表增加可选排序，不改默认排序。需要你确认作用页面和是否改变默认行为。
```

## Cross-Domain Architecture Request

用户说：“统一 defaults，修掉 frontend/backend 漂移。”

回复形态：

```md
现状
- 这会影响前端 fallback、后端 defaults、运行时 contracts、已落库数据和可能的 migration 行为。
- 即使都被叫作 defaults，它们也不是同一个概念。

方向
- 在设计 API 或 migration 前，先建立 domain matrix。
- 对比 conservative / balanced / aggressive 三个方案。

风险收益
- 收益：保护用户历史数据和运行时 contracts。
- 风险：扩大清理范围可能引入 silent behavior change。

建议
- 建议在矩阵证明 source of truth 和历史影响后，再选择 balanced 方案。
```
