---
memory_type: feedback
feedback_category: repository
topic: 应用日志用户输入边界
summary: 应用日志聊天视图只按真实 conversation id 分页展示 flow_run 对话，运行详情顶层提供 query/model，工具注册归一到 start 输入变量，start 输出留空，工具调用和兼容协议上下文归入运行追踪。
keywords:
  - application logs
  - run detail
  - user input
  - start node input
  - start input artifact
  - history
  - tool schema
  - tools
  - conversation pagination
match_when:
  - 修改应用日志、运行详情、公开 API 兼容请求映射或 debug artifact 预览时
created_at: 2026-05-19 21
updated_at: 2026-05-21 23
last_verified_at: 2026-05-21 23
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

应用日志聊天视图中的用户消息只代表用户原始发言。运行详情顶层字段使用 `query` / `model` 这类业务命名，不使用 `input_text` / `input_model`；历史对话按真实 `external_conversation_id` 分页读取，并以当前 run 为锚点加载附近消息。当前 run 的 `input_payload.history` / `input_payload.messages` 是模型输入上下文快照，不是平台会话状态；即使这些字段被 runtime debug artifact 截断或 offload，也不能作为运行详情的历史对话 fallback 来源。工具注册统一归一为 start 节点输入里的稳定变量，例如 `userinput.tools` 和 `userinput.tool_choice`；`userinput.history` / `userinput.tools` 这类数组变量如果被 runtime debug artifact 截断，调试变量面板必须加载完整 artifact 后再投影，不能只依赖 start 节点直接展开出来的 `query` / `model` / `files` 摘要；start 节点输出在日志语义上保持空对象。工具调用、兼容协议透传字段、运行参数和大 payload 预览应作为 start 节点输入、节点详情或运行追踪记录，不应被当作用户聊天内容展示。

## 原因

用户打开聊天记录时预期看到的是自己说的一句话和对应真实运行；把工具 schema、compatibility payload 或本次请求携带的 imported history 放进用户气泡会误导排查，也会把开始节点输入、模型上下文和平台会话分页三个职责混在一起。

## 适用场景

修改应用日志、运行详情、公开 API OpenAI / Anthropic 兼容映射、debug artifact offload、start 节点输入展示、对话日志抽取逻辑时命中。后端应提供用于展示的用户输入真值字段和规整后的工具注册变量，前端优先消费后端字段，不递归猜测工具 schema。
