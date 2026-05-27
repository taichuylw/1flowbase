---
created_at: "2026-05-27 10"
updated_at: "2026-05-27 10"
feedback_category: repository
topic: Agent Flow 编辑器必须阻止已删除节点引用发布
summary: 用户确认当节点被删除但 binding / templated text 仍引用该节点输出时，编辑器应静态报错、禁止发布，并在 Issues 入口用 Ant Design Badge 标出错误数量。
decision_policy: direct_reference
---

# Agent Flow 编辑器必须阻止已删除节点引用发布

规则：Agent Flow authoring document 中的 binding、templated text 或 selector 引用了不存在的节点时，必须在编辑器校验阶段生成 field-level error，不能等运行时才失败。

规则：存在这类 error 时，发布入口必须禁用；Issues 入口应使用 Ant Design `Badge` 展示当前 error 数量。

规则：错误信息要归属到持有引用的字段，使用户能在节点详情面板看到具体是哪一个字段引用了已删除节点。

原因：节点删除后残留引用是 authoring 阶段可确定的结构错误。让它进入发布或运行时会导致工作流失败位置不清晰，也会破坏 Answer / Direct Reply 终点输出的稳定性预期。

适用场景：修改 Agent Flow 编辑器校验、变量选择、templated text、节点删除、Issues 面板、发布门禁、节点详情字段错误展示时命中。
