---
created_at: 2026-06-24 00
updated_at: 2026-06-24 00
memory_type: project
topic: Claude Code Agent subagent trace visibility L3 issue
decision_policy: verify_before_decision
scope:
  - application-run-logs
  - trace-tree
  - claude-code
  - github-issue
links:
  - https://github.com/taichuy/1flowbase/issues/1115
  - https://github.com/taichuy/1flowbase/issues/981
---

# Claude Code Agent subagent trace visibility L3 issue

2026-06-24 用户确认走 Balanced 方向，并随后纠正 UI / contract 语义：不是在 `Tools` 下做一套新的 Agent 详情树，而是在父 LLM 节点展开区新增类似 `Tools` 的 `Agents` 折叠项。`Agents` 内的 subagent 应复用现有 LLM trace node 展示模型，有自己的输入、数据处理、输出和 `Tools` 子工具回调。

该方向不把 Claude Code internal runs 重新显示到顶层 run log summary 或 conversation messages；历史数据只允许重建日志 projection，不修改原始 `flow_run_events`。

已创建并修正 GitHub issue #1115，作为 #981 Trace Tree Read Model 的增量 L3 child。#981 的子 issue 清单和关闭条件已补上 #1115。后续实现前按 #1115 执行边界进入 `test-driven-development`，再接 backend/frontend 实现；若 subagent 无法复用现有 LLM node contract、无法稳定匹配 parent Agent 与 subagent run，或需要改 runtime 写路径 / internal run 过滤策略 / raw event schema，应停止并回到 problem-framing。
