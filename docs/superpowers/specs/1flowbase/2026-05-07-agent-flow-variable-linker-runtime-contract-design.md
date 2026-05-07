# Agent Flow 变量链接器与运行态契约设计

日期：2026-05-07

状态：已按开发期破坏性基线重写，待用户审阅

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

1. Variable Picker、Variables tab、Debug Variable Cache 对“变量”的定义完全一致：变量只来自公开输出契约。
2. `bindings`、`input_payload`、`output_payload`、`metrics_payload`、`error_payload`、`debug_payload` 语义互斥。
3. `output_payload` 只能包含可被下游引用的业务输出，不承载 usage、route、attempt、provider metadata、raw response、错误详情或调试索引。
4. Answer 节点只暴露 `answer`；`answer_template` 只作为 resolved input 出现在 Trace Inputs。
5. LLM 节点只把业务结果放进 output；用量、路由、尝试、finish reason 和 provider 证据全部进入 metrics/debug/error。
6. 新增节点只需声明公开输出契约，即可接入变量链接器、变量池和调试缓存。
7. 本设计按开发期破坏性基线推进；schema、默认文档、durable snapshot 和数据库可以重建。

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
2. `llm`: `prompt_messages` 是运行输入；`text` 是公开输出；`usage`、`route`、`attempts`、`finish_reason` 是运行指标。
3. `http_request`: `url`、`headers`、`query`、`body` 是运行输入；`status_code`、`body`、`headers` 才是业务输出。
4. `tool` / `plugin_node`: 参数是运行输入；插件声明的 output schema 才是公开输出。
5. `data_model_*`: `query`、`payload`、`record_id` 是运行输入；`records`、`record`、`affected_count` 是公开输出。
6. `human_input`: `prompt` 与 form schema 是运行输入；resume payload 中的用户提交值才是公开输出。

因此最终方案不能是“在某个面板不显示 `answer_template`”，而必须重建运行态分层。

### 2.3 当前代码暴露的硬问题

当前代码中存在四类边界破损：

1. 前端 selector option 直接读取 `getNodeVariableOutputs(node)`，没有统一变量链接器 source、scope 和 filter 语义。
2. 前端 debug cache 从 trace items 和 run detail 同时合并 node input 与 node output。
3. 后端 durable debug variable snapshot 把 `flow_run.input_payload` 原样 merge 到 variable cache。
4. LLM runtime 的 `output_payload` 同时包含 `text`、`message`、`tool_calls`、`finish_reason`、`route`、`usage`、`error`、`__attempt_ids` 等不同层级信息。

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
5. 运行 trace 中可以展示 input、output、metrics、debug，但变量池只存 output。

不照搬的部分：

1. 不照搬 Dify 的全量 node meta 和 panel 实现。
2. 不引入 Dify 的完整特殊变量体系。
3. 不把 Dify 的 `answer` 配置字段命名直接迁入 1flowbase；1flowbase 继续用 `answer_template` 表示 Answer 输入模板。
4. 不把 metadata/error/debug 当成 output 再靠可见性字段筛掉。

## 4. 信息架构诊断

### 4.1 问题清单

| # | 问题类型 | 位置 | 描述 | 严重度 |
|---|---|---|---|---|
| 1 | 分类不互斥 | Debug Variable Cache | 同一节点输入与输出混合展示，用户无法判断哪个可被下游引用。 | 高 |
| 2 | 层级错位 | Variables tab | 运行输入属于 Trace 深度，却出现在变量概览层。 | 高 |
| 3 | 分类不穷尽 | Runtime payload | 公开变量、指标、错误、调试事件缺少互斥容器。 | 高 |
| 4 | 入口语义混乱 | Variable Picker / Debug Cache | 变量选择器基于 outputs，调试缓存基于 input+output，两个入口对“变量”的定义不一致。 | 高 |
| 5 | 状态真值分裂 | Frontend cache / backend variable pool | 前端缓存和后端 variable pool 没有共享同一套公开输出规则。 | 高 |

### 4.2 修正后的信息深度

| 信息 | 深度 | 容器 | 规则 |
|---|---|---|---|
| 可被下游引用的变量 | L0/L1 | Variable Cache / Variable Picker | 只来自公开输出契约。 |
| 当前节点解析后的输入 | L1 | Trace item detail | 只用于调试，不进入变量缓存。 |
| 节点公开输出 | L1 | Trace Outputs / Variable Cache | 与 variable pool 同源。 |
| token、耗时、route、attempt、finish reason | L1 | Trace Metrics | 指标，不作为普通变量。 |
| provider event、raw response、artifact ref | L2 | Trace Debug | 调试证据，不进入变量选择器。 |
| 错误信息 | L1 | Trace Error | 错误态信息；只有显式错误处理策略能产出异常变量。 |

## 5. 范围

### 5.1 本阶段范围

1. 固定 Agent Flow 节点 `bindings / outputs / runtime trace / variable cache` 的分层 contract。
2. 重建 Debug Variable Cache 的语义：只展示公开输出变量。
3. 重建 durable debug variable snapshot 的语义：只聚合 Start 公开输入变量和节点公开输出变量。
4. 建立前端变量链接器接口，替代散落的 selector option 生成逻辑。
5. 将 output contract 收敛为 public-only；metadata、debug、error 不再作为 output 类型存在。
6. 明确 Answer、Template Transform、LLM、HTTP、Tool、Plugin、Data Model、Human Input 的输入输出归属。
7. 将 LLM usage、route、attempt、finish reason、provider metadata 从 output payload 移出。
8. 建立 schema 重置和默认文档重种子策略。

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
4. `output_payload` 只能保存公开业务输出，且是 variable pool 的唯一节点输出来源。
5. `metrics_payload` 承载 usage、duration、route、attempt、finish reason、preview mode。
6. `error_payload` 承载失败信息；异常变量必须通过显式错误处理策略产出。
7. `debug_payload` / provider events 承载 raw response、artifact ref、provider event 和内部排障证据。
8. Variable Picker、Variables tab、Debug Variable Cache 必须共享同一套公开输出定义。
9. 开发期以长期契约正确性优先，不为既有草稿或快照牺牲边界。

## 7. 目标概念模型

```text
Flow Node Definition
  config: static configuration
  bindings: input binding declarations
  outputs: public output contract only

Runtime Node Run
  input_payload: resolved inputs, trace only
  output_payload: public outputs, variable pool source
  metrics_payload: usage / duration / route / attempts / finish reason
  error_payload: error information
  debug_payload/provider_events: advanced evidence

Variable Linker
  sources: start inputs / node public outputs / explicit special sources
  visible nodes: graph topology + branch/container scope
  visible variables: public outputs from visible sources
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

`FlowNodeOutputDocument` 只表达公开业务输出。

```ts
export interface FlowNodeOutputDocument {
  key: string;
  title: string;
  valueType: string;
  description?: string;
}
```

规则：

1. 出现在 `outputs` 中的字段必须可进入 Variable Picker。
2. 出现在 `outputs` 中的字段必须可进入 Variables tab。
3. 出现在 `outputs` 中的字段必须可进入 runtime variable pool。
4. 出现在 `outputs` 中的字段必须可被下游 selector 引用。
5. metadata、debug、error、usage、route、attempt、finish reason、provider raw response 不允许出现在 `outputs`。
6. 需要面向 Trace 展示的非业务信息必须进入 metrics/error/debug 容器。

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
6. selector 只能引用变量链接器返回的公开变量。

### 8.4 非输出声明

节点可以声明 metrics/error/debug 的展示 schema，但这些 schema 不参与变量链接器。

```ts
export interface FlowNodeRuntimeSchemaDocument {
  metrics?: Array<{ key: string; title: string; valueType: string }>;
  errors?: Array<{ key: string; title: string; valueType: string }>;
  debug?: Array<{ key: string; title: string; valueType: string }>;
}
```

规则：

1. runtime schema 只服务 Trace、observability、debug console。
2. runtime schema 不生成 selector。
3. runtime schema 不写入 variable pool。
4. runtime schema 不出现在 Variables tab。

## 9. 变量链接器契约

### 9.1 前端能力边界

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

### 9.2 可见节点规则

1. 普通节点只能看到当前节点的上游节点。
2. Start 派生输出按 Start source 进入变量链接器。
3. `if_else` 只表达控制流；没有显式 output contract 时不暴露变量。
4. 同一 `containerId` 内按图拓扑计算可见性。
5. 容器内部节点可以看到父容器入口之前的上游公开输出。
6. loop/iteration 内部 item 变量必须作为明确 source kind 接入，不写入普通节点 outputs。
7. env/session/global 类变量必须作为 `system` 或专门 source kind 接入，不伪装成 Start input 或 node output。

### 9.3 可见变量规则

1. 只读取 Start 派生公开输入和节点 `outputs`。
2. selector path 基线保持 `[nodeId, key]`。
3. 结构化输出的深层 path 必须来自 output schema，不来自运行样本。
4. 变量链接器按 source kind 和 valueType 过滤。
5. 不提供 metadata/debug/error 的变量开关。
6. 如果 selector 指向不存在的公开变量，文档校验直接失败。

### 9.4 UI 规则

1. 变量选择器文案使用“选择上游输出”，不使用“选择缓存字段”。
2. 变量块展示 `node alias / output title`。
3. Variables tab 展示节点级 output object，不递归平铺对象内部字段。
4. Trace Inputs 展示 resolved inputs。
5. Trace Metrics 展示 usage、duration、route、attempt、finish reason。
6. Trace Debug 展示 provider events、raw response ref、artifact ref。
7. 失效 selector 在表单中显示正式错误状态，不显示“可继续运行”的提示。

## 10. Runtime 契约

### 10.1 NodeExecutionTrace

运行时 trace 固定提供五类 payload：

```text
input_payload
output_payload
metrics_payload
error_payload
debug_payload
```

规则：

1. `input_payload` 保存 resolved inputs。
2. `output_payload` 保存公开业务输出。
3. `metrics_payload` 保存 usage、duration、route、attempt、finish_reason、preview_mode。
4. `error_payload` 保存错误。
5. `debug_payload` 保存 raw response ref、provider event ref、artifact ref、internal evidence。
6. provider stream events 不进入 output payload。

### 10.2 Variable Pool

运行时 variable pool 只写入公开输出。

```text
variable_pool[node_id] = output_payload
```

禁止：

1. 把 `input_payload` 写入 variable pool。
2. 把 `metrics_payload` 写入 variable pool。
3. 把 `error_payload` 写入 variable pool，除非显式错误处理策略产出异常变量。
4. 把 `debug_payload` 写入 variable pool。
5. 把 provider raw event 写入 variable pool。

### 10.3 Debug Snapshot

持久化 debug variable snapshot 只聚合：

1. Start 节点公开输入变量。
2. 每个 node run 的 `output_payload`。

规则：

1. 不读取 `node_run.input_payload` 构造 variable cache。
2. 不读取 `flow_run.input_payload` 构造 variable cache，Start 节点除外。
3. 不读取 `metrics_payload`、`error_payload`、`debug_payload` 构造 variable cache。
4. snapshot 是变量缓存的恢复加速层，不是运行真值来源。
5. Run Context 单独展示本次运行起始输入。

## 11. 节点级契约

### 11.1 Start

来源：

1. `config.input_fields`
2. 内置 `query`
3. 内置 `files`

公开变量：

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

### 11.2 Answer

输入：

```text
bindings.answer_template
```

公开输出：

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

### 11.3 Template Transform

输入：

```text
bindings.template
```

公开输出：

```text
outputs.text
```

规则：

1. `template` 只在 Trace Inputs 中出现。
2. `text` 进入 output payload 和 variable pool。
3. 内容相同不代表字段等价。

### 11.4 LLM

输入：

```text
bindings.prompt_messages
config.model_provider
config.llm_parameters
config.response_format
```

公开输出：

```text
text
structured_output
```

指标：

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

调试证据：

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

错误：

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
3. `reasoning_content` 不进入普通公开输出；它属于 debug payload。
4. `usage` 不进入 outputs，也不进入 output payload。
5. `route`、`attempts`、`finish_reason` 不进入 outputs，也不进入 output payload。
6. `provider_metadata`、`tool_calls`、`mcp_calls`、`__*` 内部索引不进入 output payload。
7. LLM 失败时不向 variable pool 写普通 output；错误由 `error_payload` 承载。

### 11.5 HTTP Request

输入：

```text
url
method
headers
query
body
auth
```

公开输出：

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
3. retry、duration 和 network timing 进入 metrics。

### 11.6 Tool / Plugin Node

输入：

```text
declared input schema resolved values
```

公开输出：

```text
declared output schema fields
```

规则：

1. plugin 贡献的 output schema 是唯一公开变量来源。
2. plugin raw invocation metadata 进入 metrics/debug。
3. plugin 错误进入 error payload。
4. plugin 不能把 invocation metadata 伪装成 output fields。

### 11.7 Data Model Nodes

输入：

```text
record_id
payload
query
```

公开输出：

```text
data_model_list: records, total
data_model_get: record
data_model_create: record
data_model_update: record
data_model_delete: affected_count
```

规则：

1. query binding 中的 selector 参与依赖校验。
2. query resolved input 只在 Trace Inputs 展示。
3. records/record/affected_count 是 variable pool 来源。

### 11.8 Human Input

输入：

```text
prompt
form schema
delivery config
```

公开输出：

```text
submitted values
```

规则：

1. 等待态没有输出，不写 variable pool。
2. resume 后只写用户提交值。
3. prompt 不进入变量缓存。
4. checkpoint snapshot 保存恢复所需 variable pool，不保存 prompt 作为变量。

## 12. 破坏性基线与重置策略

### 12.1 文档基线

1. `FLOW_SCHEMA_VERSION` 提升到 v2。
2. 默认 flow document 重种子。
3. v2 编译器只接受 public-only outputs。
4. Start 节点 outputs 仍为空，Start 公开变量继续由 config 派生。
5. LLM 默认 outputs 移除 `usage` 和 `reasoning_content`。

### 12.2 数据库基线

1. 本地开发数据库允许 reset。
2. durable debug snapshot 可清空。
3. application draft document 可按 v2 默认文档重建。
4. flow run、node run、checkpoint 的 payload 结构按新契约写入。
5. 没有 selector 修复路径；失效 selector 由校验报错暴露。

### 12.3 Runtime 基线

1. `NodeExecutionTrace` 增加 `debug_payload`。
2. `output_payload` 写入前必须经过 public output filter。
3. LLM execution builder 不再把 metrics/debug/error 放入 output payload。
4. live debug run 和 non-stream debug run 使用同一套 payload builder。
5. checkpoint 的 variable snapshot 只保存 variable pool。

### 12.4 Frontend 基线

1. `listVisibleSelectorOptions` 改为变量链接器 façade 或直接下线。
2. SelectorField 只消费 `listAvailableVariables`。
3. Variables tab 只展示 Start inputs 和 node public outputs。
4. Trace panels 分别展示 Inputs、Outputs、Metrics、Error、Debug。
5. node preview variable cache 只从 output payload 更新。

## 13. 验收证据

### 13.1 单元测试

必须补充或调整测试覆盖：

1. Answer debug variable cache 只包含 `answer`，不包含 `answer_template`。
2. Template Transform debug variable cache 只包含 `text`，不包含 `template`。
3. LLM variable picker 只显示 `text` 和结构化输出字段，不显示 `prompt_messages`、`usage`、`reasoning_content`、`route`、`attempts`。
4. `listAvailableVariables` 只返回上游公开输出。
5. output contract 中出现 `usage`、`route`、`attempts`、`error`、`__attempt_ids` 时文档校验失败。
6. durable debug snapshot 不合并非 Start 的 node input payload。
7. durable debug snapshot 不合并 metrics/error/debug payload。
8. Data Model query selector 依赖仍能被校验。
9. LLM runtime output payload 不包含 `usage`、`route`、`attempts`、`finish_reason`、`provider_metadata`、`__*`。
10. live debug run 与普通 debug run 的 variable pool 写入规则一致。

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

### 13.2 手工验收

使用默认 Start -> LLM -> Answer 流程：

1. 运行整流调试。
2. Trace 中 LLM Inputs 显示 `prompt_messages`。
3. Trace 中 LLM Outputs 显示 `text`。
4. Trace 中 LLM Metrics 显示 usage、route、attempt、finish reason。
5. Trace 中 Answer Inputs 显示 `answer_template`。
6. Trace 中 Answer Outputs 显示 `answer`。
7. Variables tab 只显示 `node-start.query`、`node-llm.text`、`node-answer.answer`。
8. 变量选择器中不能选择 `node-answer.answer_template`、`node-llm.usage`、`node-llm.route`、`node-llm.reasoning_content`。

使用 Start -> Template Transform -> Answer 流程：

1. Template Transform Variables 只显示 `text`。
2. Answer Variables 只显示 `answer`。
3. 两个节点即使内容相同，也以不同节点的公开输出身份展示。

使用 LLM 失败流程：

1. Trace Error 显示 provider error。
2. Variable Cache 不写入 LLM error。
3. 下游节点不能选择 LLM error 字段。
4. 如果产品启用显式错误分支，异常变量必须由错误处理节点产出公开 output。

## 14. 实施预算

建议拆成 4 个实现计划：

1. Schema v2 与变量链接器基础：0.5-1 天。
2. Debug Variable Cache、durable snapshot 与 Variables tab 重建：0.5-1 天。
3. Runtime payload builder 与 LLM output/metrics/debug/error 分离：1 天。
4. 核心节点输出契约、校验与回归测试：1 天。

最小闭环不是 UI 过滤，而是：

```text
schema v2 public-only outputs
  -> variable linker
  -> runtime output filter
  -> debug variable cache only public outputs
```

## 15. 停止条件

满足以下条件即可认为本轮设计落地完成：

1. 变量选择器与 Variables tab 对“变量”的定义一致。
2. 任意非 Start 节点的 `input_payload` 不会出现在 Variable Cache。
3. 任意节点的 `metrics_payload`、`error_payload`、`debug_payload` 不会出现在 Variable Cache。
4. Answer 不再出现 `answer_template` 和 `answer` 并列。
5. Template Transform 不再出现 `template` 和 `text` 并列。
6. LLM output payload 不再包含 usage、route、attempt、finish reason、provider metadata、内部索引。
7. 新增节点只需声明 public outputs，即可接入变量链接器。
8. Trace 仍能查看完整 inputs、outputs、metrics、error 和 debug，调试能力不倒退。
