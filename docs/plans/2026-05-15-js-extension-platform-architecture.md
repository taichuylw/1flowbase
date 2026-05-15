# JS 扩展平台架构讨论稿

日期：2026-05-15
状态：讨论稿
关联文档：`docs/plans/2026-05-15-code-node-isolation-architecture.md`

## 目标

设计一套可逐步落地的 JS 扩展平台，用于支撑：

1. 后端 Code 节点执行 JavaScript 数据转换。
2. JS 第三方依赖以插件形式注册给 Code 节点使用。
3. 前端 JS 区块以插件形式加载到 1flowbase 页面或工作区。
4. 未来按节点级 / 应用级策略扩展隔离、依赖、执行器和区块通信能力。

核心结论：

- 初期只内置一个默认 JS 运行面，不引入多个运行环境选择器。
- 生产运行时不依赖 `node` / `pnpm` / `npm install`。
- 第三方包在插件开发或官方 CI 打包阶段构建成 artifact。
- 1flowbase 安装插件时只做校验、复制、登记和发布快照绑定。
- 后端 Code Runtime 与前端 Block Runtime 共享依赖插件模型，但不共享执行器。

## 总体模型

```text
JS Extension Platform
  |
  +-- JS Dependency Pack
  |     registers import aliases and built artifacts
  |
  +-- Backend Code Runtime Surface
  |     runs Code node JavaScript under node isolation profile
  |
  +-- Frontend Block Runtime Surface
        loads sandboxed browser UI blocks through BlockContext
```

两个运行面共享：

- 插件安装 / 启用 / 卸载流程
- 依赖 alias
- artifact integrity
- 应用发布快照
- 权限声明
- 1flowbase SDK / context contract

两个运行面分离：

| 维度 | 后端 Code Runtime | 前端 Block Runtime |
|---|---|---|
| 运行位置 | server / code-runner | browser / iframe sandbox |
| 用途 | 数据转换、流程执行 | 可视化、交互、页面区块 |
| 输入 | 上游节点变量 | props、页面上下文、数据源 |
| 输出 | 结构化 payload | UI event、action、state patch |
| 隔离 | VM / process / container | iframe sandbox / CSP / host mediator |
| 通信 | variable pool / trace | postMessage / BlockContext event bus |

## 插件类型

### JS Dependency Pack

用途：把第三方 JS 包注册成可 import 依赖。

示例：

- `js-zod-pack` 提供 `zod`
- `js-date-fns-pack` 提供 `date-fns`
- `js-lodash-pack` 提供 `lodash-es`

它不贡献新节点，也不贡献 UI 区块，只注册依赖 alias 和 artifact。

### Frontend Block Plugin

用途：注册一个可加载的前端 JS 区块。

示例：

- `sales-chart-block` 注册 `sales_chart`
- `kpi-table-block` 注册 `kpi_table`

区块通过受控 `BlockContext` 与 1flowbase 通信，不能直接访问宿主应用内部对象。

### Code Executor Runtime

用途：未来提供可插拔执行器。

初期不做多执行器，只保留抽象边界。后续可演进为：

- `quickjs-local-executor`
- `quickjs-process-executor`
- `bun-process-executor`
- `container-code-executor`

## 目录结构

插件源目录：

```text
api/plugins/
  capability-plugins/
    js-zod-pack/
      manifest.yaml
      package.json
      pnpm-lock.yaml
      src/
        index.ts
      artifacts/
        zod.backend.mjs
        integrity.json

    sales-chart-block/
      manifest.yaml
      package.json
      pnpm-lock.yaml
      src/
        block.ts
        style.css
      artifacts/
        sales-chart.browser.mjs
        sales-chart.css
        integrity.json
```

打包产物：

```text
api/plugins/
  packages/
    js-zod-pack-0.1.0.1flowbasepkg
    sales-chart-block-0.1.0.1flowbasepkg
```

安装结果：

```text
api/plugins/
  installed/
    js-zod-pack@0.1.0/
      manifest.yaml
      artifacts/
        zod.backend.mjs
        integrity.json

    sales-chart-block@0.1.0/
      manifest.yaml
      artifacts/
        sales-chart.browser.mjs
        sales-chart.css
        integrity.json
```

## 生命周期

### 开发 / 打包阶段

开发者或官方 CI 可以使用任意 JS 工具链：

```text
pnpm / npm / bun / yarn / vite / tsup / esbuild
```

这些工具只属于开发和打包阶段，不属于 1flowbase 生产运行依赖。

```text
Plugin source
  |
  | pnpm install / bun install / npm install
  | pnpm build / bun build / esbuild
  v
artifacts/*.mjs
  |
  | package
  v
*.1flowbasepkg
```

### 安装阶段

1flowbase 安装插件时不运行 `npm install`。

```text
.1flowbasepkg
  |
  v
Plugin Intake
  - read manifest.yaml
  - validate schema
  - verify artifact integrity
  - copy to api/plugins/installed/
  - register dependency / block catalog
```

### 应用启用阶段

应用显式启用依赖或区块：

```text
Application
  |
  +-- dependencies
  |     zod -> js-zod-pack@0.1.0
  |
  +-- blocks
        sales_chart -> sales-chart-block@0.1.0
```

发布应用时，需要把启用项和 artifact hash 写入发布快照，保证可复现。

### 运行阶段

后端 Code 节点：

```text
Code Node
  |
  | import { z } from "zod"
  v
Application Dependency Snapshot
  |
  | zod -> installed/js-zod-pack@0.1.0/artifacts/zod.backend.mjs
  v
Default Backend JS Runner
  |
  v
execute main(inputs)
```

前端区块：

```text
Browser
  |
  v
1flowbase Web App
  |
  | load sales_chart artifact URL
  v
Sandbox iframe
  |
  | sales-chart.browser.mjs
  v
BlockContext + rendered UI
```

## 示例 1：后端 zod 工具包

`js-zod-pack` 插件目录：

```text
api/plugins/capability-plugins/js-zod-pack/
  manifest.yaml
  package.json
  pnpm-lock.yaml
  src/
    index.ts
  artifacts/
    zod.backend.mjs
    integrity.json
```

manifest 草案：

```yaml
manifest_version: 1
plugin_id: js-zod-pack@0.1.0
version: 0.1.0
vendor: flowbase
display_name: Zod JS Pack
description: Provides zod for backend Code nodes.
source_kind: filesystem_dropin
trust_level: checksum_only
consumption_kind: capability_plugin
execution_mode: declarative_only
slot_codes:
  - js_dependency_pack
binding_targets:
  - workspace
selection_mode: manual_select
minimum_host_version: 0.1.0
contract_version: 1flowbase.js_dependency_pack/v1
schema_version: 1flowbase.plugin.manifest/v1

permissions:
  network: none
  secrets: none
  storage: none
  mcp: none
  subprocess: deny

runtime:
  protocol: declarative
  entry: none

js_dependencies:
  - alias: zod
    package: zod
    version: 3.24.0
    targets:
      - backend_code
    artifacts:
      backend_code: artifacts/zod.backend.mjs
    integrity: sha256-xxx
    permissions:
      network: deny
      filesystem: deny
      env: deny
    native_addon: false
    lifecycle_scripts: false
```

Code 节点使用：

```js
import { z } from "zod";

function main(inputs) {
  const name = z.string().min(1).parse(inputs.name);

  return {
    result: name.toUpperCase()
  };
}
```

## 示例 2：前端 Sales Chart 区块

`sales-chart-block` 插件目录：

```text
api/plugins/capability-plugins/sales-chart-block/
  manifest.yaml
  package.json
  pnpm-lock.yaml
  src/
    block.ts
    style.css
  artifacts/
    sales-chart.browser.mjs
    sales-chart.css
    integrity.json
```

manifest 草案：

```yaml
manifest_version: 1
plugin_id: sales-chart-block@0.1.0
version: 0.1.0
vendor: acme
display_name: Sales Chart Block
description: Frontend chart block for dashboards.
source_kind: filesystem_dropin
trust_level: checksum_only
consumption_kind: capability_plugin
execution_mode: declarative_only
slot_codes:
  - frontend_block
binding_targets:
  - workspace
selection_mode: manual_select
minimum_host_version: 0.1.0
contract_version: 1flowbase.frontend_block/v1
schema_version: 1flowbase.plugin.manifest/v1

permissions:
  network: none
  secrets: none
  storage: none
  mcp: none
  subprocess: deny

runtime:
  protocol: declarative
  entry: none

block_contributions:
  - contribution_code: sales_chart
    title: Sales Chart
    runtime: frontend_block
    entry: artifacts/sales-chart.browser.mjs
    stylesheet: artifacts/sales-chart.css
    dependencies:
      - echarts
    context_contract:
      inputs:
        - key: records
          valueType: array
      events:
        - key: record_selected
      actions:
        - key: refresh_data
    permissions:
      network: deny
      storage: none
      host_actions:
        - refresh_data
```

区块代码示例：

```js
import { createBlock } from "@1flowbase/block-sdk";
import * as echarts from "echarts";

export default createBlock({
  mount(ctx, element) {
    const chart = echarts.init(element);

    ctx.events.on("records.updated", (records) => {
      chart.setOption({
        xAxis: { type: "category" },
        yAxis: { type: "value" },
        series: [{ type: "bar", data: records.map((record) => record.amount) }]
      });
    });

    chart.on("click", (params) => {
      ctx.events.emit("record_selected", { index: params.dataIndex });
    });
  }
});
```

## 运行进程模型

初期后端 Code 节点可走内置默认 runner：

```text
api-server
  |
  | CodeInvoker
  v
Default JS Runner
  |
  | loads installed dependency artifact
  v
user code + dependency artifact
```

如果进入进程隔离阶段：

```text
api-server
  |
  | stdio/json or local RPC
  v
code-runner process
  |
  | loads installed dependency artifact
  v
user code + dependency artifact
```

前端区块进程模型：

```text
web app
  |
  | artifact URL + context bootstrap
  v
sandbox iframe
  |
  | postMessage / BlockContext SDK
  v
host mediator
```

## Node / pnpm / Bun 策略

初期生产运行不内置 `node`、`pnpm`、`bun`。

| 阶段 | 是否需要 JS 工具链 | 说明 |
|---|---|---|
| 插件开发 | 需要，开发者自选 | 可用 pnpm / npm / bun / yarn |
| 插件打包 | 需要，CI 或开发者负责 | 输出 artifact 和 integrity |
| 插件安装 | 不需要 | 1flowbase 只校验和复制 |
| Code 节点运行 | 不需要 | 默认 runner 加载已构建 artifact |
| 前端区块运行 | 不需要 | 浏览器加载 browser artifact |

未来只有在支持完整 npm runtime、native addon、postinstall、Node built-ins 或 Bun runtime 时，才考虑引入独立 `node-runner` / `bun-runner`，且必须运行在 `process` / `container` / `remote` 隔离模式下。

## 初期最小范围

第一阶段只做：

1. `js_dependency_pack` manifest 扩展。
2. 插件安装时登记 `js_dependencies`。
3. 应用级启用依赖。
4. Code 节点 import 已启用依赖。
5. 默认 JS runner 加载已构建 artifact。
6. 发布快照记录依赖和 artifact hash。

第二阶段再做：

1. `frontend_block` manifest 扩展。
2. 前端 block catalog。
3. browser artifact 加载。
4. sandbox iframe。
5. `BlockContext` 和 host mediator。

第三阶段再做：

1. 节点级隔离策略。
2. 独立 `code-runner`。
3. 可插拔 executor。
4. 更完整的 package build service。

## Issue 拆分建议

采用 `1 + N` 结构：

- 1 个总功能 issue：描述 JS 扩展平台目标、边界和验收。
- N 个开发计划 issue：每个 issue 内部包含自己的步骤、验收证据和停止条件，不用在评论里继续拆步骤。

建议拆分：

1. 总功能：JS 扩展平台。
2. 计划 1：JS Dependency Pack manifest 与插件安装登记。
3. 计划 2：应用依赖启用、发布快照和 import 校验。
4. 计划 3：后端 Code 节点默认 JS runner 与 zod 示例闭环。
5. 计划 4：Frontend Block manifest、catalog 与 sandbox 加载设计。
6. 计划 5：节点级隔离策略与未来 runner 扩展预留。

## 已发布 Issues

- 总功能：[#142](https://github.com/taichuy/1flowbase/issues/142) JS 扩展平台（后端 Code 节点依赖包 + 前端 JS 区块）
- 计划 1：[#143](https://github.com/taichuy/1flowbase/issues/143) JS Dependency Pack manifest 与插件安装登记
- 计划 2：[#144](https://github.com/taichuy/1flowbase/issues/144) 应用依赖启用、发布快照与 import 校验
- 计划 3：[#145](https://github.com/taichuy/1flowbase/issues/145) 后端 Code 节点默认 JS runner 与 zod 示例闭环
- 计划 4：[#146](https://github.com/taichuy/1flowbase/issues/146) Frontend Block manifest、catalog 与 sandbox 加载设计
- 计划 5：[#147](https://github.com/taichuy/1flowbase/issues/147) 节点级隔离策略与未来 runner 扩展预留

## 待决问题

1. `js_dependency_pack` 是否作为 `capability_plugin` 的新 slot，还是新增独立 `consumption_kind`？
2. manifest 是否允许 `runtime.protocol: declarative` / `entry: none`，还是需要沿用现有 `stdio_json` 占位？
3. 第一版 JS dependency artifact 是否只支持单文件 ESM？
4. 默认 JS runner 是 QuickJS 还是其他 runtime？
5. 前端区块是否第一版就使用 iframe sandbox，还是先用更轻的 Shadow DOM + CSP？
6. 应用依赖启用入口放在 Application settings 还是 Agent Flow editor 内？
7. 包依赖是否允许插件内 bundle 自带 transitive dependencies？
8. 是否需要官方 registry 扫描和 license policy？
