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
    assert.match(script, /does not publish/u);
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
