---
memory_type: project
topic: 多语言格式转换边界与 i18n hygiene 门禁
summary: 用户确认多语言格式转换采用现有双 locale 体系：前端 App/UI 资源使用 `zh-CN`、`en-US`，后端 profile/API locale/插件 provider catalog 使用 `zh_Hans`、`en_US`；转换只发生在边界，并把 `i18n-hygiene` 写成开发与 QA 门禁，帮助 AI 复用同 owner 既有 key。
keywords:
  - i18n
  - locale
  - hygiene
  - frontend
  - provider
  - qa
match_when:
  - 需要做多语言格式转换
  - 需要新增或调整 i18n 资源
  - 需要判断 locale 命名边界
  - 需要解释 `i18n-hygiene` 门禁用途
created_at: 2026-05-28 20
updated_at: 2026-05-28 20
last_verified_at: 2026-05-28 20
decision_policy: verify_before_decision
scope:
  - .agents/skills/frontend-development/references/i18n-rules.md
  - .agents/skills/qa-evaluation/references/i18n-hygiene-gate.md
  - scripts/node/i18n-hygiene
---

# 多语言格式转换边界与 i18n hygiene 门禁

## 时间

`2026-05-28 20`

## 谁在做什么

用户准备做多语言格式转换。AI 先检查现有策略、前端 i18n 实现、后端 locale 解析、provider catalog 和 `i18n-hygiene` 门禁后，建议保留现有双 locale 体系，并把转换边界与门禁写入项目 skill 规则。用户确认采用，并特别要求把多语言检查门禁写进去，方便 AI 复用重复 key。

## 为什么这样做

仓库已经形成两套有意分层的 locale 命名：前端 App/UI 资源使用 `zh-CN`、`en-US`，后端 profile/API locale/插件 provider catalog 使用 `zh_Hans`、`en_US`。强行统一全仓格式会牵动后端约束、用户偏好、provider catalog 与前端 i18next，收益不足。

## 为什么要做

多语言格式转换容易把“资源文件名格式”“运行态 locale”“接口字段名”和“展示文案”混在一起。明确边界可以避免为了展示便利新增 DTO 别名字段，也避免 AI 为消除重复字符串而错误上提 common 或跨 feature 复用业务 key。

## 截止日期

无

## 决策背后动机

让后续 AI 在新增文案或做格式转换时先通过 `i18n-hygiene` 查同 owner 既有 key / value，再决定复用、调整文案或保留局部 owner；error 必修，warning 需要解释保留或上提理由。

## 已落地规则

- `.agents/skills/frontend-development/references/i18n-rules.md`
- `.agents/skills/qa-evaluation/references/i18n-hygiene-gate.md`
