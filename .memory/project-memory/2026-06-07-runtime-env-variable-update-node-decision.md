---
created_at: 2026-06-07 17
memory_type: project
decision_policy: verify_before_decision
scope: agent-flow variable update node
source_issue: "#775"
---

# Runtime Environment Variable Update Node Decision

用户在 2026-06-07 17 确认：将 `Variable Assigner` 与 `Parameter Extractor` 的产品入口合并为一个变量更新节点，目标是改变环境变量值，而不是保留“参数抽取”和“流程变量赋值”两个独立概念。

已批准方向是 Balanced：第一版只更新本次运行 / 本地调试变量上下文中的 `env.xxx` 值，不写回应用级环境变量配置，不修改 `application_environment_variables`，也不新增持久化 API、权限、审计、并发覆盖或发布态持久化语义。

实现前以 GitHub issue #775 为边界。若实现中发现必须修改后端持久化、历史流程迁移、权限审计或泛化变量系统，必须停止并回到 problem-framing。
