---
memory_type: feedback
feedback_category: repository
topic: agentflow_preview_log_side_panel
summary: Agent Flow 预览日志面板应作为预览旁侧独立面板打开，不能参与预览 Dock 宽度分配挤压聊天输入。
keywords:
  - agent-flow
  - preview
  - conversation-log
  - debug-console
  - side-panel
match_when:
  - 修改 Agent Flow 预览、对话日志、追踪、运行详情面板布局
  - 在预览聊天旁新增或调整辅助面板
created_at: 2026-05-13 11
updated_at: 2026-05-13 11
last_verified_at: 无
decision_policy: direct_reference
scope:
  - web/app/src/features/agent-flow/components/debug-console
  - web/app/src/features/agent-flow/components/editor/styles/shell.css
---

# Agent Flow 预览日志旁侧面板

## 时间

`2026-05-13 11`

## 规则

Agent Flow 预览中的对话日志 / 追踪面板，应作为预览 Dock 旁边的独立面板打开，不能放进预览 Dock 的同一条 flex 布局里参与宽度分配。

## 原因

用户明确指出日志打开后把聊天输入挤窄是不对的；预览对话框是主路径，日志是旁侧辅助信息，打开日志时必须保持预览对话框自身宽度和输入区可用性。

## 适用场景

- 点击预览消息动作打开“对话日志”。
- 在预览右侧聊天主路径旁展示运行详情、追踪、节点详情等辅助信息。
- 调整 debug console Dock、conversation log、运行详情面板布局。

## 备注

布局回归应覆盖：日志面板不参与预览 Dock flex 分配，预览 Dock 允许旁侧面板溢出展示，日志面板定位到预览左侧或旁侧。
