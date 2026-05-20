---
memory_type: feedback
feedback_category: repository
topic: 具体业务规则不要内联污染通用 skill
summary: 给通用 skill 补规则时，具体业务域或单一模块规则应放到条件引用的 reference，主 SKILL.md 只保留触发索引。
keywords:
  - skill
  - reference
  - backend-development
  - progressive-disclosure
  - rules
match_when:
  - 修改通用项目 skill
  - 给 backend-development 等宽作用域 skill 增加模块规则
  - 将一次 issue 决策沉淀为 skill 规则
created_at: 2026-05-20 23
updated_at: 2026-05-20 23
last_verified_at: 2026-05-20 23
decision_policy: direct_reference
scope:
  - .agents/skills
---

# 具体业务规则不要内联污染通用 skill

## 时间

`2026-05-20 23`

## 规则

- 通用 skill 只放跨场景稳定规则和 reference 索引。
- 具体业务域、单一模块或单个 issue 派生的详细规则，应放到 `references/`，并在主 `SKILL.md` 写清何时读取。
- 不要让开发其他后端场景时默认加载 Agent Flow 节点、运行日志等窄领域规则。

## 原因

内联窄领域规则会污染通用 skill 的上下文，让 agent 在无关后端任务里带入错误约束。条件引用能保留规则，同时减少误触发。

## 适用场景

- 修改 `backend-development`、`frontend-development` 等宽作用域 skill。
- 把 issue 讨论结论沉淀为可复用规则。
- 判断某段内容应放在 `SKILL.md` 正文还是 `references/`。
