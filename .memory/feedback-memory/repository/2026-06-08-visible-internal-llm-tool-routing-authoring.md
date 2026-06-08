---
memory_type: feedback
feedback_category: repository
topic: visible-internal-llm-tool-routing-authoring
summary: `visible_internal_llm_tool` 是 1flowbase runtime 路由层语义，不是 provider 插件语义；authoring 入口应是 LLM 节点的“挂载工具”开关和节点底部外置圆形工具连接点，画布默认不显示工具名方块。
keywords:
  - visible_internal_llm_tool
  - 挂载工具
  - 工具注册
  - LLM node
  - provider plugin boundary
  - execution_role
  - bottom connector
created_at: 2026-06-08 10
updated_at: 2026-06-08 17
last_verified_at: 2026-06-08 17
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

Authoring 侧不向用户暴露 `LLM 执行角色` 或 `execution_role` 下拉。LLM 节点只提供“挂载工具”开关；开启后编辑工具注册表。每条工具注册在同一个 LLM 节点底部显示一个外置圆形工具连接点，随节点移动，不占用左右主流程连接器。工具连接点要像左右连接器一样是节点边界端口：圆心落在节点底边，常态只显示蓝点白描边；hover / focus 时轻微放大并显示蓝色 halo，同时用 tooltip 显示工具名。实现时工具 handle 必须渲染在 `.agent-flow-node-card` 内部，用真实卡片作为绝对定位基准，不能挂在 React Flow node wrapper 下面再用 wrapper 高度推底边。画布默认不显示“挂载工具”静态文案、竖线、工具名方块或外置工具区。用户从该连接点拉线时，应生成普通可编排 `graph.edges`，`sourceHandle` 使用 `visible_internal_llm_tool:<connector_id>`；这条分支可以继续连接任意后续节点，由 runtime 在主 LLM tool call 时按路由执行，主流程普通 downstream 激活需跳过该 sourceHandle。

## 原因

用户纠正过：这个能力本质上像 1flowbase 把一次工具调用路由到一个可继续编排的节点分支，而不是切换或扩展大模型供应商插件。把它做成 provider 插件能力会污染插件边界；把它做成用户可见执行角色会让 authoring 心智变成“目标节点特殊化”，而用户真正定义的是主 LLM 注册了哪些内部工具，以及每个工具连接器对应哪条可编排分支。

## 适用场景

- 修改发布模型内部 LLM 挂载、tool callback loop、Answer Presentation 投影或 provider invocation 路径。
- 修改 LLM 节点 Inspector、node schema、flow schema、画布节点连接器或编译 topology 校验。
- 评估是否要改官方 OpenAI / OpenAI-compatible provider 插件来支持内部挂载工具时，先按本规则回到 1flowbase runtime 路由层。
- 未来讨论节点横向扩展体系时，可复用“节点底部附属连接器 + 指定 sourceHandle 的普通 graph edge”形态，但不要未经 ADR 扩大为任意 node-as-tool。
