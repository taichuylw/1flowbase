---
memory_type: project
topic: state-protocol-quality-gate
summary: ACP / Anthropic-compatible 状态协议回归已被提升为固定质量门禁；2026-06-16 17 用户确认 CI/CD 只跑 mock/static 即可，`quality-gate state-protocols` 改为 `--skip-live-acp` 并在全量 `scope=ci` 通过。
keywords:
  - ACP
  - Claude Code
  - Anthropic-compatible
  - state-protocols
  - quality gate
  - agent_thought_chunk
created_at: 2026-06-16 19
updated_at: 2026-06-16 17
decision_policy: verify_before_decision
scope:
  - scripts/node/verify-state-protocols.js
  - scripts/node/acp-claude-smoke
  - scripts/node/gate-router/core.js
  - scripts/node/github-quality-gate/commands.js
---

# State Protocol Quality Gate

## 谁在做什么

用户确认 #922 暴露的 ACP / Anthropic-compatible reasoning 投影问题不是普通 smoke 缺口，而是重要状态协议回归风险。AI 已把该链路提升为固定质量门禁：`node scripts/node/verify-state-protocols.js`。

## 为什么这样做

之前只验内部 `answer_segments` / SSE fallback，没有通过真实 ACP + Claude Code 观察 `agent_thought_chunk`，导致验收漏掉 `reasoning_delta` 未投影为 ACP thought chunk 的问题。

## 为什么要做

这条门禁覆盖“后端协议投影 -> Anthropic-compatible stream -> Claude Code ACP adapter -> session/update 状态语义”的端到端状态协议链路。后续改 `compat_sse`、Anthropic public API 或 ACP smoke 脚本时，gate-router 会提示跑 `verify-state-protocols`。

## 截止日期

已在 2026-06-16 落地并推送 `dev`。

## 决策背后动机

状态协议类问题不能只靠内部单元测试宣称验收通过；真实外部 agent adapter 的状态事件是验收事实之一。该门禁默认包含脚本单测、`api-server anthropic_` 定向测试、后端 ensure 和真实 Claude Code ACP smoke。没有真实 ACP 证据时，只能标记为未验证，不下确定结论。

## 2026-06-16 13 复查状态

- `quality-gate scope=ci` run `27593947948` 对 `beta@9174b505` 聚合通过，但 `.github/workflows/quality-gate.yml` 的 CI jobs / `INPUT_EXPECTED_SCOPES` 没有包含 `state-protocols`，因此不能把该 run 作为状态协议 live smoke 通过证据。
- 单独触发 `quality-gate scope=state-protocols` run `27595015084` 指向 `beta@9174b505`，结果失败并创建 issue `#943`。
- 失败根因：`api-server` Anthropic / compat_sse 定向测试 48 个通过；真实 ACP smoke 在 `session/new` 阶段失败，artifact `acp-claude-summary.json` 显示 `Claude Code native binary not found at claude`。
- 结论边界：当前证据不能证明 ACP live 状态投影坏了，但也不能宣称通过；需要先补 GitHub Actions 中 Claude Code native binary / 可用认证环境，并把 `state-protocols` 纳入 `scope=ci` 聚合后重跑。

## 2026-06-16 15 实施状态

- `beta` 已推送 `beaa0a36`、`bd778e39`、`609526c7`、`23635fc2`：把 `state-protocols-gate` 纳入 `quality-gate scope=ci` 聚合，补 Claude Code 安装、自定义模型开关，并让 ACP smoke 在 pending JSON-RPC 超时时稳定返回证据。
- 本地回归 `node --test scripts/node/acp-claude-smoke/_tests/*.test.js scripts/node/verify-state-protocols/_tests/*.test.js scripts/node/github-quality-gate/_tests/*.test.js` 通过 47 项；`git diff --check` 通过。
- 最新线上单独 run `27602306049` 已通过 Claude Code 安装和静态/后端状态协议检查，但 live ACP smoke 失败在 `Authentication required`；日志显示 `CLAUDE_CODE_OAUTH_TOKEN`、`ANTHROPIC_AUTH_TOKEN`、`ANTHROPIC_API_KEY`、`ANTHROPIC_BASE_URL` 都为空。
- 当前不能触发全量 `scope=ci` 作为验收：单独 `state-protocols` 仍因缺少 live 鉴权 secrets 未通过。补齐 secrets 后先重跑 `scope=state-protocols`，通过后再跑全量 `scope=ci`。

## 2026-06-16 17 最终状态

- 用户确认 CI/CD 不需要真实 Claude Code / Anthropic live 鉴权，跑 mock/static 即可。
- `beta` 已推送 `65d1f9ec` 和 `02072568`：`quality-gate` 调用 `state-protocols` 时追加 `--skip-live-acp`，workflow 删除 Claude Code 安装和 `CLAUDE_CODE_OAUTH_TOKEN` / `ANTHROPIC_*` 注入；同时保留本地 Postgres，因为 Rust mock/static 定向测试需要测试库。
- 本地回归 `node --test scripts/node/acp-claude-smoke/_tests/*.test.js scripts/node/verify-state-protocols/_tests/*.test.js scripts/node/github-quality-gate/_tests/*.test.js` 通过 47 项；`git diff --check` 通过。
- 线上单独 `scope=state-protocols` run `27604582146` 通过，headSha `0207256823abf92cab3fd36333fae97448f7318a`。
- 线上全量 `scope=ci` run `27605504013` 通过，headSha `0207256823abf92cab3fd36333fae97448f7318a`，其中 `state-protocols-gate` 为 success。
- #944 已切到 `phase:user-acceptance`。不要再把缺 live secret 当作该 issue blocker。
