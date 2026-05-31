---
memory_type: feedback
feedback_category: repository
topic: application_log_floating_window_collision_open_only
summary: 应用运行日志浮窗的碰撞避让只用于打开第二个浮窗时的初始摆放；拖动和缩放过程不能强制碰撞检测；向下拖动可越过底部但必须保留可拖拽头部。
keywords:
  - application-logs
  - floating-window
  - collision
  - drag
  - viewport-bottom
match_when:
  - 修改 /applications/:id/logs 的运行详情或对话日志浮窗
  - 调整 ApplicationLogsFloatingWindow、ApplicationLogsPage 的拖动、缩放、打开位置或窗口避让逻辑
created_at: 2026-06-01 06
updated_at: 2026-06-01 06
last_verified_at: 2026-06-01 06
decision_policy: direct_reference
scope:
  - web/app/src/features/applications/pages/ApplicationLogsPage.tsx
  - web/app/src/features/applications/components/logs/ApplicationLogsFloatingWindow.tsx
---

# 应用日志浮窗碰撞只用于打开避让

## 规则

`/applications/:id/logs` 中运行详情和对话日志两个浮窗同时存在时，碰撞避让只用于打开第二个浮窗的初始摆放，避免刚打开就互相遮住。用户拖动或缩放已打开的浮窗时，只更新当前操作的浮窗位置或尺寸，不要强制重新计算两窗碰撞，也不要替用户移动另一个浮窗。

浮窗向下拖动时可以越过视口底部，窗口内容允许沉到下方不可见区域；但必须至少保留顶部可拖拽标题栏区域露出，确保用户能拖回来。不要用“窗口必须完整在视口内”的 clamp 约束拖动过程，否则高窗口只能左右移动，体验不友好。

## 原因

用户明确指出“之前做弹窗的碰撞是为了打开时候不遮住，而不是强制做碰撞检测”。持续碰撞检测会导致浮窗打开后无法自由横向拖动，尤其是对话日志浮窗会被运行详情位置锁定。用户随后补充希望“向下能够越过底部”，只要保留头部可拖拽部分即可，避免因为浮窗高度较大而几乎不能上下移动。

## 适用场景

- 修复应用运行日志浮窗拖动、缩放、重叠和层级问题。
- 调整运行详情与对话日志的初始打开位置。
- 改动 `handleRectChange`、`resolveCollision` 或视口 resize 对浮窗位置的处理。
