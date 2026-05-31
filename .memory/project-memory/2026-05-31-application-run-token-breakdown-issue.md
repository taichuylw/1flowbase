---
memory_type: project
topic: 应用日志和监控 tokens 拆分已挂 issue
summary: 用户确认应用日志和应用监控增加输入 tokens、输出 tokens、命中缓存 tokens 的 Balanced 方案；已创建 GitHub issue #556，核心边界是三类字段必须从后端接口返回，持久化到 application_run_log_summaries，运行结束后只读投影表读取，前端不得从 total_tokens 推算。
keywords:
  - application-run-logs
  - application-monitoring
  - token-breakdown
  - input_tokens
  - output_tokens
  - input_cache_hit_tokens
  - application_run_log_summaries
  - issue-556
match_when:
  - 继续实现或评审应用日志 tokens 拆分
  - 继续实现或评审应用监控 Tokens 类目
  - 涉及 application_run_log_summaries tokens 字段、历史回填或只读投影约束
created_at: 2026-05-31 23
updated_at: 2026-05-31 23
last_verified_at: 2026-05-31 23
decision_policy: verify_before_decision
scope:
  - https://github.com/taichuy/1flowbase/issues/556
  - api/crates/storage-durable/postgres/migrations
  - api/crates/storage-durable/postgres/src/orchestration_runtime_repository
  - api/apps/api-server/src/routes/applications/application_runtime
  - web/app/src/features/applications
---

# 应用日志和监控 tokens 拆分已挂 issue

## 时间

`2026-05-31 23`

## 谁在做什么

用户确认将应用日志 `/applications/:id/logs` 和应用监控 `/applications/:id/monitoring` 的 tokens 指标拆成输入 tokens、输出 tokens、命中缓存 tokens，并已挂到 GitHub issue #556。

## 为什么这样做

现有应用日志和监控已有 `total_tokens`，但用户更关心 `input_tokens`、`output_tokens`、`input_cache_hit_tokens` 三个具体指标。底层 `runtime_usage_ledger` 已具备这些字段来源，应用层缺少运行结束后的只读投影和接口展示。

## 为什么要做

该改动要让日志列表和监控 Tokens 类目能直接解释模型调用成本结构，同时保持后端作为唯一数据来源，避免前端从总 tokens 派生或兼容字段。

## 截止日期

无固定截止日期；issue #556 当前为 `phase:ready`，可进入实现。

## 决策背后动机

用户明确要求数据必须从后端接口出来，并且必须持久化到日志表里，运行完之后只读读取。旧数据若无法拆分，允许展示 `null / -`；有 `runtime_usage_ledger` 的历史记录应通过 migration 回填。

## 关联文档

- #556 `[待开发]应用日志和应用监控增加输入/输出/缓存命中 tokens`
