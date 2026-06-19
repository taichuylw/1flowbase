---
created_at: 2026-06-19 01
memory_type: feedback
feedback_category: repository
decision_policy: direct_reference
scope: repo hygiene quality gate
---

# Repo Hygiene Pressure Warnings Must Be Fixed Before Tracking

用户在 2026-06-19 反馈：`repo-hygiene` 扫出的已有大文件 / 目录压力，本来就是为了拆解修复；不能先接入 #901 跟踪机制，让它不再作为当前 active warning 阻塞门禁。

规则：`repo-hygiene` 的大文件、目录压力、弱断言、tracked artifact 等可直接修复 warning，默认先真实拆分、收纳、删除或修正，并用对应门禁和定向测试验证到 0 warning。只有确认当前任务内不能安全修复、且有明确历史债 / 架构债原因时，才允许接入 issue 跟踪，并在交付中说明后续拆解路径。

原因：质量门禁的意义是防止原有功能被破坏、同时不让新增功能压低工程质量；把可拆解 warning 直接转成 tracked warning，会掩盖当前可修复质量问题。

适用场景：处理 `repo-hygiene`、项目体检、质量门禁回归、文件 / 目录压力、tracked warning 或类似工程质量 warning 时。
