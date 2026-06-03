const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { spawnSync } = require('node:child_process');

const repoRoot = path.resolve(__dirname, '..', '..', '..', '..');

function readRepoFile(...segments) {
  return fs.readFileSync(path.join(repoRoot, ...segments), 'utf8');
}

function makeExecutable(filePath, content) {
  fs.writeFileSync(filePath, content, { mode: 0o755 });
}

function shellQuote(value) {
  return `'${String(value).replace(/'/gu, "'\\''")}'`;
}

function runInteractiveShellDeploy({ tempRoot, tempBin, input }) {
  return spawnSync(
    'script',
    [
      '-qfec',
      `sh ${shellQuote(path.join(repoRoot, 'scripts', 'shell', 'docker-deploy.sh'))}`,
      '/dev/null',
    ],
    {
      cwd: tempRoot,
      env: {
        ...process.env,
        PATH: `${tempBin}${path.delimiter}${process.env.PATH || ''}`,
      },
      input,
      encoding: 'utf8',
    },
  );
}

test('docker deploy shell script can prefill official plugin GitHub proxy URL', () => {
  const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'oneflowbase-docker-deploy-'));
  const tempBin = path.join(tempRoot, 'bin');
  const dockerDir = path.join(tempRoot, 'docker');
  fs.mkdirSync(tempBin);
  fs.mkdirSync(dockerDir);
  fs.writeFileSync(
    path.join(dockerDir, '.env.example'),
    'POSTGRES_PASSWORD=change-me\nAPI_OFFICIAL_PLUGIN_GITHUB_PROXY_URL=\n',
  );
  makeExecutable(
    path.join(tempBin, 'docker'),
    '#!/usr/bin/env sh\nif [ "$1 $2 $3" = "compose version " ]; then exit 0; fi\nexit 0\n',
  );

  const result = spawnSync(
    'sh',
    [
      path.join(repoRoot, 'scripts', 'shell', 'docker-deploy.sh'),
      '--non-interactive',
      '--plugin-github-proxy-url',
      'https://proxy.example/',
    ],
    {
      cwd: tempRoot,
      env: {
        ...process.env,
        PATH: `${tempBin}${path.delimiter}${process.env.PATH || ''}`,
      },
      encoding: 'utf8',
    },
  );

  assert.equal(result.status, 0, `${result.stdout}\n${result.stderr}`);
  assert.match(
    fs.readFileSync(path.join(dockerDir, '.env'), 'utf8'),
    /^API_OFFICIAL_PLUGIN_GITHUB_PROXY_URL=https:\/\/proxy\.example\/$/mu,
  );
});

test('docker deploy shell script asks before overwriting an existing env file', () => {
  const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'oneflowbase-docker-deploy-'));
  const tempBin = path.join(tempRoot, 'bin');
  const dockerDir = path.join(tempRoot, 'docker');
  fs.mkdirSync(tempBin);
  fs.mkdirSync(dockerDir);
  fs.writeFileSync(
    path.join(dockerDir, '.env.example'),
    [
      'FLOWBASE_WEB_VERSION=latest',
      'FLOWBASE_API_SERVER_VERSION=latest',
      'FLOWBASE_PLUGIN_RUNNER_VERSION=latest',
      'POSTGRES_PASSWORD=example-password',
      'BOOTSTRAP_ROOT_ACCOUNT=example-root',
      'BOOTSTRAP_ROOT_PASSWORD=example-root-password',
      'API_PROVIDER_SECRET_MASTER_KEY=example-secret',
      'WEB_PORT=4100',
      'API_OFFICIAL_PLUGIN_GITHUB_PROXY_URL=',
      '',
    ].join('\n'),
  );
  fs.writeFileSync(
    path.join(dockerDir, '.env'),
    [
      'FLOWBASE_WEB_VERSION=latest',
      'FLOWBASE_API_SERVER_VERSION=latest',
      'FLOWBASE_PLUGIN_RUNNER_VERSION=latest',
      'POSTGRES_PASSWORD=old-password',
      'BOOTSTRAP_ROOT_ACCOUNT=old-root',
      'BOOTSTRAP_ROOT_PASSWORD=old-root-password',
      'API_PROVIDER_SECRET_MASTER_KEY=old-secret',
      'WEB_PORT=3000',
      'API_OFFICIAL_PLUGIN_GITHUB_PROXY_URL=https://old.example/',
      '',
    ].join('\n'),
  );
  makeExecutable(
    path.join(tempBin, 'docker'),
    `#!/usr/bin/env sh
if [ "$1 $2" = "compose version" ]; then exit 0; fi
if [ "$1 $2" = "image inspect" ]; then exit 1; fi
exit 0
`,
  );

  const keepResult = runInteractiveShellDeploy({
    tempRoot,
    tempBin,
    input: `n\n${'\n'.repeat(12)}`,
  });
  assert.equal(keepResult.status, 0, `${keepResult.stdout}\n${keepResult.stderr}`);
  assert.match(keepResult.stdout, /Overwrite current docker\/.env from docker\/.env.example\?/u);
  assert.match(fs.readFileSync(path.join(dockerDir, '.env'), 'utf8'), /^POSTGRES_PASSWORD=old-password$/mu);

  const overwriteResult = runInteractiveShellDeploy({
    tempRoot,
    tempBin,
    input: `y\n${'\n'.repeat(12)}`,
  });
  assert.equal(overwriteResult.status, 0, `${overwriteResult.stdout}\n${overwriteResult.stderr}`);
  assert.match(
    fs.readFileSync(path.join(dockerDir, '.env'), 'utf8'),
    /^POSTGRES_PASSWORD=example-password$/mu,
  );
  assert.match(fs.readFileSync(path.join(dockerDir, '.env'), 'utf8'), /^WEB_PORT=4100$/mu);
  assert.doesNotMatch(fs.readFileSync(path.join(dockerDir, '.env'), 'utf8'), /old-password/u);
});

test('docker deploy shell script asks before updating local latest images', () => {
  const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'oneflowbase-docker-deploy-'));
  const tempBin = path.join(tempRoot, 'bin');
  const dockerDir = path.join(tempRoot, 'docker');
  fs.mkdirSync(tempBin);
  fs.mkdirSync(dockerDir);
  fs.writeFileSync(
    path.join(dockerDir, '.env.example'),
    [
      'FLOWBASE_WEB_VERSION=latest',
      'FLOWBASE_API_SERVER_VERSION=latest',
      'FLOWBASE_PLUGIN_RUNNER_VERSION=latest',
      'POSTGRES_PASSWORD=example-password',
      'API_OFFICIAL_PLUGIN_GITHUB_PROXY_URL=',
      '',
    ].join('\n'),
  );
  fs.writeFileSync(
    path.join(dockerDir, '.env'),
    [
      'FLOWBASE_WEB_VERSION=latest',
      'FLOWBASE_API_SERVER_VERSION=latest',
      'FLOWBASE_PLUGIN_RUNNER_VERSION=latest',
      'POSTGRES_PASSWORD=old-password',
      'API_OFFICIAL_PLUGIN_GITHUB_PROXY_URL=',
      '',
    ].join('\n'),
  );
  makeExecutable(
    path.join(tempBin, 'docker'),
    `#!/usr/bin/env sh
if [ "$1 $2" = "compose version" ]; then exit 0; fi
if [ "$1 $2" = "image inspect" ]; then exit 0; fi
if [ "$1 $2" = "compose pull" ]; then
  printf '%s\\n' 'compose pull ran'
  exit 0
fi
if [ "$1" = "info" ]; then
  if [ "$2" = "--format" ]; then
    printf '%s\\n' 'linux/amd64'
  fi
  exit 0
fi
if [ "$1 $2" = "manifest inspect" ]; then
  cat <<'EOF'
{
  "schemaVersion": 2,
  "manifests": [
    { "platform": { "architecture": "amd64", "os": "linux" } }
  ]
}
EOF
  exit 0
fi
exit 0
`,
  );

  const result = runInteractiveShellDeploy({
    tempRoot,
    tempBin,
    input: `n\nn\nn\n${'\n'.repeat(12)}`,
  });

  assert.equal(result.status, 0, `${result.stdout}\n${result.stderr}`);
  assert.match(result.stdout, /Local latest Docker images were found\. Update Docker images\?/u);
  assert.doesNotMatch(result.stdout, /Pull Docker images\?/u);
  assert.doesNotMatch(result.stdout, /compose pull ran/u);

  const updateResult = runInteractiveShellDeploy({
    tempRoot,
    tempBin,
    input: `n\ny\nn\n${'\n'.repeat(12)}`,
  });

  assert.equal(updateResult.status, 0, `${updateResult.stdout}\n${updateResult.stderr}`);
  assert.match(updateResult.stdout, /Local latest Docker images were found\. Update Docker images\?/u);
  assert.match(updateResult.stdout, /compose pull ran/u);
});

test('docker compose and env example expose an empty official plugin GitHub proxy URL by default', () => {
  const compose = readRepoFile('docker', 'docker-compose.yaml');
  const envExample = readRepoFile('docker', '.env.example');

  assert.match(
    compose,
    /^\s+API_OFFICIAL_PLUGIN_GITHUB_PROXY_URL: \$\{API_OFFICIAL_PLUGIN_GITHUB_PROXY_URL:-\}$/mu,
  );
  assert.doesNotMatch(compose, /^\s+# API_OFFICIAL_PLUGIN_GITHUB_PROXY_URL:/mu);
  assert.match(envExample, /^API_OFFICIAL_PLUGIN_GITHUB_PROXY_URL=$/mu);
});

test('docker deploy scripts document the CN accelerator prompt and default proxy URL', () => {
  const shellScript = readRepoFile('scripts', 'shell', 'docker-deploy.sh');
  const powershellScript = readRepoFile('scripts', 'powershell', 'docker-deploy.ps1');

  for (const script of [shellScript, powershellScript]) {
    assert.match(script, /Use CN GitHub plugin download accelerator\?/u);
    assert.match(script, /https:\/\/gh-proxy\.com\//u);
    assert.match(script, /API_OFFICIAL_PLUGIN_GITHUB_PROXY_URL/u);
    assert.match(script, /DOCKER_DEFAULT_PLATFORM/u);
    assert.match(script, /linux\/amd64/u);
    assert.match(script, /linux\/arm64/u);
    assert.match(script, /docker manifest inspect/u);
    assert.match(script, /docker image inspect/u);
    assert.match(script, /does not publish/u);
    assert.match(script, /Overwrite current docker\/.env from docker\/.env.example\?/u);
    assert.match(script, /Local latest Docker images were found\. Update Docker images\?/u);
  }

  assert.match(shellScript, /--plugin-github-proxy-url VALUE/u);
  assert.match(powershellScript, /\$PluginGithubProxyUrl/u);
});

test('container image workflow publishes linux amd64 and arm64 manifests', () => {
  const workflow = readRepoFile('.github', 'workflows', 'container-images.yml');

  assert.match(workflow, /docker\/setup-qemu-action@v3/u);
  assert.match(workflow, /^\s+platforms:\s+linux\/amd64,linux\/arm64$/mu);
});

test('docker deploy shell script stops before pull when the image tag lacks the detected platform', () => {
  const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'oneflowbase-docker-deploy-'));
  const tempBin = path.join(tempRoot, 'bin');
  const dockerDir = path.join(tempRoot, 'docker');
  fs.mkdirSync(tempBin);
  fs.mkdirSync(dockerDir);
  fs.writeFileSync(
    path.join(dockerDir, '.env.example'),
    [
      'FLOWBASE_WEB_VERSION=latest',
      'FLOWBASE_API_SERVER_VERSION=latest',
      'FLOWBASE_PLUGIN_RUNNER_VERSION=latest',
      'POSTGRES_PASSWORD=change-me',
      'API_OFFICIAL_PLUGIN_GITHUB_PROXY_URL=',
      '',
    ].join('\n'),
  );
  makeExecutable(
    path.join(tempBin, 'docker'),
    `#!/usr/bin/env sh
if [ "$1 $2" = "compose version" ]; then exit 0; fi
if [ "$1" = "info" ]; then
  if [ "$2" = "--format" ]; then
    printf '%s\\n' 'linux/aarch64'
  fi
  exit 0
fi
if [ "$1 $2" = "manifest inspect" ]; then
  cat <<'EOF'
{
  "schemaVersion": 2,
  "manifests": [
    { "platform": { "architecture": "amd64", "os": "linux" } }
  ]
}
EOF
  exit 0
fi
if [ "$1 $2" = "compose pull" ]; then
  printf '%s\\n' 'compose pull should not run'
  exit 42
fi
exit 0
`,
  );

  const result = spawnSync(
    'sh',
    [
      path.join(repoRoot, 'scripts', 'shell', 'docker-deploy.sh'),
      '--non-interactive',
      '--pull',
    ],
    {
      cwd: tempRoot,
      env: {
        ...process.env,
        PATH: `${tempBin}${path.delimiter}${process.env.PATH || ''}`,
      },
      encoding: 'utf8',
    },
  );

  assert.notEqual(result.status, 0, `${result.stdout}\n${result.stderr}`);
  assert.match(result.stderr, /does not publish linux\/arm64/u);
  assert.doesNotMatch(result.stdout, /compose pull should not run/u);
});
