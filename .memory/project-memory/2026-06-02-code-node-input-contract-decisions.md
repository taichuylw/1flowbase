---
memory_type: project
topic: Code 节点入参表达模型已确认并实现
summary: 用户确认 Code 节点入参 UI 使用参数名、参数类型、参数值模型，不暴露“来源”列；参数值内部支持变量 selector、固定 JSON 常量和字符串模板，参数名就是变量名和显示名，只允许字母、数字、下划线。相关实现挂在 #591/#592/#593，并已关闭子 issue。
keywords:
  - agent-flow
  - code-node
  - named_bindings
  - input-parameters
  - selector
  - constant
  - templated_text
  - issue-591
match_when:
  - 继续调整 Code 节点入参 UI
  - 讨论 named_bindings 表达式或类型保真
  - 排查单点试运行 Start 内置变量默认值
  - 评审变量来源列是否应该出现在 Code 入参 UI
created_at: 2026-06-02 11
updated_at: 2026-06-02 11
last_verified_at: 2026-06-02 11
decision_policy: verify_before_decision
scope:
  - https://github.com/taichuy/1flowbase/issues/591
  - https://github.com/taichuy/1flowbase/issues/592
  - https://github.com/taichuy/1flowbase/issues/593
  - web/app/src/features/agent-flow/components/bindings/TemplatedNamedBindingsField.tsx
  - web/app/src/features/agent-flow/lib/named-binding-expressions.ts
  - api/crates/orchestration-runtime/src/binding_runtime.rs
  - api/crates/orchestration-runtime/src/compiler.rs
  - api/crates/orchestration-runtime/src/preview_executor.rs
---

# Code 节点入参表达模型已确认并实现

## 时间

`2026-06-02 11`

## 谁在做什么

用户确认并要求实现 Code 节点入参表达改造：UI 使用参数名、参数类型、参数值，不单独暴露“来源”；参数值控件内部可以选择变量或输入固定值。

## 为什么这样做

“来源”列会把一个找值的问题扩大成用户需要理解的来源模型。当前产品目标是让用户按参数语义配置：参数名决定 Code 中可读取的变量名，参数类型用于约束值选择和固定值解析，参数值负责承载变量 selector、常量或字符串模板。

## 为什么要做

Code 节点要支持多个入参，并且变量 selector 必须保持原始 JSON 类型，不能把 array/object 通过 `{{...}}` 模板强制转成字符串。固定值也要支持 number、boolean、object、array 等 JSON 类型。

## 截止日期

本轮已实现；#591、#592、#593 已关闭。#581 仍作为总控 issue 等待人工验收。

## 决策背后动机

用户希望尽快暴露参数问题，但不要把 UI 做成手动选择“来源”的深层模型。Code 入参采用 `named_bindings.value[].value.kind` 表达真实语义：`selector` 保持类型，`constant` 保存固定 JSON 值，`templated_text` 只用于字符串模板。

## 关联实现

- 前端 Code 入参 UI：`TemplatedNamedBindingsField`
- 前端表达式抽取和复制 remap：`named-binding-expressions`
- 后端编译依赖抽取：`compiler.rs`
- 后端运行时解析：`binding_runtime.rs`
- 单点试运行 Start 默认值：`preview_executor.rs`
