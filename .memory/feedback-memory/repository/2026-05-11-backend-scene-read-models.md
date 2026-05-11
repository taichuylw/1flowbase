---
memory_type: feedback
feedback_category: repository
topic: backend-scene-read-models
summary: 前端不应在多个业务场景里自行拉取通用后端 DTO 后重复拼装、转换或派生业务状态；应优先由后端提供场景化 read model / view API。
keywords:
  - frontend api consumption
  - backend read model
  - scene api
  - dto mapping
  - maintenance boundary
match_when:
  - 评估或实现前端依赖多个接口拼装数据
  - 前端把后端通用 DTO 转成业务场景状态
  - 同一份后端数据被多个页面各自转换或解释
  - 设计新的 console / agent-flow / settings 读取接口
created_at: 2026-05-11 00
updated_at: 2026-05-11 00
last_verified_at: 2026-05-11 00
decision_policy: direct_reference
scope:
  - api
  - web/app/src/features
  - web/packages/api-client
---

# Backend Scene Read Models

## 规则

前端不应在多个业务场景里自行拉取通用后端 DTO 后重复拼装、转换或派生业务状态。跨场景可复用或带业务语义的组合数据，应优先由后端封装为场景化 read model / view API，再由前端消费。

## 原因

如果前端在不同页面各自解释同一份后端数据，后端字段、状态语义、i18n 规则或聚合关系变化时，容易出现一个前端场景已更新、另一个场景仍沿用旧逻辑的问题，后期维护成本会上升。

## 适用场景

- 前端从多个接口拉取数据后按业务对象聚合，例如供应商、插件、实例、模型、权限等。
- 前端根据后端状态字段派生业务可用性、禁用原因、展示名称或默认行为。
- 多个页面或组件需要同一类业务场景数据，但当前只能拿通用 DTO 自行转换。

## 推荐边界

- 后端：负责权限过滤、业务状态派生、跨资源聚合、i18n 解析、场景专用字段和稳定契约。
- 前端：负责查询触发、缓存 key、表单临时值、排序筛选、局部展示格式和用户交互状态。
