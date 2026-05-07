---
memory_type: project
topic: 数据源默认状态语义
summary: 数据源默认状态是未来新建 Data Model 的默认值，单个 Data Model 仍支持独立修改；修改数据源默认值不回写既有 Data Model。
keywords:
  - data-source-defaults
  - data-model-status
  - api-exposure-status
  - main-source
match_when:
  - 调整 Data Model 创建默认状态、API 暴露默认状态或数据源设置页交互
  - 设计主数据源与外部数据源的默认值继承规则
created_at: 2026-05-07 12
updated_at: 2026-05-07 12
last_verified_at: 2026-05-07 12
decision_policy: verify_before_decision
scope:
  - api/crates/control-plane
  - api/crates/storage-durable/postgres
  - api/apps/api-server/src/routes/plugins_and_models/data_sources.rs
  - web/app/src/features/settings/pages/settings-page
---

# 数据源默认状态语义

## 时间

`2026-05-07 12`

## 谁在做什么

用户确认 Data Source 的默认 Data Model 状态和默认 API 暴露状态应该是可控配置；实现侧需要让主数据源也能编辑默认值，并让 Data Model 创建流程读取对应默认值。

## 为什么这样做

数据源默认值表达的是“未来从这个数据源创建 Data Model 时采用什么初始状态”，不是批量修改既有 Data Model 的状态入口。

## 为什么要做

主数据源创建表时不自动暴露 API 或是否默认发布，属于工作区建模策略，应由用户可控；同时既有表可能已经进入不同生命周期，不能因为默认值调整被隐式覆盖。

## 截止日期

无固定截止日期；作为当前产品语义持续沿用。

## 决策背后动机

保持两层状态边界清晰：Data Source 负责默认策略，Data Model 负责自身实际状态。这样既能统一新建表的初始体验，也避免破坏既有表的独立配置和 API 暴露状态。

## 关联文档

- `api/apps/api-server/src/routes/plugins_and_models/data_sources.rs`
- `api/crates/control-plane/src/model_definition/service.rs`
- `web/app/src/features/settings/pages/settings-page/SettingsDataModelsSection.tsx`
