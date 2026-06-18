---
feedback_category: repository
decision_policy: direct_reference
created_at: 2026-06-18 12
---

# Application Run Detail Conversation Source

规则：应用日志 Run 详情的对话正文必须以当前 run 专属 conversation endpoint 为数据源，不能为了定位同 external conversation 的相邻 run，改用 `/logs/conversations/{conversation_id}/messages` 摘要接口替代。

原因：用户明确纠正“当前 run 及之前，我当前 run 的系统提示词和聊天记录为什么没有，我们修改范围没有这部分”。外部 conversation 摘要接口是一 run 一条，用于同会话 run 列表定位；run 专属 conversation endpoint 才会从当前 run input payload 中恢复 system prompt 和导入聊天历史。

适用场景：修改 `/applications/:id/logs` Run 详情、`ApplicationRunDetailPanel`、application run conversation messages、external conversation around-run 查询、对话日志入口绑定或相关测试时命中。
