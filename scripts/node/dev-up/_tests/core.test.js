const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('path');

const {
  DEFAULT_STARTUP_TIMEOUT_MS,
  parseCliArgs,
  shouldManageDocker,
  selectServiceKeys,
  getServiceDefinitions,
  listPortOccupantPids,
  manageDocker,
  startService,
  manageServices,
  ensureServiceEnvFile,
  buildServiceEnv,
  getServicePrestartCommands,
  parseWindowsNetstatPortOccupants,
  resolveCommandPath,
  runServicePrestartCommands,
  resolveComposeCommand,
  stopService,
  waitForPortToClose,
  waitForServicePort,
} = require('../core.js');

test('parseCliArgs defaults to full start', () => {
  assert.deepEqual(parseCliArgs([]), {
    action: 'start',
    scope: 'all',
    skipDocker: false,
    help: false,
  });
});

test('parseCliArgs supports backend restart without docker', () => {
  assert.deepEqual(parseCliArgs(['restart', '--backend-only', '--skip-docker']), {
    action: 'restart',
    scope: 'backend',
    skipDocker: true,
    help: false,
  });
});

test('shouldManageDocker skips docker for frontend-only runs', () => {
  assert.equal(
    shouldManageDocker({
      scope: 'frontend',
      skipDocker: false,
    }),
    false
  );
});

test('selectServiceKeys maps scopes to managed services', () => {
  assert.deepEqual(selectServiceKeys('all'), ['web', 'api-server', 'plugin-runner']);
  assert.deepEqual(selectServiceKeys('frontend'), ['web']);
  assert.deepEqual(selectServiceKeys('backend'), ['api-server', 'plugin-runner']);
});

test('getServiceDefinitions uses repo default ports and explicit backend binaries', () => {
  const repoRoot = path.resolve(__dirname, '..', '..', '..', '..');
  const services = getServiceDefinitions(repoRoot);

  assert.equal(services.web.port, 3100);
  assert.equal(services['api-server'].port, 7800);
  assert.equal(services['plugin-runner'].port, 7801);
  assert.equal(services.web.bindHost, '0.0.0.0');
  assert.equal(services.web.probeHost, '127.0.0.1');
  assert.equal(services['api-server'].bindHost, '0.0.0.0');
  assert.equal(services['api-server'].probeHost, '127.0.0.1');
  assert.deepEqual(services.web.args, ['--filter', '@1flowbase/web', 'dev']);
  assert.deepEqual(services['api-server'].args, ['run', '-p', 'api-server', '--bin', 'api-server']);
  assert.deepEqual(services['plugin-runner'].args, ['run', '-p', 'plugin-runner', '--bin', 'plugin-runner']);
});

test('getServiceDefinitions gives plugin-runner extra startup time for cold cargo builds', () => {
  const repoRoot = path.resolve(__dirname, '..', '..', '..', '..');
  const services = getServiceDefinitions(repoRoot);

  assert.equal(services.web.startupTimeoutMs, 60_000);
  assert.equal(services['api-server'].startupTimeoutMs, DEFAULT_STARTUP_TIMEOUT_MS);
  assert.equal(services['plugin-runner'].startupTimeoutMs, 60_000);
});

test('getServiceDefinitions leaves frontend pnpm startup interactive', () => {
  const repoRoot = path.resolve(__dirname, '..', '..', '..', '..');
  const services = getServiceDefinitions(repoRoot);

  assert.equal(services.web.envOverrides, undefined);
});

test('waitForServicePort honors per-service startup timeout overrides', async () => {
  const calls = [];

  const ready = await waitForServicePort(
    {
      probeHost: '127.0.0.1',
      port: 7801,
      startupTimeoutMs: 60_000,
    },
    async (host, port, timeoutMs) => {
      calls.push({ host, port, timeoutMs });
      return true;
    }
  );

  assert.equal(ready, true);
  assert.deepEqual(calls, [
    {
      host: '127.0.0.1',
      port: 7801,
      timeoutMs: 60_000,
    },
  ]);
});

test('waitForPortToClose waits until a cleared port stops accepting connections', async () => {
  const probes = [true, true, false];
  const closed = await waitForPortToClose('127.0.0.1', 3100, 1000, async () => probes.shift());

  assert.equal(closed, true);
});

test('parseWindowsNetstatPortOccupants extracts unique listening pids for a port', () => {
  const output = [
    '  Proto  Local Address          Foreign Address        State           PID',
    '  TCP    0.0.0.0:3100           0.0.0.0:0              LISTENING       31856',
    '  TCP    127.0.0.1:3100         127.0.0.1:14248        TIME_WAIT       0',
    '  TCP    127.0.0.1:3100         127.0.0.1:16943        ESTABLISHED     31856',
    '  TCP    [::]:3100              [::]:0                 LISTENING       31856',
    '  TCP    0.0.0.0:7800           0.0.0.0:0              LISTENING       7800',
  ].join('\n');

  assert.deepEqual(parseWindowsNetstatPortOccupants(output, 3100), [31856]);
});

test('listPortOccupantPids uses netstat on Windows', () => {
  const calls = [];
  const occupants = listPortOccupantPids(3100, {
    platform: 'win32',
    runCommandImpl(command, args, options) {
      calls.push({ command, args, captureOutput: options.captureOutput });
      return {
        status: 0,
        stdout: 'TCP    0.0.0.0:3100           0.0.0.0:0              LISTENING       31856',
        stderr: '',
      };
    },
  });

  assert.deepEqual(occupants, [31856]);
  assert.deepEqual(calls, [
    {
      command: 'netstat',
      args: ['-ano'],
      captureOutput: true,
    },
  ]);
});

test('listPortOccupantPids falls back to ss when lsof cannot resolve listeners', () => {
  const calls = [];
  const occupants = listPortOccupantPids(3100, {
    platform: 'linux',
    commandExistsImpl(command) {
      return command === 'lsof' || command === 'ss';
    },
    runCommandImpl(command, args, options) {
      calls.push({ command, args, captureOutput: options.captureOutput });
      if (command === 'lsof') {
        return {
          status: 1,
          stdout: '',
          stderr: '',
        };
      }

      return {
        status: 0,
        stdout: [
          'State  Recv-Q Send-Q Local Address:Port Peer Address:Port Process',
          'LISTEN 0      511          0.0.0.0:3100      0.0.0.0:*     users:(("node",pid=2468,fd=22),("node",pid=1357,fd=23))',
        ].join('\n'),
        stderr: '',
      };
    },
  });

  assert.deepEqual(occupants, [2468, 1357]);
  assert.deepEqual(
    calls.map((call) => call.command),
    ['lsof', 'ss']
  );
});

test('resolveCommandPath prefers Windows command shims that spawn can execute', () => {
  const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'oneflowbase-dev-up-command-path-'));
  fs.writeFileSync(path.join(tempRoot, 'pnpm'), '');
  fs.writeFileSync(path.join(tempRoot, 'pnpm.CMD'), '');

  assert.equal(
    resolveCommandPath('pnpm', {
      platform: 'win32',
      sourceEnv: { PATH: tempRoot },
    }),
    path.join(tempRoot, 'pnpm.cmd')
  );
});

test('startService clears an occupied frontend port before spawning during takeover', async () => {
  const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'oneflowbase-dev-up-port-occupied-'));
  const service = {
    key: 'web',
    label: 'frontend',
    cwd: path.join(tempRoot, 'web'),
    command: 'pnpm',
    args: ['--filter', '@1flowbase/web', 'dev'],
    bindHost: '0.0.0.0',
    probeHost: '127.0.0.1',
    port: 3100,
    startupTimeoutMs: DEFAULT_STARTUP_TIMEOUT_MS,
    logFile: path.join(tempRoot, 'web.log'),
    pidFile: path.join(tempRoot, 'web.json'),
  };
  const clearCalls = [];
  const waitForCloseCalls = [];
  let portOccupied = true;
  let spawned = false;
  let recordedPid = null;

  await startService(service, {
    ensureServiceEnvFileImpl() {
      return false;
    },
    requireCommandImpl() {},
    runServicePrestartCommandsImpl() {},
    readPidRecordImpl() {
      return null;
    },
    isProcessAliveImpl() {
      return false;
    },
    async isPortOpenImpl() {
      return portOccupied;
    },
    async clearPortConflictsImpl(label, ports) {
      clearCalls.push({ label, ports });
      portOccupied = false;
    },
    async waitForPortToCloseImpl(host, port, timeoutMs) {
      waitForCloseCalls.push({ host, port, timeoutMs });
      return true;
    },
    logImpl() {},
    spawnImpl() {
      spawned = true;
      return {
        pid: 4242,
        unref() {},
      };
    },
    buildServiceEnvImpl() {
      return {};
    },
    writePidRecordImpl(_service, pid) {
      recordedPid = pid;
    },
    listPortOccupantPidsImpl() {
      return [];
    },
    async waitForServicePortImpl() {
      return true;
    },
    takeOverPortOwnership: true,
  });

  assert.deepEqual(clearCalls, [
    {
      label: 'frontend',
      ports: [3100],
    },
  ]);
  assert.deepEqual(waitForCloseCalls, [
    {
      host: '127.0.0.1',
      port: 3100,
      timeoutMs: 5000,
    },
  ]);
  assert.equal(spawned, true);
  assert.equal(recordedPid, 4242);
});

test('startService reclaims an occupied service port during restart takeover before spawning', async () => {
  const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'oneflowbase-dev-up-port-takeover-'));
  const service = {
    key: 'api-server',
    label: 'api-server',
    cwd: path.join(tempRoot, 'api'),
    command: 'cargo',
    args: ['run', '-p', 'api-server', '--bin', 'api-server'],
    bindHost: '0.0.0.0',
    probeHost: '127.0.0.1',
    port: 7800,
    startupTimeoutMs: DEFAULT_STARTUP_TIMEOUT_MS,
    logFile: path.join(tempRoot, 'api-server.log'),
    pidFile: path.join(tempRoot, 'api-server.json'),
  };
  const clearCalls = [];
  const waitForCloseCalls = [];
  let portOccupied = true;
  let spawned = false;
  let recordedPid = null;

  await startService(service, {
    ensureServiceEnvFileImpl() {
      return false;
    },
    requireCommandImpl() {},
    runServicePrestartCommandsImpl() {},
    readPidRecordImpl() {
      return null;
    },
    isProcessAliveImpl() {
      return false;
    },
    async isPortOpenImpl() {
      return portOccupied;
    },
    async clearPortConflictsImpl(label, ports) {
      clearCalls.push({ label, ports });
      portOccupied = false;
    },
    async waitForPortToCloseImpl(host, port, timeoutMs) {
      waitForCloseCalls.push({ host, port, timeoutMs });
      return true;
    },
    logImpl() {},
    spawnImpl() {
      spawned = true;
      return {
        pid: 4242,
        unref() {},
      };
    },
    buildServiceEnvImpl() {
      return {};
    },
    writePidRecordImpl(_service, pid) {
      recordedPid = pid;
    },
    listPortOccupantPidsImpl() {
      return [];
    },
    async waitForServicePortImpl() {
      return true;
    },
    takeOverPortOwnership: true,
  });

  assert.deepEqual(clearCalls, [
    {
      label: 'api-server',
      ports: [7800],
    },
  ]);
  assert.deepEqual(waitForCloseCalls, [
    {
      host: '127.0.0.1',
      port: 7800,
      timeoutMs: 5000,
    },
  ]);
  assert.equal(spawned, true);
  assert.equal(recordedPid, 4242);
});

test('stopService clears the service port when the pid record is missing', async () => {
  const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'oneflowbase-dev-up-stop-orphan-'));
  const service = {
    key: 'web',
    label: 'frontend',
    repoRoot: tempRoot,
    bindHost: '0.0.0.0',
    probeHost: '127.0.0.1',
    port: 3100,
    logFile: path.join(tempRoot, 'web.log'),
    pidFile: path.join(tempRoot, 'web.json'),
  };
  const clearCalls = [];
  const logs = [];
  let portOccupied = true;

  await stopService(service, {
    readPidRecordImpl() {
      return null;
    },
    async isPortOpenImpl() {
      return portOccupied;
    },
    async clearPortConflictsImpl(label, ports) {
      clearCalls.push({ label, ports });
      portOccupied = false;
    },
    async waitForPortToCloseImpl() {
      return true;
    },
    logImpl(message) {
      logs.push(message);
    },
  });

  assert.deepEqual(clearCalls, [
    {
      label: 'frontend',
      ports: [3100],
    },
  ]);
  assert.deepEqual(logs, ['frontend 未发现 pid 记录，正在清理端口占用', 'frontend 端口占用已清理']);
});

test('startService resolves the command path before spawning', async () => {
  const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'oneflowbase-dev-up-spawn-command-'));
  const service = {
    key: 'web',
    label: 'frontend',
    cwd: path.join(tempRoot, 'web'),
    command: 'pnpm',
    args: ['--filter', '@1flowbase/web', 'dev'],
    bindHost: '0.0.0.0',
    probeHost: '127.0.0.1',
    port: 3100,
    startupTimeoutMs: DEFAULT_STARTUP_TIMEOUT_MS,
    logFile: path.join(tempRoot, 'web.log'),
    pidFile: path.join(tempRoot, 'web.json'),
  };
  let spawnedCommand = null;
  let spawnedOptions = null;

  await startService(service, {
    ensureServiceEnvFileImpl() {
      return false;
    },
    requireCommandImpl() {},
    runServicePrestartCommandsImpl() {},
    readPidRecordImpl() {
      return null;
    },
    isProcessAliveImpl() {
      return false;
    },
    async isPortOpenImpl() {
      return false;
    },
    logImpl() {},
    spawnImpl(command, _args, options) {
      spawnedCommand = command;
      spawnedOptions = options;
      return {
        pid: 4244,
        unref() {},
      };
    },
    buildServiceEnvImpl() {
      return {};
    },
    resolveCommandPathImpl() {
      return 'C:\\tools\\pnpm.cmd';
    },
    writePidRecordImpl() {},
    async waitForServicePortImpl() {
      return true;
    },
    platform: 'win32',
    takeOverPortOwnership: true,
  });

  assert.equal(spawnedCommand, 'C:\\tools\\pnpm.cmd');
  assert.equal(spawnedOptions.shell, true);
  assert.equal(spawnedOptions.detached, false);
});

test('startService truncates stale service logs before spawning', async () => {
  const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'oneflowbase-dev-up-log-truncate-'));
  const service = {
    key: 'web',
    label: 'frontend',
    cwd: path.join(tempRoot, 'web'),
    command: 'pnpm',
    args: ['--filter', '@1flowbase/web', 'dev'],
    bindHost: '0.0.0.0',
    probeHost: '127.0.0.1',
    port: 3100,
    startupTimeoutMs: DEFAULT_STARTUP_TIMEOUT_MS,
    logFile: path.join(tempRoot, 'web.log'),
    pidFile: path.join(tempRoot, 'web.json'),
  };

  fs.writeFileSync(service.logFile, 'old vite ready line\n', 'utf8');

  await startService(service, {
    ensureServiceEnvFileImpl() {
      return false;
    },
    requireCommandImpl() {},
    runServicePrestartCommandsImpl() {},
    readPidRecordImpl() {
      return null;
    },
    isProcessAliveImpl() {
      return false;
    },
    async isPortOpenImpl() {
      return false;
    },
    logImpl() {},
    spawnImpl() {
      return {
        pid: 4245,
        unref() {},
      };
    },
    buildServiceEnvImpl() {
      return {};
    },
    writePidRecordImpl() {},
    async waitForServicePortImpl() {
      return true;
    },
    listPortOccupantPidsImpl() {
      return [];
    },
    takeOverPortOwnership: true,
  });

  assert.equal(fs.readFileSync(service.logFile, 'utf8'), '');
});

test('startService records the listener pid when it differs from the spawned shell pid', async () => {
  const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'oneflowbase-dev-up-listener-pid-'));
  const service = {
    key: 'web',
    label: 'frontend',
    cwd: path.join(tempRoot, 'web'),
    command: 'pnpm',
    args: ['--filter', '@1flowbase/web', 'dev'],
    bindHost: '0.0.0.0',
    probeHost: '127.0.0.1',
    port: 3100,
    startupTimeoutMs: DEFAULT_STARTUP_TIMEOUT_MS,
    logFile: path.join(tempRoot, 'web.log'),
    pidFile: path.join(tempRoot, 'web.json'),
  };
  const recordedPids = [];

  await startService(service, {
    ensureServiceEnvFileImpl() {
      return false;
    },
    requireCommandImpl() {},
    runServicePrestartCommandsImpl() {},
    readPidRecordImpl() {
      return null;
    },
    isProcessAliveImpl() {
      return false;
    },
    async isPortOpenImpl() {
      return false;
    },
    logImpl() {},
    spawnImpl() {
      return {
        pid: 1111,
        unref() {},
      };
    },
    buildServiceEnvImpl() {
      return {};
    },
    resolveCommandPathImpl() {
      return 'C:\\tools\\pnpm.cmd';
    },
    writePidRecordImpl(_service, pid) {
      recordedPids.push(pid);
    },
    async waitForServicePortImpl() {
      return true;
    },
    listPortOccupantPidsImpl() {
      return [2222];
    },
    platform: 'win32',
    takeOverPortOwnership: true,
  });

  assert.deepEqual(recordedPids, [1111, 2222]);
});

test('startService restarts a running managed service when takeover is requested', async () => {
  const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'oneflowbase-dev-up-managed-takeover-'));
  const service = {
    key: 'web',
    label: 'frontend',
    cwd: path.join(tempRoot, 'web'),
    command: 'pnpm',
    args: ['--filter', '@1flowbase/web', 'dev'],
    bindHost: '0.0.0.0',
    probeHost: '127.0.0.1',
    port: 3100,
    startupTimeoutMs: DEFAULT_STARTUP_TIMEOUT_MS,
    logFile: path.join(tempRoot, 'web.log'),
    pidFile: path.join(tempRoot, 'web.json'),
  };
  const stopCalls = [];
  let portOpen = true;
  let spawned = false;
  let recordedPid = null;

  await startService(service, {
    ensureServiceEnvFileImpl() {
      return false;
    },
    requireCommandImpl() {},
    runServicePrestartCommandsImpl() {},
    readPidRecordImpl() {
      return { pid: 3100 };
    },
    isProcessAliveImpl() {
      return true;
    },
    async isPortOpenImpl() {
      return portOpen;
    },
    async stopServiceImpl(stoppedService) {
      stopCalls.push(stoppedService.key);
      portOpen = false;
    },
    logImpl() {},
    spawnImpl() {
      spawned = true;
      return {
        pid: 4243,
        unref() {},
      };
    },
    buildServiceEnvImpl() {
      return {};
    },
    writePidRecordImpl(_service, pid) {
      recordedPid = pid;
    },
    listPortOccupantPidsImpl() {
      return [];
    },
    async waitForServicePortImpl() {
      return true;
    },
    takeOverPortOwnership: true,
  });

  assert.deepEqual(stopCalls, ['web']);
  assert.equal(spawned, true);
  assert.equal(recordedPid, 4243);
});

test('manageServices treats start as a service takeover', async () => {
  const service = {
    key: 'web',
    label: 'frontend',
  };
  const calls = [];

  await manageServices('start', [service], {
    async startServiceImpl(startedService, options) {
      calls.push({
        key: startedService.key,
        takeOverPortOwnership: options.takeOverPortOwnership,
      });
    },
  });

  assert.deepEqual(calls, [
    {
      key: 'web',
      takeOverPortOwnership: true,
    },
  ]);
});

test('manageDocker restart clears middleware port conflicts before bringing services up', async () => {
  const composeCalls = [];
  const clearCalls = [];

  await manageDocker('/repo-root', 'restart', {
    ensureMiddlewareEnvImpl() {},
    getMiddlewareHostPortsImpl() {
      return [35432];
    },
    async clearPortConflictsImpl(label, ports) {
      clearCalls.push({ label, ports });
    },
    runMiddlewareComposeImpl(_repoRoot, args) {
      composeCalls.push(args);
      return {
        status: 0,
        stdout: '',
        stderr: '',
      };
    },
  });

  assert.deepEqual(clearCalls, [
    {
      label: 'docker 中间件',
      ports: [35432],
    },
  ]);
  assert.deepEqual(composeCalls, [['down'], ['up', '-d']]);
});

test('api-server example env files use workspace bootstrap naming', () => {
  const repoRoot = path.resolve(__dirname, '..', '..', '..', '..');
  const developmentExample = fs.readFileSync(
    path.join(repoRoot, 'api', 'apps', 'api-server', '.env.example'),
    'utf8'
  );
  const productionExample = fs.readFileSync(
    path.join(repoRoot, 'api', 'apps', 'api-server', '.env.production.example'),
    'utf8'
  );

  assert.match(developmentExample, /^BOOTSTRAP_WORKSPACE_NAME=/mu);
  assert.doesNotMatch(developmentExample, /^BOOTSTRAP_TEAM_NAME=/mu);
  assert.match(productionExample, /^BOOTSTRAP_WORKSPACE_NAME=/mu);
  assert.doesNotMatch(productionExample, /^BOOTSTRAP_TEAM_NAME=/mu);
});

test('ensureServiceEnvFile seeds api env defaults and buildServiceEnv loads them', () => {
  const tempRepoRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'oneflowbase-dev-up-env-'));
  const apiServerDir = path.join(tempRepoRoot, 'api', 'apps', 'api-server');
  const envExamplePath = path.join(apiServerDir, '.env.example');

  fs.mkdirSync(apiServerDir, { recursive: true });
  fs.writeFileSync(
    envExamplePath,
    [
      '# api defaults',
      'API_DATABASE_URL=postgres://from-example',
      'BOOTSTRAP_WORKSPACE_NAME=\"1flowbase\"',
    ].join('\n')
  );

  const services = getServiceDefinitions(tempRepoRoot);
  const apiService = services['api-server'];

  assert.equal(fs.existsSync(apiService.envFile), false);
  assert.equal(ensureServiceEnvFile(apiService), true);
  assert.equal(fs.existsSync(apiService.envFile), true);

  const env = buildServiceEnv(apiService, {
    API_DATABASE_URL: 'postgres://from-shell',
    EXTRA_FLAG: 'enabled',
  });

  assert.equal(env.API_DATABASE_URL, 'postgres://from-shell');
  assert.equal(env.BOOTSTRAP_WORKSPACE_NAME, '1flowbase');
  assert.equal(env.EXTRA_FLAG, 'enabled');
});

test('buildServiceEnv applies service env overrides after shell env', () => {
  const env = buildServiceEnv(
    {
      envOverrides: {
        CI: 'true',
      },
    },
    {
      CI: 'false',
      PATH: '/bin',
    }
  );

  assert.equal(env.CI, 'true');
  assert.equal(env.PATH, '/bin');
});

test('ensureServiceEnvFile leaves existing api-server env values untouched even if they use old branding', () => {
  const tempRepoRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'oneflowbase-dev-up-legacy-env-'));
  const apiServerDir = path.join(tempRepoRoot, 'api', 'apps', 'api-server');
  const envExamplePath = path.join(apiServerDir, '.env.example');
  const envPath = path.join(apiServerDir, '.env');

  fs.mkdirSync(apiServerDir, { recursive: true });
  fs.writeFileSync(
    envExamplePath,
    [
      'API_DATABASE_URL=postgres://postgres:1flowbase@127.0.0.1:35432/1flowbase',
      'API_COOKIE_NAME=flowbase_console_session',
      'BOOTSTRAP_WORKSPACE_NAME=1flowbase',
    ].join('\n')
  );
  fs.writeFileSync(
    envPath,
    [
      'API_DATABASE_URL=postgres://postgres:sevenflows@127.0.0.1:35432/sevenflows',
      'API_COOKIE_NAME=flowse_console_session',
      'BOOTSTRAP_WORKSPACE_NAME=1Flowse',
      'BOOTSTRAP_ROOT_PASSWORD=change-me',
    ].join('\n')
  );

  const services = getServiceDefinitions(tempRepoRoot);
  const apiService = services['api-server'];

  assert.equal(ensureServiceEnvFile(apiService), false);

  const env = buildServiceEnv(apiService, {});

  assert.equal(env.API_DATABASE_URL, 'postgres://postgres:sevenflows@127.0.0.1:35432/sevenflows');
  assert.equal(env.API_COOKIE_NAME, 'flowse_console_session');
  assert.equal(env.BOOTSTRAP_WORKSPACE_NAME, '1Flowse');
  assert.equal(env.BOOTSTRAP_ROOT_PASSWORD, 'change-me');
});

test('resolveComposeCommand falls back to standalone docker-compose v2', () => {
  const command = resolveComposeCommand({
    resetCache: true,
    runCommandImpl(command, args) {
      if (command === 'docker' && args[0] === 'compose') {
        return {
          status: 1,
          stdout: '',
          stderr: 'docker compose plugin missing\n',
        };
      }

      if (command === 'docker-compose') {
        return {
          status: 0,
          stdout: 'Docker Compose version v2.33.1\n',
          stderr: '',
        };
      }

      return {
        status: 1,
        stdout: '',
        stderr: '',
      };
    },
  });

  assert.deepEqual(command, { command: 'docker-compose', baseArgs: [] });
});

test('getServicePrestartCommands resets api root password in development mode', () => {
  const tempRepoRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'oneflowbase-dev-up-prestart-'));
  const apiServerDir = path.join(tempRepoRoot, 'api', 'apps', 'api-server');
  const envExamplePath = path.join(apiServerDir, '.env.example');

  fs.mkdirSync(apiServerDir, { recursive: true });
  fs.writeFileSync(
    envExamplePath,
    ['API_ENV=development', 'API_DATABASE_URL=postgres://from-example'].join('\n')
  );

  const services = getServiceDefinitions(tempRepoRoot);
  const apiService = services['api-server'];
  ensureServiceEnvFile(apiService);

  const commands = getServicePrestartCommands(apiService, {});

  assert.deepEqual(
    commands.map((command) => ({
      command: command.command,
      args: command.args,
      cwd: command.cwd,
    })),
    [
      {
        command: 'cargo',
        args: ['run', '-p', 'api-server', '--bin', 'reset_root_password'],
        cwd: path.join(tempRepoRoot, 'api'),
      },
    ]
  );
  assert.equal(commands[0].env.API_ENV, 'development');
});

test('getServicePrestartCommands checks frontend dependencies with visible pnpm prompts', () => {
  const repoRoot = path.resolve(__dirname, '..', '..', '..', '..');
  const services = getServiceDefinitions(repoRoot);
  const commands = getServicePrestartCommands(services.web, { CI: 'false' });

  assert.deepEqual(
    commands.map((command) => ({
      description: command.description,
      command: command.command,
      args: command.args,
      cwd: command.cwd,
      captureOutput: command.captureOutput,
      ci: command.env.CI,
    })),
    [
      {
        description: 'frontend 依赖检查（需要清空重装时由 pnpm 在终端提示确认）',
        command: 'pnpm',
        args: ['install'],
        cwd: path.join(repoRoot, 'web'),
        captureOutput: false,
        ci: 'false',
      },
    ]
  );
});

test('getServicePrestartCommands skips api root reset in production mode', () => {
  const tempRepoRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'oneflowbase-dev-up-prod-'));
  const apiServerDir = path.join(tempRepoRoot, 'api', 'apps', 'api-server');
  const envExamplePath = path.join(apiServerDir, '.env.example');

  fs.mkdirSync(apiServerDir, { recursive: true });
  fs.writeFileSync(
    envExamplePath,
    ['API_ENV=production', 'API_DATABASE_URL=postgres://from-example'].join('\n')
  );

  const services = getServiceDefinitions(tempRepoRoot);
  const apiService = services['api-server'];
  ensureServiceEnvFile(apiService);

  assert.deepEqual(getServicePrestartCommands(apiService, {}), []);
});

test('runServicePrestartCommands rebuilds local postgres db after migration checksum mismatch', () => {
  const tempRepoRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'oneflowbase-dev-up-recover-'));
  const apiServerDir = path.join(tempRepoRoot, 'api', 'apps', 'api-server');
  const dockerDir = path.join(tempRepoRoot, 'docker');

  fs.mkdirSync(apiServerDir, { recursive: true });
  fs.mkdirSync(dockerDir, { recursive: true });

  fs.writeFileSync(
    path.join(apiServerDir, '.env.example'),
    [
      'API_ENV=development',
      'API_DATABASE_URL=postgres://postgres:1flowbase@127.0.0.1:35432/1flowbase',
      'BOOTSTRAP_WORKSPACE_NAME=1flowbase',
      'BOOTSTRAP_ROOT_ACCOUNT=root',
      'BOOTSTRAP_ROOT_EMAIL=root@example.com',
      'BOOTSTRAP_ROOT_PASSWORD=change-me',
    ].join('\n')
  );
  fs.writeFileSync(path.join(dockerDir, 'middleware.env'), 'POSTGRES_PORT=35432\n');

  const services = getServiceDefinitions(tempRepoRoot);
  const apiService = services['api-server'];
  ensureServiceEnvFile(apiService);

  const commandCalls = [];
  const composeCalls = [];
  let attempt = 0;

  runServicePrestartCommands(apiService, {
    runCommandImpl(command, args, options) {
      commandCalls.push({ command, args, options });
      attempt += 1;
      if (attempt === 1) {
        return {
          status: 1,
          stdout: '',
          stderr: 'Error: migration 20260412183000 was previously applied but has been modified\n',
        };
      }

      return {
        status: 0,
        stdout: '',
        stderr: '',
      };
    },
    runMiddlewareComposeImpl(repoRoot, args) {
      composeCalls.push({ repoRoot, args });
      return {
        status: 0,
        stdout: '',
        stderr: '',
      };
    },
  });

  assert.equal(commandCalls.length, 2);
  assert.ok(commandCalls.every((entry) => entry.options.captureOutput === true));
  assert.deepEqual(
    composeCalls.map((entry) => entry.args),
    [
      [
        'exec',
        '-T',
        'db',
        'psql',
        '-U',
        'postgres',
        '-d',
        'postgres',
        '-c',
        'DROP DATABASE IF EXISTS "1flowbase" WITH (FORCE);',
      ],
      [
        'exec',
        '-T',
        'db',
        'psql',
        '-U',
        'postgres',
        '-d',
        'postgres',
        '-c',
        'CREATE DATABASE "1flowbase";',
      ],
    ]
  );
});

test('runServicePrestartCommands lets frontend pnpm prompts write to the terminal', () => {
  const repoRoot = path.resolve(__dirname, '..', '..', '..', '..');
  const services = getServiceDefinitions(repoRoot);
  const commandCalls = [];

  runServicePrestartCommands(services.web, {
    sourceEnv: { CI: 'false' },
    runCommandImpl(command, args, options) {
      commandCalls.push({ command, args, options });
      return {
        status: 0,
        stdout: '',
        stderr: '',
      };
    },
  });

  assert.deepEqual(
    commandCalls.map((entry) => ({
      command: entry.command,
      args: entry.args,
      cwd: entry.options.cwd,
      captureOutput: entry.options.captureOutput,
      ci: entry.options.env.CI,
    })),
    [
      {
        command: 'pnpm',
        args: ['install'],
        cwd: path.join(repoRoot, 'web'),
        captureOutput: false,
        ci: 'false',
      },
    ]
  );
});

test('runServicePrestartCommands rebuilds local postgres db after missing resolved migration drift', () => {
  const tempRepoRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'oneflowbase-dev-up-missing-migration-'));
  const apiServerDir = path.join(tempRepoRoot, 'api', 'apps', 'api-server');
  const dockerDir = path.join(tempRepoRoot, 'docker');

  fs.mkdirSync(apiServerDir, { recursive: true });
  fs.mkdirSync(dockerDir, { recursive: true });

  fs.writeFileSync(
    path.join(apiServerDir, '.env.example'),
    [
      'API_ENV=development',
      'API_DATABASE_URL=postgres://postgres:1flowbase@127.0.0.1:35432/1flowbase',
      'BOOTSTRAP_WORKSPACE_NAME=1flowbase',
      'BOOTSTRAP_ROOT_ACCOUNT=root',
      'BOOTSTRAP_ROOT_EMAIL=root@example.com',
      'BOOTSTRAP_ROOT_PASSWORD=change-me',
    ].join('\n')
  );
  fs.writeFileSync(path.join(dockerDir, 'middleware.env'), 'POSTGRES_PORT=35432\n');

  const services = getServiceDefinitions(tempRepoRoot);
  const apiService = services['api-server'];
  ensureServiceEnvFile(apiService);

  const commandCalls = [];
  const composeCalls = [];
  let attempt = 0;

  runServicePrestartCommands(apiService, {
    runCommandImpl(command, args, options) {
      commandCalls.push({ command, args, options });
      attempt += 1;
      if (attempt === 1) {
        return {
          status: 1,
          stdout: '',
          stderr: 'Error: migration 20260422121000 was previously applied but is missing in the resolved migrations\n',
        };
      }

      return {
        status: 0,
        stdout: '',
        stderr: '',
      };
    },
    runMiddlewareComposeImpl(repoRoot, args) {
      composeCalls.push({ repoRoot, args });
      return {
        status: 0,
        stdout: '',
        stderr: '',
      };
    },
  });

  assert.equal(commandCalls.length, 2);
  assert.ok(commandCalls.every((entry) => entry.options.captureOutput === true));
  assert.deepEqual(
    composeCalls.map((entry) => entry.args),
    [
      [
        'exec',
        '-T',
        'db',
        'psql',
        '-U',
        'postgres',
        '-d',
        'postgres',
        '-c',
        'DROP DATABASE IF EXISTS "1flowbase" WITH (FORCE);',
      ],
      [
        'exec',
        '-T',
        'db',
        'psql',
        '-U',
        'postgres',
        '-d',
        'postgres',
        '-c',
        'CREATE DATABASE "1flowbase";',
      ],
    ]
  );
});
