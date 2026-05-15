# Code 节点与隔离架构讨论稿

日期：2026-05-15
状态：讨论稿
范围：Agent Flow 内置 Code 节点、后端执行模型、节点级隔离、执行器扩展点

## 背景

当前 Agent Flow 已经存在 Code 节点的前端与运行契约雏形：

- 前端节点定义：`web/app/src/features/agent-flow/lib/node-definitions/nodes/code.ts`
- 运行 UI 契约：`web/app/src/features/agent-flow/lib/node-definitions/contracts.ts`
- 输出契约编辑：`web/app/src/features/agent-flow/schema/node-schema-fragments.ts`

但后端 debug runtime 当前没有执行 `code` 节点。`orchestration-runtime` 的执行引擎遇到未支持节点类型会返回 `unsupported debug node type`，所以 Code 节点不是只补一个前端入口，而是要补齐编译、执行、隔离、调试、输出契约和策略合并链路。

本稿目标是把前期讨论收敛成可评审的架构方向。

## 核心判断

Code 节点应作为内置节点存在，但节点内的用户代码必须按不可信载荷处理。

需要区分两类信任：

| 维度 | 说明 | Code 节点结论 |
|---|---|---|
| 节点实现是否可信 | 节点定义、编译逻辑、运行协议是否由平台维护 | 可信，属于内置节点 |
| 节点运行内容是否可信 | 节点实例配置里的用户代码是否可信 | 不可信，必须受隔离策略约束 |

因此，隔离边界不应该绑定在“插件是否可信”上，而应该绑定在“节点实例运行风险”上。

## 设计原则

1. 隔离策略是节点实例级决策。
2. 应用级提供默认隔离策略。
3. Workspace / Tenant / System 级提供硬上限和禁用项。
4. 执行器只是满足某种隔离能力的运行后端，不拥有最终安全决策权。
5. 编排核心只依赖抽象执行能力，不直接耦合 QuickJS、Deno、容器或远程 worker。
6. 前端不能执行用户代码，所有 debug / run 结果必须来自后端 runtime。
7. 第一版优先做“纯数据转换节点”，不要开放网络、文件、数据库、npm 包和环境变量。

## 推荐定位

第一版 Code 节点建议定位为：

> 安全、可调试、可输出结构化数据的 JavaScript transform 节点。

用户写代码处理输入变量并返回结构化对象。节点输出必须匹配输出契约，后续节点只能引用契约暴露的 public output。

建议的用户代码形态：

```js
function main(inputs) {
  return {
    result: inputs.text?.trim()
  };
}
```

第一版不建议支持：

- 文件系统访问
- 数据库访问
- 环境变量访问
- 默认网络请求
- npm / node_modules
- 动态 import
- 跨运行共享全局状态

## 分层架构

推荐的策略合并和执行链路：

```text
System / Tenant Policy
  -> Workspace Policy
    -> Application Policy
      -> Node Isolation Profile
        -> Executor Selection
          -> Code Executor
```

推荐的运行链路：

```text
FlowAuthoringDocument
  -> FlowCompiler
  -> CompiledPlan / CompiledNode
  -> Runtime Preparation 合并策略
  -> execute_code_node
  -> CodeInvoker
  -> Code Executor
  -> RawNodeExecutionResult
  -> PublicOutputContract 投影
  -> variable_pool / trace / debug artifacts
```

### 责任划分

| 层 | 责任 |
|---|---|
| 前端节点 UI | 编辑输入绑定、代码、输出契约、隔离配置入口和调试结果展示 |
| `orchestration-runtime` | 编译节点、解析绑定、调用 CodeInvoker、投影输出、生成 trace |
| `control-plane` / runtime preparation | 合并 system / tenant / workspace / application / node 策略 |
| CodeInvoker | 编排核心到执行器的抽象端口 |
| Code Executor | 具体执行 JS / Wasm / 远程沙箱，并返回结构化结果 |
| Runner / Worker | 第二阶段承载进程级或容器级隔离 |

## 节点级隔离模型

节点实例应有显式隔离配置，应用级可提供默认值，系统级可提升最低隔离等级。

示例：

```json
{
  "isolation": {
    "mode": "process",
    "timeout_ms": 3000,
    "memory_mb": 64,
    "network": "deny",
    "secrets": "none",
    "filesystem": "deny"
  }
}
```

应用级默认策略示例：

```json
{
  "default_code_isolation": "process",
  "max_timeout_ms": 5000,
  "max_memory_mb": 128,
  "allow_network": false
}
```

系统 / 租户级策略示例：

```json
{
  "min_code_isolation": "process",
  "allow_in_process_user_code": false,
  "allow_code_network": false,
  "max_code_timeout_ms": 5000,
  "max_code_memory_mb": 128
}
```

### 隔离级别

| 隔离级别 | 适用场景 | 说明 |
|---|---|---|
| `in_process` | 可信内置轻逻辑 | 不适合普通用户代码 |
| `vm_limited` | 私有部署、低风险脚本 | QuickJS in-process，依赖 VM timeout / memory / interrupt |
| `process` | 多租户默认高风险节点 | 独立 runner 进程，配合 OS 资源限制 |
| `container` | 强隔离 SaaS / 企业环境 | cgroup / seccomp / network policy / filesystem policy |
| `remote` | 专用隔离集群 | 由远程执行池承载，平台只保留协议 |

Code 节点在私有部署中可以允许 `vm_limited`；在 SaaS 多租户环境中建议系统策略强制提升为 `process` 或 `container`。

## 执行器扩展点

执行器不决定节点是否安全，只声明自己支持的能力。

示例：

```text
quickjs-local-executor
  languages: [javascript]
  isolation_modes: [vm_limited]

quickjs-process-executor
  languages: [javascript]
  isolation_modes: [process]

container-code-executor
  languages: [javascript, python]
  isolation_modes: [container]
```

运行时根据已合并的 `NodeIsolationProfile` 选择满足条件的 executor。找不到 executor 时，应在编译验证或运行准备阶段失败，而不是运行中降级。

### 与插件体系的关系

当前项目已有插件分类：

- `CapabilityPlugin`：workspace 显式选择的能力贡献，例如 canvas node、tool、trigger。
- `RuntimeExtension`：实现已注册 runtime slot，例如 `model_provider`、`data_source`、`file_processor`。
- `HostExtension`：system/root 级可信宿主扩展和基础设施能力。

Code 节点本体建议保持内置节点，不建议把 Code 节点完全交给普通 `CapabilityPlugin`。原因是 Code 节点是通用执行基础设施，涉及多租户隔离、资源配额、secret 暴露、网络策略和审计，风险高于普通能力节点。

第二阶段可以新增 runtime slot，例如：

```text
RuntimeSlotCode::CodeExecutor => "code_executor"
```

不同 RuntimeExtension 可以实现不同 code executor，但必须由宿主策略选择和约束。

```text
内置 Code 节点
  -> CodeInvoker
  -> code_executor runtime slot
  -> plugin-runner / code-runner worker
  -> QuickJS / Deno / Wasm / Container
```

## Rust 执行库选型

不建议自研 JS 引擎。平台应自研执行协议和隔离策略，JS 执行复用成熟 runtime。

| 方案 | 推荐度 | 优点 | 风险 |
|---|---:|---|---|
| `rquickjs` | 高 | 轻量，嵌入简单，QuickJS 语义够用，支持 memory limit / stack limit / interrupt handler | 不是 V8；同进程执行不能作为强安全边界 |
| `deno_core` / `rusty_v8` | 中高 | V8 兼容性强，适合未来 ops / event loop / 模块系统 | 重，构建复杂；`deno_core` 不是完整 Deno |
| `Boa` | 中低 | 纯 Rust，嵌入式 JS 引擎 | 仍偏 experimental，生产兼容性风险较高 |
| `Wasmtime` | 中 | sandbox 能力强，适合多语言扩展 | 用户写 JS 不自然，需要 JS -> Wasm 编译链 |
| 外部 Node / Deno 进程 | 中 | 兼容性最好 | 运维、冷启动、协议和资源隔离复杂 |

推荐路线：

1. 第一阶段使用 `rquickjs` 做本地 executor。
2. 通过 `CodeInvoker` trait 隔离具体实现。
3. 第二阶段把 executor 移到独立 runner / runtime slot。
4. 如果未来确实需要高 JS 兼容性、模块系统或 npm，再评估 `deno_core` / Deno worker。

## 后端接口与类型建议

### Compiled metadata

Code 节点编译后应保留运行所需元数据，例如：

```rust
pub struct CompiledCodeRuntime {
    pub language: String,
    pub source: String,
    pub entrypoint: String,
    pub isolation_profile: NodeIsolationProfile,
}
```

`CompiledNode` 可新增：

```rust
pub code_runtime: Option<CompiledCodeRuntime>
```

或把高风险运行 metadata 放在统一 runtime metadata 中。关键是不要让执行引擎直接从松散 `config` 中临时猜字段。

### CodeInvoker

推荐新增抽象端口：

```rust
#[async_trait]
pub trait CodeInvoker: Send + Sync {
    async fn invoke_code_node(
        &self,
        runtime: &CompiledCodeRuntime,
        input_payload: serde_json::Value,
        output_contract: PublicOutputContract,
    ) -> anyhow::Result<CodeInvocationOutput>;
}
```

返回结构可对齐现有 payload builder：

```rust
pub struct CodeInvocationOutput {
    pub output_payload: serde_json::Value,
    pub error_payload: Option<serde_json::Value>,
    pub metrics_payload: serde_json::Value,
    pub debug_payload: serde_json::Value,
}
```

执行引擎接入后应复用：

- `resolve_node_inputs`
- `RawNodeExecutionResult`
- `PublicOutputContract::build_node_payloads`
- `project_node_variable_payload`
- `NodeExecutionTrace`

## 安全策略

### 默认禁止

第一版默认禁止：

- network
- filesystem
- env
- secrets
- subprocess
- dynamic imports
- native modules
- cross-run global cache

### 必须限制

每次执行必须有：

- timeout
- memory limit
- stack limit
- output size limit
- log size limit
- input payload size limit
- deterministic error envelope

### 多租户上下文

每次执行应携带：

```text
tenant_id
workspace_id
application_id
run_id
node_id
executor_id
isolation_profile
permission_policy
network_policy
secret_policy
```

runner 不应从全局环境隐式读取这些信息，必须由宿主显式传入。

## 前端体验建议

第一版 UI 应覆盖：

- 节点选择器可添加 Code。
- Inspector 支持输入变量绑定。
- 代码编辑器支持 JavaScript 语法高亮。
- `language` 使用选择器，不使用自由文本。
- 输出契约继续使用现有 `output_contract_definition`。
- 展示最近一次运行输入、输出、错误和日志。
- 隔离配置默认收起，只在高级设置中展示。

前端不应：

- 使用 `eval` 或浏览器 JS 执行用户代码。
- 展示原始异常对象或后端内部 stack。
- 在没有真实后端字段时伪造运行状态。

## 阶段路线

### Phase 1：内建纯转换节点

目标：跑通 Code 节点闭环。

范围：

- 内置 Code 节点配置字段补齐。
- 后端编译 Code runtime metadata。
- `orchestration-runtime` 支持 `code` 执行分支。
- 本地 `rquickjs` executor。
- 输入绑定、输出契约投影、trace、错误 envelope。
- 禁止网络、文件、环境变量和模块加载。

验收证据：

- Code 节点成功返回结构化输出。
- 后续节点能引用 Code 输出。
- 死循环被 timeout 中断。
- 内存 / 栈限制生效或有明确测试替代证据。
- 语法错误、运行时错误、输出契约错误可区分。

### Phase 2：节点级隔离策略与 runner

目标：把隔离从 executor 实现细节提升为节点级运行策略。

范围：

- 定义 `NodeIsolationProfile`。
- 合并 system / tenant / workspace / application / node 策略。
- CodeInvoker 根据 profile 选择 executor。
- 新增 `code-runner` 或复用 `plugin-runner` 承载进程级执行。
- 支持 `process` 隔离模式。
- runner 返回统一 `CodeInvocationOutput`。

验收证据：

- 应用级默认隔离策略可生效。
- 节点级配置可在硬上限内调整。
- 系统策略可强制禁止 `vm_limited`。
- 找不到满足隔离级别的 executor 时运行前失败。
- runner 崩溃不会带崩 api-server。

### Phase 3：runtime slot 与多执行器

目标：让执行器实现插件化，但策略仍由宿主管控。

范围：

- 新增 `code_executor` runtime slot。
- RuntimeExtension 声明支持语言和隔离模式。
- 宿主维护 executor inventory。
- 支持按 workspace / application 选择 executor。
- 增加 executor health / readiness / version fingerprint。

验收证据：

- 不同 executor 可被注册和选择。
- executor 不能越过宿主策略。
- executor 能力不足时有明确编译或准备阶段错误。
- 运行 trace 记录 executor id、isolation mode、limits。

### Phase 4：可选高级能力

仅在基础隔离稳定后讨论：

- 网络 allowlist
- secret 显式授权注入
- npm / package bundle
- TypeScript
- Python / Wasm 多语言
- 企业级远程执行池

## 待决问题

1. 第一版是否允许 `async main(inputs)`？
2. Code 节点默认输出是固定 `result`，还是强制用户定义输出契约？
3. 私有部署是否允许 `vm_limited`，SaaS 是否强制 `process`？
4. 隔离配置是节点字段、应用设置，还是二者都有？
5. runner 是复用 `plugin-runner`，还是新增专用 `code-runner`？
6. 日志如何限流、脱敏和落 trace？
7. 输出契约不匹配时，是节点失败，还是只投影可匹配字段并给 warning？
8. 是否需要 deterministic mode，例如禁用 Date / Math.random？
9. 网络能力未来是否按 host capability 注入，而不是直接开放 `fetch`？
10. 执行器版本变化是否要影响应用发布快照和可复现运行？

## 当前建议结论

1. Code 节点本体作为内置节点，不作为普通 CapabilityPlugin。
2. 用户代码按不可信载荷处理，隔离策略绑定到节点实例。
3. 应用级提供默认隔离，租户 / 系统级提供硬上限。
4. 第一版使用 `rquickjs` 实现纯 JS transform，禁止副作用。
5. 后端通过 `CodeInvoker` 抽象执行器，避免把 QuickJS 细节写死进执行引擎。
6. 第二阶段引入 `NodeIsolationProfile` 和独立 runner。
7. 第三阶段再考虑 `code_executor` runtime slot，让执行器实现可插拔。

## 参考资料

- rquickjs docs: https://docs.rs/rquickjs/latest/rquickjs/
- Deno open source crates: https://deno.com/blog/open-source
- Boa: https://github.com/boa-dev/boa
- Wasmtime security: https://docs.wasmtime.dev/security.html
