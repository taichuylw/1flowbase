---
memory_type: feedback
feedback_category: repository
topic: console resource CRUD reuse
summary: 后台资源不应每次从零手写 CRUD；优先沉淀可复用的资源 CRUD/kernel 能力，但不能裸暴露物理表。
keywords:
  - console resource CRUD
  - batch delete
  - filter
  - filterByTk
  - NocoBase
  - Resource Action Kernel
match_when:
  - 设计或实现后台资源列表、检索、批量操作、CRUD 接口
  - 为每个物理表或控制台资源新增手写定制 CRUD route/service/repository
created_at: 2026-05-20 08
updated_at: 2026-05-20 08
last_verified_at: 无
decision_policy: direct_reference
scope:
  - api/
  - web/packages/api-client/
---

# 后台资源 CRUD 复用

## 时间

`2026-05-20 08`

## 规则

后台资源的检索、分页、排序、单条删除和批量删除等通用 CRUD 能力应优先复用或沉淀成 Resource CRUD/kernel 形态，避免每个资源从零手写一套定制 CRUD。

复用边界应以业务资源为单位，而不是把物理表作为外部 API 直接裸暴露；物理表仍是实现细节，权限、审计、状态校验、保护规则和业务 hook 必须留在资源边界内。

列表筛选接口优先参考 NocoBase 的 `filter` 语义，而不是为每个资源发明 `search` 这类定制参数；选择态批量操作优先使用 `filterByTk`，需要按条件批量操作时再使用 `filter`。

## 原因

用户认为每个物理表/资源都重新写 CRUD 会导致重复、定制化和不一致；但直接按表暴露会绕过数据一致性、状态一致性和权限审计。

## 适用场景

新增或调整 console 后台资源接口时，优先考虑标准 `list/filter/get/create/update/delete/batch_delete` 能力，并把真正的业务动作单独作为定制 action。
