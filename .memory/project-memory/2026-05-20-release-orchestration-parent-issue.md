---
memory_type: project
topic: 系统升级治理拆成默认值升级和发布编排两层 issue
summary: 用户确认在 #341 之外新增 #344 作为系统升级编排、备份、回滚与发布兼容窗口的总 issue；#341 只负责 system defaults upgrade，#344 负责 release orchestration、preflight、backup、upgrade lock、health check、rollback CLI 和 expand/migrate/contract。
keywords:
  - release-orchestration
  - system-defaults
  - upgrade
  - rollback
  - backup
match_when:
  - 需要拆分或实现系统升级、默认值升级、回滚、备份、发布兼容窗口相关任务
  - 需要判断 #341 和 #344 的职责边界
created_at: 2026-05-20 16
updated_at: 2026-05-20 16
last_verified_at: 2026-05-20 16
decision_policy: verify_before_decision
scope:
  - https://github.com/taichuy/1flowbase/issues/341
  - https://github.com/taichuy/1flowbase/issues/344
  - api
  - web
---

# 系统升级治理拆成默认值升级和发布编排两层 issue

## 时间

`2026-05-20 16`

## 谁在做什么

用户与 AI 根据 `docs/draft/sys_vs.md` 的建议，把系统升级治理拆成父子 issue：#341 继续承载 system defaults upgrade，新增 #344 承载 release orchestration。

## 为什么这样做

#341 解决的是默认值、内置节点契约、历史默认值升级策略；它不能单独承诺完整的用户升级失败回滚。完整回滚还需要 preflight、backup checkpoint、upgrade lock、health check、CLI rollback 和发布兼容窗口。

## 为什么要做

如果把 release orchestration 全塞进 #341，会导致 system defaults 方案过大、边界混乱；如果不单独规划 #344，用户升级失败时会缺少备份、状态追踪、健康检查和回滚路径。

## 截止日期

未指定。

## 决策背后动机

- #341 定位为 Layer 2：system_defaults 后端真值、upgrade_policy、preview/apply、system_default_upgrade_runs/items、默认值历史处理。
- #344 定位为 Layer 3：release orchestration、preflight、backup checkpoint、upgrade lock、SQL migration status、health check、rollback CLI、Flow protected snapshot、expand/migrate/contract 发布兼容窗口。
- #341 不承诺完整系统回滚，只为 #344 提供默认值升级的 run/items 依据。
- 完整用户升级失败恢复能力由 #344 统一设计。
