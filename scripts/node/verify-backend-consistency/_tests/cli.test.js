const test = require('node:test');
const assert = require('node:assert/strict');

const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');

const {
  buildCommands,
  main,
  runBackendConsistencyCommandSequence,
} = require('../../verify-backend-consistency.js');

test('buildCommands targets backend consistency suites without workspace-wide reruns', () => {
  const commands = buildCommands({ cargoJobs: 4, cargoTestThreads: 1 });

  assert.deepEqual(
    commands.map((command) => [command.label, command.args[2], command.args[5]]),
    [
      ['consistency-control-plane-state-transitions', 'control-plane', 'state_transition_tests'],
      ['consistency-control-plane-workspace-session', 'control-plane', 'workspace_session'],
      ['consistency-control-plane-model-definition-service', 'control-plane', 'model_definition_service_tests'],
      ['consistency-control-plane-model-definition-runtime-sync', 'control-plane', 'model_definition_runtime_sync_tests'],
      ['consistency-control-plane-resource-action-kernel', 'control-plane', 'resource_action_tests'],
      ['consistency-runtime-acl', 'runtime-core', 'runtime_acl_tests'],
      ['consistency-runtime-engine', 'runtime-core', 'runtime_engine_tests'],
      ['consistency-storage-migration-smoke', 'storage-postgres', 'migration_smoke'],
      ['consistency-storage-model-definition-repository', 'storage-postgres', 'model_definition_repository_tests'],
      ['consistency-storage-runtime-record-repository', 'storage-postgres', 'runtime_record_repository_tests'],
      ['consistency-storage-orchestration-runtime-repository', 'storage-postgres', 'orchestration_runtime_repository_tests'],
      ['consistency-storage-physical-schema-repository', 'storage-postgres', 'physical_schema_repository_tests'],
      ['consistency-storage-workspace-scope', 'storage-postgres', 'workspace_scope_tests'],
      ['consistency-api-model-definition-routes', 'api-server', 'model_definition_routes'],
      ['consistency-api-runtime-model-routes', 'api-server', 'runtime_model_routes'],
      ['consistency-api-workspace-routes', 'api-server', 'workspace_routes'],
      ['consistency-api-file-management-routes', 'api-server', 'file_management_routes'],
    ]
  );

  for (const command of commands) {
    assert.equal(command.command, 'cargo');
    assert.equal(command.cwd, 'api');
    assert.deepEqual(command.env, {
      CARGO_BUILD_JOBS: '4',
      CARGO_INCREMENTAL: '0',
    });
    assert.deepEqual(command.args.slice(0, 2), ['test', '-p']);
    assert.deepEqual(command.args.slice(3, 5), ['--jobs', '4']);
    assert.deepEqual(command.args.slice(6), ['--', '--test-threads=1']);
  }
});

test('main routes backend consistency through the heavy managed gate', async () => {
  let capturedOptions = null;

  const status = await main([], {
    repoRoot: '/repo-root',
    env: {},
    runtimeConfig: {
      backend: {
        cargoJobs: 2,
        cargoTestThreads: 1,
      },
      locks: {
        waitTimeoutMinutes: 30,
        waitTimeoutMs: 30 * 60 * 1000,
        pollIntervalMs: 5000,
      },
    },
    managedRunnerImpl(options) {
      capturedOptions = options;
      return 0;
    },
  });

  assert.equal(status, 0);
  assert.equal(capturedOptions.scope, 'verify-backend-consistency');
  assert.equal(capturedOptions.lockMode, 'heavy');
  assert.equal(capturedOptions.commandDisplay, 'node scripts/node/verify-backend-consistency.js');
  assert.deepEqual(
    capturedOptions.commands,
    buildCommands({ cargoJobs: 2, cargoTestThreads: 1 })
  );
});

test('runBackendConsistencyCommandSequence writes per-target result evidence', () => {
  const repoRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'oneflowbase-backend-consistency-results-'));
  let now = 1000;
  const commands = buildCommands({ cargoJobs: 2, cargoTestThreads: 1 }).slice(0, 2);

  const status = runBackendConsistencyCommandSequence({
    repoRoot,
    env: {},
    scope: 'verify-backend-consistency',
    commands,
    nowImpl: () => {
      now += 1250;
      return now;
    },
    spawnSyncImpl() {
      return {
        status: 0,
        stdout: 'test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s\n',
        stderr: '',
      };
    },
    writeStdout() {},
    writeStderr() {},
  });

  assert.equal(status, 0);

  const reportPath = path.join(
    repoRoot,
    'tmp',
    'test-governance',
    'backend-consistency-targets.json'
  );
  const report = JSON.parse(fs.readFileSync(reportPath, 'utf8'));

  assert.deepEqual(report.targets.map((target) => ({
    label: target.label,
    status: target.status,
    durationMs: target.durationMs,
    passedCount: target.passedCount,
    failedCount: target.failedCount,
  })), [
    {
      label: 'consistency-control-plane-state-transitions',
      status: 'passed',
      durationMs: 1250,
      passedCount: 3,
      failedCount: 0,
    },
    {
      label: 'consistency-control-plane-workspace-session',
      status: 'passed',
      durationMs: 1250,
      passedCount: 3,
      failedCount: 0,
    },
  ]);
});

test('runBackendConsistencyCommandSequence records failed and skipped target states', () => {
  const repoRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'oneflowbase-backend-consistency-failure-'));
  const commands = buildCommands({ cargoJobs: 2, cargoTestThreads: 1 }).slice(0, 3);
  let callCount = 0;

  const status = runBackendConsistencyCommandSequence({
    repoRoot,
    env: {},
    scope: 'verify-backend-consistency',
    commands,
    nowImpl: () => 0,
    spawnSyncImpl() {
      callCount += 1;

      if (callCount === 1) {
        return {
          status: 0,
          stdout: 'test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s\n',
          stderr: '',
        };
      }

      return {
        status: 101,
        stdout: 'test result: FAILED. 1 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s\n',
        stderr: 'error: test failed\n',
      };
    },
    writeStdout() {},
    writeStderr() {},
  });

  assert.equal(status, 101);
  assert.equal(callCount, 2);

  const report = JSON.parse(fs.readFileSync(path.join(
    repoRoot,
    'tmp',
    'test-governance',
    'backend-consistency-targets.json'
  ), 'utf8'));

  assert.deepEqual(report.targets.map((target) => ({
    status: target.status,
    passedCount: target.passedCount,
    failedCount: target.failedCount,
  })), [
    { status: 'passed', passedCount: 2, failedCount: 0 },
    { status: 'failed', passedCount: 1, failedCount: 1 },
    { status: 'skipped', passedCount: null, failedCount: null },
  ]);
});
