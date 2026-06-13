---
created_at: 2026-06-13 00
memory_type: feedback
feedback_category: repository
decision_policy: direct_reference
scope: qa project health gate
---

# QA Project Health Gate Must Not Anchor On One Failed Script Report

用户在 2026-06-13 反馈：更新 QA 后，AI 做“评估体检”时容易只盯着当前错误脚本报告，疑似影响上下文判断。

规则：项目体检 / Project Health Gate 不能把当前失败脚本报告当成评估主线；应先确认 lane、范围和质量维度矩阵，再把脚本失败、artifact、warningFiles 和日志归入对应维度作为证据。

原因：脚本失败是证据来源，不是完整体检范围。若先被当前错误报告锚定，AI 会遗漏 UI 逻辑、契约、状态一致性、架构边界、测试缺口、热点预防层等项目级维度。

适用场景：用户要求“AI 评估体检”“项目体检”“全量评估项目现状代码”“Project Health Gate”或类似项目级 QA 审计时。
