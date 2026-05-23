---
memory_type: project
topic: LLM tool token attribution implemented
summary: 用户确认并实现 #420：LLM usage 作为唯一总账；tool 行展示生成工具调用那轮 LLM 的 input/cache hit/output，以及工具结果回填后下一轮 LLM 的 context input/cache hit 和 tool result input。input/cache 字段是 round 级归因，不做 per-tool 精确拆账。
keywords:
  - llm-tool-token-attribution
  - tool-callback
  - provider-usage
  - cache-hit
  - issue-420
match_when:
  - 继续调整 LLM 工具调用 token 统计
  - 讨论 tool call / tool result 与 LLM usage 的归因关系
  - 需要判断 cache hit 是否能拆到单个 tool
  - 修改对话日志追踪面板的 tool callback token 展示
created_at: 2026-05-23 23
updated_at: 2026-05-24 00
last_verified_at: 2026-05-24 00
decision_policy: verify_before_decision
scope:
  - https://github.com/taichuy/1flowbase/issues/420
  - https://github.com/taichuy/1flowbase/issues/421
  - https://github.com/taichuy/1flowbase/issues/422
  - https://github.com/taichuy/1flowbase/issues/423
  - api/crates/orchestration-runtime/src/execution_engine.rs
  - web/app/src/features/agent-flow/components/debug-console/conversation
---

# LLM Tool Token Attribution Implemented

## 时间

`2026-05-23 23`

## 谁在做什么

用户确认 #420 的实现方向后，后端和前端分别完成 LLM 工具调用 token 归因。用户后续纠正：工具折叠行不能只有工具调用 output tokens，还必须能看到生成工具调用那轮 LLM 的 input/cache hit，以及 tool result 作为下一轮上下文后的 context input/cache hit。#423 负责后端 runtime payload，#421 负责前端追踪面板展示，#422 完成独立测试验收；#420 保持打开，进入用户最终验收。

## 为什么这样做

LLM 节点的 provider usage 是唯一总账，但用户需要解释一次 agent 任务里哪些 tool call 和 tool result 撑大了 output / input token。工具节点不应成为第二套账，只提供按阶段切分的归因明细。

## 为什么要做

对话日志里需要能看出：工具调用阶段消耗的是生成 tool call 那轮 LLM 的 input + output，工具结果回填阶段消耗的是下一轮 LLM 的 context input；cache hit 是对应 LLM round 的请求级 usage，可以放到对应 tool 行作为 round-level attribution，但不能把多条 tool 行相加当真实账单，也不能伪造单 tool 精确缓存拆分。

## 截止日期

基础实现已在 `2026-05-23 23` 完成提交、独立 QA 和子 issue 关闭；input/cache hit 补充修正在 `2026-05-24 00` 完成验证；#420 等用户最终验收。

## 决策背后动机

用户希望统计口径足够简单、能解释真实消耗、同时不误导账单：tool token 是 LLM 总账里的组成部分，不和 LLM usage 重复求和。工具行可以帮助用户观察每个 tool 对上下文和输出的影响，但 LLM 节点 usage 仍是唯一总账；估算字段必须用 `token_count_method: estimated` 明示。

## 验证证据

- 后端提交：`f90beb00 Track LLM tool token attribution`
- 前端提交：`0eeccabc Show LLM tool token attribution`
- 懒加载索引提交：`905d7960 Include tool token attribution in artifact index`
- 后端验证：`cargo test -p orchestration-runtime llm_tool` 在 `api/` 下通过，5 passed。
- 后端 artifact 验证：`cargo test -p api-server application_runtime_routes_waiting_run_detail_offloads_large_llm_rounds` 在 `api/` 下通过，1 passed。
- 前端验证：`pnpm --dir web/app test src/features/agent-flow/_tests/debug-console/debug-conversation-log-panel.test.tsx` 通过，6 passed。
- 前端构建：`pnpm --dir web/app build` 通过。

## 关联 Issues

- #420 `[就绪]LLM 工具调用按阶段统计 token 归因`
- #421 `[就绪]#420 前端展示 LLM tool token 归因`
- #422 `[就绪]#420 独立测试与验收 LLM tool token 归因`
- #423 `[就绪]#420 后端记录 LLM tool token 归因`
