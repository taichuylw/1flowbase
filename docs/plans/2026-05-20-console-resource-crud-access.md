# Console Resource CRUD 访问改造审计说明

日期：2026-05-20

## 目标

这份文档用于整理当前 Console Resource CRUD 改造状态，方便后续总体审计。

这次改造的目标不是把物理表直接暴露成外部 API，而是收敛后台资源的基础访问协议，减少每个资源都从零手写一套 CRUD 约定。

目标方向：

- Console 资源对外使用统一访问协议。
- 通用 CRUD 能力沉淀到后端共享模块。
- 业务规则继续留在资源自己的 service command。
- 真正定制化行为继续使用显式 action API。

## 当前状态

第一版可复用 CRUD 基础层已经落在：

- `api/crates/control-plane/src/resource_crud.rs`

当前第一个接入资源是：

- 资源：`state_model`
- HTTP route：`api/apps/api-server/src/routes/plugins_and_models/model_definitions.rs`
- 领域记录：`domain::ModelDefinitionRecord`

这还不是全仓所有 Console 资源都完成迁移，而是第一版标准实现和后续资源接入参考。

## 现在已经统一的部分

### 资源描述符

每个资源声明稳定的资源编码和主键。

```rust
const STATE_MODEL_RESOURCE: ResourceCrudDescriptor =
    ResourceCrudDescriptor::new("state_model", "id");
```

route 层通过这个 descriptor 复用列表过滤和批量选择能力。

### 列表筛选输入

列表接口优先使用 NocoBase 风格的 `filter` query 参数，不再为每个资源发明 `search` 这类定制参数。

当前示例：

```http
GET /api/console/models?filter={"code":{"$includes":"customer"}}
```

当前基础层支持的操作符：

- `$eq`
- `$ne`
- `$includes`
- `$notIncludes`
- `$in`
- `$and`
- `$or`

字段操作符支持两种写法：

```json
{ "code": { "$includes": "customer" } }
```

```json
{ "code.$includes": "customer" }
```

### 批量选择

批量动作使用 `filterByTk` 表示前端勾选行，使用 `filter` 表示按条件批量操作。

勾选行批删：

```http
POST /api/console/models:batchDelete
```

```json
{
  "filterByTk": ["model-id-1", "model-id-2"],
  "confirmed": true
}
```

条件批删：

```http
POST /api/console/models:batchDelete
```

```json
{
  "filter": {
    "code": {
      "$includes": "temp_"
    }
  },
  "confirmed": true
}
```

共享 helper 负责先解析出目标 ID，之后仍然交给资源自己的 service command 执行业务删除。

### 可过滤字段暴露

每个资源通过实现 `ResourceFilterTarget` 显式声明哪些字段可以参与筛选。

当前 `state_model` 暴露的字段包括：

- `id`
- `scope_kind`
- `scope_id`
- `code`
- `title`
- `status`
- `api_exposure_status`
- `availability_status`
- `data_source_instance_id`
- `source_kind`
- `external_resource_key`
- `external_table_id`
- `physical_table_name`

未知字段不会自动穿透到存储层，避免把物理表结构裸露出去。

## 当前 state_model 接口形态

当前 `state_model` 的访问接口是：

```http
GET    /api/console/models
GET    /api/console/models/{id}
POST   /api/console/models
PATCH  /api/console/models/{id}
DELETE /api/console/models/{id}?confirmed=true
POST   /api/console/models:batchDelete
```

route 已经复用基础 CRUD 能力处理：

- 解析 `filter`
- 应用列表筛选
- 解析 `filterByTk`
- 通过 `filterByTk` 或 `filter` 选择批量目标 ID

route 仍然显式调用 service command 处理：

- create
- update
- delete
- batch delete
- 字段变更
- scope grant
- advisor finding

这个拆分是刻意保留的。

## 为什么 create/update/delete/get 暂不做完全泛型

当前不建议做“给一张物理表就自动生成完整 CRUD handler”。

原因：

- 权限检查是资源级的。
- 审计事件是资源级的。
- 删除和更新可能需要确认参数。
- 一些资源有内建记录或保护记录。
- 一些变更会触发 runtime registry rebuild。
- 一些操作有状态流转约束。
- 物理表是实现细节，不应该直接等同于外部 API 资源。

以 `state_model` 批量删除为例，它必须：

- 要求 `confirmed=true`
- 校验 `state_model.manage` 权限
- 拒绝未授权删除内建或保护模型
- 通过 repository 边界删除
- 写入审计日志
- 最后只 rebuild 一次 runtime registry

如果做成泛型物理表删除，这些领域规则很容易被绕开。

## 推荐的标准资源接入模式

后续新增 Console 资源建议按以下形态接入。

### 1. 声明资源 descriptor

```rust
const RESOURCE: ResourceCrudDescriptor =
    ResourceCrudDescriptor::new("resource_code", "id");
```

### 2. 声明可过滤字段

```rust
impl ResourceFilterTarget for domain::SomeRecord {
    fn field_value(&self, field: &str) -> Option<String> {
        match field {
            "id" => Some(self.id.to_string()),
            "code" => Some(self.code.clone()),
            "title" => Some(self.title.clone()),
            _ => None,
        }
    }
}
```

### 3. route 列表接口复用筛选 helper

```rust
let filter = parse_resource_filter(query.filter.as_deref())?;
records = RESOURCE.filter_records(records, filter.as_ref())?;
```

### 4. route 批量接口复用选择 helper

```rust
let ids = RESOURCE.select_batch_ids(
    records,
    ResourceBatchSelection::new(body.filter_by_tk, body.filter),
    parse_id,
    |record| record.id,
)?;
```

### 5. 写操作继续走资源自己的 command

```rust
mutation_service.batch_delete_xxx(BatchDeleteXxxCommand {
    actor_user_id,
    ids,
    confirmed: body.confirmed,
}).await?;
```

也就是说：协议和选择机制统一，业务规则显式保留。

## Runtime records 当前缺口

Runtime records 现在仍然有一套历史 filter 序列化方式，主要在前端 API client 和 runtime record 查询路径中。

它应该逐步并入同一套公开 `filter` 协议，但不能直接复用当前 `ResourceFilterTarget` 的内存 matcher。

原因是 runtime records 是动态 schema 记录，字段来自数据建模定义，不是固定 Rust struct 字段。

推荐下一步演进为：

```text
JSON filter
  -> 共享 Filter AST
  -> 固定 Console 资源：内存 matcher
  -> Runtime records：runtime query / storage condition compiler
```

这意味着：

- `state_model` 继续使用 `ResourceFilterTarget`。
- runtime records 使用相同 Filter AST，但编译到 runtime query 或存储查询条件。
- 前端和外部调用方看到同一套 `filter` 形态。
- 后端执行方式仍然按资源类型区分。

## 查询协议边界

`filter`、pagination 和 `sort` 是平级查询能力，不应该互相嵌套。

推荐共享查询协议保持这个形态：

```http
GET /api/console/resources?filter={...}&sort=created_at:desc&page=1&page_size=20
```

职责边界：

- `filter`：只表达过滤条件。
- `sort`：只表达排序字段和方向。
- `page` / `page_size`：只表达分页位置和分页大小。

基础 CRUD 模块可以统一解析这些参数，但语义上它们是并列的 query contract，不把 pagination 或 sort 放进 `filter`。

## Route Helper 与 Service Command 边界

`get/create/update/delete` 可以有 route-level helper，用来统一协议层样板。

route-level helper 可以负责：

- 统一 ID 解析。
- 统一 query/body 参数解析。
- 统一 `filter/filterByTk/sort/page/page_size` 解析。
- 统一成功响应形态。
- 统一常见错误字段名称。

但 helper 不应该替代资源自己的 service command。

“保持 service command 显式”的意思是：业务写入口仍然必须是明确命名的 command，而不是 route helper 直接对 repository 或物理表做泛型写入。

应该保留这种形态：

```rust
mutation_service.delete_model(DeleteModelDefinitionCommand {
    actor_user_id,
    model_id,
    confirmed,
}).await?;
```

而不是变成：

```rust
generic_crud.delete("state_model", model_id).await?;
```

这样可以确保权限、审计、状态流转、保护记录、runtime registry rebuild 等领域规则不会被协议层 helper 绕开。

## 目标模块演进

当前实现先集中在一个文件：

```text
api/crates/control-plane/src/resource_crud.rs
```

如果继续增长，建议拆成目录模块：

```text
api/crates/control-plane/src/resource_crud/
  mod.rs
  descriptor.rs
  filter.rs
  batch.rs
  ast.rs
```

职责建议：

- `descriptor.rs`：资源身份、主键和资源元信息。
- `filter.rs`：query/body filter 解析与校验。
- `ast.rs`：结构化 filter expression 类型。
- `batch.rs`：`filterByTk` 解析和批量目标选择。
- `mod.rs`：统一导出和 facade。

拆分时机已经确认：在第二个资源接入前先拆目录。

原因是第二个资源接入时应直接使用稳定模块边界，而不是继续扩大单文件后再迁移。

## 前端契约

底层 API client 现在发送：

```ts
fetchConsoleDataModels({
  data_source_instance_id: 'main_source',
  filter: {
    code: {
      $includes: 'customer'
    }
  }
});
```

client 会把 `filter` 序列化成 query 参数里的 JSON 字符串。

批量删除使用：

```ts
batchDeleteConsoleDataModels(
  {
    filterByTk: ['model-1', 'model-2'],
    confirmed: true
  },
  csrfToken
);
```

settings feature API wrapper 只透传同一份协议。

## 验证证据

本次改造已经跑过定向验证：

```bash
cargo test -p control-plane resource_crud_tests -- --nocapture
cargo test -p control-plane model_definition_runtime_sync_tests -- --nocapture
cargo test -p api-server application::model_definition_routes::model_crud -- --nocapture
pnpm --dir web/packages/api-client test -- console-data-models.test.ts
pnpm --dir web/app test -- src/features/settings/api/_tests/data-models-api.test.ts
cargo fmt --check
git diff --check
```

覆盖点：

- filter parser 和 matcher 行为
- `filterByTk` 解析
- descriptor 过滤记录
- descriptor 批量选择 ID
- `state_model` 列表筛选 route
- `state_model` 批量删除 route
- 批量删除后 runtime registry 只 rebuild 一次
- 前端 API client 请求形态
- settings feature API wrapper 请求形态

## 已确认决策

本轮审计已确认以下方向：

1. `resource_crud.rs` 在第二个资源接入前先拆成目录模块。
2. 下一步优先统一 runtime records 的 `filter` 协议。
3. `filter`、pagination、`sort` 是平级能力；`filter` 只负责过滤。
4. `get/create/update/delete` 需要 route-level helper 统一响应形态和 ID 解析。
5. route-level helper 只处理协议样板，业务写入口仍保持 service command 显式。
6. OpenAPI 在接口协议稳定后全局描述共享 `filter` 协议，基础 CRUD 直接复用这一套。

## 推荐下一步

不要做物理表 CRUD 生成器。

推荐下一步是：

1. 先把 `resource_crud.rs` 拆成 `resource_crud/` 目录模块。
2. 从当前 JSON matcher 中抽出真正的 `Filter AST`。
3. 保留 `state_model` 作为固定 record 的参考实现。
4. 给 runtime records 增加接受同一 AST 的 query compiler，并统一公开 `filter` 协议。
5. 定义全局基础 CRUD 查询协议：`filter`、`sort`、`page`、`page_size` 平级。
6. 增加 route-level helper，统一 ID 解析和响应形态，但不替代 service command。
7. 接口协议稳定后，在 OpenAPI 全局描述共享协议，后续基础 CRUD 直接复用。
