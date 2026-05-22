---
memory_type: project
topic: LLM 节点首 token 时间口径
summary: LLM 节点首 token 定义为 provider 对外流式输出的第一个 ReasoningDelta 或 TextDelta，并落入节点 metrics 与 failover attempt ledger。
keywords:
  - llm
  - first_token_at
  - time_to_first_token_ms
  - runtime observability
match_when:
  - 调整 LLM 节点运行指标、runtime observability、provider stream、failover attempt ledger 或 debug metrics 时
created_at: 2026-05-22 16
updated_at: 2026-05-22 16
last_verified_at: 2026-05-22 16
decision_policy: verify_before_decision
scope:
  - api/crates/control-plane/src/orchestration_runtime.rs
  - api/crates/orchestration-runtime/src/execution_engine.rs
  - api/crates/control-plane/src/orchestration_runtime/persistence.rs
  - api/crates/control-plane/src/orchestration_runtime/live_debug_run/observability.rs
---

# LLM 节点首 token 时间口径

## 时间

`2026-05-22 16`

## 谁在做什么

用户确认 LLM 节点需要记录首 token 时间；Codex 按后端 runtime 指标实现。

## 为什么这样做

首 token 是模型对外流式输出的第一个 token，不区分思考与正文。实现口径是第一次收到 `ReasoningDelta` 或 `TextDelta` 时记录 `first_token_at` 与 `time_to_first_token_ms`。

## 为什么要做

这个指标用于衡量 LLM 节点从 provider invocation 到首次输出的等待时间，不能由前端 SSE 到达时间反推。

## 截止日期

当前任务内完成。

## 决策背后动机

用户明确说“首 token 就是思考的第一个，就是他输出出来的第一个”，因此 tool call、usage、finish、error 都不属于首 token。

## 关联文档

- `api/crates/control-plane/src/_tests/orchestration_runtime/runtime_observability.rs`
