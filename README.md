# 1flowbase

> **对话即是壁垒，AI应用原生底座**

---

## 💡 核心特征

*   💬 **AI 聊天记录提炼总结**：你可以将你的 AI 聊天记录上传到当前 1flowbase 进行提炼总结。
*   🔄 **大模型中转与转发**：你可以将你的大模型放到 1flowbase 中做中转转发。
*   🛠️ **应用后端与低代码**：你可以将 1flowbase 作为你应用后端，动态生成表和低代码设计页面。

---

## 📂 仓库布局 (Repo Layout)

*   `web/`：前端根目录，基于 `pnpm + Turbo` 运作。入口应用位于 `web/app`，共享包位于 `web/packages/*`。
*   `api/`：后端根目录，基于 Rust workspace。服务入口位于 `api/apps/*`，共享 crate 位于 `api/crates/*`。
*   `api/plugins/`：插件源码工作区、HostExtension 清单与模板。
*   `docker/`：本地中间件（PostgreSQL/Redis等）容器编排。
*   `scripts/`：仓库级开发、测试、验证与调试脚本。详细说明见 [scripts/README.md](scripts/README.md)。

---

## 🚀 快速开始

### 运行环境要求
*   **Node.js**: `>= 24.0.0`
*   **Rust**: 最新稳定版编译器 (Workspace)
*   **Docker**: 用于启动本地开发所需中间件

### 本地分步启动

#### 1. 启动中间件
```bash
docker compose -f docker/docker-compose.middleware.yaml up -d
```

#### 2. 启动前端
```bash
cd web
pnpm install
pnpm dev
```
*   前端默认访问地址：`http://127.0.0.1:3100`

#### 3. 启动后端
首次启动请确保从 `api/apps/api-server/.env.example` 复制一份并配置好 `.env`。
```bash
cd api
# 启动 API 服务
cargo run -p api-server --bin api-server
# 启动插件运行器
cargo run -p plugin-runner --bin plugin-runner
```
*   API 服务地址：`http://127.0.0.1:7800`
*   插件运行器地址：`http://127.0.0.1:7801`

### Docker 一键部署

下面的命令不会安装 Docker。部署脚本只会先检查本机是否已经有可用的 Docker/Compose 环境，然后把 `docker/` 目录拉到当前目录，复制 `docker/.env.example` 为 `docker/.env`。随后脚本会进入交互配置，空输入保留提示中的当前值，再让你选择是否拉取镜像、是否启动容器。脚本输出保持英文，避免终端编码问题。

#### Shell

```bash
curl -fsSL https://raw.githubusercontent.com/taichuy/1flowbase/main/scripts/shell/docker-deploy.sh | sh
```

#### PowerShell

```powershell
irm https://raw.githubusercontent.com/taichuy/1flowbase/main/scripts/powershell/docker-deploy.ps1 | iex
```

#### Windows CMD

```bat
powershell -NoProfile -ExecutionPolicy Bypass -Command "irm https://raw.githubusercontent.com/taichuy/1flowbase/main/scripts/powershell/docker-deploy.ps1 | iex"
```

交互配置项包括：

- `POSTGRES_PASSWORD`
- `BOOTSTRAP_ROOT_ACCOUNT`
- `BOOTSTRAP_ROOT_PASSWORD`
- `API_PROVIDER_SECRET_MASTER_KEY`
- `WEB_PORT`

之后脚本会继续询问：

- `Pull Docker images? [y/N]`
- `Start 1flowbase now? [y/N]`

如果选择暂时不启动，之后可以手动执行：

```bash
cd docker
docker compose pull
docker compose up -d
```

非交互环境可以使用 `--db-password`、`--root-account`、`--root-password`、`--provider-secret`、`--web-port`、`--pull`、`--start`、`--no-pull`、`--no-start` 和 `--non-interactive` 控制行为。

---

## ⚙️ 脚本启动

为了简化本地的开发流程，仓库提供了一套统一的 Node 脚本进行一键式开发启动：

```bash
# 全量启动前端、后端、中间件与插件运行器
node scripts/node/dev-up.js

# 仅启动前后端进程，跳过 Docker 中间件
node scripts/node/dev-up.js --skip-docker

# 常用操作命令
node scripts/node/dev-up.js status   # 查看各服务状态
node scripts/node/dev-up.js stop     # 停止所有本地服务
node scripts/node/dev-up.js restart  # 重启服务
```

关于页面调试、自动化测试、清理缓存等的更多高级脚本用法，请参阅 [scripts/README.md](scripts/README.md)。

---

## 🤝 贡献

我们非常欢迎社区与团队成员的贡献！在提交 PR 前，请确保完成以下代码验证：

### 本地测试与校验
```bash
# 运行仓库级完整门禁 (包括后端格式化/Clippy/测试, 前端校验与契约测试)
node scripts/node/verify.js repo
```

### 协作规则
*   开发与质量控制规则以根目录下的 [AGENTS.md](AGENTS.md) 为准。
*   前端质量要求参见 [web/AGENTS.md](web/AGENTS.md)。
*   后端质量要求参见 [api/AGENTS.md](api/AGENTS.md)。

---
## 鸣谢

感谢 [Linux.do](https://linux.do/) 学ai 上L站

---

## License

This project is licensed under [Apache-2.0](LICENSE).

---

## Contributors

<p align="center">
  <a href="https://github.com/taichuy/1flowbase/graphs/contributors">
    <img src="https://contrib.rocks/image?repo=taichuy/1flowbase&max=50" alt="Contributors" />
  </a>
</p>

## Star History

<p align="center">
  <a href="https://www.star-history.com/#taichuy/1flowbase&Date" target="_blank">
    <img src="https://api.star-history.com/svg?repos=taichuy/1flowbase&type=Date" alt="Star History" width="600">
  </a>
</p>

<div align="center">

**If you like it, give us a star**

[Report Bug](https://github.com/taichuy/1flowbase/issues) · [Request Feature](https://github.com/taichuy/1flowbase/issues)

</div>
