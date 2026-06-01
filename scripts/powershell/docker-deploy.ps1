param(
  [string]$DbPassword = $env:FLOWBASE_DB_PASSWORD,
  [string]$RootAccount = $env:FLOWBASE_ROOT_ACCOUNT,
  [string]$RootPassword = $env:FLOWBASE_ROOT_PASSWORD,
  [string]$ProviderSecret = $env:FLOWBASE_PROVIDER_SECRET,
  [string]$WebPort = $env:FLOWBASE_WEB_PORT,
  [switch]$NoStart,
  [switch]$PrepareOnly
)

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
$ShouldStart = -not ($NoStart -or $PrepareOnly -or $env:FLOWBASE_NO_START -eq "1" -or $env:FLOWBASE_NO_START -eq "true")

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

function Set-EnvValue([string]$Key, [string]$Value, [string]$Path) {
  $Lines = [System.Collections.Generic.List[string]]::new()
  if (Test-Path $Path) {
    foreach ($Line in Get-Content $Path) {
      $Lines.Add($Line)
    }
  }

  $Found = $false
  for ($Index = 0; $Index -lt $Lines.Count; $Index++) {
    if ($Lines[$Index] -match "^$([regex]::Escape($Key))=") {
      $Lines[$Index] = "$Key=$Value"
      $Found = $true
    }
  }

  if (-not $Found) {
    $Lines.Add("$Key=$Value")
  }

  Set-Content -Path $Path -Value $Lines -Encoding UTF8
}

if (-not (Get-Command docker -ErrorAction SilentlyContinue)) {
  Fail "Docker is required. Install Docker Desktop or Docker Engine first."
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

if ($DbPassword) {
  Set-EnvValue "POSTGRES_PASSWORD" $DbPassword ".\docker\.env"
  Write-Host "Updated POSTGRES_PASSWORD in docker/.env."
}
if ($RootAccount) {
  Set-EnvValue "BOOTSTRAP_ROOT_ACCOUNT" $RootAccount ".\docker\.env"
  Write-Host "Updated BOOTSTRAP_ROOT_ACCOUNT in docker/.env."
}
if ($RootPassword) {
  Set-EnvValue "BOOTSTRAP_ROOT_PASSWORD" $RootPassword ".\docker\.env"
  Write-Host "Updated BOOTSTRAP_ROOT_PASSWORD in docker/.env."
}
if ($ProviderSecret) {
  Set-EnvValue "API_PROVIDER_SECRET_MASTER_KEY" $ProviderSecret ".\docker\.env"
  Write-Host "Updated API_PROVIDER_SECRET_MASTER_KEY in docker/.env."
}
if ($WebPort) {
  Set-EnvValue "WEB_PORT" $WebPort ".\docker\.env"
  Write-Host "Updated WEB_PORT in docker/.env."
}

if (-not $ShouldStart) {
  Write-Host "Docker files are ready in ./docker."
  Write-Host "No containers were started because -NoStart was used."
  Write-Host "To start later, run: cd docker && docker compose pull && docker compose up -d"
  exit 0
}

docker info *> $null
if ($LASTEXITCODE -ne 0) {
  Fail "Docker is installed but the daemon is not reachable. Start Docker and try again."
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
