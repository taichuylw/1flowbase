---
memory_type: project
topic: 应用运行详情改造限定在查询链路
summary: 用户确认应用日志运行详情必须完全迁到 lazy trace tree：日志投影/read model 是保护层，允许在日志层动刀并建立索引化树读模型；日志优化不得把运行真值层、执行状态流或 runtime 写路径变成日志 UI 的可变实现面。
keywords:
  - application-run-detail
  - application-logs
  - lazy-loading
  - query-path
  - read-model
  - console-api
  - frontend-backend
  - issue-979
match_when:
  - 继续实现或评审应用日志运行详情懒加载
  - 调整 /api/console/applications/:id/logs/runs 相关查询接口
  - 讨论 application run detail 返回体瘦身、trace 按需加载或消息详情查询
created_at: 2026-06-17 00
updated_at: 2026-06-18 15
last_verified_at: 2026-06-18 15
decision_policy: verify_before_decision
scope:
  - https://github.com/taichuy/1flowbase/issues/979
  - api/apps/api-server/src/routes/applications/application_runtime
  - api/crates/storage-durable/postgres/src/orchestration_runtime_repository
  - web/app/src/features/applications
  - web/app/src/features/agent-flow
---

# 应用运行详情完全迁到 Lazy Trace Tree

## 时间

`2026-06-18 01`

## 谁在做什么

用户确认当前应用日志运行详情性能问题只在查询链路内解决，但方向不是“请求体瘦身”：GitHub issue #979 需要完全迁到 lazy trace tree，日志读路径不保留旧 full detail GET，前端按根树、展开子节点、加载节点内容三段懒加载消费。

## 为什么这样做

当前 `/api/console/applications/:id/logs/runs/:run_id` 在消息详情链路返回过大的响应体，而且“瘦身但仍一次读全”不能解决多级 trace 展开需求；问题集中在查询接口形态和展示读取方式，不在执行引擎或业务状态本身。

## 为什么要做

需要让控制台日志详情按用户展开路径获取内容：根树只给 summary，展开节点再取 children，点开节点再取 content，避免工作流复杂度上升时日志详情请求体线性膨胀，同时不把问题扩大成业务逻辑或运行时行为改造。

## 截止日期

无固定截止日期；GitHub issue #979 当前为 `phase:discussion`，待用户确认后可进入实现。

## 决策背后动机

用户明确要求“不动业务本身”，并进一步要求“不用保留，直接完全迁到 lazy tree”。因此实现时不得顺手调整执行逻辑、状态流转、callback 语义、持久化业务规则或其他非查询职责；日志读路径也不得继续依赖旧 full detail GET。

## 2026-06-18 15 边界校正

用户纠正：日志投影/read model 是特意划分出来的保护层。优化可以在日志层动刀，包括建立可索引的 Trace Tree read model、调整日志 DTO/API 和前端消费；但运行真值层不应为日志详情 UI 重构而改变，不能把 runtime truth、执行状态写路径或 callback 语义直接变成日志树实现面。

## 2026-06-18 15 #981 设计收敛

用户确认 #981 按日志投影保护层方案更新：Trace Tree read model 拆 `application_run_trace_nodes` 与 `application_run_trace_node_contents`；stable locator 由日志层基于 source component 生成 deterministic locator；stitched trace 在 root 暴露 collapsed context group，子树按需展开；projection 触发采用 terminal 后 eager projection + first-read bounded repair/backfill + active-run watermark refresh 的组合策略。

projection 失败必须有可诊断状态，不得静默 fallback 到 full detail；projection version 升级默认按需 rebuild + 后台 rolling rebuild，不默认批量重建全部历史，只有破坏性 schema/权限/安全/关键字段语义错误或用户明确要求时才做显式维护型批量重建。
