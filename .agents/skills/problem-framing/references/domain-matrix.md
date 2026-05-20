# Domain Matrix

当任务涉及 defaults、contracts、schema、state、permissions、migration、historical data、runtime behavior 或 user-owned content 时，使用本参考。

## Required Columns

| Object / field / behavior | Owner | Source of truth | Persisted? | User editable? | Runtime contract? | Historical data impact | Required evidence | Unacceptable failure mode |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
|  |  |  |  |  |  |  |  |  |

## Rules

- 在设计 API、service、enum、目录结构、migration 或 upgrade command 前，先填写矩阵。
- 未知项标为 `unknown`；不要把未知项改写成设计结论。
- 如果某一行涉及用户内容或历史数据影响，进入实现前必须让用户显式批准。
- 如果 source of truth 不清楚，停止并请求决策，不要自行添加兼容代码。

## Common Rows

- 前端展示 fallback
- 后端默认值
- 已落库的用户设置
- 运行时 contract
- 数据库 migration
- audit / preview / rollback 行为
- 权限或策略决策
- 生成或导入的系统内容
