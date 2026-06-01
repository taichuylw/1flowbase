---
memory_type: feedback
feedback_category: repository
topic: version-bump-packaging-boundary
summary: 版本升级脚本服务打包版本源，不处理 Docker env/tag 或插件 manifest。
keywords:
  - bump-version
  - docker env
  - packaging version
  - release script
match_when:
  - 调整版本升级、发版、打包脚本时
  - 讨论前端、后端、Docker、插件版本边界时
created_at: 2026-06-01 08
updated_at: 2026-06-01 08
last_verified_at: 2026-06-01 08
decision_policy: direct_reference
scope:
  - scripts/node/bump-version
  - docker/.env
  - docker/.env.example
  - api/plugins
---

# Version Bump Packaging Boundary

## 时间

`2026-06-01 08`

## 规则

版本升级脚本的目标是让打包更方便，只更新前端 package、后端 Rust package 与相关 lockfile 中的自有包版本源。

不要把 Docker `.env`、Docker `.env.example`、Docker compose 镜像 tag 或插件 manifest 纳入自动升级范围。

## 原因

Docker env 文件属于部署配置入口，不是打包版本源；插件是手动安装/发布的独立边界，不应被主仓库打包脚本顺手修改。

## 适用场景

后续调整 `scripts/node/bump-version`、发版脚本、打包脚本或版本字段扫描逻辑时，先按这个边界收窄目标。
