---
memory_type: project
topic: 系统默认值升级策略收口为系统版本加升级策略
summary: 用户确认系统默认值不维护 per-default version、手写 hash 或 provenance；默认值集中到后端系统默认值目录，版本使用系统信息版本，升级时实时读取数据库值计算匹配，历史处理由 upgrade_policy 和字段是否开放给用户决定。第一轮已在 latest 合并后端 system_defaults、默认值升级账本迁移和前端默认文档漂移防线。
keywords:
  - system-defaults
  - upgrade-policy
  - defaults
  - provenance
  - backend
match_when:
  - 需要设计或实现系统默认值目录
  - 需要判断默认值升级是否覆盖历史数据
  - 需要处理内置节点默认参数、默认 Flow 文档或供应商主实例默认策略
created_at: 2026-05-20 16
updated_at: 2026-05-20 17
last_verified_at: 2026-05-20 17
decision_policy: verify_before_decision
scope:
  - api
  - web
  - https://github.com/taichuy/1flowbase/issues/341
---

# 系统默认值升级策略收口为系统版本加升级策略

## 时间

`2026-05-20 16`

## 谁在做什么

用户与 AI 正在为系统默认值、默认 Flow 文档、内置节点契约和供应商主实例默认策略设计统一后端真值与升级维护机制。

## 为什么这样做

默认值不是实体类，也不是可频繁调的运行时配置；它们应集中在后端稳定系统默认值目录中。此前方案中 per-default version、手写 hash、字段级 provenance 过重且职责不合理。

## 为什么要做

默认值会随系统升级变化，但历史数据是否更新不能靠前端兜底或隐式猜测。需要用统一规则说明哪些系统维护默认可升级，哪些开放给用户的默认只影响新建，不改历史。

## 截止日期

未指定。

## 决策背后动机

- 默认值版本不单独维护，使用系统信息版本，例如 API server health/runtime profile 暴露的系统版本。
- 默认值不维护手写 hash；升级命令执行时读取数据库当前值，按 canonical JSON、深比较或运行时 hash 判断是否命中旧系统默认。
- 不引入 provenance；不在 Flow document 或业务记录里写字段级默认来源。
- 保留 `upgrade_policy`，并用它标记默认值历史处理方式。
- 系统内部维护的默认值可以由升级任务自动更新历史。
- 开放给用户编辑的默认值只影响新建对象，历史值不自动覆盖，除非显式迁移。
- 简单默认值用 Rust 常量；结构化默认值通过后端 `system_defaults` 模块的函数或 registry 暴露。

## 2026-05-20 17 第一轮实现状态

- 已在 `latest` 合并 `api/crates/domain/src/system_defaults/`，默认 Flow 文档由后端 system defaults 生成。
- 已新增 `system_default_upgrade_runs` / `system_default_upgrade_items` PostgreSQL 迁移账本。
- 已补前端默认 Flow 文档漂移测试，保持当前前端默认文档、node factory 和 runtime contract 对齐。
- 已把供应商新实例默认加入主实例的 fallback 改为消费 `domain::DEFAULT_AUTO_INCLUDE_NEW_PROVIDER_INSTANCES`。
- #345、#346、#347 已关闭；#341 继续保留作为总任务，后续做 node contract API、upgrade preview/apply service、provider 参数默认归一。
