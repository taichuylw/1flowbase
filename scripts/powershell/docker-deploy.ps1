param(
  [string]$DbPassword = $env:FLOWBASE_DB_PASSWORD,
  [string]$RootAccount = $env:FLOWBASE_ROOT_ACCOUNT,
  [string]$RootPassword = $env:FLOWBASE_ROOT_PASSWORD,
  [string]$ProviderSecret = $env:FLOWBASE_PROVIDER_SECRET,
  [string]$WebPort = $env:FLOWBASE_WEB_PORT,
  [string]$PluginGithubProxyUrl = $env:FLOWBASE_OFFICIAL_PLUGIN_GITHUB_PROXY_URL,
  [switch]$Pull,
  [switch]$NoPull,
  [switch]$Start,
  [switch]$NoStart,
  [switch]$PrepareOnly,
  [switch]$NonInteractive
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
$DefaultOfficialPluginGithubProxyUrl = "https://gh-proxy.com/"
$ShouldPrompt = -not ($NonInteractive -or $env:FLOWBASE_NON_INTERACTIVE -eq "1" -or $env:FLOWBASE_NON_INTERACTIVE -eq "true")
$PullImages = $null
$StartContainers = $null

if ($Pull -or $env:FLOWBASE_PULL_IMAGES -eq "1" -or $env:FLOWBASE_PULL_IMAGES -eq "true") {
  $PullImages = $true
}
if ($NoPull -or $env:FLOWBASE_PULL_IMAGES -eq "0" -or $env:FLOWBASE_PULL_IMAGES -eq "false") {
  $PullImages = $false
}
if ($Start -or $env:FLOWBASE_START_CONTAINERS -eq "1" -or $env:FLOWBASE_START_CONTAINERS -eq "true") {
  $StartContainers = $true
}
if ($NoStart -or $PrepareOnly -or $env:FLOWBASE_START_CONTAINERS -eq "0" -or $env:FLOWBASE_START_CONTAINERS -eq "false") {
  $StartContainers = $false
}
if (-not $PluginGithubProxyUrl -and $env:API_OFFICIAL_PLUGIN_GITHUB_PROXY_URL) {
  $PluginGithubProxyUrl = $env:API_OFFICIAL_PLUGIN_GITHUB_PROXY_URL
}

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

function Prompt-EnvValue([string]$Key, [string]$Label) {
  $CurrentValue = Read-EnvValue $Key ".\docker\.env"
  if ($CurrentValue) {
    $InputValue = Read-Host "$Label [$CurrentValue]"
  } else {
    $InputValue = Read-Host "$Label"
  }

  if ($InputValue) {
    Set-EnvValue $Key $InputValue ".\docker\.env"
    Write-Host "Updated $Key in docker/.env."
  } else {
    if ($CurrentValue) {
      Write-Host "Keeping ${Key}: $CurrentValue"
    } else {
      Write-Host "Keeping ${Key}: empty"
    }
  }
}

function Convert-ToYesNo([string]$Value) {
  if ($Value -match "^(y|yes|true|1)$") {
    return $true
  }
  return $false
}

function Invoke-NativeQuiet([scriptblock]$Command) {
  $PreviousErrorActionPreference = $ErrorActionPreference
  $ErrorActionPreference = "Continue"
  try {
    & $Command *> $null
    return $LASTEXITCODE -eq 0
  } finally {
    $ErrorActionPreference = $PreviousErrorActionPreference
  }
}

function Prompt-YesNo([string]$Question, [bool]$Default) {
  $Suffix = if ($Default) { "[Y/n]" } else { "[y/N]" }
  $InputValue = Read-Host "$Question $Suffix"
  if (-not $InputValue) {
    return $Default
  }
  return Convert-ToYesNo $InputValue
}

function Prompt-OfficialPluginGithubProxyUrl() {
  $CurrentValue = Read-EnvValue "API_OFFICIAL_PLUGIN_GITHUB_PROXY_URL" ".\docker\.env"
  $UseProxy = Prompt-YesNo "Use CN GitHub plugin download accelerator?" ([bool]$CurrentValue)

  if ($UseProxy) {
    $DefaultValue = if ($CurrentValue) { $CurrentValue } else { $DefaultOfficialPluginGithubProxyUrl }
    $InputValue = Read-Host "Official plugin GitHub raw proxy URL [$DefaultValue]"
    if ($InputValue) {
      Set-EnvValue "API_OFFICIAL_PLUGIN_GITHUB_PROXY_URL" $InputValue ".\docker\.env"
    } else {
      Set-EnvValue "API_OFFICIAL_PLUGIN_GITHUB_PROXY_URL" $DefaultValue ".\docker\.env"
    }
    Write-Host "Updated API_OFFICIAL_PLUGIN_GITHUB_PROXY_URL in docker/.env."
  } else {
    Set-EnvValue "API_OFFICIAL_PLUGIN_GITHUB_PROXY_URL" "" ".\docker\.env"
    Write-Host "Disabled API_OFFICIAL_PLUGIN_GITHUB_PROXY_URL in docker/.env."
  }
}

function Normalize-DockerArchitecture([string]$Architecture) {
  switch ($Architecture.ToLowerInvariant()) {
    "amd64" { return "amd64" }
    "x86_64" { return "amd64" }
    "arm64" { return "arm64" }
    "aarch64" { return "arm64" }
    "arm64/v8" { return "arm64" }
    default { return $Architecture.ToLowerInvariant() }
  }
}

function Normalize-DockerPlatform([string]$Platform) {
  $Parts = $Platform -split "/", 2
  if ($Parts.Count -eq 2) {
    $OsName = $Parts[0].ToLowerInvariant()
    $ArchName = Normalize-DockerArchitecture $Parts[1]
  } else {
    $OsName = "linux"
    $ArchName = Normalize-DockerArchitecture $Platform
  }

  return "$OsName/$ArchName"
}

function Get-EffectiveDockerPlatform() {
  if ($env:DOCKER_DEFAULT_PLATFORM) {
    return Normalize-DockerPlatform $env:DOCKER_DEFAULT_PLATFORM
  }

  $RawPlatform = docker info --format "{{.OSType}}/{{.Architecture}}" 2>$null
  if ($LASTEXITCODE -ne 0 -or -not $RawPlatform) {
    Fail "Could not detect Docker server platform."
  }

  return Normalize-DockerPlatform $RawPlatform
}

function Get-EnvOrFileValue([string]$Key, [string]$Path, [string]$DefaultValue) {
  $EnvValue = [Environment]::GetEnvironmentVariable($Key)
  if ($EnvValue) {
    return $EnvValue
  }

  $FileValue = Read-EnvValue $Key $Path
  if ($FileValue) {
    return $FileValue
  }

  return $DefaultValue
}

function Get-FlowbaseImageRefs([string]$Path) {
  $WebVersion = Get-EnvOrFileValue "FLOWBASE_WEB_VERSION" $Path "latest"
  $ApiVersion = Get-EnvOrFileValue "FLOWBASE_API_SERVER_VERSION" $Path "latest"
  $PluginRunnerVersion = Get-EnvOrFileValue "FLOWBASE_PLUGIN_RUNNER_VERSION" $Path "latest"

  return @(
    "ghcr.io/taichuy/1flowbase-web:$WebVersion",
    "ghcr.io/taichuy/1flowbase-api-server:$ApiVersion",
    "ghcr.io/taichuy/1flowbase-plugin-runner:$PluginRunnerVersion"
  )
}

function Test-FlowbaseUsesLatestImageTags([string]$Path) {
  return (
    (Get-EnvOrFileValue "FLOWBASE_WEB_VERSION" $Path "latest") -eq "latest" -and
    (Get-EnvOrFileValue "FLOWBASE_API_SERVER_VERSION" $Path "latest") -eq "latest" -and
    (Get-EnvOrFileValue "FLOWBASE_PLUGIN_RUNNER_VERSION" $Path "latest") -eq "latest"
  )
}

function Test-LocalLatestFlowbaseImages([string]$Path) {
  if (-not (Test-FlowbaseUsesLatestImageTags $Path)) {
    return $false
  }

  foreach ($Image in Get-FlowbaseImageRefs $Path) {
    if (-not (Invoke-NativeQuiet { docker image inspect $Image })) {
      return $false
    }
  }

  return $true
}

function Test-ImageManifestPlatform([string]$Image, [string]$Platform) {
  $ManifestLines = docker manifest inspect $Image 2>$null
  if ($LASTEXITCODE -ne 0 -or -not $ManifestLines) {
    return $null
  }

  $Manifest = $ManifestLines -join "`n"
  $Parts = $Platform -split "/", 2
  $OsName = [regex]::Escape($Parts[0])
  $ArchName = [regex]::Escape($Parts[1])

  return (
    $Manifest -match "`"os`"\s*:\s*`"$OsName`"" -and
    $Manifest -match "`"architecture`"\s*:\s*`"$ArchName`""
  )
}

function Assert-FlowbaseImagePlatformSupport() {
  $Platform = Get-EffectiveDockerPlatform
  Write-Host "Detected Docker platform: $Platform"

  if ($Platform -ne "linux/amd64" -and $Platform -ne "linux/arm64") {
    Fail "This 1flowbase Docker package supports linux/amd64 and linux/arm64. Detected Docker platform: $Platform."
  }

  foreach ($Image in Get-FlowbaseImageRefs ".\.env") {
    $SupportsPlatform = Test-ImageManifestPlatform $Image $Platform
    if ($null -eq $SupportsPlatform) {
      Write-Host "Could not verify Docker image platform support for $Image; continuing to Docker pull."
    } elseif (-not $SupportsPlatform) {
      Fail "Docker image $Image does not publish $Platform. Rebuild/publish the 1flowbase multi-platform images, or set DOCKER_DEFAULT_PLATFORM=linux/amd64 as a temporary workaround on ARM machines."
    }
  }
}

if (-not (Get-Command docker -ErrorAction SilentlyContinue)) {
  Fail "Docker is required. Install Docker Desktop or Docker Engine first."
}

$UseDockerComposePlugin = $false
if (Invoke-NativeQuiet { docker compose version }) {
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

$PromptConfigValues = $false
if (-not (Test-Path ".\docker\.env")) {
  Copy-Item ".\docker\.env.example" ".\docker\.env"
  Write-Host "Created docker/.env from docker/.env.example."
  $PromptConfigValues = $true
} else {
  Write-Host "Using existing docker/.env."
  if ($ShouldPrompt) {
    $OverwriteEnv = Prompt-YesNo "Overwrite current docker/.env from docker/.env.example?" $false
    if ($OverwriteEnv) {
      Copy-Item ".\docker\.env.example" ".\docker\.env" -Force
      Write-Host "Overwrote docker/.env from docker/.env.example."
      $PromptConfigValues = $true
    } else {
      Write-Host "Keeping existing docker/.env."
    }
  }
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
if ($PluginGithubProxyUrl) {
  Set-EnvValue "API_OFFICIAL_PLUGIN_GITHUB_PROXY_URL" $PluginGithubProxyUrl ".\docker\.env"
  Write-Host "Updated API_OFFICIAL_PLUGIN_GITHUB_PROXY_URL in docker/.env."
}

if ($ShouldPrompt -and $PromptConfigValues) {
  Write-Host "Configure docker/.env. Press Enter to keep the value shown in brackets."
  Prompt-EnvValue "POSTGRES_PASSWORD" "Database password"
  Prompt-EnvValue "BOOTSTRAP_ROOT_ACCOUNT" "Root account"
  Prompt-EnvValue "BOOTSTRAP_ROOT_PASSWORD" "Root password"
  Prompt-EnvValue "API_PROVIDER_SECRET_MASTER_KEY" "API provider secret master key"
  Prompt-EnvValue "WEB_PORT" "Web port"
  Prompt-OfficialPluginGithubProxyUrl
}

if ($null -eq $PullImages) {
  if ($ShouldPrompt) {
    if (Test-LocalLatestFlowbaseImages ".\docker\.env") {
      $PullImages = Prompt-YesNo "Local latest Docker images were found. Update Docker images?" $false
    } else {
      $PullImages = Prompt-YesNo "Pull Docker images?" $false
    }
  } else {
    $PullImages = $false
  }
}

if ($null -eq $StartContainers) {
  if ($ShouldPrompt) {
    $StartContainers = Prompt-YesNo "Start 1flowbase now?" $false
  } else {
    $StartContainers = $false
  }
}

if (-not $PullImages -and -not $StartContainers) {
  Write-Host "Docker files are ready in ./docker."
  Write-Host "No images were pulled and no containers were started."
  Write-Host "To start later, run: cd docker && docker compose pull && docker compose up -d"
  exit 0
}

if (-not (Invoke-NativeQuiet { docker info })) {
  Fail "Docker is installed but the daemon is not reachable. Start Docker and try again."
}

Set-Location ".\docker"
Assert-FlowbaseImagePlatformSupport

if ($PullImages) {
  if ($UseDockerComposePlugin) {
    docker compose pull
  } else {
    docker-compose pull
  }
} else {
  Write-Host "Skipping image pull."
}

if ($StartContainers) {
  if ($UseDockerComposePlugin) {
    docker compose up -d
  } else {
    docker-compose up -d
  }
} else {
  Write-Host "Skipping container startup."
  Write-Host "To start later, run: cd docker && docker compose up -d"
  exit 0
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
