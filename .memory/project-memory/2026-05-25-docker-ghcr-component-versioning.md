---
created_at: 2026-05-25 08
memory_type: project
decision_policy: verify_before_decision
---

# Docker / GHCR Component Versioning

当前 Docker / GHCR 发布方案由 Codex 按用户确认方向落地：镜像版本由维护者按组件独立维护，用户侧只消费部署模板并执行 `docker compose up -d`。

动机：用户不应该处理版本组合复杂度；但维护者发布时也不应该因为只改前端就重新发布 API 或 plugin-runner 镜像。GitHub 镜像发布必须由组件 manifest 版本变化或手动带版本触发，普通源码变更不得发布镜像。

约定：

- `web/app/package.json` 的 `version` 变化只发布 `ghcr.io/taichuy/1flowbase-web:vX.Y.Z`。
- `api/apps/api-server/Cargo.toml` 的 `version` 变化只发布 `ghcr.io/taichuy/1flowbase-api-server:vX.Y.Z`。
- `api/apps/plugin-runner/Cargo.toml` 的 `version` 变化只发布 `ghcr.io/taichuy/1flowbase-plugin-runner:vX.Y.Z`。
- `docker/.env.example` 使用 `FLOWBASE_WEB_VERSION`、`FLOWBASE_API_SERVER_VERSION`、`FLOWBASE_PLUGIN_RUNNER_VERSION` 分别 pin 三个镜像。
- 发布前用 `scripts/node/verify-container-version.js` 校验镜像 tag 与组件 manifest 版本一致。

后续调整发布流程、文档或 workflow 时，先核对当前代码和 issue #454。
