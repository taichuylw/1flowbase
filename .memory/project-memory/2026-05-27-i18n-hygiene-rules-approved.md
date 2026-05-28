---
memory_type: project
topic: i18n key/value hygiene gate 已确认
summary: 用户在 `2026-05-27 00` 确认多语言资源采用“就近 owner + 脚本统一检查”的平衡方案；`2026-05-28 21` 进一步确认全仓 locale 统一为 `zh_Hans/en_US`，不再保留前端 `zh-CN/en-US` 文件名。`i18n-hygiene` 继续阻塞同 owner key/value 问题，跨 owner 重复 key/value 先以 warning 暴露，AI 在 QA 或修复时根据 owner 语义决定是否上提 common。
keywords:
  - i18n
  - i18n-hygiene
  - frontend
  - qa
  - key-value
match_when:
  - 需要新增或调整前端多语言资源
  - 需要判断 i18n key/value 是否应上提 common
  - 需要维护或解释 i18n-hygiene 质量门禁
created_at: 2026-05-27 00
updated_at: 2026-05-28 21
last_verified_at: 2026-05-28 21
decision_policy: verify_before_decision
scope:
  - web/app/src/**/i18n
  - api/plugins/**/i18n
  - scripts/node/i18n-hygiene
  - .agents/skills/frontend-development
  - .agents/skills/qa-evaluation
---

# i18n key/value hygiene gate 已确认

## 决策

- 多语言资源跟随最近 owner 维护，不集中塞进一个全局目录。
- 全仓 locale 文件名统一固定为 `zh_Hans.json` 与 `en_US.json`，前端 UI、插件/provider 使用同一套 canonical locale。
- 同一 owner 下两种语言 key 必须对齐；JSON 重复 key、缺 locale 文件、非法文件名属于 error。
- 同 owner、同 locale 内重复 value 属于 error，优先复用已有 key 或调整文案语义。
- 跨 owner 重复 key/value 属于 warning，不自动合并；只有语义完全一致且稳定时才上提 common。

## 动机

用户希望目录设计对脚本和 AI 都友好：脚本能精确报出 owner、locale、key、value，AI 在 QA 或修复时能根据报告直接定位并复盘，不靠人工大海捞针。

## 后续

- 维护 `i18n-hygiene` 脚本与 QA gate 时，优先保持报告可修复性。
- 新增 common 前要确认它是短 UI 词，不是业务句子。
