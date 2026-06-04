---
memory_type: project
topic: application-publication-current-only
summary: 应用公开 API 发布语义收口为 current publication，不保留发布历史。
keywords:
  - application_publication_versions
  - current publication
  - 发布
  - 草稿
  - 固定版本
match_when:
  - 调整应用公开 API 发布、发布快照、flow_versions 裁剪或 publication 数据迁移时。
  - 讨论“发布历史”“当前发布”“固定版本”“草稿”之间的数据语义时。
created_at: 2026-06-04 12
updated_at: 2026-06-04 12
last_verified_at: 2026-06-04 12
decision_policy: verify_before_decision
scope:
  - api/crates/control-plane/src/application_public_api
  - api/crates/storage-durable/postgres/src/application_public_api_repository
  - api/crates/storage-durable/postgres/src/flow_repository
  - api/crates/storage-durable/postgres/migrations
---

# 应用发布只保留当前发布

## 时间

`2026-06-04 12`

## 谁在做什么

用户确认 1flowbase 应用公开 API 发布不需要保留发布历史；Codex 在 #671 中按该语义实现 current publication 写入、历史数据迁移和草稿保存裁剪修复。

## 为什么这样做

旧模型让 `application_publication_versions` 保留多条发布历史，并以外键引用 `flow_versions`。当草稿保存触发 `flow_versions` 历史裁剪时，旧发布历史引用会阻止删除，导致保存接口返回 500。

## 为什么要做

产品上只需要三类状态：草稿、发布、固定。发布是当前线上快照，重新发布应更新当前发布；长期保留由用户主动固定的 `flow_versions` 承担。

## 截止日期

无固定截止日期；#671 已进入用户验收。

## 决策背后动机

减少发布历史带来的数据状态分叉，让数据库约束表达“每个应用最多一个当前发布”，并避免历史发布记录阻塞版本裁剪。

## 关联文档

- GitHub issue: https://github.com/taichuy/1flowbase/issues/671
