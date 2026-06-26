---
feedback_category: repository
decision_policy: direct_reference
created_at: 2026-06-18 09
---

# Application Log Lazy Trace 展开即打开

规则：应用日志 / 对话日志的 lazy trace tree 中，展开追踪节点本身就是打开节点详情的动作；不要在展开后的详情槽位里再放一个“详情”按钮，也不要要求用户先展开再二次点击才请求节点内容。节点展开时按后端 lazy tree 契约加载该节点 children，并在 `has_content` 为真时同时加载该节点 content。

原因：用户明确纠正“这个按钮是多余，我们展开折叠就应该触发打开事件了”。两段式按钮会制造多余操作，还会让展开状态和内容打开状态分裂，违背 lazy tree 的逐层展开心智。

适用场景：修改 `/applications/:id/logs` 对话日志、Agent Flow debug console `ConversationLogPanel`、lazy trace tree、节点详情 content 查询或相关测试时命中。
