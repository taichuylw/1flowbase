---
title: Application run log DB query guide issue
created_at: 2026-06-12 00
updated_at: 2026-06-12 00
decision_policy: verify_before_decision
status: reference_issue_created
tags:
  - application-run-logs
  - runtime-observability
  - github-issue
links:
  - https://github.com/taichuy/1flowbase/issues/871
---

# Application Run Log DB Query Guide Issue

用户在 `2026-06-12 00` 要求把应用运行日志的接口、文件、表和 SQL 查询顺序挂到 GitHub issue，方便以后排查时检索。

已创建 GitHub issue #871：`[文档]应用运行日志数据库排查入口与 SQL 手册`。

该 issue 是排查手册，不代表产品实现任务。内容覆盖 `/applications/{application_id}/logs` 相关 console API、前端入口、后端 handler、Postgres 仓储文件、迁移文件、核心表，以及从候选会话/运行 id 解析到 `flow_run_id` 后继续查询 `flow_runs`、`node_runs`、`flow_run_events`、`flow_run_checkpoints`、`flow_run_callback_tasks`、`application_conversation_messages`、`runtime_events`、`runtime_spans`、`runtime_usage_ledger` 等表的 SQL 顺序。

由于 `taichuy/1flowbase` 是公开仓库，issue 中使用占位符，没有写入本轮真实应用 id、会话 id、数据库凭据、用户输入或模型输出。
