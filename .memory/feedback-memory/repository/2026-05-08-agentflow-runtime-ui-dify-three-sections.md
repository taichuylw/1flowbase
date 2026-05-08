---
created_at: 2026-05-08 11
memory_type: feedback
decision_policy: direct_reference
feedback_category: repository
scope: agent-flow runtime UI and output contract
---

# AgentFlow Runtime UI Follows Dify Three Sections

规则：Agent Flow 节点运行详情的产品层信息架构按 Dify 风格收敛为 `输入 / 数据处理 / 输出`，其中“数据处理”固定展示；所有节点只要 `debug_payload` 非空，就在“上次运行 / 工作流展开”的“数据处理”里展示，不再只限定 LLM。

规则：运行态持久化必须把公开输出、指标和数据处理分开。`output_payload` 只保存节点公开输出；`metrics_payload` 保存使用量、路由、耗时、attempt 等指标；`debug_payload` 保存 provider stream events、assistant message、tool calls、raw/context refs 等数据处理证据。数据处理数据转移到 `debug_payload` 后，不应继续在 `output_payload` 里放一份副本。

规则：前端节点详情里“输出”只能展示持久化公开输出；如果历史或运行时 trace 中仍有与“数据处理”相同的字段，展示层也要避免重复显示到“输出”。变量选择器只展示输出契约声明的 selector。

规则：异常输出如果是节点错误处理策略显式产出的变量，例如 `error_message`、`error_type` 或 fallback result，应进入输出对象；是否允许下游引用仍由输出契约/错误处理策略决定。

规则：需要自定义输出变量的节点应显式支持输出契约编辑与校验/runtime 对齐，不要把所有内置节点都假定成单一固定 `output`；不支持自定义输出的节点应明确保持 fixed output contract。

适用场景：审查或改写 Agent Flow 节点详情、debug console、variable cache、变量选择器、node runtime contract、output contract editor、validate-document 与相关 spec/plan。
