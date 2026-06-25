---
created_at: 2026-06-24 23
updated_at: 2026-06-25 10
memory_type: project
topic: Claude Code provider error semantics and upstream details L3 issue
decision_policy: verify_before_decision
scope:
  - claude-code
  - provider-runtime
  - application-public-api
  - github-issue
links:
  - https://github.com/taichuy/1flowbase/issues/1116
  - https://github.com/taichuy/1flowbase/issues/1005
---

# Claude Code provider error semantics and upstream details L3 issue

2026-06-24 已创建 GitHub issue #1116，作为后续修复 Claude Code 通过 `1flowbase` provider 调用时错误语义和上游错误详情结构化的 L3 入口。

背景：run `019efa40-31aa-71a3-ac92-775ea79368c6` 的 conversation 拼接还原成功，失败不是 Claude Code 漏传 `source_instance_id`，而是 active publication 仍指向旧 compiled plan，旧 plan 使用的 provider instance `019ef412-caeb-7793-973a-ff086a459d23` 已 `included_in_main=false`，host 将该状态误报成 `invalid input: source_instance_id`。

Issue #1116 范围只包含 host/provider 错误语义修复和 Anthropic provider HTTP 非 2xx 的结构化 `provider_details`；不负责重新发布当前 application，不改变 provider routing / main instance 策略，也不处理上游 `channel:client_restricted` 根因。Related #1005 已建立 `client_protocol_envelope` 通道，后续实现需保持该通道不退化。

2026-06-25 已完成 #1116 开发、独立验收和 GitHub issue 关闭。主仓 `dev` 提交 `eb3ab0e233879ece61070d8a1c6e68a41daa7b08` 拆分 `RuntimeProviderInvoker::resolve_llm_instance` 错误语义；官方插件仓 `main` 提交 `d4dda105553fb6724ed13f21e583f54fba7e0f3f` 为 Anthropic provider 非 2xx 上游错误保留结构化 `provider_details` 并让 streaming invoke 输出 `type:error` NDJSON。独立验收 subagent PASS，验证证据已评论到 https://github.com/taichuy/1flowbase/issues/1116#issuecomment-4795292071，issue 已关闭。
