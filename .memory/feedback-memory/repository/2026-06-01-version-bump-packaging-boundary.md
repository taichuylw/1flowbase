---
memory_type: feedback
feedback_category: repository
topic: version-bump-packaging-boundary
summary: 版本升级脚本服务主仓打包版本源，不处理 Docker env/tag 或插件 manifest；官方 provider 源码修复需要生效发布时，必须手动升级对应插件 manifest version。
keywords:
  - bump-version
  - docker env
  - packaging version
  - release script
  - official provider
  - manifest version
match_when:
  - 调整版本升级、发版、打包脚本时
  - 讨论前端、后端、Docker、插件版本边界时
  - 修改官方 provider 插件源码并需要让已安装包或官方 registry 获得新版本时
created_at: 2026-06-01 08
updated_at: 2026-06-14 17
last_verified_at: 2026-06-14 17
decision_policy: direct_reference
scope:
  - scripts/node/bump-version
  - docker/.env
  - docker/.env.example
  - api/plugins
  - ../1flowbase-official-plugins/runtime-extensions/model-providers
---

# Version Bump Packaging Boundary

## 时间

`2026-06-01 08`

## 规则

版本升级脚本的目标是让打包更方便，只更新前端 package、后端 Rust package 与相关 lockfile 中的自有包版本源。

不要把 Docker `.env`、Docker `.env.example`、Docker compose 镜像 tag 或插件 manifest 纳入自动升级范围。

官方 provider 插件源码改动如果要影响已安装包、官方 registry 或 release assets，不能只提交源码；必须同步手动升级对应 `../1flowbase-official-plugins/runtime-extensions/model-providers/<provider_code>/manifest.yaml` 的 `version:`，再推送触发 provider release。主仓 `bump-version` 仍不替插件 manifest 自动升级。

## 原因

Docker env 文件属于部署配置入口，不是打包版本源；插件是手动安装/发布的独立边界，不应被主仓库打包脚本顺手修改。

官方 provider 的发布由插件仓 manifest version 驱动。只改 provider Rust 源码不会触发正式 `.1flowbasepkg` 发布，也不会让已有安装实例拿到新包。

## 适用场景

后续调整 `scripts/node/bump-version`、发版脚本、打包脚本或版本字段扫描逻辑时，先按这个边界收窄目标。

后续修复 `../1flowbase-official-plugins/runtime-extensions/model-providers/*` 并需要交付可安装版本时，完成源码修复和验证后同步 bump 对应 manifest patch version；不要把“源码已推送”误当成“插件包已升级发布”。
