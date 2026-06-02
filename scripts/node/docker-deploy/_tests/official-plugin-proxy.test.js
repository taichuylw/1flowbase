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
  }

  assert.match(shellScript, /--plugin-github-proxy-url VALUE/u);
  assert.match(powershellScript, /\$PluginGithubProxyUrl/u);
});
