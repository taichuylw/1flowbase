---
memory_type: feedback
feedback_category: repository
topic: refactor-must-preserve-visible-result
summary: 前后端职责迁移或数据获取重构时，用户可见展示结果是验收契约；不能只证明架构更干净而忽略 UI 输出等价。
keywords:
  - frontend backend refactor
  - visible result parity
  - api migration
  - ui regression
  - read model
match_when:
  - 将原本前端处理的数据迁移到后端接口或 read model
  - 前端 mock / 本地派生逻辑替换为真实 API 数据
  - 为数据一致性调整 API、DTO、query 或页面展示映射
  - 重构后需要判断是否影响用户可见 UI 结果
created_at: 2026-05-11 16
updated_at: 2026-05-11 16
last_verified_at: 无
decision_policy: direct_reference
scope:
  - web
  - api
  - .agents/skills/frontend-development
  - .agents/skills/backend-development
  - .agents/skills/qa-evaluation
---

# Refactor Must Preserve Visible Result

## 规则

前后端职责迁移、API 替换、本地派生逻辑迁移到后端 read model 时，用户可见展示结果必须被当成验收契约。除非用户明确批准视觉、文案、排序、状态语义或交互反馈变化，否则重构前后的首屏结构、关键状态、列表顺序、空态、错误态、按钮可用性和主路径结果应保持等价。

## 原因

这类重构的目标通常是提升数据一致性和维护边界，但如果没有保护可见结果，AI 容易把“数据来源变干净”误判成“任务完成”，实际破坏用户已认可的前端效果。

## 适用场景

- 前端先做出效果，后续改成从后端接口获取真实数据。
- 前端本地聚合、筛选、排序、状态派生迁移到后端。
- 通用 DTO 替换为场景化 read model / view API。
- API 字段、状态枚举或 query key 调整会影响页面展示。

## 执行偏好

- 开工前先记录旧 UI 的可见基线：关键截图、样例数据、状态映射、排序规则和交互反馈。
- 实现时明确哪些变化属于“数据来源变化”，哪些属于“用户可见行为变化”；后者必须单独说明。
- 验收时用真实 API 或固定 mock 对比旧输出，不只跑类型、lint 或后端测试。
