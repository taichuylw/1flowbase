---
created_at: 2026-06-10 10
feedback_category: frontend_interaction
decision_policy: direct_reference
scope: agent-flow schema field editor array and enum typing
---

# Schema 数组和枚举类型必须显式可选

规则：`agent-flow` Schema 字段编辑器里，数组字段不能只记录 `type: array`；必须让用户显式选择 `Array<String>`、`Array<Number>`、`Array<Object>` 等 `items.type` 语义，并在保存时输出标准 JSON Schema。enum 行的类型也不能固定只读，应跟随字段或数组元素类型，并在可编辑控件中呈现。

原因：只存数组本身会导致从 JSON Schema 回到字段模式后把 `array<string>` / `array<number>` 误转成 `array<object>`；enum 类型只读会让用户无法表达枚举值的真实类型，也会让数组元素枚举和 `items.type` 脱节。

适用场景：调整 `JsonSchemaSettingsPanel`、工具注册输入参数、输出契约或类似 schema 字段树时，必须保护 `items.type`、`items.enum` 和字段级 `enum` 的往返一致性。
