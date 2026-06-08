---
memory_type: feedback
feedback_category: repository
topic: visible-internal-llm-tool-routing-authoring
summary: `visible_internal_llm_tool` 是 1flowbase runtime 路由层语义，不是 provider 插件语义；authoring 入口应是 LLM 节点的“挂载工具”开关和节点底部工具注册连接器，不暴露 `LLM 执行角色`。
keywords:
  - visible_internal_llm_tool
  - 挂载工具
  - 工具注册
  - LLM node
  - provider plugin boundary
  - execution_role
  - bottom connector
created_at: 2026-06-08 10
updated_at: 2026-06-08 10
last_verified_at: 2026-06-08 10
decision_policy: direct_reference
scope:
  - api/crates/orchestration-runtime
  - web/app/src/features/agent-flow
  - web/packages/flow-schema
  - ../1flowbase-official-plugins/runtime-extensions/model-providers
---

# Visible Internal LLM Tool Routing And Authoring

## 规则

`visible_internal_llm_tool` 是 1flowbase runtime 路由层能力：主 LLM 触发内部工具调用后，由 1flowbase 拦截并定向执行目标 LLM 节点；目标 LLM 节点照常使用自己的 provider/model 配置。provider 插件只接收一次正常 LLM 调用，不应该感知“内部挂载工具”这个产品语义，也不应该为了该能力调整供应商 wire shape。

Authoring 侧不向用户暴露 `LLM 执行角色` 或 `execution_role` 下拉。LLM 节点只提供“挂载工具”开关；开启后编辑工具注册表。每条工具注册在同一个 LLM 节点底部显示一个附属工具连接器，随节点移动，不占用左右主流程连接器，不生成普通 `graph.edges`，不进入 topology indegree/downstream。

## 原因

用户纠正过：这个能力本质上像 1flowbase 把一次工具调用路由到另一个 LLM 节点继续输出，而不是切换或扩展大模型供应商插件。把它做成 provider 插件能力会污染插件边界；把它做成用户可见执行角色会让 authoring 心智变成“目标节点特殊化”，而用户真正定义的是主 LLM 注册了哪些内部工具。

## 适用场景

- 修改发布模型内部 LLM 挂载、tool callback loop、Answer Presentation 投影或 provider invocation 路径。
- 修改 LLM 节点 Inspector、node schema、flow schema、画布节点连接器或编译 topology 校验。
- 评估是否要改官方 OpenAI / OpenAI-compatible provider 插件来支持内部挂载工具时，先按本规则回到 1flowbase runtime 路由层。
- 未来讨论节点横向扩展体系时，可复用“节点底部附属连接器、非普通 topology edge”的 authoring 形态，但不要未经 ADR 扩大为任意 node-as-tool。
