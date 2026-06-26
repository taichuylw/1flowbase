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
- 组件发布时同时更新该镜像的 `latest` tag。
- `docker/docker-compose.yaml` 必须支持无本地 env 文件的一条命令启动；`docker/.env.example` 只是统一覆盖模板。
- 2026-06-20 起，Docker 一键部署支持数据库模式选择：默认 `DATABASE_MODE=internal` 使用内置 PostgreSQL；用户选择 `external` 时由部署脚本收集外部 PostgreSQL host、port、database、user、password、sslmode 并生成 `API_DATABASE_URL`。外部模式必须使用不包含内置 `db` 服务的 `docker/docker-compose.external-db.yaml`，避免企业已有数据库场景被迫启动本地 PostgreSQL。
- 默认使用 `latest`，并通过 `FLOWBASE_WEB_VERSION`、`FLOWBASE_API_SERVER_VERSION`、`FLOWBASE_PLUGIN_RUNNER_VERSION` 支持按组件 pin 具体版本。
- 不再要求用户复制 `api/api.env`、`plugin-runner/plugin-runner.env` 或 `postgres/postgres.env`；配置入口收敛到 `docker/.env`。
- 发布前用 `scripts/node/cli/verify-container-version.js` 校验镜像 tag 与组件 manifest 版本一致。
- 2026-06-03 起，CI 使用 buildx/QEMU 为官方组件镜像发布 `linux/amd64` 和 `linux/arm64` manifest；Rust Dockerfile 的 cargo target cache 按目标平台拆分，避免多平台构建共用 release target。
- 2026-06-03 起，`scripts/shell/docker-deploy.sh` 和 `scripts/powershell/docker-deploy.ps1` 在拉取或启动前检测有效 Docker 平台（优先尊重 `DOCKER_DEFAULT_PLATFORM`），并对三张官方业务镜像做 manifest 平台预检；当前 tag 缺少本机平台时提前失败并提示重新发布多架构镜像。

后续调整发布流程、文档或 workflow 时，先核对当前代码和 issue #454。
