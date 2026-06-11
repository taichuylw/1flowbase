---
topic: visible_internal_llm_tool 显式 external_tool_policy
status: approved
decision_policy: verify_before_decision
approved_at: 2026-06-12
related_issues: ["#810", "#813"]
---

# 工具 LLM 注册显式声明 external_tool_policy（用户已拍板）

## 谁在做什么

AI 按用户确认的方向 2 实现：在 `visible_internal_llm_tool` 注册条目上新增
`external_tool_policy: "forbidden" | "inherited"`，替换 media 参数隐式判定。

## 为什么这样做

- 现状：分支工具 LLM 是否继承 run-context 客户端工具，由"本次调用带没带
  `media` 参数"硬编码决定（`media_context.rs` 的
  `visible_internal_llm_tool_llm_resolved_inputs`）。违反"路由行为可配置、
  不写死"的项目原则。
- 用户已确认三点：
  1. 走方向 2（显式模式字段），不做工具 LLM 注册中心（方向 3 明确不做）。
  2. 字段名 `external_tool_policy`，值 `"forbidden" | "inherited"`，与
     `internal_llm_node_policy` 风格一致；存量未填默认 `forbidden`。
  3. 本期 `inherited` 只继承 run-context 客户端工具；分支 LLM 嵌套挂工具
     LLM（嵌套路由）不在本期。
- media 注入图片内容块与模式正交，保留。
- 开发初期不做兼容回退：直接删除 media 隐式判定，存量配置靠默认值 +
  DB/接口可改覆盖。

## 截止与状态

2026-06-12 拍板后立即实现（用户要求挂 issue 后连续推进，无需再次确认）。
2026-06-12 实现完成：后端 runtime + compiler 校验 + 前端注册面板/i18n 已落地，
issue #869 待用户验收。存量行为唯一收紧点：不带 media 且未配置字段的注册，
从隐式继承工具改为默认 forbidden；需要开放工具时在注册面板或 DB 配置
`external_tool_policy: "inherited"`。
