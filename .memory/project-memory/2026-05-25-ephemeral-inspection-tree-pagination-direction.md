---
memory_type: project
title: 易失层内存观察改为树状路径与分页方向
created_at: 2026-05-25 20
updated_at: 2026-05-25 20
decision_policy: verify_before_decision
scope:
  - api/crates/storage-ephemeral
  - api/crates/control-plane/src/ports/infrastructure.rs
  - api/apps/api-server/src/routes/settings/host_infrastructure.rs
  - web/app/src/features/settings/components/host-infrastructure
status: implemented_pending_l0_acceptance
keywords:
  - storage-ephemeral
  - memory-observation
  - inspection-path
  - pagination
  - byte-budget
  - host-infrastructure
---

# 易失层内存观察改为树状路径与分页方向

## 当前实现状态

`2026-05-25` 已按 GitHub issue 树进入实现：#472-#480 覆盖 contract、后端 provider / route、前端 tree lazy loading / paged entries、QA 验收；#471 作为 L0 保留给用户最终验收。

实现后的边界仍是：后端提供 `inspection_path`、cursor 和 metadata；前端不解析 storage key；bulk list/tree/search 只传 metadata；value reveal 走 metadata / preview / full mode 并有 value size 上限。当前 local in-memory providers 使用轻量 metadata snapshot 后分页，没有引入持久化 provider 索引重构。

## 谁在做什么？

用户在 `2026-05-25 20` 确认易失层不需要历史兼容，因为重启后内存内容自然消失；下一步准备把方案挂到 GitHub issue 上。AI 负责把设计收敛成 issue 树，不进入实现。

## 为什么这样做？

当前内存观察页按 contract 一次性拉取 entries，并且 overview 为了统计也会触发全量 list。用户担心系统运行久后，虽然内存读取快，但 JSON 序列化和网络 IO 包会变大，造成观察页延迟。

## 为什么要做？

本轮方向是统一易失层的可观察检索模型：所有 ephemeral entry 都需要有树状 `inspection_path`，用于内存观察页的树状懒加载、搜索和分页；但不强行让所有运行时 storage key 都变成同一种深层 key。

## 当前方向

- `cache-store`、`rate-limit-store`、`distributed-lock` 适合让 runtime storage key 直接树状化。
- `session-store`、`task-queue`、`event-bus`、`runtime-event-stream` 保留自身运行时访问模型，同时由后端生成统一树状 `inspection_path`。
- 批量观察接口必须从后端开始分页，使用 `limit + byte_limit + cursor`；不要只做前端分页。
- `byte_limit` 需要参考 4096 字节内存页的尺度控制响应包，例如默认 64 KiB、最大 256 KiB 这类页倍数预算。
- list 只返回轻量 metadata，value 必须按需 reveal；前端不得自己拆 storage key 还原树。
- `byte_limit` 只约束批量 metadata 响应，不让超大单条 value 变成不可观察黑洞；单条 reveal 需要区分 metadata、preview、full，默认只返回受限 preview，超大 value 返回 `value_too_large` 与可观察 metadata。
- 后续 issue 需要把 `max_value_size_bytes` / `max_payload_size_bytes` 纳入易失层 contract，避免 100MB 级 base64 等 payload 仅靠观察页兜底。

## 截止日期？

当前只是已确认的方案方向，尚未创建或确认 issue 内容；进入实现前必须先完成 L0/L1/L2/L3 issue gate。

## 决策背后动机？

热路径和观察路径应该分离：运行时 get/claim/poll/replay 保持各类型最自然、最高效的访问方式；内存观察页统一消费后端提供的 `inspection_path`、cursor 和 metadata，避免全量扫描、全量发包和前端猜 key 语义。

## 验收证据

后续 issue 至少需要覆盖：overview 不再为了统计拉全量 entries；entries/tree/search 接口支持 cursor 与 byte budget；7 类 ephemeral contract 在内存观察页可树状懒加载；大数量 session/cache/rate-limit/runtime events 不再一次性返回全量 metadata；超大 value 在列表中可见 metadata，默认 reveal 只取 preview，full reveal 受硬上限保护。
