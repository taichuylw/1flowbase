---
created_at: 2026-05-07 20
memory_type: feedback
decision_policy: direct_reference
feedback_category: repository
scope: agent-flow variable cache
---

# AgentFlow Variable Cache Uses Object Level Entries

规则：AgentFlow 变量缓存面板应按节点输出对象展示缓存条目，不默认把对象内所有字段递归平铺成独立变量。

原因：变量缓存的目标是帮助用户理解某个节点当前缓存的完整对象；把 `LLM/user_prompt`、`LLM/__attempt_ids[0]` 等字段全部铺出来会制造噪声，也弱化对象整体边界。

适用场景：调试变量缓存面板、durable variable snapshot 恢复后的缓存展示、节点预览运行产生的变量缓存展示。变量选择器或模板绑定需要选择具体输出字段时，仍可保留字段级 selector。
