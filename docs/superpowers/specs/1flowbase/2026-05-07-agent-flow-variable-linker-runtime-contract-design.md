# Agent Flow 变量链接器与运行态契约设计

日期：2026-05-07

状态：已按开发期破坏性基线重写，并补入持久化、缓存、变量展示、流式 replay、插件版本锁定和 Data Model 写入审计项；待拆 implementation plan

取代文档：无

关联问题：
- Answer 节点调试变量缓存同时展示 `answer_template` 与 `answer`，且值相同，导致用户误以为存在两个等价输出变量。
- Template Transform、LLM、HTTP、Tool、Data Model 等节点也可能因为运行输入、运行输出和运行指标混合展示而产生同类认知混乱。
- 当前开发期允许重置数据库、重建默认草稿和改写 schema，因此本设计不为既有草稿、selector、snapshot 建立保留路径。

关联代码：
- `web/packages/flow-schema/src/index.ts`
- `web/app/src/features/agent-flow/lib/selector-options.ts`
- `web/app/src/features/agent-flow/lib/start-node-variables.ts`
- `web/app/src/features/agent-flow/lib/node-definitions/nodes/answer.ts`
- `web/app/src/features/agent-flow/lib/node-definitions/nodes/template-transform.ts`
- `web/app/src/features/agent-flow/schema/agent-flow-field-renderers.tsx`
- `web/app/src/features/agent-flow/components/bindings/SelectorField.tsx`
- `web/app/src/features/agent-flow/hooks/runtime/useAgentFlowDebugSession.ts`
- `web/app/src/features/agent-flow/lib/node-definitions/types.ts`
- `web/app/src/features/agent-flow/schema/node-schema-registry.ts`
- `web/app/src/features/agent-flow/schema/node-schema-fragments.ts`
- `web/app/src/features/agent-flow/schema/node-schema-adapter.ts`
- `web/app/src/features/agent-flow/components/detail/NodeDetailPanel.tsx`
- `web/app/src/features/agent-flow/components/inspector/NodeInspector.tsx`
- `web/app/src/features/agent-flow/lib/plugin-node-definitions.ts`
- `api/crates/orchestration-runtime/src/execution_engine.rs`
- `api/crates/orchestration-runtime/src/execution_state.rs`
- `api/crates/control-plane/src/orchestration_runtime/live_debug_run/continuation.rs`
- `api/crates/control-plane/src/orchestration_runtime/persistence.rs`
- `api/apps/api-server/src/routes/applications/application_runtime.rs`

参考实现：
- `../dify/web/app/components/workflow/hooks/use-workflow.ts`
- `../dify/web/app/components/workflow/hooks/use-workflow-variables.ts`
- `../dify/web/app/components/workflow/hooks/use-nodes-available-var-list.ts`
- `../dify/web/app/components/workflow/nodes/_base/hooks/use-available-var-list.ts`
- `../dify/web/app/components/workflow/nodes/_base/components/variable/var-reference-picker.tsx`
- `../dify/web/app/components/workflow/nodes/_base/components/variable/utils.ts`
- `tmp/graphon-inspect/graphon/nodes/answer/answer_node.py`

## 1. 文档目标

本文固定 1flowbase Agent Flow 节点链接器、变量链接器、运行态 payload 和调试变量缓存的边界。

核心目标：

1. Variable Picker、Variables tab、Debug Variable Cache 对“变量”的定义完全一致：变量只来自输出变量契约声明。
2. `bindings`、`input_payload`、`process_data`、`output_payload` 语义互斥。
3. `output_payload` 是节点完整业务产物对象；usage、route、attempt、provider metadata、raw response ref、错误详情或调试索引如需展示，作为该输出对象字段存在。
4. Answer 节点只暴露 `answer`；`answer_template` 只作为 resolved input 出现在 Trace Inputs。
5. LLM 节点的 provider stream events 属于“数据处理”，不混入输出；`text`、`usage`、结构化输出、reasoning/debug refs 等进入输出对象，哪些字段可被下游引用由输出变量契约声明。
6. 节点 meta、默认值、卡片、详情面板、运行面板、端口、策略和插件贡献纳入同一个 Node Runtime UI Contract。
7. 新增节点只需声明节点契约，即可接入节点选择器、画布卡片、详情面板、变量链接器、变量池和调试缓存。
8. 本设计按开发期破坏性基线推进；schema、默认文档、durable snapshot 和数据库可以重建。
9. RuntimeEventStream、durable run records、debug snapshot 和 frontend preview cache 各自有明确 owner，不互相伪装成真值。
10. 插件节点贡献按宿主声明式 contract v2 接入，不允许插件直接注入 React panel、基础设施连接或未注册 renderer。
11. Data Model CRUD 节点的输出、权限、作用域、副作用和重跑语义按动作矩阵固定。

本文不是 implementation plan；实现前需要拆单独 plan。

## 2. 当前问题事实

### 2.1 Answer 重复字段不是单点 bug

当前默认文档中 Answer 节点定义为：

```ts
bindings.answer_template = { kind: 'templated_text', value: '{{node-llm.text}}' }
outputs = [{ key: 'answer', title: '对话输出', valueType: 'string' }]
```

运行时 Answer 和 Template Transform 使用同一类执行分支：

```text
resolved input / rendered template -> first output key
```

因此 Answer 的 `input_payload.answer_template` 与 `output_payload.answer` 在内容上天然可能相等。

问题不在于二者值相等，而在于 Debug Variable Cache 把 `input_payload` 和 `output_payload` 合并到同一个 node cache。

### 2.2 同类风险会扩散到其他节点

只要一个节点的输出是对输入的加工、转发或渲染，就可能出现同类困惑：

1. `template_transform`: `bindings.template` 与 `outputs.text` 可能相同。
2. `llm`: `prompt_messages` 是运行输入；`text`、`usage`、`route`、`attempts`、`finish_reason` 是运行输出对象的一部分；下游默认只可引用输出契约声明的字段。
3. `http_request`: `url`、`headers`、`query`、`body` 是运行输入；`status_code`、`body`、`headers` 才是业务输出。
4. `tool` / `plugin_node`: 参数是运行输入；插件 output schema 声明哪些输出 selector 可被下游引用。
5. `data_model_*`: `query`、`payload`、`record_id` 是运行输入；`records`、`record`、`deleted_id`、`affected_count` 是默认可引用输出 selector。
6. `human_input`: `prompt` 与 form schema 是运行输入；resume payload 中的用户提交值进入输出对象，并按表单声明暴露 selector。

因此最终方案不能是“在某个面板不显示 `answer_template`”，而必须重建运行态分层。

### 2.3 当前代码暴露的硬问题

当前代码中存在五类边界破损：

1. 前端 selector option 直接读取 `getNodeVariableOutputs(node)`，没有统一变量链接器 source、scope 和 filter 语义。
2. 前端 debug cache 从 trace items 和 run detail 同时合并 node input 与 node output。
3. 后端 durable debug variable snapshot 把 `flow_run.input_payload` 原样 merge 到 variable cache。
4. LLM runtime 的 `output_payload` 已承载完整节点产物，但当前缺少“完整输出对象”和“下游变量声明”的显式分离，导致实现倾向于把 output 当只面向下游变量的过滤对象。
5. 节点定义层只有轻量 `NodeDefinition + schema UI` 骨架，节点 meta、default config、node card、detail panel、single-run/debug form、ports、retry/error policy、plugin contribution 没有形成统一契约。

这些问题必须按契约重建，不应在 UI 层用过滤规则掩盖。

## 3. Dify 参考结论

Dify 的成熟点不是某个 Answer 节点字段命名，而是把两类链接分开：

1. 节点连线器：负责执行拓扑、分支、handle、节点插入删除。
2. 变量链接器：负责可见变量、变量类型、作用域、特殊变量和变量选择器。

Dify 的变量链接器链路是：

```text
current node
  -> getBeforeNodesInSameBranchIncludeParent / getTreeLeafNodes
  -> getNodeAvailableVars
  -> toNodeAvailableVars
  -> variable picker type / file / constant filtering
```

可吸收的设计原则：

1. 变量选择器不从运行缓存反推变量。
2. 变量选择器只来自节点定义或节点配置声明的输出契约。
3. 上游可见性由图拓扑和容器/分支作用域决定。
4. env、conversation、sys、rag 等特殊变量作为独立 source 进入变量链接器，不伪装成普通节点输入。
5. 运行 trace 中展示 input、可选 process_data、完整 output；变量池只暴露 output contract 声明的 selector。
6. node meta 和 panel 是节点契约的一部分，不能散落在 picker、card、schema adapter、field renderer、runtime panel 和 plugin contribution 多处。
7. 节点默认值、配置面板、运行面板、单节点试跑表单、错误处理、重试策略和输出变量应该由同一节点契约派生。

吸收但不复制的部分：

1. 不复制 Dify 的 React 组件、store 结构和节点目录实现细节。
2. 不引入 Dify 的完整特殊变量体系。
3. 不把 Dify 的 `answer` 配置字段命名直接迁入 1flowbase；1flowbase 继续用 `answer_template` 表示 Answer 输入模板。
4. 不把 metadata/error/debug 自动当成下游变量，再靠可见性字段筛掉。
5. 1flowbase 采用自己的 Node Runtime UI Contract：用声明式 contract 驱动节点选择器、卡片、Inspector、Detail Panel、运行态面板、变量链接器和插件节点贡献。
6. Dify 的运行结果面板只在存在 `process_data` 时展示“数据处理”，LLM 输出变量中包含 `usage`；1flowbase 吸收这一点：不要为所有节点硬造数据处理层，也不要把 usage/debug/raw refs 拆成顶层分类。

## 4. 信息架构诊断

### 4.1 问题清单

| # | 问题类型 | 位置 | 描述 | 严重度 |
|---|---|---|---|---|
| 1 | 分类不互斥 | Debug Variable Cache | 同一节点输入与输出混合展示，用户无法判断哪个可被下游引用。 | 高 |
| 2 | 层级错位 | Variables tab | 运行输入属于 Trace 深度，却出现在变量概览层。 | 高 |
| 3 | 分类维度错误 | Runtime payload / Last Run | 把完整节点产物拆成 output、metrics、error、debug 多个顶层分类，和 Dify 的输入 / 可选数据处理 / 输出心智模型不一致。 | 高 |
| 4 | 入口语义混乱 | Variable Picker / Debug Cache | 变量选择器基于 outputs，调试缓存基于 input+output，两个入口对“变量”的定义不一致。 | 高 |
| 5 | 状态真值分裂 | Frontend cache / backend variable pool | 前端缓存和后端 variable pool 没有共享同一套输出变量声明规则。 | 高 |
| 6 | 节点契约分裂 | Node definition / schema UI / panel / runtime | 节点 meta、默认值、卡片、面板、端口、运行态和插件贡献分散定义，新增节点需要多处特判。 | 高 |

### 4.2 修正后的信息深度

| 信息 | 深度 | 容器 | 规则 |
|---|---|---|---|
| 可被下游引用的变量 | L0/L1 | Variable Cache / Variable Picker | 只来自输出变量契约声明，不从运行样本临时推断。 |
| 当前节点解析后的输入 | L1 | Trace item detail / node_run.input_payload | 必须持久化并可审计、可 full-load；不作为输出，不进入变量缓存。 |
| 数据处理对象 | L1 | Trace Process Data | 可选；只有节点确实存在中间转换/处理对象时展示。 |
| 节点完整输出对象 | L1 | Trace Outputs / Last Run Outputs | 展示节点最终产物；usage、错误、debug ref、raw ref 等需要呈现时都作为对象字段。 |
| 下游变量声明 | L1 | Output Contract / Variable Picker | 声明完整输出对象中哪些 selector 可被下游引用；声明不复制运行值。 |

## 5. 范围

### 5.1 本阶段范围

1. 固定 Agent Flow 节点 `bindings / outputs / runtime trace / variable cache` 的分层 contract。
2. 重建 Debug Variable Cache 的语义：只展示输出变量契约声明的 selector。
3. 重建 durable debug variable snapshot 的语义：只聚合 Start 公开输入变量和节点输出契约声明的 selector。
4. 建立前端变量链接器接口，替代散落的 selector option 生成逻辑。
5. 建立 Node Runtime UI Contract，统一 node meta、default config、card、panel、ports、runtime schema、policy 和 plugin contribution。
6. 将 output contract 收敛为 selector declaration；metadata、debug、error、usage 等可以存在于完整输出对象，但只有契约声明的 selector 能被下游引用。
7. 明确 Answer、Template Transform、LLM、HTTP、Tool、Plugin、Data Model、Human Input 的输入输出归属。
8. 将 LLM usage、route、attempt、finish reason、provider metadata 纳入完整输出对象，并用输出契约控制下游可见性。
9. 建立 schema 重置和默认文档重种子策略。
10. 建立 snapshot key、snapshot schema version、document hash、run scope 和失效策略。
11. 建立 RuntimeEventStream 与 durable debug event 的流式返回契约。
12. 建立 plugin node contribution v2 到 Node Runtime UI Contract 的映射和校验规则。
13. 建立 Data Model 节点动作级输出、权限、scope 和副作用矩阵。
14. 建立大对象 offload、预览截断、full-load API 和 run retention/GC 契约。
15. 建立 LLM 流式事件 cursor/replay/幂等消费契约。
16. 建立 plugin contribution 版本锁定、compile snapshot 和 stale contribution 处理规则。

### 5.2 非目标

1. 不重写画布连线交互。
2. 不实现 Dify 全量变量体系。
3. 不把所有节点字段重命名成 Dify 命名。
4. 不把运行输入从 Trace 中删除。
5. 不让前端变量缓存替代后端 variable pool。
6. 不在本 spec 中实现代码。

## 6. 设计原则

1. 节点连线只表达执行拓扑，变量链接只表达数据依赖。
2. `bindings` 是输入声明，不是输出变量。
3. `input_payload` 是运行时解析结果，只服务 Trace 和调试。
4. `process_data` 是可选中间处理对象；没有中间转换心智的节点不展示该段。
5. `output_payload` 保存唯一完整节点产物对象，不再被定义为只包含下游变量的裁剪对象。
6. `outputs` / output contract 只声明 `output_payload` 上哪些 selector 可作为下游变量；声明不复制运行值。
7. usage、duration、route、attempt、finish reason、错误、debug refs、raw refs 如需展示，进入 `output_payload` 或其 artifact/ref 字段；是否可被下游引用由 output contract 决定。
8. Variable Picker、Variables tab、Debug Variable Cache 必须共享同一套输出变量声明。
9. Node Picker、Node Card、Inspector、Detail Panel、Last Run Panel 必须共享同一套节点契约。
10. 开发期以长期契约正确性优先，不为既有草稿或快照牺牲边界。
11. PostgreSQL run records 是 durable truth；RuntimeEventStream 是短期实时通道；debug snapshot 和 frontend preview cache 只做读取加速。
12. 缓存 key、失效规则、snapshot 恢复顺序归 runtime resource owner；cache-store / Redis 只作为 HostExtension provider 实现宿主 contract。
13. `output_payload` 写入前必须经过 output object builder；builder 负责完整输出对象结构、ref/offload 和输出契约 selector 校验。
14. LLM live delta 先进入 RuntimeEventStream；最终 provider stream events 收敛到 process_data / 数据处理，不进入 output_payload；变量池只读取 output contract 声明的 selector。
15. 大对象、raw response、artifact 和内部证据默认以 ref 进入 output_payload，不内联大对象本体；provider event 默认进入 process_data 或其 artifact ref。
16. 写入型 Data Model 节点必须声明副作用等级，调试运行、重跑和 checkpoint 恢复必须能解释是否会重复写。
17. 插件贡献只能声明宿主支持的能力和 schema；基础设施缓存、队列、锁、事件总线不由 RuntimeExtension 或 CapabilityPlugin 直连。
18. 缓存、snapshot 和 draft debug 变量必须按 workspace、actor、draft、document hash 和 run scope 隔离；任何跨 actor 或跨 workspace 恢复都视为数据泄露。
19. 变量显示身份使用稳定 selector key，不使用 output title 作为身份；title 只作为辅助展示文案。
20. 插件、Data Model、LLM 等 executor 的 raw output 不能直接成为持久化 payload；必须先经过 payload builder 与 schema 校验。

## 7. 目标概念模型

```text
Flow Node Definition
  config: static configuration
  bindings: input binding declarations
  outputs: output variable selector contract, no runtime values

Node Runtime UI Contract
  meta: label / summary / icon / category / help / capabilities
  defaults: default config / bindings / outputs
  card: node card blocks
  panel: inspector and detail panel sections
  ports: handles and connection rules
  runtime: input / optional process_data / output display schema
  policies: retry / error handling / timeout / single-run form

Runtime Node Run
  input_payload: resolved inputs, trace only
  process_data: optional intermediate processing object
  output_payload: complete node result object
  output_contract: selectors over output_payload that downstream may reference

Variable Linker
  sources: start inputs / node output contract selectors / explicit special sources
  visible nodes: graph topology + branch/container scope
  visible variables: declared selectors from visible sources
  filters: valueType / source kind
```

## 8. Flow Schema 契约

### 8.1 Schema 基线

本设计引入新的 authoring schema baseline。实现时应同步更新前端 schema package、后端默认文档、编译器输入校验和本地开发数据库种子。

```ts
export const FLOW_SCHEMA_VERSION = '1flowbase.flow/v2';
```

规则：

1. v2 文档不读取 v1 草稿。
2. 本地开发数据库可直接重置。
3. 默认应用草稿按 v2 重新生成。
4. 编译器遇到非 v2 文档直接拒绝编译。
5. API 返回的 draft document 必须是 v2，否则视为数据基线错误。

### 8.2 输出契约

`FlowNodeOutputDocument` 只表达下游可引用变量声明。它描述 `output_payload` 上的 selector，不保存运行值，也不要求 `output_payload` 只包含这些字段。

```ts
export interface FlowNodeOutputDocument {
  key: string;
  title: string;
  valueType: string;
  selector?: string[];
  description?: string;
}
```

规则：

1. 出现在 `outputs` 中的字段必须可进入 Variable Picker。
2. 出现在 `outputs` 中的字段必须可进入 Variables tab。
3. 出现在 `outputs` 中的字段必须可进入 runtime variable pool 的逻辑视图。
4. 出现在 `outputs` 中的字段必须可被下游 selector 引用。
5. `selector` 缺省时等价于 `[key]`；结构化字段必须显式声明 selector path，不从运行样本推断。
6. metadata、debug、error、usage、route、attempt、finish reason、provider raw response 可以在完整 `output_payload` 中存在；只有写入 `outputs` 的 selector 才能被下游引用。
7. output contract 允许节点自定义输出变量；校验目标是 selector 是否能被节点输出结构支持，而不是运行时另存一份公开输出对象。

### 8.3 绑定契约

`FlowBinding` 继续表达当前节点需要的输入。

```ts
bindings: Record<string, FlowBinding>
```

规则：

1. `bindings.*` 永远表示输入声明。
2. binding key 使用输入语义名称，例如 `answer_template`、`prompt_messages`、`query`、`payload`、`record_id`。
3. binding key 不需要与 output key 同名。
4. binding 解析后的值进入 `input_payload`。
5. binding 不能直接进入变量缓存。
6. selector 只能引用变量链接器返回的输出变量声明。

### 8.4 运行展示声明

节点可以声明输入、可选数据处理和输出对象的展示 schema；展示 schema 不参与变量链接器，变量链接器只读取 output contract。

```ts
export interface FlowNodeRuntimeSchemaDocument {
  inputs?: Array<{ key: string; title: string; valueType: string }>;
  processData?: Array<{ key: string; title: string; valueType: string }>;
  outputs?: Array<{ key: string; title: string; valueType: string }>;
}
```

规则：

1. runtime schema 只服务 Trace、observability、debug console。
2. runtime schema 不生成 selector。
3. runtime schema 不写入 variable pool。
4. runtime schema 不出现在 Variables tab。

## 9. Node Runtime UI Contract

### 9.1 当前状态

1flowbase 已有轻量节点 UI 骨架：

1. `NodeDefinition` 提供 label、summary、helpHref、sections、fields。
2. `node-schema-registry` 将节点定义转换为 card、detail tabs、runtime slots。
3. `NodeDetailPanel` 使用统一 dock panel，包含 `设置` 与 `上次运行`。
4. `NodeInspector` 使用 schema renderer 渲染字段和 view block。
5. `plugin-node-definitions` 能根据 capability plugin contribution 生成 picker option 和 outputs。

这套骨架方向正确，但仍缺少长期契约：

1. node picker、node factory、node card、inspector、detail panel、last run panel 和 runtime trace 没有共享完整节点 contract。
2. default config、default bindings、default outputs 仍主要由 node factory 和具体节点文件拼接。
3. ports/handles、container 能力、single-run/debug form、retry/error policy 还没有进入节点契约。
4. plugin contribution 只接入 picker 和 output schema，没有接入 panel schema、runtime schema 和 policy schema。
5. 节点运行态面板仍按通用 Inputs/Outputs/Metadata 展示，无法表达节点自己的 metrics、error、debug 证据结构。

### 9.2 目标契约

每个节点类型必须能编译出一个 Node Runtime UI Contract。

```ts
export interface AgentFlowNodeRuntimeUiContract {
  schemaVersion: '1flowbase.node-runtime-ui/v1';
  nodeType: FlowNodeType;
  meta: {
    label: string;
    summary: string;
    icon?: string;
    category: string;
    helpHref: string | null;
    capabilities: string[];
  };
  defaults: {
    config: Record<string, unknown>;
    bindings: Record<string, FlowBinding>;
    outputs: FlowNodeOutputDocument[];
  };
  ports: {
    inputs: Array<{ key: string; title: string; required: boolean }>;
    outputs: Array<{ key: string; title: string; branchKey?: string }>;
  };
  card: {
    blocks: SchemaBlock[];
  };
  panel: {
    header: SchemaBlock[];
    tabs: Array<{
      key: string;
      title: string;
      blocks: SchemaBlock[];
    }>;
  };
  runtime: {
    inputs: Array<{ key: string; title: string; valueType: string }>;
    processData?: Array<{ key: string; title: string; valueType: string }>;
    outputs: Array<{ key: string; title: string; valueType: string }>;
  };
  policies: {
    retry?: Record<string, unknown>;
    errorHandling?: Record<string, unknown>;
    timeout?: Record<string, unknown>;
    singleRunForm?: SchemaBlock[];
  };
}
```

规则：

1. Node Picker 只读取 `meta` 和 `defaults`。
2. Node Factory 只读取 `defaults` 创建节点文档。
3. Canvas Card 只读取 `card` 和 `meta`。
4. Inspector 与 Detail Panel 只读取 `panel`。
5. Variable Linker 只读取 `defaults.outputs` 或节点实例上的 output contract selector。
6. Last Run Panel 只读取 `runtime` 描述 inputs、可选 process_data、outputs 的展示结构。
7. 端口和连线 handle 只读取 `ports`，不由画布组件硬编码节点类型。
8. retry、error handling、timeout、single-run/debug form 只读取 `policies`。
9. schema adapter 只负责读写节点实例值，不负责决定某类节点有哪些面板能力。

### 9.3 Builtin 节点接入

内置节点的 contract 来源为 `web/app/src/features/agent-flow/lib/node-definitions/**`。

规则：

1. 每个内置节点有且只有一个 contract builder。
2. contract builder 输出 meta、defaults、ports、card、panel、runtime、policies。
3. 节点类型的默认 outputs 必须来自 contract builder。
4. 节点 detail panel section 必须来自 contract builder。
5. 节点卡片展示和 help link 必须来自 contract builder。
6. 节点试跑需要的输入表单必须来自 contract builder 的 `policies.singleRunForm`。
7. 节点运行态 input/process_data/output 的展示结构必须来自 contract builder 的 `runtime`。

### 9.4 Plugin 节点接入

Capability Plugin 的 node contribution 需要映射到同一个 Node Runtime UI Contract。

插件贡献最小字段：

```text
contribution identity:
  plugin_id
  plugin_version
  plugin_unique_identifier
  package_id
  contribution_checksum
  contribution_code
  node_shell
  schema_version

ui/runtime contract:
  title
  description
  category
  input_schema
  output_schema
  panel_schema
  runtime_schema
  policy_schema
```

规则：

1. plugin contribution 不能直接提供 React panel。
2. plugin contribution 只能使用宿主注册的 field/view renderer。
3. plugin `output_schema.outputs` 必须等价于下游可引用 selector 声明。
4. plugin `panel_schema` 只描述配置表单和静态 view block。
5. plugin `runtime_schema` 只描述 Trace 展示结构，不生成变量。
6. plugin `policy_schema` 只描述 retry、error handling、timeout 和 single-run/debug form。
7. dependency status 不为 ready 时，Node Picker 可以展示禁用项，但不能创建不可编译节点。
8. 节点实例保存的是编译时 contribution identity 与 output schema snapshot，不从当前已安装插件动态反推旧节点输出。
9. plugin 升级后，既有节点必须显式 recompile 或接受迁移提示；运行时发现 package 缺失、checksum 不匹配或 output schema 漂移时，编译失败，不降级执行。

插件 contribution v2 必须补齐以下硬边界：

```text
schema_version: 1flowbase.node-contribution/v2
plugin_unique_identifier: provider/package identity
compiled_contribution_hash: immutable compile snapshot
panel_schema: host-renderer blocks only
runtime_schema: inputs / optional process_data / outputs display schema
policy_schema: timeout / retry / error handling / side effect / single-run form
renderer_allowlist: field and view renderer codes
output_schema.outputs: output variable selector declarations
```

校验规则：

1. v2 contribution 不接受 unknown renderer code。
2. v2 contribution 允许声明 `usage`、错误或 debug ref 等输出 selector，但必须显式标注 valueType、selector 和可引用语义；禁止仅用 `__*` 内部索引作为面向用户的变量 key。
3. v2 contribution 必须声明 `side_effect`：`none`、`external_read`、`external_write` 或 `durable_write`。
4. v2 contribution 的 `output_schema.outputs` 是变量链接器来源；`runtime_schema.outputs` 可以展示完整输出对象，不要求与 output_schema 同 key。
5. v1 contribution 可在开发期直接拒绝编译或通过重种子替换，不写兼容 mapper。
6. RuntimeExtension / CapabilityPlugin 不能声明或消费 `cache-store`、`distributed-lock`、`event-bus`、`task-queue` 等宿主基础设施连接。
7. HostExtension provider 可以实现基础设施 contract，但 cache 数据不能成为 Agent Flow 变量真值。
8. plugin executor 返回未声明 output key 时，仍可进入完整 `output_payload`；但除非 output schema 声明 selector，否则不得进入 variable pool 或 Variable Picker。
9. renderer allowlist 是宿主能力白名单，不是插件自带能力；未知 field/view renderer 不能进入持久化草稿。
10. plugin invocation metadata、凭据状态、重试、provider route 和内部调用 ID 可以进入完整输出对象；是否可下游引用由 output schema 显式决定。

### 9.5 Panel 信息深度

Node Runtime UI Contract 固定节点详情的信息深度：

| 面板区域 | 深度 | 内容 | 变量关系 |
|---|---|---|---|
| Node Card | L0 | 类型、别名、摘要、关键状态 | 不展示 resolved input。 |
| Inspector Config | L1 | 当前节点配置、bindings、策略 | 可以编辑 selector，但不展示变量缓存。 |
| Output Contract | L1 | 当前节点可下游引用的输出 selector | Variable Linker 来源，不保存运行值。 |
| Last Run Inputs | L1 | resolved inputs | Trace only。 |
| Last Run Process Data | L1 | 可选中间处理对象 | 只有节点确实有 process_data 时展示。 |
| Last Run Outputs | L1 | 完整输出对象 | 变量池只暴露 output contract 声明的 selector。 |

规则：

1. Config panel 不读取 runtime cache 生成字段。
2. Last Run panel 不反推 selector options。
3. Output Contract panel 只展示 selector declaration，不展示运行值。
4. 运行证据可以在 Last Run Outputs 中展示；未被 output contract 声明的字段不能进入变量选择器。
5. 单节点试跑表单只解决运行输入收集，不改变节点 contract。

## 10. 变量链接器契约

### 10.1 前端能力边界

将 `listVisibleSelectorOptions(document, nodeId)` 收敛为语义明确的变量链接器。

```ts
export type AgentFlowVariableSourceKind =
  | 'start_input'
  | 'node_output'
  | 'system';

export interface AgentFlowAvailableVariable {
  sourceKind: AgentFlowVariableSourceKind;
  nodeId: string;
  nodeLabel: string;
  key: string;
  title: string;
  valueType: string;
  selector: string[];
  displayLabel: string;
}

export interface AgentFlowVariableScopeOptions {
  sourceKinds?: AgentFlowVariableSourceKind[];
  valueTypes?: string[];
  containerId?: string | null;
  branchId?: string | null;
}

export function listAvailableVariables(
  document: FlowAuthoringDocument,
  nodeId: string,
  options?: AgentFlowVariableScopeOptions
): AgentFlowAvailableVariable[];
```

### 10.2 可见节点规则

1. 普通节点只能看到当前节点的上游节点。
2. Start 派生输出按 Start source 进入变量链接器。
3. `if_else` 只表达控制流；没有显式 output contract 时不暴露变量。
4. 同一 `containerId` 内按图拓扑计算可见性。
5. 容器内部节点可以看到父容器入口之前的上游输出变量声明。
6. loop/iteration 内部 item 变量必须作为明确 source kind 接入，不写入普通节点 outputs。
7. env/session/global 类变量必须作为 `system` 或专门 source kind 接入，不伪装成 Start input 或 node output。

### 10.3 可见变量规则

1. 只读取 Start 派生公开输入和节点 `outputs` selector declaration。
2. selector path 基线保持 `[nodeId, key]`；如果 `outputs[*].selector` 存在，运行取值时映射到 `output_payload` 的 selector path。
3. 结构化输出的深层 path 必须来自 output schema，不来自运行样本。
4. 变量链接器按 source kind 和 valueType 过滤。
5. 不提供“从运行样本临时选择字段”的变量开关。
6. 如果 selector 指向不存在的输出变量声明，文档校验直接失败。

### 10.4 UI 规则

1. 变量选择器文案使用“选择上游输出”，不使用“选择缓存字段”。
2. 变量块主展示使用 `node.alias/key`；`output title` 只能作为辅助说明，不参与变量身份、selector、cache key 或测试断言。
3. Variables tab 展示节点级 output contract 变量，不递归平铺完整输出对象内部字段。
4. Trace Inputs 展示 resolved inputs。
5. Trace Process Data 只在节点返回 `process_data` 时展示。
6. Trace Outputs 展示完整 `output_payload`。
7. 失效 selector 在表单中显示正式错误状态，不显示“可继续运行”的提示。
8. Debug Variable Cache 按节点输出对象展示，不把对象内部字段递归平铺成独立缓存条目。
9. Variable Picker 需要选择结构化字段时，只能按 output schema 展开字段级 selector，不从运行样本展开。
10. Run Context、Environment、Session、Trace Inputs 都不能放进 Variable Cache 分组；这些内容保留独立只读分组。

## 11. Runtime 契约

### 11.1 NodeExecutionTrace

运行时 trace 固定提供三类核心 payload：

```text
input_payload
process_data
output_payload
```

规则：

1. `input_payload` 保存 resolved inputs，必须持久化到 node run trace，用于调试、审计、回放和 full-load。
2. `process_data` 保存可选中间处理对象；没有中间处理语义的节点保持 `null` 或空。
3. `output_payload` 保存唯一完整节点产物对象。
4. usage、duration、route、attempt、finish_reason、preview_mode、错误、raw response ref、artifact ref、internal evidence 等需要面向用户解释的业务产物，都归入 `output_payload` 的约定字段或 ref。
5. provider stream events 不直接成为变量；需要保留时进入 `process_data.provider_events` 或其 artifact ref，并在 Last Run 中显示为“数据处理”。
6. `input_payload` 不是输出，不进入 Variable Cache、Variable Picker 或 variable pool。

### 11.2 Variable Pool

运行时 variable pool 只暴露输出变量契约声明的 selector。

```text
variable_pool[node_id][output.key] = read(output_payload, output.selector ?? [output.key])
```

`variable_pool` 是 `node_run.output_payload + node.outputs` 的派生逻辑视图；实现可以为了执行效率缓存解析后的值，但 durable truth 仍是 node run 的完整 `output_payload` 和节点 output contract。

禁止：

1. 把 `input_payload` 写入 variable pool。
2. 把未声明 selector 的 `output_payload` 字段写入 variable pool。
3. 把错误、debug ref 或 provider raw event 自动写入 variable pool；除非 output contract 或错误处理策略显式声明这些 selector。
4. 从运行样本动态推断 selector。
5. 为下游变量另存一份与 `output_payload` 重复的 durable payload。

### 11.3 Debug Snapshot

持久化 debug variable snapshot 只聚合：

1. Start 节点公开输入变量。
2. 每个 node run 按 output contract 从 `output_payload` 派生出的变量。

规则：

1. 不读取 `node_run.input_payload` 构造 variable cache。
2. 不读取 `flow_run.input_payload` 构造 variable cache，Start 节点除外。
3. 不从未声明 selector 的 `output_payload` 字段构造 variable cache。
4. snapshot 是变量缓存的恢复加速层，不是运行真值来源。
5. Run Context 单独展示本次运行起始输入。

### 11.4 持久化、缓存与 snapshot 真值

运行态存储分成四层：

| 层 | Owner | 用途 | 真值关系 |
|---|---|---|---|
| Flow Run / Node Run | durable storage | 当前状态、payload、错误、指标、审计入口 | durable truth |
| RuntimeEventStream | runtime service | live delta、node lifecycle、首 token 加速 | realtime channel |
| Debug Event / Artifact | observability / object storage | 可恢复调试事件、大对象和 raw ref | durable evidence |
| Debug Variable Snapshot | runtime resource owner | editor 打开时恢复变量缓存 | acceleration cache |

snapshot key 固定包含：

```text
application_id
workspace_id
actor_user_id
draft_id
document_hash
flow_schema_version
snapshot_schema_version
debug_session_id
latest_completed_or_running_run_id
```

规则：

1. `document_hash` 改变后旧 snapshot 不再参与恢复。
2. `flow_schema_version` 或 `snapshot_schema_version` 改变后旧 snapshot 直接失效。
3. snapshot 合并顺序固定为：Start 公开输入 -> 按 run order 从 node output contract 派生出的变量。
4. 同一节点多次运行时，默认以最新 node run 派生变量覆盖旧值；需要历史对比时走 Run Detail，不走 Variable Cache。
5. snapshot 不从多个 draft 或多个 document hash 混合恢复。
6. frontend preview cache 可以先展示 RuntimeEventStream 最新 output，但最终必须被 durable run detail / snapshot 对齐。
7. cache-store / Redis / local ring buffer 只保存加速数据；缓存丢失不能影响 durable run 的可解释性。
8. audit、billing、checkpoint 和 callback 所需事件不能只停留在易失缓存。
9. snapshot 不跨 `workspace_id`、`actor_user_id` 或 `debug_session_id` 恢复；共享应用的不同编辑者必须看到各自的 draft debug 变量。
10. snapshot 只读取状态为 succeeded 或显式 waiting-success checkpoint 的节点输出变量声明；failed/cancelled/running 中未完成节点不进入 durable variable cache，除非错误处理策略明确产出可引用异常变量。
11. snapshot 合并排序必须稳定：`flow_run.started_at, flow_run.id, node_run.index, node_run.started_at, node_run.id`；同一节点覆盖旧值时必须可解释到具体 `node_run_id`。
12. 运行中 snapshot 属于 partial cache，response 必须携带 `snapshot_completeness` 或等价状态；前端需要标识它不是完整 durable truth。

### 11.5 Payload Builder 与输出变量 selector

每个节点运行完成后统一经过 payload builder：

```text
resolved inputs
  -> node executor
  -> raw execution result
  -> output object builder
  -> output contract selector validation
  -> node_run payload persistence
  -> variable_pool logical view update
```

规则：

1. output object builder 生成唯一完整 `output_payload`，不按下游变量声明裁剪节点产物。
2. output contract selector validation 只校验声明给下游引用的 selector；未声明字段可以保留在完整输出对象中。
3. LLM、Plugin、HTTP、Data Model 都不能绕过统一 payload builder 直接写 variable pool。
4. failure path 可以写完整错误输出对象；如需可被下游引用的异常变量，必须由显式错误处理策略或 output contract 声明 selector。
5. live debug run 与 non-stream debug run 必须使用同一 payload builder。
6. checkpoint 的 `variable_snapshot` 只保存 output contract 派生变量。
7. payload builder 是 `node_run.output_payload` 和 selector 派生变量的唯一写入口；executor raw result 只能作为 builder input 或 output ref/artifact。
8. unknown output key 的默认策略是保留在完整输出对象但不暴露为变量；如果节点声明了 strict output object schema，才按 schema reject-and-record。
9. builder 需要返回 `input_payload`、可选 `process_data`、完整 `output_payload` 三个互斥对象；互斥失败直接视为 runtime contract error。

### 11.6 RuntimeEventStream 与 LLM 流式返回

RuntimeEventStream 是运行事件通道，不是变量缓存，也不是 key/value cache。

LLM 流式事件分层：

| 事件 / 数据 | 去向 | 是否进 variable pool |
|---|---|---|
| `text_delta` | RuntimeEventStream + durable debug event | No |
| `reasoning_delta` | RuntimeEventStream + durable debug event + `output_payload.reasoning_content/ref` | 仅在 contract 声明时 |
| `usage_delta` / `usage_snapshot` | RuntimeEventStream + `output_payload.usage` | 仅在 contract 声明时 |
| provider raw event | `process_data.provider_events` / provider event artifact ref | No |
| final answer text | `output_payload.text` | Yes，若 contract 声明 `text` |
| structured output | `output_payload.structured_output` | Yes，仅在 contract 声明时 |
| finish reason / route / attempts | `output_payload.finish_reason/route/attempts` | 仅在 contract 声明时 |
| provider metadata / tool calls / MCP calls | `output_payload` ref/object fields | 仅在 contract 声明时 |

规则：

1. SSE 首 token 可以早于 node run durable 更新，但不能早于 run accepted event。
2. text delta 不触发每 token variable cache rebuild。
3. final `output_payload.text` 是最终可复制的业务答案；是否可下游引用取决于 output contract 是否声明 `text`。
4. 如果 provider 在首 token 后失败，已发出的 delta 保留在 RuntimeEventStream / durable event / 输出对象 ref 中；是否写入可引用异常变量取决于错误处理策略。
5. RuntimeEventStream provider 可以从 local ring buffer 升级到 Redis Streams / NATS / Kafka 等 HostExtension provider；Core 只依赖 `RuntimeEventStream` contract。
6. durable debug event 持久化可以异步，但持久化失败必须可诊断。
7. 每个 stream event envelope 必须包含 `event_id`、`run_id`、`node_run_id`、`event_type`、`sequence`、`created_at`；delta 事件还必须包含 `delta_index` 和 `content_type`。
8. SSE reconnect 使用 `last_event_id` 或等价 cursor replay；前端按 `event_id/sequence` 幂等应用，不能重复拼接 delta。
9. durable debug event 读模型可以合并 delta，但必须保留 `node_run_id`、`event_type`、`sequence_start`、`sequence_end`、`content_type`、`is_truncated` 和 artifact/ref 信息。
10. RuntimeEventStream buffer 溢出策略必须显式：丢弃最旧 live event 时写 warning event 或服务端日志；durable-required event 不能只依赖已丢弃的 live buffer。
11. terminal event 包括 `flow_finished`、`flow_failed`、`flow_cancelled`、`waiting_human`、`waiting_callback`；terminal 之后同一 run 的 stream 必须关闭或进入明确等待态。

### 11.7 输出对象中的 debug/ref 字段

完整 `output_payload` 可以承载调试证据索引和大对象 ref；这些字段不会因为存在于输出对象中自动成为下游变量。

建议字段：

```text
raw_response_ref
provider_events_ref
artifact_refs
context_projection_id
attempt_ids
winner_attempt_id
tool_call_refs
mcp_call_refs
internal_evidence
```

规则：

1. 小型 debug metadata 可以内联；大文本、大 JSON、文件和 raw provider response 只存 ref。
2. debug/ref 字段不进入 Variables tab、Variable Picker、Debug Variable Cache，除非 output contract 显式声明 selector。
3. Trace Outputs 读取完整 `output_payload` 和 artifacts；Variable Picker 只读取 output contract。
4. debug artifact 的生命周期跟随 run retention policy，不跟随 editor preview cache。

### 11.8 大对象、offload 与预览截断

大对象处理不是 UI 优化，而是持久化与调试契约的一部分。

规则：

1. `input_payload`、`process_data`、`output_payload` 和 draft variable snapshot 都必须有 inline size budget。
2. 超过 inline budget 的文本、JSON、文件列表和 provider raw response 必须 offload 到 object storage，并在 durable record 中保存 ref。
3. 预览 payload 必须携带 `is_truncated`、`original_size_bytes`、`preview_size_bytes`、`content_type`、`artifact_ref`。
4. Trace 与 Variable Cache 默认读取预览；用户展开完整值时走 full-load API，不从 cache-store 反查。
5. offload artifact 生命周期跟随 run retention / draft variable retention；删除 run 或 draft variable 时必须进入 GC 队列。
6. Variable Picker 不能从 offload 内容推断字段；结构化字段只来自 output schema。
7. offload 失败时，完整输出对象写入失败或降级必须有正式错误字段或 artifact 状态，不能把大对象截断后伪装成完整输出。

## 12. 节点级契约

### 12.1 Start

来源：

1. `config.input_fields`
2. 内置 `query`
3. 内置 `files`

Start 输入变量：

```text
node-start.<custom_input_key>
node-start.query
node-start.files
```

规则：

1. Start 节点 `outputs` 字段保持空数组。
2. Start 公开变量由 `getStartNodeVariableOutputs` 派生。
3. flow run 的外部输入只通过 Start 公开变量进入 variable pool。
4. Start 的 resolved input 可以在 Run Context 展示。

### 12.2 Answer

输入：

```text
bindings.answer_template
```

输出变量声明：

```text
outputs.answer
```

运行展示：

```text
Trace Inputs: answer_template
Trace Outputs: answer
Variable Cache: node-answer.answer
```

禁止：

1. `VariableCache.node-answer.answer_template`
2. `VariablePicker.node-answer.answer_template`
3. `output_payload.answer_template`

### 12.3 Template Transform

输入：

```text
bindings.template
```

输出变量声明：

```text
outputs.text
```

规则：

1. `template` 只在 Trace Inputs 中出现。
2. `text` 进入完整 `output_payload`，并因 output contract 声明进入 variable pool 逻辑视图。
3. 内容相同不代表字段等价。

### 12.4 LLM

输入：

```text
bindings.prompt_messages
config.model_provider
config.llm_parameters
config.response_format
```

输出变量声明：

```text
text
usage
structured_output
```

输出对象字段：运行摘要

```text
usage
route
attempts
duration_ms
provider_code
provider_instance_id
model_id
finish_reason
event_count
```

输出对象字段：调试证据

```text
provider_metadata
tool_calls
mcp_calls
raw_response_ref
context_projection_id
attempt_ids
winner_attempt_id
provider_events_ref
```

输出对象字段：错误

```text
error_kind
message
provider_code
provider_instance_id
attempt_index
failed_after_first_token
```

规则：

1. `text` 是默认下游引用对象。
2. `structured_output` 只在 response format 要求结构化输出时出现。
3. `reasoning_content` 可以进入完整 output payload；是否可下游引用由 output contract 决定，默认不开放。
4. `usage` 进入完整 output payload，且默认声明为可引用 object 变量。
5. `route`、`attempts`、`finish_reason` 可以进入完整 output payload，默认不作为下游变量。
6. `provider_metadata`、`tool_calls`、`mcp_calls`、`__*` 内部索引可以以对象或 ref 进入完整 output payload；默认不作为下游变量。
7. LLM 失败时写入完整错误输出对象；除非错误处理策略声明异常 selector，否则不进入 variable pool。
8. `text` 不包含 reasoning；如果 provider 以 `<think>` 混合返回，payload builder 必须拆分 answer text 与 reasoning debug。
9. `message` 不是默认可引用变量；如未来开放 message object，必须显式声明为结构化 output selector。
10. live stream 中的 `reasoning_delta` 可以展示和持久化，但不能进入 Answer 节点默认输入，除非用户显式选择 debug source，当前阶段不开放。

### 12.5 HTTP Request

输入：

```text
url
method
headers
query
body
auth
```

输出变量声明：

```text
status_code
body
headers
```

指标：

```text
duration_ms
retry_count
```

规则：

1. request url、headers、query、body 只在 Trace Inputs 中展示。
2. response status、body、headers 进入 output payload。
3. retry、duration 和 network timing 进入完整 `output_payload` 的约定字段；是否可下游引用由 output contract 决定。

### 12.6 Tool / Plugin Node

输入：

```text
declared input schema resolved values
```

输出变量声明：

```text
declared output schema fields
```

规则：

1. plugin 贡献的 output schema 是唯一变量声明来源。
2. plugin raw invocation metadata 可以进入完整输出对象的 metadata/ref 字段。
3. plugin 错误进入完整输出对象的 error 字段。
4. plugin invocation metadata 是否可被下游引用，必须通过 output schema 显式声明。

### 12.7 Data Model Nodes

输入：

```text
record_id
payload
query
```

输出变量声明：

```text
data_model_list: records, total
data_model_get: record
data_model_create: record
data_model_update: record
data_model_delete: deleted_id, affected_count
```

规则：

1. query binding 中的 selector 参与依赖校验。
2. query resolved input 只在 Trace Inputs 展示。
3. records/record/deleted_id/affected_count 是默认 output contract selector。
4. Data Model 节点按节点类型固定 action，不读取 `config.action`。
5. runtime scope 使用 `actor.current_workspace_id` 或 `SYSTEM_SCOPE_ID` 对应的 Data Model grant，不允许回退到旧 `team/app` alias。
6. metadata 不健康、未发布、未授权或 scope grant 不满足时，进入完整输出对象的 error 字段；除非错误处理策略声明异常 selector，否则不写 variable pool。
7. runtime metadata 缺失或版本落后于编译时 schema hash 时必须失败，不能跳过字段、排序或 relation 校验后继续执行。
8. `data_model_delete` 成功输出必须同时包含 `deleted_id` 与 `affected_count`；如果底层存储无法返回影响行数，runtime adapter 必须补成可解释的 `affected_count`，不能省略字段。

动作矩阵：

| 节点 | 输入 | 输出 | 副作用 | 重跑规则 |
|---|---|---|---|---|
| `data_model_list` | `query` | `records`, `total` | `external_read` | 可重复 |
| `data_model_get` | `record_id` | `record` | `external_read` | 可重复 |
| `data_model_create` | `payload` | `record` | `durable_write` | 需要 idempotency key 或显式重跑确认 |
| `data_model_update` | `record_id`, `payload` | `record` | `durable_write` | 需要 idempotency key 或显式重跑确认 |
| `data_model_delete` | `record_id` | `deleted_id`, `affected_count` | `durable_write` | 需要 idempotency key 或显式重跑确认 |

调试规则：

1. 默认整流调试允许执行 Data Model read。
2. Data Model write 在 debug run 中必须有 `side_effect_policy`：`disabled`、`confirm_each_run` 或 `allow_with_idempotency`。
3. checkpoint 恢复不能隐式重复执行已经成功的 write node。
4. write node 的 audit/outbox 归 Data Model runtime action owner，不由节点 UI 或插件私自补。
5. `disabled` 时节点不执行写入，产出正式 `output_payload.error.code = "DATA_MODEL_SIDE_EFFECT_DISABLED"`；除非错误处理策略声明异常 selector，否则不写 variable pool。
6. `confirm_each_run` 时运行进入等待确认态；确认记录必须包含 actor、node_id、run_id、resolved payload hash 和过期时间。
7. `allow_with_idempotency` 必须生成同一 run 内稳定的 idempotency key：`workspace_id + application_id + draft_id + run_id + node_id + action + resolved payload hash`；checkpoint replay 命中同一 key 时读取已记录结果，不再次写入。
8. Data Model write idempotency 的目标是防同一 `run_id` 内的 checkpoint/replay 重复写；跨 debug run 的重复执行仍按新 run 处理，除非未来另行引入业务级去重键。
9. write 成功后必须持久化 side-effect receipt，至少包含 action、model_code、record_id/deleted_id、affected_count、idempotency_key、actor、scope_id、node_run_id、created_at。
10. audit/outbox 写入失败时，Data Model write 不能被当作完全成功；需要明确补偿或失败策略。

### 12.8 Human Input

输入：

```text
prompt
form schema
delivery config
```

输出变量声明：

```text
submitted values
```

规则：

1. 等待态没有输出，不写 variable pool。
2. resume 后只写用户提交值。
3. prompt 不进入变量缓存。
4. checkpoint snapshot 保存恢复所需 variable pool，不保存 prompt 作为变量。

## 13. 破坏性基线与重置策略

### 13.1 文档基线

1. `FLOW_SCHEMA_VERSION` 提升到 v2。
2. 默认 flow document 重种子。
3. v2 节点定义必须能生成 Node Runtime UI Contract。
4. v2 编译器只接受 output contract selector declarations。
5. Start 节点 outputs 仍为空，Start 公开变量继续由 config 派生。
6. LLM 默认 outputs 声明 `text` 与 `usage`；`reasoning_content` 不默认暴露为下游变量。

### 13.2 数据库基线

1. 本地开发数据库允许 reset。
2. durable debug snapshot 可清空。
3. application draft document 可按 v2 默认文档重建。
4. flow run、node run、checkpoint 的 payload 结构按新契约写入。
5. 没有 selector 修复路径；失效 selector 由校验报错暴露。
6. debug snapshot 表或等价 cache schema 必须包含 workspace、actor、draft、document hash、schema version、debug session 和 latest run scope。
7. debug artifact/offload 表或对象索引必须支持 ref、size、content type、retention 和 GC 状态。

### 13.3 Runtime 基线

1. `NodeExecutionTrace` 增加可选 `process_data`，并将 `output_payload` 固定为完整输出对象。
2. `output_payload` 写入前必须经过 output object builder。
3. LLM execution builder 将 text、usage、route、attempt、finish_reason、error/debug refs 收敛到完整 output payload；output contract 决定下游可见变量。
4. live debug run 和 non-stream debug run 使用同一套 payload builder。
5. checkpoint 的 variable snapshot 只保存 variable pool。
6. RuntimeEventStream 事件与 node run durable payload 使用同一 node/run id 关联。
7. durable debug events 保存 text/reasoning delta 的合并读模型，不作为 variable pool 来源。
8. `flow_run.output_payload` 只保存最终业务输出，不保存整流变量缓存。
9. 输出对象中的大对象字段只保存 ref。
10. RuntimeEventStream 事件必须有 sequence/cursor，支持 reconnect replay 与前端幂等消费。
11. Data Model write 节点必须写 side-effect receipt，并用 idempotency key 防止 checkpoint replay 重复写。
12. plugin contribution 编译时保存 immutable identity/hash/output schema snapshot，运行时不从当前插件动态反推旧节点契约。

### 13.4 Frontend 基线

1. node definitions 输出 Node Runtime UI Contract。
2. node picker、node factory、node card、NodeDetailPanel、NodeInspector、NodeLastRunTab 改为消费 contract。
3. `listVisibleSelectorOptions` 改为变量链接器 façade 或直接下线。
4. SelectorField 只消费 `listAvailableVariables`。
5. Variables tab 只展示 Start inputs 和 node output contract selector。
6. Trace panels 展示 Inputs、可选 Process Data、完整 Outputs。
7. node preview variable cache 只从 output payload 更新。
8. Debug Variable Cache 使用 object-level 节点条目展示。
9. Run Context / Environment / Session 独立展示，不合并进 Variable Cache。
10. Variable Picker 结构化字段展开只读 output schema。
11. 变量展示身份固定为 `node.alias/key`；output title 只做辅助文案。
12. Trace / Variable Cache 的大对象默认展示预览和 truncation 状态，完整值通过 full-load API 展开。

## 14. 验收证据

### 14.1 单元测试

必须补充或调整测试覆盖：

1. Answer debug variable cache 只包含 `answer`，不包含 `answer_template`。
2. Template Transform debug variable cache 只包含 `text`，不包含 `template`。
3. LLM variable picker 显示 `text`、`usage` 和结构化输出字段；不显示 `prompt_messages`、未声明的 `reasoning_content`、`route`、`attempts`。
4. `listAvailableVariables` 只返回上游 output contract selector。
5. output contract 中出现 `usage`、`route`、`attempts`、`error` 等 selector 时必须显式声明 valueType、selector 和节点能力；仅内部索引类 `__*` 面向用户暴露时文档校验失败。
6. durable debug snapshot 不合并非 Start 的 node input payload。
7. durable debug snapshot 不合并未被 output contract 声明的 usage/error/debug/ref 字段。
8. Data Model query selector 依赖仍能被校验。
9. LLM runtime output payload 包含完整输出对象，至少能表达 `text`、`usage`、`route`、`attempts`、`finish_reason` 和 provider/debug refs；Variable Picker 只展示 contract 声明字段。
10. live debug run 与普通 debug run 的 variable pool 写入规则一致。
11. 内置节点 contract 能驱动 node picker、node factory、node card、Inspector、Detail Panel 和 Last Run Panel。
12. plugin contribution 能映射为 Node Runtime UI Contract，且不能提供未注册 renderer 或 React panel。
13. runtime display schema 不会生成 selector option；selector 只来自 output contract。
14. snapshot key 包含 `draft_id`、`document_hash`、`flow_schema_version` 和 `snapshot_schema_version`，且 schema/hash 改变后不恢复旧 cache。
15. 未经 output contract 声明的 debug/ref 字段不会进入 Variables tab、Variable Picker 或 Debug Variable Cache。
16. streamed `text_delta` 不触发逐 token variable cache rebuild。
17. streamed `reasoning_delta` 能恢复到 reasoning 输出字段或 ref，但不进入 `output_payload.text`；是否进入 variable pool 取决于 output contract。
18. plugin contribution v2 拒绝 unknown renderer、React panel、基础设施连接和非法 output selector。
19. Data Model delete 输出 `deleted_id` 与 `affected_count`，前后端输出契约一致。
20. Data Model write 节点在 debug run 中按 `side_effect_policy` 执行，不允许 checkpoint 恢复重复写。
21. 变量块展示使用 `node.alias/key`，不会把 output title 当作 selector identity。
22. snapshot key 包含 `workspace_id`、`actor_user_id`、`debug_session_id` 和 latest run scope；跨 workspace/actor/draft/document hash 不恢复旧 cache。
23. snapshot 只聚合 succeeded 或 waiting-success checkpoint 的 output contract selector；failed/cancelled/running 未完成节点不进入 durable variable cache，除非错误处理策略声明异常 selector。
24. stream event 带 `event_id/sequence`，断线重连不会重复拼接 `text_delta` 或 `reasoning_delta`。
25. durable debug event 合并读模型保留 sequence 范围、content type、node_run_id 和 truncation/ref 信息。
26. 大对象 offload 后 Variable Cache 显示预览、`is_truncated` 和 full-load ref，不把截断内容当完整变量。
27. plugin contribution 编译时锁定 package identity/hash/output schema；插件缺失、checksum 不匹配或 stale contribution 会编译失败。
28. plugin executor 返回 unknown output key 时可以保留在完整 output payload，但不会进入 variable pool；strict output object schema 失败时记录校验错误。
29. Data Model write 的 idempotency key、side-effect receipt、audit/outbox 失败策略有单元测试覆盖。

建议命令：

```bash
pnpm --filter @1flowbase/web test -- agent-flow
cargo test -p orchestration-runtime
cargo test -p api-server
```

warning 与 coverage 产物统一落到：

```text
tmp/test-governance/
```

### 14.2 手工验收

使用默认 Start -> LLM -> Answer 流程：

1. 运行整流调试。
2. Trace 中 LLM Inputs 显示 `prompt_messages`。
3. Trace 中 LLM Outputs 显示 `text`。
4. Trace 中 LLM Outputs 显示 `usage`、route、attempt、finish reason 等完整输出字段。
5. Trace 中 Answer Inputs 显示 `answer_template`。
6. Trace 中 Answer Outputs 显示 `answer`。
7. Variables tab 只显示 `node-start.query`、`node-llm.text`、`node-answer.answer`。
8. 变量选择器中不能选择 `node-answer.answer_template`、未被 LLM output contract 声明的 `route`、`reasoning_content` 等字段；`usage` 若被默认契约声明则可选择。

使用 Start -> Template Transform -> Answer 流程：

1. Template Transform Variables 只显示 `text`。
2. Answer Variables 只显示 `answer`。
3. 两个节点即使内容相同，也以不同节点的输出变量身份展示。

使用 LLM 失败流程：

1. Trace Outputs 显示 provider error 字段或错误 ref。
2. Variable Cache 不写入 LLM error。
3. 下游节点不能选择 LLM error 字段。
4. 如果产品启用显式错误分支，异常变量必须由错误处理策略或错误处理节点声明 output selector。

使用 LLM 流式流程：

1. SSE 先收到 `flow_accepted` 或 heartbeat，再收到 `text_delta` / `reasoning_delta`。
2. `text_delta` 立即显示在回答区。
3. `reasoning_delta` 显示在独立 reasoning 输出字段或详情区，不拼入最终 answer。
4. node finished 后 Variables tab 才出现 `node-llm.text`。
5. 每 token 到达时 Variable Cache 不重建。

使用 Data Model 写入流程：

1. `data_model_create/update/delete` 在 debug run 中展示 side effect policy。
2. policy 为 `disabled` 时节点不执行写入，Trace Outputs 给出正式错误字段。
3. policy 为 `allow_with_idempotency` 时，同一 `run_id` 的 checkpoint/replay 重复执行使用同一 idempotency key；跨 debug run 会生成新 key。
4. delete 后 Variables tab 展示 `deleted_id` 与 `affected_count`。

## 15. 实施预算

建议拆成 9 个实现计划：

1. Schema v2、Node Runtime UI Contract 与变量链接器基础：1 天。
2. Node Picker、Node Factory、Node Card、Inspector、Detail Panel contract 化：1 天。
3. Debug Variable Cache、durable snapshot 与 Variables tab 重建：0.5-1 天。
4. Runtime payload builder、完整输出对象与 output contract selector 派生：1 天。
5. RuntimeEventStream、LLM streaming、reasoning/debug event 与 durable read model 对齐：1 天。
6. Plugin contribution v2、renderer allowlist、policy schema 与 manifest 校验：1 天。
7. 核心节点输出契约、Data Model side effect matrix、校验与回归测试：1 天。
8. 大对象 offload、truncation preview、full-load API 与 artifact retention/GC：1 天。
9. Data Model write idempotency、side-effect receipt、audit/outbox 与 checkpoint replay：1 天。

最小闭环不是 UI 过滤，而是：

```text
schema v2 node runtime contract + output selector declarations
  -> variable linker
  -> runtime output object builder
  -> debug variable cache only declared output selectors
```

## 16. 停止条件

满足以下条件即可认为本轮设计落地完成：

1. 变量选择器与 Variables tab 对“变量”的定义一致。
2. 任意非 Start 节点的 `input_payload` 会保留在 Trace item detail / node run 审计记录中，但不会出现在 Variable Cache。
3. 任意未被 output contract 声明的 usage/error/debug/ref 字段不会出现在 Variable Cache。
4. Answer 不再出现 `answer_template` 和 `answer` 并列。
5. Template Transform 不再出现 `template` 和 `text` 并列。
6. LLM output payload 是完整输出对象，Variables tab 只暴露 contract 声明字段。
7. 新增内置节点只需声明 Node Runtime UI Contract，即可接入节点选择器、节点工厂、卡片、面板、变量链接器和运行态展示。
8. 新增插件节点贡献只需声明宿主支持的 contract schema，即可接入 picker、panel、runtime 和变量链接器。
9. Trace 仍能查看完整 inputs、可选 process_data 和完整 outputs，调试能力不倒退。
10. snapshot 恢复受 `document_hash`、schema version 和 latest run scope 约束，不跨草稿或旧 schema 混合恢复。
11. RuntimeEventStream 只承担实时事件，不成为变量缓存或持久化真值。
12. reasoning 可流式展示和恢复，但不会自动成为下游变量。
13. plugin contribution 不能绕过宿主 renderer、policy、output object builder 和基础设施边界。
14. Data Model 写入节点的副作用、重跑和 checkpoint 恢复语义明确。
15. snapshot 恢复不跨 workspace、actor、draft、debug session 或 document hash。
16. RuntimeEventStream 支持 cursor replay，断线重连不会重复拼接 delta。
17. 大对象以 preview/ref/full-load 方式展示，截断值不会伪装成完整变量。
18. 插件节点按编译时 identity/hash/output schema 执行，stale contribution 不会静默降级。
19. Data Model write 有 idempotency key、side-effect receipt 和 audit/outbox 失败处理。
