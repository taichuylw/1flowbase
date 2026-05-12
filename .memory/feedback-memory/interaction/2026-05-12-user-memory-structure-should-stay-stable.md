---
memory_type: feedback
feedback_category: interaction
topic: user-memory-structure-should-stay-stable
summary: 优化 `.memory/user-memory.md` 时，不应擅自重分顶层结构；用户偏好类条目应统一收纳在原有 `## 用户偏好` 下。
keywords:
  - user-memory
  - memory-structure
  - user-preferences
  - prompt-optimization
match_when:
  - 优化或整理 `.memory/user-memory.md`
  - 合并重复用户偏好
  - 参考 GPT 提示词指南调整用户偏好表达
created_at: 2026-05-12 22
updated_at: 2026-05-12 22
last_verified_at: 2026-05-12 22
decision_policy: direct_reference
scope:
  - .memory/user-memory.md
---

# 用户偏好结构应保持稳定

## 时间

`2026-05-12 22`

## 规则

优化 `.memory/user-memory.md` 时，可以压缩重复表达、消除冲突、调整单条规则措辞，但不要擅自把用户偏好拆成多个新的顶层分组。用户偏好类条目应统一收纳在原有 `## 用户偏好` 下，除非用户明确要求重构文件结构。

## 原因

用户希望记忆文件的结构稳定，后续阅读和检索时能按既有入口定位信息。过度分组会改变用户已经习惯的组织方式，即使内容本身更清晰，也会增加检索和维护成本。

## 适用场景

- 整理、压缩或优化 `.memory/user-memory.md`。
- 根据提示词指南减少重复和冲突。
- 将新长期偏好写入用户记忆。
