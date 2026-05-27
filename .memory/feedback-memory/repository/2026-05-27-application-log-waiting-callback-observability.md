---
memory_type: feedback
feedback_category: repository
topic: application-log-waiting-callback-observability
summary: waiting_callback 是可查看的持久化日志快照态，不应默认把问题推成客户端 cancel / callback 协议。
keywords:
  - application logs
  - waiting_callback
  - observability
  - polling
  - openai-compatible
created_at: 2026-05-27 00
updated_at: 2026-05-27 00
last_verified_at: 2026-05-27 00
decision_policy: direct_reference
scope:
  - web/app/src/features/applications
  - api/apps/api-server/src/routes/applications/application_runtime.rs
---

# Application Log Waiting Callback Observability

## 规则

排查应用日志里 `waiting_callback`、`waiting_human` 等等待态时，先区分“日志可观测性”和“客户端控制协议”。只要后端已经持久化运行、节点、事件、callback task 和 debug artifact，日志页就应该支持打开详情并查看到等待点；不要默认要求客户端追加 cancel 或继续 callback 才能看日志。

## 原因

用户纠正：客户端终止后仍等待回调不是这次主问题，1flowbase 应该先保证回调等待态下也能打开对话日志和运行详情。客户端如何终止、是否补 cancel 协议属于另一个边界，不应阻塞日志查看体验。

## 适用场景

修改应用日志、运行详情浮窗、对话日志、OpenAI-compatible 工具回调展示、等待态轮询逻辑或运行状态语义时命中。
