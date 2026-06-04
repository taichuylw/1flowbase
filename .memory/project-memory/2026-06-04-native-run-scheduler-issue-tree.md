---
memory_type: project
title: Native Run Scheduler issue tree confirmed
created_at: 2026-06-04 17
updated_at: 2026-06-04 17
decision_policy: verify_before_decision
scope:
  - api/apps/plugin-runner
  - api/crates/control-plane
  - api/crates/orchestration-runtime
  - api/crates/storage-durable
  - api/crates/storage-ephemeral
  - api/apps/api-server
status: issue_tree_ready
keywords:
  - native-run-scheduler
  - client-session-key
  - provider-runtime-budget
  - waiting-callback
  - codex-compatibility
  - claude-code-compatibility
  - issue-tree
---

# Native Run Scheduler Issue Tree Confirmed

## 谁在做什么

用户在 `2026-06-04 17` 确认 Native Run Scheduler 方案并要求拆到 L3。AI 已在 GitHub 建立完整 issue tree：#674 是 L0 总控，#673 是 L1 ADR / decision，#675-#680 是 L2 workstream，#681-#692 是 L3 execution / QA tasks。

## 为什么这样做

本次事故暴露 host provider stdio runtime 30s timeout、client session key 不稳定、waiting_callback lease 占用、provider/model concurrency 和 Codex / ClaudeCode 协议兼容验收缺口。用户确认采用 host-owned Native Run Scheduler：scheduler 负责 execution lifecycle；插件只做 provider wire conversion；Answer Presentation 不纳入本 issue。

## 为什么要做

后续实现必须从明确 L3 开始，避免把 scheduler、provider wire conversion、Answer Presentation、客户端协议兼容和 QA 混成一个大改动。Codex/OpenAI Responses 与 ClaudeCode/Anthropic Messages 必须进入 contract fixtures 和 smoke 验收。

## 当前 issue tree

- L0 #674 `[待开发]Native Run Scheduler 调度治理与客户端兼容总控`
- L1 #673 `[待开发]定义 Native Run Scheduler 调度边界`
- L2 #675 Scheduler admission key、lease 与 durable run lifecycle
  - L3 #681 protocol-aware scheduler key derivation 与 admission metadata
  - L3 #682 active provider invocation lease acquire/release
  - L3 #683 run terminal、cancel、expire durable lifecycle 写入口
- L2 #676 Provider runtime budget 与 timeout 分层
  - L3 #684 provider invocation budget 默认 300s 并支持覆盖
  - L3 #685 wall-clock、first-token、stream-idle timeout 观测与错误分类
- L2 #677 waiting_callback resume、cancel 与 TTL lifecycle
  - L3 #686 waiting_callback 释放 active invocation lease 并持久化 pending callback
  - L3 #687 callback resume/cancel/TTL expire 重新进入 scheduler lifecycle
- L2 #678 Codex / OpenAI Responses compatibility
  - L3 #688 Codex OpenAI Responses contract fixtures
  - L3 #689 Codex 本地 compatibility endpoint smoke
- L2 #679 ClaudeCode / Anthropic Messages compatibility
  - L3 #690 ClaudeCode Anthropic Messages contract fixtures
  - L3 #691 ClaudeCode 本地 ANTHROPIC_BASE_URL smoke
- L2 #680 Native Run Scheduler QA evidence 与交付门禁
  - L3 #692 Native Run Scheduler QA acceptance matrix

## 执行门禁

实现只能从已确认的 L3 issue 开始；涉及可测试行为变化时先使用 `test-driven-development`。进入 QA / 交付时使用 `qa-evaluation`，warning、coverage、QA 产物统一落到 `tmp/test-governance/`。若任一 L3 发现需要改变外部协议 shape、历史数据 migration、或触碰 #497 Answer Presentation truth，必须停止并回到 #673 决策。

## 验收方向

最终验收需要覆盖：provider runtime budget 默认 300s、timeout 分类、active invocation lease release、waiting_callback resume/cancel/TTL、Codex OpenAI Responses fixtures/smoke、ClaudeCode Anthropic Messages fixtures/smoke、session key regression，以及 #692 的 QA acceptance matrix。
