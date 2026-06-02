---
feedback_category: repository
topic: LLM 上下文变量应由上游 schema 发现而不是在 Code 输出中硬编码 history
summary: 用户纠正上下文变量模型边界：Code 节点输出契约不应内置或硬编码 history 结构；LLM 节点作为下游应基于上游已暴露的 JSON Schema，自动发现 history-compatible 变量进入上下文下拉。
created_at: 2026-06-01 18
decision_policy: direct_reference
applies_until: 2026-12-31
---

# LLM Context Schema Discovery Boundary

用户在 #581-#586 规划实现中纠正：不要把“history 结构”硬编码进 Code 节点输出契约。Code 节点只提供通用的结构化输出契约和可选 JSON Schema 校验；用户需要构造上下文变量时，可以自行复制、粘贴或填写与 history 兼容的 JSON Schema。

LLM 节点的上下文下拉应作为下游消费方，读取上游已经暴露的变量契约，并把 `valueType=array/array[object]` 且 JSON Schema 与 history 消息数组兼容的变量注册为候选。`Start/history` 可以因为系统变量自身暴露了兼容 schema 而进入候选，但这不是 Code 输出模板的硬编码特例。

适用场景：

- 调整 Agent Flow LLM 上下文 selector、变量池、上下文候选过滤或 schema 校验。
- 调整 Code / Plugin / Start 节点输出契约的 JSON Schema 表达。
- 讨论 history 结构复用时，优先保持“生产方声明 schema、消费方按 schema 发现”的边界。
