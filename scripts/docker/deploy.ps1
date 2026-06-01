$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"

$FlowbaseRepo = if ($env:FLOWBASE_REPO) { $env:FLOWBASE_REPO } else { "taichuy/1flowbase" }
$FlowbaseRef = if ($env:FLOWBASE_REF) { $env:FLOWBASE_REF } else { "main" }
$FlowbaseArchiveUrl = if ($env:FLOWBASE_ARCHIVE_URL) {
  $env:FLOWBASE_ARCHIVE_URL
} else {
  "https://codeload.github.com/$FlowbaseRepo/tar.gz/refs/heads/$FlowbaseRef"
}
$FlowbaseArchiveDockerDir = "1flowbase-$FlowbaseRef/docker"

function Fail([string]$Message) {
  Write-Host $Message
  exit 1
}

function Read-EnvValue([string]$Key, [string]$Path) {
  if (-not (Test-Path $Path)) {
    return $null
  }

  foreach ($Line in Get-Content $Path) {
    if ($Line -match "^$([regex]::Escape($Key))=(.*)$") {
      return $matches[1]
    }
  }

  return $null
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
  Write-Host "Downloading 1flowbase docker files."
  Invoke-WebRequest -Uri $FlowbaseArchiveUrl -OutFile $Archive
  tar -xzf $Archive -C $TempDir $FlowbaseArchiveDockerDir
  $ExtractedDockerDir = Join-Path $TempDir ($FlowbaseArchiveDockerDir -replace "/", [System.IO.Path]::DirectorySeparatorChar)
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

$WebPort = Read-EnvValue "WEB_PORT" ".\.env"
$RootAccount = Read-EnvValue "BOOTSTRAP_ROOT_ACCOUNT" ".\.env"
$RootPassword = Read-EnvValue "BOOTSTRAP_ROOT_PASSWORD" ".\.env"

if (-not $WebPort) { $WebPort = "3100" }
if (-not $RootAccount) { $RootAccount = "root" }
if (-not $RootPassword) { $RootPassword = "1flowbase" }

Write-Host "1flowbase is starting. Web: http://127.0.0.1:$WebPort"
Write-Host "Initial root account: $RootAccount"
Write-Host "Initial root password: $RootPassword"
