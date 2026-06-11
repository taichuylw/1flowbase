const test = require('node:test');
const assert = require('node:assert/strict');
const path = require('node:path');
const { spawnSync } = require('node:child_process');

const {
  parseCliArgs,
  runMergeCurrentToMainLatest,
} = require('../core.js');

const rootCliEntry = path.resolve(__dirname, '..', '..', 'merge-current-to-main-latest.js');

function createSpawnSyncMock(responses = []) {
  const calls = [];

  function spawnSync(command, args, options) {
    calls.push({ command, args, options });
    const response = responses.shift() || {};

    return {
      status: response.status ?? 0,
      stdout: response.stdout ?? '',
      stderr: response.stderr ?? '',
      error: response.error,
    };
  }

  return {
    calls,
    spawnSync,
  };
}

function commandArgs(calls) {
  return calls.map((call) => call.args);
}

test('parseCliArgs uses origin, main, and latest by default', () => {
  assert.deepEqual(parseCliArgs([]), {
    allowDirty: false,
    help: false,
    latestBranch: 'latest',
    mainBranch: 'main',
    remote: 'origin',
  });
});

test('root script entry keeps the documented command available', () => {
  const result = spawnSync(process.execPath, [rootCliEntry, '--help'], {
    encoding: 'utf8',
  });

  assert.equal(result.status, 0);
  assert.match(result.stdout, /scripts\/node\/merge-current-to-main-latest\.js/u);
  assert.match(result.stdout, /scripts\/node\/cli\/merge-current-to-main-latest\.js/u);
});

test('runMergeCurrentToMainLatest merges the current branch into main, then main into latest', () => {
  const mock = createSpawnSyncMock([
    { stdout: 'feature/shipping\n' },
    { stdout: '' },
  ]);
  const output = [];

  const status = runMergeCurrentToMainLatest({
    options: parseCliArgs([]),
    repoRoot: '/repo',
    spawnSyncImpl: mock.spawnSync,
    writeStdout: (text) => output.push(text),
    writeStderr: () => {},
  });

  assert.equal(status, 0);
  assert.deepEqual(commandArgs(mock.calls), [
    ['branch', '--show-current'],
    ['status', '--porcelain'],
    ['fetch', 'origin', 'main', 'latest'],
    ['switch', 'main'],
    ['pull', '--ff-only', 'origin', 'main'],
    ['merge', '--no-edit', 'feature/shipping'],
    ['push', 'origin', 'main'],
    ['switch', 'latest'],
    ['pull', '--ff-only', 'origin', 'latest'],
    ['merge', '--no-edit', 'main'],
    ['push', 'origin', 'latest'],
  ]);
  assert.match(output.join(''), /done/u);
});

test('runMergeCurrentToMainLatest stops before switching branches when the worktree is dirty', () => {
  const mock = createSpawnSyncMock([
    { stdout: 'feature/shipping\n' },
    { stdout: ' M web/app/src/App.tsx\n' },
  ]);

  const status = runMergeCurrentToMainLatest({
    options: parseCliArgs([]),
    repoRoot: '/repo',
    spawnSyncImpl: mock.spawnSync,
    writeStdout: () => {},
    writeStderr: () => {},
  });

  assert.equal(status, 1);
  assert.deepEqual(commandArgs(mock.calls), [
    ['branch', '--show-current'],
    ['status', '--porcelain'],
  ]);
});

test('runMergeCurrentToMainLatest stops immediately when the main merge fails', () => {
  const mock = createSpawnSyncMock([
    { stdout: 'feature/shipping\n' },
    { stdout: '' },
    {},
    {},
    {},
    { status: 1, stderr: 'CONFLICT\n' },
  ]);

  const status = runMergeCurrentToMainLatest({
    options: parseCliArgs([]),
    repoRoot: '/repo',
    spawnSyncImpl: mock.spawnSync,
    writeStdout: () => {},
    writeStderr: () => {},
  });

  assert.equal(status, 1);
  assert.deepEqual(commandArgs(mock.calls), [
    ['branch', '--show-current'],
    ['status', '--porcelain'],
    ['fetch', 'origin', 'main', 'latest'],
    ['switch', 'main'],
    ['pull', '--ff-only', 'origin', 'main'],
    ['merge', '--no-edit', 'feature/shipping'],
  ]);
});
