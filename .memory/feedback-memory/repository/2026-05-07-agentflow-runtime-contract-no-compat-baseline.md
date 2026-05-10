---
created_at: 2026-05-07 21
memory_type: feedback
decision_policy: direct_reference
feedback_category: repository
scope: agent-flow runtime contract
---

# AgentFlow Runtime Contract Uses Destructive Baseline

规则：Agent Flow 变量链接器、运行态 payload、变量缓存、schema 契约这类底层架构调整，在当前开发初期按可重置数据库、可重写旧字段的破坏性基线推进；不要为了旧草稿、旧 selector、旧 snapshot 写兼容提示、隐藏字段或渐进迁移路径。

原因：这类契约会长期支撑节点扩展、运行调试、变量引用和持久化恢复；如果早期保留兼容口，会把输入、输出、指标、错误和 debug 证据继续混在同一层，后续新增节点会继承错误边界。

适用场景：审查或改写 Agent Flow schema、变量链接器、debug variable cache、runtime snapshot、node execution trace、LLM output/metrics 分层相关 spec、plan 与实现。
