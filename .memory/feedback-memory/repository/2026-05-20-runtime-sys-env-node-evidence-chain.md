---
created_at: "2026-05-20 17"
updated_at: "2026-05-20 17"
feedback_category: repository
topic: 运行日志 sys/env 变量必须进入节点级证据链
summary: 用户纠正“只在对话日志里新增变量 tab”的方案不足；sys/env 尤其 env 是可变运行上下文，历史日志需要在 Start 输入、下游实际消费输入、变量修改过程和最终回复输出中形成证据链；env 不按 secret 脱敏，最终 sys/env 写入真实 output_payload。
decision_policy: direct_reference
---

# 运行日志 sys/env 变量必须进入节点级证据链

规则：Agent Flow 运行日志中，`sys` 和 `env` 不能只放在独立变量面板或当前配置视图里。运行开始时的 `sys/env` 快照应进入 Start 节点输入；下游节点只展示实际解析/消费后的输入；变量被修改时在“数据处理”记录 `path/before/after` 过程，并在节点输出体现修改结果；直接回复 / Answer 节点应把最终系统变量和环境变量快照写入真实 `output_payload`。

规则：`env` 不按 secret 做脱敏或隐藏处理，在本系统语义里它就是全局变量。最终 `sys/env` 是未来原生 API 调用和连续对话集成的重要依据，后端应直接输出真实值，而不是只提供前端展示层 payload。

原因：环境变量可变，当前应用配置不能代表历史运行真值。运行日志的核心价值是审计“当时拿到什么、用了什么、改了什么、最后是什么”，只新增变量 tab 会丢掉节点级因果链。

适用场景：设计或修改 Agent Flow 运行日志、Start 节点 input payload、节点 `输入 / 数据处理 / 输出` 展示、变量修改节点、Answer / Direct Reply 输出、debug artifact/full-load、历史运行回放和变量快照接口时命中。

边界：不要新增假的上下文节点污染工作流追踪；不要把完整 `sys/env` 塞进每个下游节点；不要把最终 `sys/env` 做成只存在于前端的展示层数据。
