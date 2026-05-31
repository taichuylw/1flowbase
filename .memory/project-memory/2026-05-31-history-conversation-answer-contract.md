---
summary: 自 `2026-05-31 00` 起，历史对话接口的 answer / message content 被确认为用户内容接口契约，必须返回可直接展示的完整正文；运行追踪和 raw debug payload 仍可返回 runtime debug artifact preview，并由详情视图按需加载完整值。
decision_policy: verify_before_decision
tags:
  - backend-contract
  - application-runtime
  - conversation-history
  - runtime-debug-artifact
---

# History Conversation Answer Contract

用户确认：历史对话读取由后端接口保证返回完整正文，前端不负责在历史消息页面自行还原压缩结构。

动机是把用户内容接口和调试追踪接口分开：历史对话、最终输出气泡、用户可复制正文属于用户内容接口，应直接消费完整 `answer` / message content；运行详情里的 raw trace / JSON 面板可以保留 `__runtime_debug_artifact`、`preview`、`artifact_ref` 等预览结构。

落地边界：

- 后端可以继续把大 payload offload 成 runtime debug artifact。
- 历史对话接口组装响应时，如果 `answer` 字段是 artifact preview，应由后端按 `artifact_ref` hydrate 完整值后返回。
- 前端历史对话 UI 不新增 `preview` / `artifact_ref` 兼容还原逻辑。
- 运行详情 raw trace 仍保留预览语义，必要时由详情入口加载完整 artifact。

