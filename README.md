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

下面的命令不会安装 Docker。它只会先检查本机是否已经有可用的 Docker/Compose 环境，然后把 `docker/` 目录拉到当前目录，复制 `docker/.env.example` 为 `docker/.env`，拉取镜像并启动 1flowbase。命令输出保持英文，避免终端编码问题。

#### Shell

```bash
set -euo pipefail

fail() {
  printf '%s\n' "$1" >&2
  exit 1
}

command -v docker >/dev/null 2>&1 || fail "Docker is required. Install Docker Engine or Docker Desktop first."
docker info >/dev/null 2>&1 || fail "Docker is installed but the daemon is not reachable. Start Docker and try again."

if docker compose version >/dev/null 2>&1; then
  compose() { docker compose "$@"; }
elif command -v docker-compose >/dev/null 2>&1; then
  compose() { docker-compose "$@"; }
else
  fail "Docker Compose is required. Install the Docker Compose plugin or docker-compose first."
fi

if [ -d ./docker ]; then
  echo "Using existing ./docker directory."
else
  command -v tar >/dev/null 2>&1 || fail "tar is required to unpack the 1flowbase archive."
  if command -v curl >/dev/null 2>&1; then
    download() { curl -fsSL "$1" -o "$2"; }
  elif command -v wget >/dev/null 2>&1; then
    download() { wget -qO "$2" "$1"; }
  else
    fail "curl or wget is required to download the 1flowbase docker files."
  fi

  tmpdir="$(mktemp -d)"
  trap 'rm -rf "$tmpdir"' EXIT
  archive="$tmpdir/1flowbase.tar.gz"
  download "https://codeload.github.com/taichuy/1flowbase/tar.gz/refs/heads/main" "$archive"
  tar -xzf "$archive" -C "$tmpdir" "1flowbase-main/docker"
  mv "$tmpdir/1flowbase-main/docker" ./docker
  echo "Downloaded ./docker."
fi

if [ ! -f ./docker/.env ]; then
  cp ./docker/.env.example ./docker/.env
  echo "Created docker/.env from docker/.env.example."
else
  echo "Using existing docker/.env."
fi

cd docker
compose pull
compose up -d
web_port="$(grep -E '^WEB_PORT=' .env | cut -d= -f2- || true)"
root_account="$(grep -E '^BOOTSTRAP_ROOT_ACCOUNT=' .env | cut -d= -f2- || true)"
root_password="$(grep -E '^BOOTSTRAP_ROOT_PASSWORD=' .env | cut -d= -f2- || true)"
echo "1flowbase is starting. Web: http://127.0.0.1:${web_port:-3100}"
echo "Initial root account: ${root_account:-root}"
echo "Initial root password: ${root_password:-1flowbase}"
```

#### PowerShell

```powershell
$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"

function Fail([string]$Message) {
  Write-Host $Message
  exit 1
}

if (-not (Get-Command docker -ErrorAction SilentlyContinue)) {
  Fail "Docker is required. Install Docker Desktop or Docker Engine first."
}

docker info *> $null
if ($LASTEXITCODE -ne 0) {
  Fail "Docker is installed but the daemon is not reachable. Start Docker and try again."
}

$UseDockerComposePlugin = $false
docker compose version *> $null
if ($LASTEXITCODE -eq 0) {
  $UseDockerComposePlugin = $true
} elseif (-not (Get-Command docker-compose -ErrorAction SilentlyContinue)) {
  Fail "Docker Compose is required. Install the Docker Compose plugin or docker-compose first."
}

if (Test-Path ".\docker") {
  Write-Host "Using existing ./docker directory."
} else {
  if (-not (Get-Command tar -ErrorAction SilentlyContinue)) {
    Fail "tar is required to unpack the 1flowbase archive."
  }

  $TempDir = Join-Path ([System.IO.Path]::GetTempPath()) ("1flowbase-" + [System.Guid]::NewGuid().ToString("N"))
  New-Item -ItemType Directory -Path $TempDir | Out-Null
  $Archive = Join-Path $TempDir "1flowbase.tar.gz"
  Invoke-WebRequest -Uri "https://codeload.github.com/taichuy/1flowbase/tar.gz/refs/heads/main" -OutFile $Archive
  tar -xzf $Archive -C $TempDir "1flowbase-main/docker"
  $ExtractedDockerDir = Join-Path (Join-Path $TempDir "1flowbase-main") "docker"
  Move-Item -Path $ExtractedDockerDir -Destination ".\docker"
  Remove-Item -Recurse -Force $TempDir
  Write-Host "Downloaded ./docker."
}

if (-not (Test-Path ".\docker\.env")) {
  Copy-Item ".\docker\.env.example" ".\docker\.env"
  Write-Host "Created docker/.env from docker/.env.example."
} else {
  Write-Host "Using existing docker/.env."
}

Set-Location ".\docker"
if ($UseDockerComposePlugin) {
  docker compose pull
  docker compose up -d
} else {
  docker-compose pull
  docker-compose up -d
}
$EnvValues = @{}
Get-Content ".\.env" | ForEach-Object {
  if ($_ -match "^([^#=]+)=(.*)$") {
    $EnvValues[$matches[1]] = $matches[2]
  }
}
$WebPort = if ($EnvValues.ContainsKey("WEB_PORT") -and $EnvValues["WEB_PORT"]) { $EnvValues["WEB_PORT"] } else { "3100" }
$RootAccount = if ($EnvValues.ContainsKey("BOOTSTRAP_ROOT_ACCOUNT") -and $EnvValues["BOOTSTRAP_ROOT_ACCOUNT"]) { $EnvValues["BOOTSTRAP_ROOT_ACCOUNT"] } else { "root" }
$RootPassword = if ($EnvValues.ContainsKey("BOOTSTRAP_ROOT_PASSWORD") -and $EnvValues["BOOTSTRAP_ROOT_PASSWORD"]) { $EnvValues["BOOTSTRAP_ROOT_PASSWORD"] } else { "1flowbase" }
Write-Host "1flowbase is starting. Web: http://127.0.0.1:$WebPort"
Write-Host "Initial root account: $RootAccount"
Write-Host "Initial root password: $RootPassword"
```

整套容器会启动 `web`、`api`、`plugin-runner` 和 `db`。访问地址和初始 root 账号以 `docker/.env` 中的 `WEB_PORT`、`BOOTSTRAP_ROOT_ACCOUNT` 和 `BOOTSTRAP_ROOT_PASSWORD` 为准。

生产部署时请在启动前编辑 `docker/.env`，修改数据库密码、`API_PROVIDER_SECRET_MASTER_KEY` 和 root 密码。

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
