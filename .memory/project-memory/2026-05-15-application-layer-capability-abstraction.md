---
summary: 用户确认应用层应作为稳定宿主抽象，后续 MCP 应用、聚合接口应用、画布流程应用等具体类型都应通过应用能力 provider 扩展画布、日志、监控、发布和调用，而不是把 agent_flow 的 flow_run/node_run 语义硬编码成所有应用的公共事实。
decision_policy: verify_before_decision
updated_at: 2026-05-15 18
---

# Application Layer Capability Abstraction

用户在 `2026-05-15 18` 明确确认：`Application` 应作为一等宿主抽象，具体应用类型后续可能包括 `agent_flow`、专门的 MCP 应用、聚合接口应用、创建接口后进入画布流程节点编排的应用等。

当前动机是避免继续把 `agent_flow` 的 `flow_run / node_run / target_node_id` 等实现细节扩散成所有应用类型的通用日志、监控和画布模型。应用层应只承载稳定事实和能力挂载点；具体实现由应用类型 provider / adapter 承担。

已落地的第一步：

- 控制台应用日志响应新增通用 application run envelope，包含 `application_id`、`application_type`、`run_object_kind`、`run_kind`、`source`、`protocol`、`subject`、`actor`、`correlation`。
- 现有 `agent_flow` 日志仍保留旧 `flow_run / node_runs / checkpoints / callback_tasks / events` 字段兼容，同时新增 `detail.kind = "agent_flow"` 和 typed detail。
- 列表层的 `ApplicationRunSummary` 已补充 public API / external correlation 元数据，避免只在详情层才知道调用来源。

后续涉及新增应用类型或调整 logs / monitoring / canvas / publish / invoke 时，应优先检查是否能挂在应用能力抽象上，而不是直接改 `application_runtime.rs` 里的 flow-specific DTO 或持久化假设。
