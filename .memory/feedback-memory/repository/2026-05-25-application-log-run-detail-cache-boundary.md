---
memory_type: feedback
feedback_category: repository
topic: application-log-run-detail-cache-boundary
summary: 应用日志 run detail 低频且可能很大，不应整包进入易失缓存；优先缓存 summary page 和轻量元数据。
keywords:
  - application-logs
  - run-detail
  - cache
  - memory-observation
  - runtime-debug-artifact
created_at: 2026-05-25 10
updated_at: 2026-05-25 10
last_verified_at: 2026-05-25 10
decision_policy: direct_reference
scope:
  - api/apps/api-server/src/routes/applications/application_runtime.rs
  - api/apps/api-server/src/routes/applications/application_runtime/application_log_cache.rs
---

# Application Log Run Detail Cache Boundary

## 规则

应用日志 `run-detail` 属于低频查看的大对象，不应把完整详情响应整包写入易失缓存。应用日志缓存优先覆盖 `summary-page` 这类高频列表读取；如果后续需要优化详情性能，优先设计 lightweight detail / metadata 缓存，完整 payload、events、node_runs、debug artifact 继续走 durable store 和 artifact 懒加载。

## 原因

用户在内存观察中看到单条 `application-logs:run-detail` 缓存达到 MB 级后确认：运行详情通常只由创建者打开一次，重复读取概率低；详情已经有懒加载和分层查询方向，整包缓存收益低且会污染内存观察视图。

## 适用场景

- 修改应用日志详情接口、缓存策略或内存观察展示。
- 为运行详情、debug artifact、node run/event payload 设计缓存。
- 评估大对象是否进入 Moka / Redis 等易失层。
