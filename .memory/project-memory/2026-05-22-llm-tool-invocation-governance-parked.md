---
memory_type: project
topic: LLM 工具调用治理方向已挂 issue 暂不动工
summary: 用户确认 LLM 工具调用治理方向先挂到 GitHub issue 暂不动工；已创建 #400 作为 #372 的 L1 discussion child，核心边界是统计只做画像，策略门禁在 tool intent 离开服务端执行面前拦截，denied tool call 只进入观察面并返回受控 policy feedback。
keywords:
  - llm-tool-governance
  - tool-callback
  - capability-invocation
  - allowlist
  - denylist
  - policy-feedback
  - issue-400
match_when:
  - 继续讨论 LLM 节点工具调用白名单、黑名单、高危工具拦截或 policy feedback
  - 规划 #372 发布应用原生 LLM 工具回调闭环的安全治理子方向
  - 需要判断 LLM 工具调用治理是否已经进入实现
created_at: 2026-05-22 15
updated_at: 2026-05-22 15
last_verified_at: 2026-05-22 15
decision_policy: verify_before_decision
scope:
  - https://github.com/taichuy/1flowbase/issues/400
  - https://github.com/taichuy/1flowbase/issues/372
  - api/crates/orchestration-runtime/src/execution_engine.rs
  - api/crates/control-plane/src/orchestration_runtime
  - api/crates/control-plane/src/capability_runtime.rs
  - api/crates/domain/src/runtime_observability.rs
---

# LLM 工具调用治理方向已挂 issue 暂不动工

## 时间

`2026-05-22 15`

## 谁在做什么

用户把 LLM 工具调用治理、高危工具拦截、allowlist / denylist、工具调用统计库和 policy feedback 的方向挂到 GitHub issue，作为未来设计基础，当前不进入实现。

## 为什么这样做

讨论中确认真正风险点在 LLM 节点 / provider adapter / tool callback 场景，而不是普通 Code 节点。服务端只有在 tool intent 离开执行面之前拥有策略门禁，才能承诺拦截；如果工具已由 provider builtin 或外部客户端执行，服务端只能审计、拒收结果或标记 external side effect。

## 为什么要做

需要避免“只统计、不拦截”的假安全感。后续设计应把执行面和观察面分开：黑名单命中的 tool call 不转发到 CapabilityRuntime、MCP executor 或 client-side executor，但必须记录为 denied，并可向模型回传受控 `tool_call_denied` 结果，让模型改用其他已开放工具。

## 截止日期

无。该方向暂时停留在 `phase:discussion`，不拆 L2 / L3，不动工。

## 决策背后动机

用户希望先积累 LLM 每次工具调用，形成工具调用库和常用安全指令推荐，但最终安全策略不能自动从高频调用推导为白名单。高频统计只提供推荐证据，allowlist / denylist / approval policy 仍由用户或管理员明确批准。

## 关联文档

- #400 `[讨论]LLM 工具调用治理与高危工具拦截策略`
- #372 `[讨论]发布应用支持原生 LLM 工具回调闭环`
