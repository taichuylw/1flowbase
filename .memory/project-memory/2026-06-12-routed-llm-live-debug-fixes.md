---
topic: 智能路由现网联调与 Responses 兼容链路修复
status: delivered-awaiting-acceptance
decision_policy: verify_before_decision
delivered_at: 2026-06-12
related_issues: ["#872", "#869", "#862"]
---

# 智能路由联调修复（#872，待用户验收）

## 谁在做什么

AI 按用户指令完成 #869 验收后两个现网问题的联调修复，6 笔提交在 dev：
`4ad9a107`（混发轮内联执行）、`44adfc0c`（Responses SSE item 生命周期）、
`c73fbda8`（回放 item 映射）、`b2c2a6c6`（扁平工具定义）、
`ffd9c0d9`（resume 不强制 previous_response_id）、`0d2ccab8`（并行调用合并）。

## 关键结论（影响后续决策）

- codex CLI 0.138 只支持 responses wire（chat 已废弃）；codex 验证配置：
  `~/.codex/config.toml` 的 `[model_providers.flowbase]` + 独立 profile 文件
  `~/.codex/flowbase.config.toml`，API key 走 env `FLOWBASE_API_KEY`。
- 本地 api-server 以宿主进程跑（`api/apps/api-server/.env` + 手工 source 启动，
  注意 .env 含 JSON 值不能直接 bash source，要逐行 export）。
- "一个问题处理两次"中 Claude Code 的 `<session>` 标题探测伴生 run 是客户端行为
  + #862 既定映射语义；收口需另立 control-plane policy 决策，未做。
- 三案例 E2E 与 DB 真值核验通过：3 问 = 3 run 全 succeeded，图片只路由一次，
  回调均在同 run 内 resume。

## 截止与状态

2026-06-12 联调完成，#872 待用户人工验收；验收后可考虑关闭 #869/#872。
