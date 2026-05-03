---
feedback_category: repository
decision_policy: direct_reference
created_at: 2026-05-03 18
updated_at: 2026-05-03 18
scope:
  - frontend
  - agent-flow
  - variable-picker
---

# 变量快速检索应脱离裁剪容器悬浮

## 规则

变量选择、快速检索、mention/typeahead 这类建议面板如果位于带边框、滚动或 `overflow: hidden` 的编辑容器内，应作为更上层浮层渲染，而不是留在输入框内部被边框裁剪。

## 原因

这类控件的视觉语义是“选择建议浮层”，不是编辑器内容的一部分。挂在编辑器内部会被 field frame、Inspector 滚动容器或卡片边框遮挡，用户会误判为检索结果缺失或控件破损。

## 适用场景

- Agent Flow 节点 Inspector 的模板文本字段变量选择。
- contenteditable / Lexical / Ant Design 输入控件中的 typeahead、mention、变量选择器。
- 父级存在 `overflow: hidden`、`overflow-y: auto`、边框 frame 或 modal/drawer 叠层。
