---
memory_type: project
topic: system-only-llm-prompt-runtime-compat
summary: 2026-05-31 修复两节点串联时下游 LLM 只有 system prompt 导致 provider messages 为空的问题；runtime 保留 system 作为 provider instructions，同时把 node prompt system-only 内容补成 user turn，避免 OpenAI Codex passthrough 要求 instructions 非空时再次失败。
keywords:
  - llm prompt_messages
  - system-only prompt
  - prompt_messages_empty
  - openai responses
  - anthropic /v1/messages
  - answer presentation
created_at: 2026-05-31 10
updated_at: 2026-05-31 10
last_verified_at: 2026-05-31 10
decision_policy: verify_before_decision
scope:
  - api/crates/orchestration-runtime/src/execution_engine.rs
  - api/crates/orchestration-runtime/src/_tests/llm_prompt_messages_validation_tests.rs
  - /v1/messages
  - /api/v1/agent/runs
---

# System-Only LLM Prompt Runtime Compat

2026-05-31，Claude Code 通过 Anthropic-compatible `POST /v1/messages` 调用应用时，单 LLM 节点正常；两个 LLM 节点串联且第二个节点把上游结果拼到 Answer 前，会出现 `LLM node requires at least one non-empty user or assistant prompt message`。

根因：第二个 LLM 节点编译后只有一条非空 `role=system` prompt message。runtime 先把 system 提升为 provider `system/instructions`，导致 provider `messages=[]`，在调用 provider 前被 `prompt_messages_empty` 拦截。首次修复如果只把 system 改成 user，会让 OpenAI Codex passthrough 的 `/responses` 上游返回 `OpenAI codex passthrough requires a non-empty instructions field`。

当前边界：仅当 provider messages 为空，且 system 来源是节点自己的 `node_prompt` / `prompt_messages` 时，runtime 保留完整 `effective_system`，并额外补一条 user message 承载同一段已渲染内容；run-level/history system 不会被伪造成 user turn。debug payload 用 `compatibility_promotions.source_kind=node_prompt_system_only` 暴露该兼容。

验证证据：`cargo test -p orchestration-runtime llm_prompt_messages_validation_tests -- --nocapture`、`cargo test -p orchestration-runtime`、`cargo fmt --all -- --check`、`cargo build -p api-server` 均通过；临时 `127.0.0.1:7802` 对用户提供的应用密钥跑 `POST /v1/messages`，flow `019e7bbf-2d66-7d53-9f05-143f67d4e0d1` 成功，`node-llm`、`node-llm-1`、`node-answer` 均为 `succeeded`。
