const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');

const { buildCommands, main } = require('../../verify-repo.js');

test('buildCommands composes hygiene, i18n hygiene, script tests, contract tests, frontend full gate and backend verify gate', () => {
  const repoRoot = '/repo-root';

  assert.deepEqual(buildCommands({ repoRoot }), [
    {
      label: 'repo-hygiene',
      command: process.execPath,
      args: [path.join(repoRoot, 'scripts', 'node', 'tooling.js'), 'repo-hygiene'],
      cwd: repoRoot,
    },
    {
      label: 'repo-i18n-hygiene',
      command: process.execPath,
      args: [path.join(repoRoot, 'scripts', 'node', 'tooling.js'), 'i18n-hygiene'],
      cwd: repoRoot,
    },
    {
      label: 'repo-script-tests',
      command: process.execPath,
      args: [path.join(repoRoot, 'scripts', 'node', 'test.js'), 'scripts'],
      cwd: repoRoot,
    },
    {
      label: 'repo-contract-tests',
      command: process.execPath,
      args: [path.join(repoRoot, 'scripts', 'node', 'test.js'), 'contracts'],
      cwd: repoRoot,
    },
    {
      label: 'repo-frontend-full',
      command: process.execPath,
      args: [path.join(repoRoot, 'scripts', 'node', 'test.js'), 'frontend', 'full'],
      cwd: repoRoot,
    },
    {
      label: 'repo-backend-full',
      command: process.execPath,
      args: [path.join(repoRoot, 'scripts', 'node', 'verify.js'), 'backend'],
      cwd: repoRoot,
    },
  ]);
});

test('buildCommands can select repository gate slices for parallel CI jobs', () => {
  const repoRoot = '/repo-root';

  assert.deepEqual(
    buildCommands({ repoRoot, target: 'tooling' }).map((command) => command.label),
    ['repo-hygiene', 'repo-i18n-hygiene', 'repo-script-tests', 'repo-contract-tests']
  );
  assert.deepEqual(
    buildCommands({ repoRoot, target: 'frontend' }).map((command) => command.label),
    ['repo-frontend-full']
  );
  assert.deepEqual(
    buildCommands({ repoRoot, target: 'backend' }).map((command) => command.label),
    ['repo-backend-full']
  );
});

test('main runs repository full gate in order and captures advisory output', async () => {
  const repoRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'oneflowbase-verify-repo-'));
  const calls = [];

  const status = await main([], {
    repoRoot,
    env: {},
    writeStdout() {},
    writeStderr() {},
    spawnSyncImpl(command, args, options) {
      calls.push({ command, args, options });

      return {
        status: 0,
        stdout: '',
        stderr: `warning: ${args.slice(0, 2).join('/')} advisory\n`,
      };
    },
  });

  assert.equal(status, 0);
  assert.equal(calls.length, 6);
  assert.deepEqual(
    calls.map((call) => call.args),
    [
      [path.join(repoRoot, 'scripts', 'node', 'tooling.js'), 'repo-hygiene'],
      [path.join(repoRoot, 'scripts', 'node', 'tooling.js'), 'i18n-hygiene'],
      [path.join(repoRoot, 'scripts', 'node', 'test.js'), 'scripts'],
      [path.join(repoRoot, 'scripts', 'node', 'test.js'), 'contracts'],
      [path.join(repoRoot, 'scripts', 'node', 'test.js'), 'frontend', 'full'],
      [path.join(repoRoot, 'scripts', 'node', 'verify.js'), 'backend'],
    ]
  );

  const warningLogPath = path.join(repoRoot, 'tmp', 'test-governance', 'verify-repo.warnings.log');
  assert.equal(fs.existsSync(warningLogPath), true);
  const warningLog = fs.readFileSync(warningLogPath, 'utf8');
  assert.match(warningLog, /warning: .*tooling\.js\/repo-hygiene advisory/u);
  assert.match(warningLog, /warning: .*tooling\.js\/i18n-hygiene advisory/u);
  assert.match(warningLog, /warning: .*test\.js\/scripts advisory/u);
  assert.match(warningLog, /warning: .*test\.js\/contracts advisory/u);
  assert.match(warningLog, /warning: .*test\.js\/frontend advisory/u);
  assert.match(warningLog, /warning: .*verify\.js\/backend advisory/u);
});

test('main runs only the requested repository gate slice', async () => {
  const repoRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'oneflowbase-verify-repo-slice-'));
  const calls = [];

  const status = await main(['frontend'], {
    repoRoot,
    env: {},
    writeStdout() {},
    writeStderr() {},
    spawnSyncImpl(command, args, options) {
      calls.push({ command, args, options });
      return { status: 0, stdout: '', stderr: '' };
    },
  });

  assert.equal(status, 0);
  assert.equal(calls.length, 1);
  assert.deepEqual(calls[0].args, [path.join(repoRoot, 'scripts', 'node', 'test.js'), 'frontend', 'full']);
});

test('main passes the inherited lock token through every repository gate command', async () => {
  const repoRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'oneflowbase-verify-repo-'));
  const calls = [];

  const status = await main([], {
    repoRoot,
    env: { ONEFLOWBASE_VERIFY_LOCK_TOKEN: 'chain-token' },
    writeStdout() {},
    writeStderr() {},
    spawnSyncImpl(command, args, options) {
      calls.push({ command, args, options });
      return { status: 0, stdout: '', stderr: '' };
    },
  });

  assert.equal(status, 0);
  assert.equal(calls.length, 6);
  assert.equal(calls[0].options.env.ONEFLOWBASE_VERIFY_LOCK_TOKEN, 'chain-token');
  assert.equal(calls[1].options.env.ONEFLOWBASE_VERIFY_LOCK_TOKEN, 'chain-token');
  assert.equal(calls[2].options.env.ONEFLOWBASE_VERIFY_LOCK_TOKEN, 'chain-token');
  assert.equal(calls[3].options.env.ONEFLOWBASE_VERIFY_LOCK_TOKEN, 'chain-token');
  assert.equal(calls[4].options.env.ONEFLOWBASE_VERIFY_LOCK_TOKEN, 'chain-token');
  assert.equal(calls[5].options.env.ONEFLOWBASE_VERIFY_LOCK_TOKEN, 'chain-token');
});

test('main routes the repository gate through the heavy managed runner', async () => {
  let capturedOptions = null;

  const status = await main([], {
    repoRoot: '/repo-root',
    env: {},
    managedRunnerImpl(options) {
      capturedOptions = options;
      return 0;
    },
  });

  assert.equal(status, 0);
  assert.equal(capturedOptions.scope, 'verify-repo');
  assert.equal(capturedOptions.lockMode, 'heavy');
  assert.equal(capturedOptions.commandDisplay, 'node scripts/node/verify-repo.js');
});
