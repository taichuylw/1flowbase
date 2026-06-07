---
memory_type: project
title: MCP management issue tree confirmed
created_at: 2026-06-07 17
updated_at: 2026-06-07 17
decision_policy: verify_before_decision
scope:
  - https://github.com/taichuy/1flowbase/issues/770
  - https://github.com/taichuy/1flowbase/issues/771
  - https://github.com/taichuy/1flowbase/issues/772
  - https://github.com/taichuy/1flowbase/issues/773
status: issue_tree_discussion
keywords:
  - MCP management
  - mcp.search
  - mcp.get
  - capability binding
  - system settings
  - issue-tree
---

# MCP Management Issue Tree Confirmed

## 谁在做什么

用户在 `2026-06-07 17` 确认 MCP 管理的阶段性方向：MCP 注册应分层、可组合；默认只注入 `mcp.search` / `mcp.get`；MCP capability 应绑定后端接口 / service / 领域动作进行转换，复用后端权限、字段语义和审计。AI 已创建 GitHub issue tree：#770 是 L0 总控，#771 是 L1 ADR，#772 是 L2 workstream，#773 是首个 L3 execution task。

## 为什么这样做

MCP 的 `tools` / `resources` / `prompts` 是能力类型维度，1flowbase 的系统级 / 应用级 / Flow/Start / LLM 节点级是作用域与权限维度。两者需要组合，而不是把所有工具一次性注册给大语言模型。默认只注入目录层元工具，可以让模型先搜索、再获取完整能力定义、再按需 materialize 具体工具。

## 为什么要做

如果 MCP capability 自己长出一套业务权限和字段语义，会和后端 contract 分叉。把 MCP 定义成后端 capability 的协议投影，可以让权限一致性、接口字段语义、参数映射、返回值映射和审计保持同源。

## 当前 issue tree

- L0 #770 `[讨论]MCP 分层能力目录与系统管理入口`
- L1 #771 `[讨论]ADR：MCP 作为后端接口能力投影与渐进式注入`
- L2 #772 `[讨论]MCP 管理与运行时能力目录工作流`
- L3 #773 `[讨论]在系统设置挂载 MCP 管理审阅入口`

## 执行门禁

当前全部 issue 仍在 `phase:discussion`。实现前需要用户确认 #773 的页面边界；若 #773 需要后端 catalog API、DB migration、真实 MCP runtime 执行、权限策略或字段 contract 变更，必须停止并回到 #771/#772 拆新 issue。
