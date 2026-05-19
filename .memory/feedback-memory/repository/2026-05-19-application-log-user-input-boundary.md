---
memory_type: feedback
feedback_category: repository
topic: 应用日志用户输入边界
summary: 应用日志聊天视图只显示用户原始发言，工具注册归一到 start 输入变量，start 输出留空，工具调用和兼容协议上下文归入运行追踪。
keywords:
  - application logs
  - run detail
  - user input
  - start node input
  - tool schema
match_when:
  - 修改应用日志、运行详情、公开 API 兼容请求映射或 debug artifact 预览时
created_at: 2026-05-19 21
updated_at: 2026-05-19 22
last_verified_at: 2026-05-19 22
decision_policy: direct_reference
scope:
  - web/app/src/features/applications
  - api/apps/api-server/src/routes/applications/application_runtime
  - api/crates/control-plane/src/application_public_api
---

# 应用日志用户输入边界

## 时间

`2026-05-19 21`

## 规则

应用日志聊天视图中的用户消息只代表用户原始发言。工具注册统一归一为 start 节点输入里的稳定变量，例如 `userinput.tools` 和 `userinput.tool_choice`；start 节点输出在日志语义上保持空对象。工具调用、兼容协议透传字段、运行参数和大 payload 预览应作为 start 节点输入、节点详情或运行追踪记录，不应被当作用户聊天内容展示。

## 原因

用户打开聊天记录时预期看到的是自己说的一句话；把工具 schema 或 compatibility payload 放进用户气泡会误导排查，也会把开始节点输入和对话展示两个职责混在一起。

## 适用场景

修改应用日志、运行详情、公开 API OpenAI / Anthropic 兼容映射、debug artifact offload、start 节点输入展示、对话日志抽取逻辑时命中。后端应提供用于展示的用户输入真值字段和规整后的工具注册变量，前端优先消费后端字段，不递归猜测工具 schema。
