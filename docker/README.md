# 1flowbase Docker

## 仅启动中间件

```powershell
Copy-Item .\middleware.env.example .\middleware.env
docker compose -f .\docker-compose.middleware.yaml up -d
```

当前默认本地中间件只包含 PostgreSQL。Redis 不作为默认依赖；后续通过 Redis HostExtension / infra plugin 接入。RustFS 不作为默认依赖；对象存储由后台文件存储配置选择，默认使用本地存储。

默认映射端口：

- PostgreSQL: `35432`

`docker/sandbox/config.yaml` 是默认 / reference sandbox backend 的配置模板，后续如果要接代理、限网或更严格隔离，可以直接在这里扩展。它当前更适合作为可选参考执行后端，而不是普通 workflow 开发的硬前置依赖。

## 使用 GHCR 镜像启动整套服务

首次部署先复制配置模板：

```bash
cd docker
cp .env.example .env
cp api/api.env.example api/api.env
cp plugin-runner/plugin-runner.env.example plugin-runner/plugin-runner.env
cp postgres/postgres.env.example postgres/postgres.env
```

编辑 `.env`，至少修改：

- `POSTGRES_PASSWORD`
- `API_PROVIDER_SECRET_MASTER_KEY`
- `BOOTSTRAP_ROOT_PASSWORD`

启动：

```bash
docker compose up -d
```

如果本机只有旧版 Compose，也可以把 `docker compose` 替换为 `docker-compose`。

默认服务：

- `web`: `http://127.0.0.1:3100`
- `api`: compose 内部 `http://api:7800`
- `plugin-runner`: compose 内部 `http://plugin-runner:7801`
- `db`: compose 内部 `db:5432`

默认镜像：

- `ghcr.io/taichuy/1flowbase-web:${FLOWBASE_WEB_VERSION}`
- `ghcr.io/taichuy/1flowbase-api-server:${FLOWBASE_API_SERVER_VERSION}`
- `ghcr.io/taichuy/1flowbase-plugin-runner:${FLOWBASE_PLUGIN_RUNNER_VERSION}`

`web` 镜像内置 nginx，用于托管前端静态文件并把 `/api`、`/health`、`/openapi.json` 反代到 `api:7800`。生产部署默认不挂载后端二进制和前端 `dist`，这些构建产物必须随镜像 tag 发布。

默认持久化和可编辑配置都在 `docker/` 下：

- `postgres/data/pgdata/`: PostgreSQL 数据
- `api/storage/`: API 本地文件存储
- `api/plugins/packages/`: 插件包缓存
- `api/plugins/installed/`: 插件安装产物
- `api/plugins/host-extension/dropins/`: HostExtension drop-in 目录
- `web/nginx.conf`: web 镜像的 nginx 覆盖配置

注意：不要把整个 `api/plugins` 挂载到容器里；镜像内置的 `api/plugins/host-extensions` 和 `api/plugins/sets` 是启动所需的官方插件工作区，只挂载上面的可写子目录。

## 本地构建缓存

Dockerfile 已启用 BuildKit cache mount：

- Rust 镜像缓存 cargo registry、git checkout 和 release `target` 中间产物。
- web 镜像缓存 pnpm store，并把依赖安装层和源码层拆开。

第一次本地构建仍然会慢；之后只改业务源码时，会复用依赖下载和大部分中间编译产物。

本地需要 Docker BuildKit/buildx 可用。先确认：

```bash
docker buildx version
```

从仓库根目录手动构建：

```bash
docker buildx build --load -f docker/api-server.Dockerfile -t ghcr.io/taichuy/1flowbase-api-server:local .
docker buildx build --load -f docker/plugin-runner.Dockerfile -t ghcr.io/taichuy/1flowbase-plugin-runner:local .
docker buildx build --load -f docker/web.Dockerfile -t ghcr.io/taichuy/1flowbase-web:local .
```

使用本地镜像启动：

```bash
cd docker
FLOWBASE_WEB_VERSION=local FLOWBASE_API_SERVER_VERSION=local FLOWBASE_PLUGIN_RUNNER_VERSION=local docker compose up -d
```

CI 发布镜像时也会继续使用 GitHub Actions cache，因此同一镜像的后续 tag 构建会复用远端缓存。镜像按组件版本 tag 发布：`web/v0.1.0` 只发布 `1flowbase-web:v0.1.0`，`api-server/v0.1.0` 只发布 `1flowbase-api-server:v0.1.0`，`plugin-runner/v0.1.0` 只发布 `1flowbase-plugin-runner:v0.1.0`。
