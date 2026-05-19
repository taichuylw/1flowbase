---
memory_type: feedback
feedback_category: repository
topic: Code 节点 console.log 应按 info 日志等级处理
summary: Code 节点运行日志前后端要保持一致；`console.log` 是默认信息日志，应由后端归一为 `level: "info"`，前端也要兼容历史 `level: "log"` 并展示为 INFO。
keywords:
  - agent-flow
  - code-node
  - console.log
  - console_logs
  - info
  - frontend-backend-consistency
match_when:
  - 调整 Code 节点运行日志或 debug_payload.console_logs
  - 设计日志等级字段或展示标签
  - 处理历史 console log 数据兼容展示
created_at: 2026-05-19 16
updated_at: 2026-05-19 16
last_verified_at: 2026-05-19 16
decision_policy: direct_reference
scope:
  - api/crates/orchestration-runtime
  - web/app/src/features/agent-flow
---
# Code 节点 console.log 应按 info 日志等级处理

## 规则

Code 节点运行日志的等级语义以前后端一致为准：`console.log` 是默认信息日志，后端应写为 `level: "info"`；前端展示层必须兼容历史 `level: "log"`，统一显示为 `INFO`。

## 原因

用户明确指出日志等级不应出现 `log` 这种非产品语义等级；`log` 只是 JavaScript console 方法名，产品日志等级应使用 `info / warn / error`。

## 适用场景

- 修改 `debug_payload.console_logs` 结构。
- 调整 Code 节点上次运行、调试面板、运行详情的日志展示。
- 处理旧运行记录中仍存在 `level: "log"` 的展示兼容。
