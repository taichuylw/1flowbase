---
feedback_category: repository
decision_policy: direct_reference
created_at: 2026-06-18 10
---

# Application Log Lazy Trace Tools Index

规则：应用日志 / 对话日志 lazy trace tree 中，`callback_kind = llm_tool_calls` 不作为普通 trace child row 展示；但归属 LLM node 或 LLM node group 的工具调用、route、fusion 信息不能丢，必须由后端合并进 LLM content 的轻量 Tools 索引。展开 Tools 组只展示工具列表，不批量加载详情；展开单个 tool callback 时才请求该 tool 的 detail payload，并按 tool id 缓存。

原因：用户明确纠正“这些工具详情也是逐个展开懒加载吗”和“中间工具调用、相关路由没有了不合理”。之前只过滤 `llm_tool_calls` trace child 会去掉多余 `LLM_tool_calls` 行，但如果不把 callback task 转为 LLM content 内的工具索引，UI 会丢失工具调用、route / fusion 摘要和逐个 detail 入口。

适用场景：修改 `/applications/:id/logs` lazy trace tree、`trace_tree_responses.rs`、`runtime_debug_artifacts.rs`、`ConversationLogPanel`、`LlmToolTraceTree`、LLM tool callback / visible internal route trace 展示或相关测试时命中。
