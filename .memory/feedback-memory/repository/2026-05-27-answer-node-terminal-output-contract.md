---
created_at: "2026-05-27 09"
updated_at: "2026-05-27 09"
feedback_category: repository
topic: Answer 节点作为会话终点必须沉淀完整输出
summary: 用户确认 Direct Reply / Answer 节点标记会话工作流结束；即使模板或绑定局部报错，也不能让节点 output 变成空对象，已渲染 answer、结构化 error、sys/env 等终态字段都应落到真实 output_payload。
decision_policy: direct_reference
---

# Answer 节点作为会话终点必须沉淀完整输出

规则：Direct Reply / Answer 节点是会话工作流的结束节点。运行到 Answer 后，应尽可能沉淀完整终态输出包；不能因为某个 selector、模板变量或绑定局部失败，把 `node_runs.output_payload` 打成 `{}`。

规则：Answer 的主输出字段仍是 `answer`。错误必须作为结构化 `error` 字段保存，并同步保留到节点 `error_payload` / 流程失败原因；不要只把错误混进自然语言 answer，也不要只在前端展示层临时拼出来。

规则：模板里能解析的部分要继续输出；不能解析的 selector 写入 `error.details`。失败流程的 `flow_runs.output_payload` 应优先使用终点 Answer 的完整 output，而不是在 Answer 已经有终态输出时回退到上游 LLM 输出。

原因：Answer 节点标记会话终态，日志和 API 都应能看到当时实际产出的 answer、错误和上下文快照。空 output 会让历史日志、会话详情和 OpenAI chat 兼容响应失去可观测性。

适用场景：修改 Agent Flow 运行时、Answer / Direct Reply 节点、模板绑定解析、节点输出持久化、失败流程输出选择、对话日志详情和 OpenAI chat 兼容输出时命中。
