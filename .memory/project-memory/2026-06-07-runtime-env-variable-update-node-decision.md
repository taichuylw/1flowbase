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

用户在 2026-06-07 19 追加验收反馈：环境变量更新节点的值输入不能只支持选择上游输出。选中 `string` 环境变量后，值输入应支持类似 HTTP 节点的 templated text，即普通文本混合 `{{node.xxx}}` 变量引用。节点输出也不能固定为 `env json`，应跟随选中的环境变量名与 `value_type`，例如 `env.hi string`。

2026-06-07 22 已按反馈修正实现：`state_write` 环境变量更新使用 typed `value` 表达，`string` 使用 templated text，`number` / `boolean` 使用 constant，复杂 object / array 第一版禁用；节点输出按选中环境变量生成，不再固定 `env json`。

2026-06-07 22 用户进一步否定 env-only 方向，并确认重新拆分变量模型：`env.xxx` 是应用级初始固定环境变量，运行中只读；`conversation.xxx` 是单次运行 / 单次会话内可读写会话变量；`sys.xxx` 是系统只读变量。#775 已关闭，后续实现依据改为 #781「新增会话变量与变量赋值节点」。

后续实现以 GitHub issue #781 为边界。若实现中发现必须新增持久化会话变量表、权限审计、跨运行状态存储，或需要重新允许变量赋值节点写 `env.xxx`，必须停止并回到 problem-framing。
