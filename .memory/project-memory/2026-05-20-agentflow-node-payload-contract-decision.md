---
memory_type: project
topic: agentflow-node-payload-contract-decision
summary: Agent Flow 运行日志节点 payload 采用三段真值结构和字段级 artifact，不再按整节点 payload 无脑摘要。
keywords:
  - agentflow
  - runtime logs
  - node payload
  - debug artifact
  - payload contract
match_when:
  - 调整 Agent Flow 运行日志、节点输入输出、debug artifact、Start 节点 sys/env/history/tools 展示或接口 contract 时
created_at: 2026-05-20 23
updated_at: 2026-05-20 23
last_verified_at: 2026-05-20 23
decision_policy: verify_before_decision
scope:
  - api/crates/control-plane/src/orchestration_runtime
  - api/apps/api-server/src/routes/applications/application_runtime
  - web/app/src/features/agent-flow
  - .agents/skills/backend-development/SKILL.md
---

# Agent Flow Node Payload Contract Decision

## 时间

`2026-05-20 23`

## 谁在做什么

用户确认 issue #343 后续按激进方案推进运行日志节点 payload contract：节点运行记录稳定暴露 `input_payload`、`debug_payload`、`output_payload` 三段，展示摘要和 artifact 只能作为独立 view / display metadata，不替代或重塑节点真值。

## 为什么这样做

当前实现对 Start 节点生成 `start_input_summary`，并在详情查询 offload 时可能把整节点 payload 替换成 preview 指针，导致 `sys/env` 等真值在展示链路里丢失。用户判断整包压缩不合理，压缩应按字段语义处理。

## 为什么要做

运行日志要能说明每个节点实际消费、数据处理和真实产出。后续需要保留短真值字段，同时允许明显长字段单独 preview / artifact，避免一个超长 history 或 tools 把整个节点输入变成摘要。

## 截止日期

无固定截止日期；后续修复 issue #343 或调整运行日志 contract 时优先遵守。

## 决策背后动机

项目处于开发初期，用户优先要长期一致性和清晰真值边界。Start 节点 `query`、`model`、`files`、`sys`、`env` 先完整保留，其中 `env` 沿用不脱敏决策，`files` 先保留完整数组；`history`、`tools` 等明显可能超长字段用字段级 preview / artifact。其他节点同理，禁止用整包 preview / summary 替代节点运行 payload。

## 关联文档

- `.agents/skills/backend-development/SKILL.md`
- GitHub issue #343
