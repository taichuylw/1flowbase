---
memory_type: feedback
feedback_category: repository
topic: application_run_detail_reuse_preview
summary: 应用运行日志的运行详情应复用 Agent Flow 预览组件；对话日志应由外层打开兄弟面板；侧边窗口内部自行管理头部、底部和滚动。
keywords:
  - application-logs
  - run-detail
  - agent-flow
  - preview
  - debug-console
match_when:
  - 修改应用运行日志页、运行详情面板或运行追踪展示
  - 在应用侧展示 Agent Flow 调试运行对话、工作流节点或对话日志
created_at: 2026-05-13 22
updated_at: 2026-05-13 23
last_verified_at: 2026-05-13 23
decision_policy: direct_reference
scope:
  - web/app/src/features/applications/components/logs
  - web/app/src/features/agent-flow/components/debug-console
---

# 应用运行详情复用预览组件

## 规则

应用运行日志的 L1 运行详情，应复用 Agent Flow 预览 / debug console 的组件能力，包括消息展示、工作流节点展开、思考区、复制输出和“查看对话日志 / 追踪 / 节点详情”等交互；不要在应用日志侧维护一套相似但不完整的对话详情面板。

运行详情里的“查看对话日志”不应在运行详情内部覆盖当前预览内容。应用日志页应由外层页面控制 `onOpenMessageLog`，在运行详情旁边打开一个兄弟面板，让列表、对话日志和运行详情共存。

分栏页外层不应统一接管侧边窗口底部和内容滚动。外层只负责列表、对话日志、运行详情三列的占位与同高；每个侧边窗口内部自行划分固定头部、固定底部操作区 / 输入框和中间滚动内容。聊天类窗口默认是“顶部标题固定、底部输入框或操作区固定、聊天内容区域滚动”。

## 原因

用户明确指出运行详情“应该复用预览组件，包括打开查看详情这些”，随后又指出点击查看对话详情应“将列表撑开另外打开一个而不是直接跳转”。当前独立实现会漏掉预览已有的日志打开、追踪查看和节点详情能力；把日志面板放在运行详情内部则会形成覆盖 / 跳转感，打断当前运行详情的上下文。

用户进一步指出底部不应由页面统一管理，顶部和底部输入框应固定在各自侧边窗口内，只有聊天内容滚动。这样可以避免页面级空白、外层滚动条卡住、顶部说明被遮挡，以及多个窗口互相抢滚动上下文。

## 适用场景

- 修改 `/applications/:id/logs` 的运行详情。
- 调整 `ApplicationRunDetailPanel`、`AgentFlowDebugConsole`、`ConversationLogPanel` 或运行详情映射。
- 新增应用侧运行观测、运行追踪、节点输入输出查看。
- 调整应用运行日志页中列表、运行详情、对话日志三者的分栏关系。
