# JS 扩展平台架构讨论稿

日期：2026-05-15
状态：讨论稿
关联文档：`docs/plans/2026-05-15-code-node-isolation-architecture.md`

## 目标

设计一套可逐步落地的 JS 扩展平台，用于支撑：

1. 后端 Code 节点执行 JavaScript 数据转换。
2. JS 第三方依赖以插件形式注册给 Code 节点使用。
3. 前端 JS 区块以插件形式加载到 1flowbase 页面或工作区。
4. 新增“前台”路由作为面向所有访问者的页面浏览入口，并通过页面级设计模式开放低代码配置。
5. 未来按节点级 / 应用级策略扩展隔离、依赖、执行器和区块通信能力。

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
  +-- Front Stage Page Surface
  |     public page route with permission-gated design mode
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
- 前台页面 schema / block schema 的存储和版本

两个运行面分离：

| 维度 | 后端 Code Runtime | 前端 Block Runtime |
|---|---|---|
| 运行位置 | server / code-runner | browser / iframe sandbox |
| 用途 | 数据转换、流程执行 | 可视化、交互、页面区块 |
| 输入 | 上游节点变量 | props、页面上下文、数据源 |
| 输出 | 结构化 payload | UI event、action、state patch |
| 隔离 | VM / process / container | iframe sandbox / CSP / host mediator |
| 通信 | variable pool / trace | postMessage / BlockContext event bus |

## NocoBase 调研结论

参考本地 `../nocobase` 后，NocoBase 有两条相关实现线：

1. 页面设计模式：`DesignableSwitch` 负责切换 UI Editor，`BlockItem` 给区块包工具栏、拖拽和错误边界，`Designable` 把 `insertAdjacent` / `patch` / `remove` / `batchPatch` 写到 `uiSchemas`。
2. `plugin-block-iframe`：通过插件把 `Iframe` 组件注册到 schema renderer，再通过 `SchemaInitializer` 插入 `BlockItem + Iframe` schema。它支持 URL / HTML 两种模式，HTML 模式会把模板变量渲染成 `data:text/html` 后交给 iframe。
3. `plugin-workflow-js`：工作流 JavaScript 节点通过 `worker_threads.Worker` 启动 worker，再用 Node `vm.createContext` / `vm.runInContext` 执行脚本；参数、输出、日志和 timeout 模型对后端 Code 节点有参考价值。

可借鉴点：

- 前台页面应该有浏览态和设计态，而不是在普通浏览页面里直接混入配置入口。
- 区块外层应该统一提供工具栏、拖拽、复制、删除、配置和错误兜底。
- 区块入口使用 initializer 插入 schema，而不是让页面直接拼 UI。
- 插件注册组件、设置项、初始化入口的方式清晰。
- schema storage 的 `insertAdjacent` / `patch` / `remove` / `batchPatch` 写入模型适合复用。
- 工作流 JS 的参数、输出、日志、timeout、测试执行入口值得参考。
- 变量模板和 block settings 可以作为区块配置经验。

不建议照搬点：

- 用户 HTML 直接变成 iframe `data:text/html` 不适合作为社区 JS UI Block 的主模型。
- iframe `allow` 只解决浏览器 feature policy 的一部分，不能替代 1flowbase 的 BlockContext / action / event 权限模型。
- Node `vm` 和 worker thread 不应作为强安全边界；NocoBase changelog 中也出现过 Workflow JavaScript 上下文泄漏类安全修复。
- 普通社区区块不应直接注册宿主 React / Ant Design 组件。

对 1flowbase 的结论：

```text
NocoBase schema block 和设计模式思路可借鉴；
NocoBase iframe/html block 不作为安全 UI 扩展主路线；
1flowbase 应采用 Restricted JS UI Block：
  用户 JS -> 受限 runtime -> 输出 UI schema -> Host Renderer 渲染受控 primitives。
```

## 前台路由与设计模式决策

新增一路由叫“前台”，用于承载面向所有内部登录用户的空间页面。前台一开始可以是空白页面；有设计权限的人可以进入设计模式，新增和配置页面区块。

核心模型：

```text
Front Stage Route
  |
  +-- View Mode
  |     logged-in users can view
  |     blocks run with current user permissions
  |     no add/config/move/delete controls
  |
  +-- Design Mode
        only users with design permissions can enter
        add blocks through initializer
        configure data source and fields
        edit JS Block code
        move / duplicate / delete blocks
        persist changes to schema storage
```

信息架构上，前台页面不应该按“容器类型”拆功能，而应按信息深度拆：

| 深度 | 用户意图 | 前台页面行为 |
|---|---|---|
| L0 浏览 | 看当前页面内容 | 默认进入浏览态，只显示区块 |
| L1 聚焦 | 看某个区块或记录详情 | 区块内弹窗 / 当前区块交互 |
| L2 管理 | 配置页面和区块 | 设计模式内完成新增、移动、配置、删除 |
| L3 执行 | 编写 JS、配置数据写入 | 区块配置面板或代码编辑器内完成 |

### 设计模式权限

第一版权限保持简单：

- `frontstage.page.design`

所有内部登录用户默认可以浏览前台页面。设计模式入口只在当前用户具备 `frontstage.page.design` 时显示。普通用户看不到设计开关，也不会加载区块配置工具栏。

拥有 `frontstage.page.design` 的用户默认具备当前页面的完整设计能力：

- 新增区块
- 配置区块
- 配置区块数据源
- 移动区块
- 调整区块宽度和高度
- 复制区块
- 删除区块
- 编辑 JS Block 代码
- 选择内置模板并注入代码
- 试运行 JS Block
- 保存并持久化

第一版不再拆分 `ui_block.javascript.write`、`frontstage.block.data.configure` 等细粒度权限。能设计页面，就能写 JS 和配置数据。设计者需要对自己写入的页面结构和 JS 代码负责。

### 数据配置与运行权限

设计者配置的是“这个区块最多能访问什么”，运行时仍按当前用户权限决定“这个用户实际能访问什么”。

```text
Design Mode data config
  model: orders
  fields: id, title, amount, status
  actions: query/create/update/delete
  |
  v
Runtime ctx.data
  current user
  current workspace
  data model permission
  field permission
  row/filter/pagination limits
```

因此，设计权限不等于数据越权。即使设计者给区块配置了 `orders.update`，普通访问者运行区块时也必须具备对应数据权限。

### 保存策略

第一版不做草稿、发布、版本、回滚和频繁修改日志。

原则：

- 普通区块配置改动即时写入 schema storage。
- 布局、拖拽、宽度、高度改动即时写入 schema storage。
- JS Block 从空白代码开始，用户可以选择内置模板注入代码。
- JS Block 支持试运行，用户确认效果后保存；保存即持久化到 schema storage / code storage。
- 保存后的结果立即成为前台浏览态运行结果。

### 区块新增与管理流程

```text
open Front Stage page
  |
  | if has frontstage.page.design
  v
toggle Design Mode
  |
  v
Add Block
  |
  +-- Built-in data table
  +-- Built-in create form
  +-- Built-in edit form
  +-- Built-in search + table
  +-- JS UI Block
  |
  v
configure model / fields / actions / layout
  |
  v
preview / run block when needed
  |
  v
persist schema patch
```

区块工具栏仅在设计模式显示：

- 配置
- 移动
- 复制
- 删除
- 编辑 JS
- 预览运行

第一版采用即时保存，复用 schema storage 的 patch 模型。JS 代码不做发布流；试运行只是保存前的验证手段。

### JS Block 配置面板

建议按 tab 组织：

| Tab | 内容 |
|---|---|
| 基础 | 标题、描述、宽度、高度、可见条件 |
| 数据 | 模型、字段、query/create/update/delete、默认过滤、排序、分页 |
| 代码 | 空白 JS 编辑器、内置模板注入、试运行、保存、错误提示 |
| 上下文 | 允许暴露的 `ctx.currentUser` / `ctx.page` / `ctx.params` / `ctx.data` 等 |
| 运行限制 | timeout、最大返回 schema 节点数、最大行数、payload size |

第一版不做区块级“谁可见、谁可运行、谁可编辑”。前台页面面向所有内部登录用户可见；是否可以设计页面由 `frontstage.page.design` 控制。

### 布局能力

第一版直接支持成熟的页面编排能力，不做过窄的最小版本：

- 栅格布局。
- 顺序拖拽。
- 区块宽度调整。
- 区块高度调整。
- 区块配置即时持久化。

这部分参考 NocoBase 的成熟低代码页面编排经验。

布局数据建议放在 schema 的 `x-layout` 字段，避免和组件自身 `x-component-props` 混在一起。

示例：

```json
{
  "x-layout": {
    "grid": {
      "x": 0,
      "y": 0,
      "w": 12,
      "h": 8
    },
    "order": 10
  }
}
```

### 页面管理

前台需要页面管理能力。第一版采用经典左右布局：

```text
FrontStage Manager
  |
  +-- Left Sidebar
  |     dynamic route tree
  |     create group
  |     create page
  |     rename / reorder / delete
  |
  +-- Right Content
        selected page canvas
        top design toolbar
        block grid
```

左侧侧边栏是动态路由树，由有设计权限的用户自己创建和维护。

支持两种节点：

- 分组：只用于收纳和整理页面，本身没有页面内容，不可作为页面渲染。
- 页面：可以挂在根下，也可以挂在分组下；页面有自己的 schema root 和区块内容。

第一版页面树规则：

- 最多两层：分组 -> 页面。
- 不支持分组套分组。
- 页面可以直接放在根下，也可以放在分组下。
- 页面和分组允许重名；唯一性依赖系统生成的 UUID，不依赖标题。
- `pageId` 使用 UUID，不根据标题或 slug 生成。
- `slug` 字段只预留，不在页面管理 UI 暴露。
- 删除分组或页面为硬删，不做回收站和恢复。

访问规则：

- `/frontstage/:workspaceId` 加载当前 workspace 页面树里的第一个 `page`，按左侧页面树顺序从上到下查找，分组本身跳过。
- `/frontstage/:workspaceId/:pageId` 加载指定页面。
- 如果 workspace 没有任何页面，进入设计态后由用户自己创建；系统不自动创建页面。
- 普通浏览用户访问没有页面内容的前台时，看到空态，不自动生成页面。
- 如果当前浏览页面被删除，跳转到页面树里的第一个 `page`；如果没有任何页面，则显示空态。

### 设计工具栏

设计模式入口和设计操作放在页面顶部工具栏，而不是右上角悬浮按钮。

顶部工具栏只在有 `frontstage.page.design` 权限时显示设计入口。进入设计态后，工具栏承载：

- 退出设计
- 新增区块
- 页面管理
- 当前页面设置
- JS Block 试运行 / 保存入口
- 布局调整状态提示

### 前台页面 schema 草案

```json
{
  "x-component": "FrontStagePage",
  "x-uid": "frontstage_page_home",
  "properties": {
    "block_orders": {
      "x-component": "JsUiBlock",
      "x-decorator": "BlockItem",
      "x-block-type": "js-ui",
      "x-layout": {
        "grid": { "x": 0, "y": 0, "w": 12, "h": 8 },
        "order": 10
      },
      "x-data": {
        "source": {
          "model": "orders",
          "fields": ["id", "title", "amount", "status"],
          "actions": ["query", "create", "update", "delete"]
        },
        "runtime": {
          "timeoutMs": 1000,
          "maxRows": 100,
          "maxSchemaNodes": 300
        }
      },
      "x-component-props": {
        "codeRef": "js_block_code_orders",
        "template": "table-with-modal-edit"
      }
    }
  }
}
```

### 路由建议

第一版使用固定前台路由：

```text
/frontstage/:workspaceId
/frontstage/:workspaceId/:pageId
```

- `workspaceId`：当前前台所属的空间 / workspace id，对应后端现有 `workspace_id` / 当前登录态里的 `current_workspace_id`。
- `pageId`：空间前台内的具体页面 id。一个空间可以有首页、列表页、详情页、看板页等多个页面。
- `/frontstage/:workspaceId`：不带 `pageId` 时加载该空间页面树里的第一个 `page`。
- `/frontstage/:workspaceId/:pageId`：加载该空间前台下指定页面。

中文产品名确定为“前台”，技术路由确定为 `/frontstage/:workspaceId/:pageId`。

### 存储决策

前台页面内容直接复用 schema storage。额外增加一张很薄的 `frontstage_pages` 表，负责页面元信息、路由树和默认首页解析。

`frontstage_pages` 草案：

```text
frontstage_pages
  id
  workspace_id
  parent_id
  kind              page | group
  title             nullable
  slug              optional, reserved for readable URL
  schema_root_uid   only for page
  rank              string rank for drag insert
  created_at
  updated_at
```

约束：

- `kind = group` 时不挂 schema root，只作为页面树收纳节点。
- `kind = page` 时必须有 `schema_root_uid`。
- `slug` 是可读 URL 片段，例如 `sales-dashboard`；第一版路由使用 `pageId`，所以 `slug` 只作为可选预留，不作为访问主键。
- 默认首页不固定存储 `is_default`，按页面树排序取第一个 `page`，分组跳过。
- 删除分组或页面使用硬删。
- 删除清理在同一个事务内同步完成，不做后台异步清理。
- 分组删除允许级联删除下面的页面；被删除页面对应的 schema root、区块 schema 和 JS Block code 一并清理。
- 同级页面和分组允许重名；唯一性依赖 UUID。
- 标题允许为空；页面识别只依赖 UUID。
- 第一版不做复制页面。
- 页面树排序使用 rank 字符串，方便拖拽插入，不使用简单数字 `sort_order`。
- 页面内容、区块 schema、区块布局仍写入 schema storage。
- JS Block 代码和 schema 分开存，schema 中只放 `codeRef`。

JS Block 代码存储表定名为 `frontstage_block_codes`，第一版要求是：

- 按 workspace 隔离。
- code 与 schema 通过 `codeRef` 关联。
- 保存即覆盖当前代码，不做版本和回滚。
- 试运行使用编辑器当前代码，保存后成为浏览态运行代码。

### 开发执行顺序

第一版按“后端先行、前端接入”的顺序推进。

后端阶段先完成稳定真值：

1. 数据迁移：`frontstage_pages`、`frontstage_block_codes`。
2. Domain / service：页面树、分组、页面、rank 排序、默认首页解析。
3. Repository：按 workspace 查询页面树、创建、更新、重排、硬删、级联删除。
4. Schema storage 集成：创建页面时建立 schema root；删除页面 / 分组时同步事务清理 schema root、区块 schema 和 JS Block code。
5. 权限：内部登录用户可读；`frontstage.page.design` 才能写。
6. API：页面树查询、创建分组、创建页面、重命名、重排、删除、读取页面 schema、保存 JS Block code。
7. 后端测试：权限、两层限制、允许重名、rank 排序、默认首页解析、硬删事务清理、codeRef 存取。

前端阶段只消费后端真值：

1. API client DTO 和 feature api。
2. `/frontstage/:workspaceId`、`/frontstage/:workspaceId/:pageId` 路由。
3. 浏览态空态 / 页面渲染。
4. 顶部设计工具栏。
5. 左侧页面管理树：分组、页面、重命名、rank 排序、硬删。
6. 页面 canvas 和 schema renderer 接入。
7. `x-layout` 栅格、顺序拖拽、宽高调整。
8. JS Block 配置入口、`codeRef` 读取保存、内置模板注入。
9. 前端测试：无权限不显示设计态、页面树行为、删除后跳转、空态、布局持久化、JS code 保存。

## Permission-gated JS Block 决策

第一版前端 JS Block 不是强沙箱产品，而是权限授予后的受控前端脚本能力。

```text
是否允许写 JS Block = 权限控制
有权限的人写出业务逻辑问题 = 使用者 / 管理员责任
平台负责阻断：
  1. 外部网络请求
  2. 宿主页面越界
  3. 当前用户权限绕过
  4. 非受控 UI 破坏
```

第一版权限来源：

- 内部登录用户可以浏览前台页面。
- `frontstage.page.design` 用户可以进入设计模式。
- 能设计页面，就能写 JS Block、配置区块数据源、管理区块。
- 第一版不拆分 `ui_block.javascript.write`、`ui_block.javascript.manage`、`ui_block.javascript.install_template` 等细粒度权限。

数据访问权限复用当前登录用户、当前 workspace 和数据模型权限，不为 JS Block 单独创建越权身份。

编辑 JS 区块时建议展示正式风险提示：

```text
JS 区块会以当前用户权限读取和修改数据。
请只安装或编辑你信任的区块代码。
平台会阻止外部网络请求和宿主页面越界访问，但不会保证业务逻辑正确。
```

## AntD-compatible Facade 决策

第一版社区 JS UI Block 不直接开放 `react` / `antd` / `@1flowbase/ui` 真实组件实现，但也不要求用户直接手写 JSON UI schema。

采用折中模型：

```text
AI / 用户写组件式 JS
  |
  | imports @1flowbase/block-sdk
  | imports @1flowbase/antd-facade
  v
AntD-compatible facade
  |
  | returns restricted UI schema
  v
Host Renderer
  |
  | renders real 1flowbase / Ant Design components
  v
Visible UI Block
```

这样 AI 和社区开发者可以写接近 AntD 组件风格的代码，但不会拿到真实 React 组件、DOM、ref、portal、modal context、router、store 或 query client。

示例：

```js
import { defineBlock } from "@1flowbase/block-sdk";
import { Button, Stack, Stat, Table } from "@1flowbase/antd-facade";

export default defineBlock({
  render(ctx) {
    return Stack({
      gap: "md",
      children: [
        Stat({ label: "收入", value: ctx.props.revenue }),
        Table({
          columns: [
            { key: "name", title: "名称" },
            { key: "amount", title: "金额" }
          ],
          data: ctx.props.records,
          onRowClick: ctx.event("record_selected")
        }),
        Button({
          type: "primary",
          children: "刷新",
          onClick: ctx.action("refresh_data")
        })
      ]
    });
  }
});
```

Facade 输出的不是 React element，而是受限 UI schema：

```json
{
  "type": "Table",
  "props": {
    "columns": [
      { "key": "name", "title": "名称" },
      { "key": "amount", "title": "金额" }
    ],
    "dataRef": "props.records",
    "onRowClick": {
      "kind": "event",
      "name": "record_selected"
    }
  }
}
```

### 第一版允许导入

- `@1flowbase/block-sdk`
- `@1flowbase/antd-facade`

### 第一版禁止导入

- `react`
- `react-dom`
- `antd`
- `@1flowbase/ui`
- 任意 npm 包
- dynamic import

### 第一版边界

静态校验：

- 只允许白名单 import。
- 禁止 `window`、`document`、`localStorage`、`sessionStorage`、`cookie`、`fetch`、`XMLHttpRequest`、`WebSocket`、`navigator.sendBeacon`。
- 禁止 dynamic import。
- 禁止 `eval`、`new Function`。
- 事件建议通过 `ctx.action(...)` 或 `ctx.event(...)` 声明。

运行时：

- 在 Web Worker 内运行。
- 提供更自由的 JS 基础能力：`JSON`、`Math`、`Date`、`Intl`、`URL`、`URLSearchParams`、`RegExp`、`structuredClone`、`Promise`、`console`、`setTimeout`、`clearTimeout`。
- 不提供外部网络能力。
- 不提供宿主 DOM / window 引用。
- 不提供真实 module resolver。
- 注入 block SDK、facade 和受控上下文。

UI schema 校验：

- 组件类型必须在白名单内。
- 每个组件 props 必须通过 schema 校验。
- 函数不能穿透到 Host Renderer。
- style 只能使用 token-based 受限对象。

Host Renderer：

- action / event 经过 manifest context contract 校验。
- table 行数、列数和 cell 类型受限制。
- modal 作为受控 overlay primitive 开放。
- global feedback 默认不开放。

### 第一版 facade 组件候选

允许：

- 布局：`Stack`、`Inline`、`Grid`、`Divider`
- 文本：`Text`、`Title`、`Caption`、`Badge`
- 数据查看：`Table`、`Descriptions`、`Empty`、`Alert`
- 表单：`Form`、`FormItem`、`Input`、`Textarea`、`Select`、`Checkbox`、`Switch`、`DatePicker`、`NumberInput`
- 操作：`Button`、`IconButton`
- 弹窗：`Modal`

暂缓：

- `Drawer`
- `Upload`
- `RichText`
- `Tree`
- `Transfer`
- `Form.List`
- `List`
- Table 自定义 render 函数
- 任意 Dropdown overlay
- 任意 Tooltip content render
- `message` / `notification` 全局 API

当前决策：第一版按 AntD-compatible facade 设计；底层仍然是 UI schema 和 Host Renderer，不把真实 AntD 暴露给用户代码。第一版聚焦基础数据查看与交互：表格、表单、输入框、按钮、弹窗。

### 数据访问决策

第一版开放 `ctx.data` 的受控 CRUD，但它不是浏览器 `fetch`，也不是任意 API 调用。它只能访问 1flowbase 已建模的数据表 / 数据模型，并且必须经过宿主权限、字段、筛选、分页和写入约束。

示例：

```js
const rows = await ctx.data.query("orders", {
  select: ["id", "customer_name", "amount", "status"],
  filter: { status: "paid" },
  limit: 20
});

const created = await ctx.data.create("orders", {
  customer_name: "Acme",
  amount: 1200,
  status: "draft"
});

const updated = await ctx.data.update("orders", created.id, {
  status: "paid"
});

await ctx.data.delete("orders", created.id);
```

约束：

- 查询对象必须来自 1flowbase 元数据中已存在的数据模型。
- 字段必须在模型字段白名单内。
- 默认分页，禁止无上限查询。
- 不开放任意 URL、任意 SQL、任意 HTTP。
- 读写都按当前登录用户、当前 workspace、当前数据模型权限执行。
- 写入只允许走 `ctx.data.create/update/delete`，不能绕过数据模型 service。
- 查询和写入结果只进入当前 block 的 props/state，不直接改写宿主 store。
- 表单新增 / 修改数据属于第一版核心场景。

### 上下文开放决策

第一版可以向有权限的 JS Block 暴露更丰富的上下文，但必须保持当前用户权限和宿主边界。

建议上下文：

```js
ctx.currentUser
ctx.workspace
ctx.application
ctx.page
ctx.params
ctx.props
ctx.state
ctx.patch
ctx.data
ctx.actions
ctx.events
ctx.theme
ctx.ui
```

原则：

- 身份上下文只读。
- 数据 CRUD 按当前用户权限。
- 外部请求不直接开放。
- 宿主 router / store / query client 不暴露。
- 区块间通信只走 `ctx.events` / host mediator。

### 静态校验决策

第一版采用白名单开放策略，但不把普通 JS 逻辑限制得过死：

- 只允许 `@1flowbase/block-sdk` 和 `@1flowbase/antd-facade`。
- 禁止其他 import。
- 禁止宿主 DOM、外部网络、存储和动态代码执行能力。
- 允许常规数据加工、条件渲染、表单校验、状态计算、Modal 交互逻辑。
- 用户代码先经过 AST 校验 / transform，再进入 Web Worker 执行，不直接运行原始代码。

### 前端运行位置决策

前端 JS Block 是浏览器客户端能力，运行在 1flowbase 封装的 `BlockHost` / restricted block runtime 内。它不通过后端执行用户 UI 代码。

第一版执行容器采用 Web Worker：

```text
Browser
  |
  v
1flowbase BlockHost
  |
  | worker message bridge
  v
Restricted JS Block Worker
  |
  | facade returns UI schema / data intents
  v
BlockHost
  |
  v
Host Renderer
```

后端只负责：

- 提供 block code / catalog / metadata。
- 提供 `ctx.data.query/create/update/delete` 对应的受控数据接口。
- 做权限、字段、分页、写入约束和审计。
- 持久化区块定义和 schema UI 配置。

后端不执行前端 UI block 代码。

### 区块保存决策

第一版 JS UI Block 复用后端现有 schema storage，不新增独立 `schema_ui` 存储体系。

保存内容包括：

- block id / code
- block schema
- block props
- context contract
- allowed data models / fields / actions
- runtime limits
- owner application / page / workspace

第一版前端 UI Block 不进入外部发布 artifact 快照；它只作为当前空间前台页面 UI 配置的一部分持久化，面向内部登录用户使用。

### 崩溃隔离与资源限制

JS Block 可以自由一些，但必须保证只影响当前区块，不破坏页面其他部分。

第一版限制：

- Worker 执行超时。
- message size 上限。
- UI schema node count 上限。
- Table row cap。
- data query page size 上限。
- data mutation payload size 上限。
- Block 级错误面板兜底。

崩溃处理：

```text
worker runtime error / timeout / schema invalid
  -> terminate current block worker
  -> show block error panel
  -> keep page shell and other blocks running
```

### 错误码决策

第一版稳定错误码包括：

- `import_denied`
- `syntax_invalid`
- `transform_failed`
- `runtime_timeout`
- `runtime_error`
- `schema_invalid`
- `query_denied`
- `create_denied`
- `update_denied`
- `delete_denied`
- `action_denied`
- `event_denied`

CRUD 错误按动作区分，方便用户理解和 AI 修复。

### 删除确认决策

第一版 `ctx.data.delete` 不内置二次确认。是否确认由区块模板自行通过受控 `Modal` primitive 实现；平台只负责权限校验和错误返回。

### 内置模板决策

第一版提供少量内置模板，帮助 AI 和用户形成边界感：

- 空白模板。
- 数据表格查看模板。
- 表单新增数据模板。
- 表单编辑数据模板。
- 搜索筛选 + 表格模板。

模板来源第一版只做 1flowbase 内置模板，不开放插件贡献模板。

空白模板不是空字符串，而是一个完整 JS Block 骨架。它需要把基础写法要素都放出来，方便用户直接交给 AI 改写：

- import 写法。
- `defineBlock` 结构。
- 生命周期 / 初始化入口。
- `state` 示例。
- `render(ctx)` 示例。
- `ctx.data.query/create/update/delete` 用法注释。
- `ctx.patch` 示例。
- `ctx.events` / `ctx.actions` 示例。
- 错误处理示例。
- 返回 UI schema / facade component 的示例。

模板只使用：

- `@1flowbase/block-sdk`
- `@1flowbase/antd-facade`
- `ctx.data.query/create/update/delete`
- `ctx.patch`
- `ctx.event`

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

### Restricted JS UI Block

用于社区或用户自定义 UI 区块。它不是普通前端插件，也不是 iframe 内随便跑网页，而是：

```text
User JS Block Code
  |
  | only imports @1flowbase/block-sdk
  v
Restricted Block Runtime
  |
  | returns UI schema + event intents
  v
1flowbase Host Renderer
  |
  | renders approved UI primitives
  v
Visible UI Block
```

第一版只开放：

- `@1flowbase/block-sdk`
- `@1flowbase/antd-facade`
- 声明式 UI primitives
- theme tokens
- BlockContext actions / events / data
- 受限 style object

第一版不开放：

- React / ReactDOM
- Ant Design / `@1flowbase/ui` 真实组件实现
- 宿主 router / store / query client
- `window` / `document`
- `localStorage`
- `fetch`
- `eval` / `new Function`
- dynamic import
- 任意 CSS / 全局 selector / style tag
- 任意 npm 包

推荐分级：

| Level | 名称 | 说明 |
|---|---|---|
| 1 | Restricted UI Schema Block | 用户 JS 输出 UI schema，宿主渲染 primitives；社区区块默认从这里开始 |
| 2 | Native Trusted Block | 高权限 / 可信用户可 import React、AntD、`@1flowbase/ui` 真实组件，运行在独立 React root 内 |
| 3 | Sandbox Visual Block | iframe 内自行渲染 canvas/svg，只通过 BlockContext 通信；适合复杂图表 |
| 4 | Full Custom Frontend Plugin | 官方或企业插件，构建期集成，风险由管理员承担 |

### Native Trusted Block 路线

真实 React / AntD 组件不是完全不能开放，但不应并入第一版默认社区能力。它应作为后续高信任模式单独设计。

```text
Level 1: Facade Block
  -> @1flowbase/antd-facade
  -> UI schema
  -> Host Renderer

Level 2: Native Trusted Block
  -> React / AntD / @1flowbase/ui
  -> independent React root
  -> scoped providers
  -> block-level crash boundary
```

适用对象：

- 有 `ui_block.javascript.native` 权限的高级用户。
- 官方签名区块。
- 企业管理员安装并承担风险的私有区块。

不建议开放给普通社区默认区块。

边界：

- 每个 block 使用独立 React root。
- 每个 block 有自己的 ErrorBoundary。
- 每个 block 有 scoped ConfigProvider / theme / locale。
- popup / portal 必须挂到 `ctx.blockRoot`。
- 崩溃只卸载当前 block。
- 数据访问仍走 `ctx.data`，不直接给 API client / query client。
- 外部请求仍不直接开放。

需要禁止或适配：

- `fetch` / XHR / WebSocket / sendBeacon
- `localStorage` / `sessionStorage` / cookie
- `document.body` 操作
- `ReactDOM.createPortal`
- AntD `message` / `notification` 全局 API
- AntD `Modal.*` 静态方法
- `Upload`
- 任意全局 CSS selector

需要强制 patch / adapter：

- `Modal.getContainer = ctx.blockRoot`
- `Select.getPopupContainer = ctx.blockRoot`
- `Dropdown.getPopupContainer = ctx.blockRoot`
- `Tooltip.getPopupContainer = ctx.blockRoot`
- `ConfigProvider` 使用 scoped theme

成本：

- JSX 编译
- React / AntD / `@1flowbase/ui` 版本锁定
- 真实依赖 module injection
- AntD portal containment
- scoped CSS
- ref / DOM 越界控制
- source transform
- 热重载 / 预览
- 错误隔离
- 权限审计

当前建议：第一版继续做 Facade Block。Native Trusted Block 作为后续独立计划，不混入 `#146`。

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
import { defineBlock } from "@1flowbase/block-sdk";

export default defineBlock({
  state: {
    loading: false
  },

  render(ctx, ui) {
    return ui.stack({
      gap: "md",
      children: [
        ui.text({ value: "销售概览", variant: "sectionTitle" }),
        ui.stat({ label: "收入", value: ctx.props.revenue }),
        ui.table({
          columns: [
            { key: "name", title: "名称" },
            { key: "amount", title: "金额" }
          ],
          rows: ctx.props.records,
          onRowClick: "select_record"
        }),
        ui.button({
          label: "刷新",
          onClick: "refresh"
        })
      ]
    });
  },

  actions(ctx) {
    return {
      async refresh() {
        const data = await ctx.actions.invoke("refresh_data", {});
        return ctx.patch({ revenue: data.revenue, records: data.records });
      },
      async select_record(payload) {
        ctx.events.emit("record_selected", { id: payload.row.id });
      }
    };
  }
});
```

如果后续需要复杂 canvas/svg/chart runtime，可以作为 Level 2 `Sandbox Visual Block` 单独设计，而不是第一版社区 UI Block 默认能力。

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

前端受限 UI 区块运行模型：

```text
web app
  |
  | block code + props + context contract
  v
restricted JS worker runtime
  |
  | UI schema + event/data intents
  v
host renderer
  |
  | approved UI primitives
  v
visible block
```

前端 sandbox visual block 运行模型：

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
2. 前台路由、页面设计模式和区块 initializer。
3. 前端 block catalog。
4. Restricted JS UI Block runtime。
5. UI schema primitives 和 Host Renderer。
6. `BlockContext`、`ctx.data` CRUD 和 host mediator。
7. Web Worker 执行容器与区块级错误兜底。
8. sandbox iframe 仅作为后续 Level 2 visual block 的隔离载体，不作为第一版社区 UI Block 主模型。

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
5. 计划 4：Frontend Block Runtime、UI Schema primitives 与受限区块加载设计。
6. 计划 5：节点级隔离策略与未来 runner 扩展预留。
7. 计划 6：Native Trusted Block 运行时。
8. 计划 7：前台路由、页面设计模式与区块编排管理。

## 已发布 Issues

- 总功能：[#142](https://github.com/taichuy/1flowbase/issues/142) JS 扩展平台（后端 Code 节点依赖包 + 前端 JS 区块）
- 计划 1：[#143](https://github.com/taichuy/1flowbase/issues/143) JS Dependency Pack manifest 与插件安装登记
- 计划 2：[#144](https://github.com/taichuy/1flowbase/issues/144) 应用依赖启用、发布快照与 import 校验
- 计划 3：[#145](https://github.com/taichuy/1flowbase/issues/145) 后端 Code 节点默认 JS runner 与 zod 示例闭环
- 计划 4：[#146](https://github.com/taichuy/1flowbase/issues/146) Frontend Block Runtime、UI Schema primitives 与受限区块加载设计
- 计划 5：[#147](https://github.com/taichuy/1flowbase/issues/147) 节点级隔离策略与未来 runner 扩展预留
- 计划 6：[#148](https://github.com/taichuy/1flowbase/issues/148) Native Trusted Block 运行时（真实 React/AntD 组件，高信任模式）
- 计划 7：[#149](https://github.com/taichuy/1flowbase/issues/149) 前台路由、页面设计模式与区块编排管理

## 待决问题

1. `js_dependency_pack` 是否作为 `capability_plugin` 的新 slot，还是新增独立 `consumption_kind`？
2. manifest 是否允许 `runtime.protocol: declarative` / `entry: none`，还是需要沿用现有 `stdio_json` 占位？
3. 第一版 JS dependency artifact 是否只支持单文件 ESM？
4. 前端 JS Block 的 Web Worker runtime 如何实现模块注入、超时和终止？
6. 应用依赖启用入口放在 Application settings 还是 Agent Flow editor 内？
7. 包依赖是否允许插件内 bundle 自带 transitive dependencies？
8. 是否需要官方 registry 扫描和 license policy？
9. Level 2 Sandbox Visual Block 是否需要独立 issue，而不是并入 Level 1 UI Schema Block？

已决：

- 后端 Code 节点默认 JS runner 采用 `rquickjs`。
- 第一版 facade 组件聚焦表格、表单、输入框、按钮、弹窗，不开放 `List`。
- 前端 JS Block 是浏览器客户端能力，不通过后端执行用户 UI 代码。
- 第一版开放 `ctx.data.query/create/update/delete`，但只允许访问 1flowbase 元数据中的受控数据模型。
- 前端 UI Block 不进入外部发布 artifact；它作为当前空间前台页面 UI 配置的一部分处理，只面向内部登录用户使用。
- 第一版前端 JS Block 执行容器采用 Web Worker，崩溃只影响当前区块。
- 区块定义复用后端现有 schema storage。
- 用户代码先 AST 校验 / transform，再进入 Web Worker。
- `ctx.data` CRUD 错误码按 `query_denied / create_denied / update_denied / delete_denied` 区分。
- 前端 JS Block 定位为 permission-gated free JS：JS 逻辑更自由，平台硬限制外部网络、宿主越界和权限绕过。
- 新增“前台”作为面向所有内部登录用户的页面浏览入口，初始可为空白页。
- 前台页面需要页面级设计模式；有权限用户才能开启设计、添加区块、配置数据和编辑 JS Block。
- 区块数据配置只定义区块最大访问范围，运行时数据读写仍按当前登录用户权限执行。
- 中文产品名确定为“前台”，技术路由确定为 `/frontstage/:workspaceId/:pageId`。
- `workspaceId` 表示当前前台所属空间 / workspace，对应后端 `workspace_id`；`pageId` 表示空间前台内的具体页面 UUID；不带 `pageId` 时加载页面树里的第一个 `page`。
- 前台只面向内部登录用户，不做匿名访问。
- 第一版权限简化为：内部登录用户可浏览；拥有 `frontstage.page.design` 即可设计页面、配置数据、写 JS、管理区块。
- 第一版设计模式不做草稿、发布、版本、回滚和频繁修改日志；保存即持久化。
- JS Block 从空白代码开始，可选择内置模板注入代码，试运行后保存持久化。
- 第一版直接支持栅格布局、顺序拖拽、区块宽度和高度调整。
- 第一版 `ctx.data` 支持完整 CRUD，不对 delete 做额外产品层拆分。
- 第一版页面内容复用 schema storage，并增加薄表 `frontstage_pages` 管理页面元信息、分组和页面树。
- workspace 没有任何页面时，进入设计态后由用户自己创建；系统不自动创建页面。
- 设计模式入口和页面管理放在页面顶部工具栏。
- 布局数据放在 schema 的 `x-layout` 字段。
- JS Block 代码和 schema 分开存，schema 中只放 `codeRef`。
- 第一版模板只做内置模板，不开放插件贡献模板；空白模板必须提供完整 JS Block 骨架，方便用户交给 AI 改写。
- 前台需要页面管理，采用左侧动态分组 / 页面树 + 右侧页面内容的经典左右布局；分组只收纳页面，本身没有页面内容。
- 第一版页面树最多两层：分组 -> 页面；不支持分组套分组。
- 页面和分组允许重名，唯一性依赖系统生成的 UUID。
- `pageId` 使用 UUID，不根据标题或 slug 生成。
- `slug` 字段只预留，不在页面管理 UI 暴露。
- 删除分组或页面为硬删；删除分组时允许级联删除下面的页面。
- 删除页面 / 分组时，schema storage 和 `frontstage_block_codes` 清理在同步事务内完成。
- 默认首页不单独固定，按页面树排序取第一个 `page`，分组跳过。
- 页面树排序使用 rank 字符串，方便拖拽插入。
- JS Block 代码存储表定名为 `frontstage_block_codes`。
- 第一版不做复制页面；需要复制时用户自己复制 JS 代码。
- 页面和分组标题允许为空，识别只依赖 UUID。
- 删除当前浏览页面后跳转到页面树里的第一个 `page`；如果没有任何页面，则显示空态。
- 前台页面不发布给外部用户，不进入外部发布 artifact。
