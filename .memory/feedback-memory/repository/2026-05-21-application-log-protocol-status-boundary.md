---
memory_type: feedback
feedback_category: repository
topic: application-log-protocol-status-boundary
summary: 应用日志可把 compatibility_mode 展示为协议；运行状态文案暂不处理，后续前端多语言统一接管。
keywords:
  - application logs
  - conversation log
  - protocol
  - status
  - i18n
created_at: 2026-05-21 11
updated_at: 2026-05-21 11
last_verified_at: 2026-05-21 11
decision_policy: direct_reference
scope:
  - web/app/src/features/applications
  - web/app/src/features/agent-flow/components/debug-console
---

# Application Log Protocol Status Boundary

## 规则

应用运行 `compatibility_mode` 字段需要展示时，UI 文案可叫“协议”，但前后端接口和代码字段名应保持 `compatibility_mode`。展示位置优先放在 `/applications/:id/logs` 表格列和对话日志 `详情 -> 元数据` 中。不要顺手调整运行状态文案或 `completed` 这类状态映射，状态多语言后续由前端国际化统一处理。

## 原因

用户明确确认协议列可以加在应用日志和对话日志里，但补充说明 `状态: completed` 暂时不用管。协议展示和状态本地化属于两个不同交付点，混在一起会扩大变更范围。

## 适用场景

修改应用日志表格、运行详情浮窗、对话日志元数据、公开 API / 兼容协议运行展示时命中。
