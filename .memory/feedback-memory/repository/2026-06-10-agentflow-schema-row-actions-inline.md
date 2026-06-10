---
created_at: 2026-06-10 09
feedback_category: frontend_interaction
decision_policy: direct_reference
scope: agent-flow schema field row actions
---

# Schema 字段行级新增操作应进入操作列

规则：`agent-flow` 的 Schema 字段编辑表里，行级新增操作不要单独掉到子区域底部；应作为加号 icon 放在当前字段行的操作列，并与删除等行操作并排。

原因：底部独立加号会和字段层级、enum 子项、子字段边界混在一起，用户很难判断它属于哪一行；放入操作列能复用已有“操作”栏模型，降低误读。

适用场景：调整 `JsonSchemaSettingsPanel` 或同类字段树 / schema 表格时，`string` 的 enum 新增、`object` / `array` 的子字段新增等行级动作都优先收进当前行操作组。
