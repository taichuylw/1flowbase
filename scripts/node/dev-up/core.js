const { log, parseCliArgs, selectServiceKeys, shouldManageDocker, usage } = require('./cli.js');
const {
  buildServiceEnv,
  ensureServiceEnvFile,
  getServicePrestartCommands,
  resolveCommandPath,
} = require('./env.js');
const { manageDocker, resolveComposeCommand } = require('./middleware.js');
const {
  listPortOccupantPids,
  manageServices,
  parseWindowsNetstatPortOccupants,
  startService,
  stopService,
  waitForPortToClose,
  waitForServicePort,
} = require('./process.js');
const { runServicePrestartCommands } = require('./postgres-reset.js');
const {
  DEFAULT_STARTUP_TIMEOUT_MS,
  ensureRuntimeDirs,
  getRepoRoot,
  getRuntimePaths,
  getServiceDefinitions,
} = require('./services.js');

const DEV_DATABASE_MAINTENANCE_HINT_ACTIONS = new Set(['start', 'ensure', 'restart']);

function shouldShowDevDatabaseMaintenanceHint(options) {
  return DEV_DATABASE_MAINTENANCE_HINT_ACTIONS.has(options.action) && options.scope !== 'frontend';
}

function buildDevDatabaseMaintenanceHintLines() {
  return [
    '开发库不会在 dev-up 时自动清理；test schema 或备份变多时先 dry-run，确认后把 --dry-run 换成 --apply。',
    'test schema: node scripts/node/dev-db-maintenance.js test-schemas --dry-run --older-than 3d --keep 20',
    'PGDATA 备份建议只留 1 份: node scripts/node/dev-db-maintenance.js backups --dry-run --keep 1 --older-than 7d',
    '备份清理只处理 docker/volumes/postgres.empty-* / postgres.backup-*，不会删除当前 docker/volumes/postgres。',
  ];
}

function writeDevDatabaseMaintenanceHint(writeLog = log) {
  for (const line of buildDevDatabaseMaintenanceHintLines()) {
    writeLog(line);
  }
}

async function main(argv = process.argv.slice(2)) {
  const options = parseCliArgs(argv);
  if (options.help) {
    usage();
    return 0;
  }

  const repoRoot = getRepoRoot();
  const runtimePaths = getRuntimePaths(repoRoot);
  ensureRuntimeDirs(runtimePaths);

  const serviceDefinitions = getServiceDefinitions(repoRoot);
  const services = selectServiceKeys(options.scope).map((key) => serviceDefinitions[key]);

  if (shouldManageDocker(options)) {
    await manageDocker(repoRoot, options.action);
  } else if (options.skipDocker) {
    log('已跳过 Docker 中间件管理');
  }

  if (shouldShowDevDatabaseMaintenanceHint(options)) {
    writeDevDatabaseMaintenanceHint();
  }

  await manageServices(options.action, services);
  return 0;
}

module.exports = {
  DEFAULT_STARTUP_TIMEOUT_MS,
  buildDevDatabaseMaintenanceHintLines,
  buildServiceEnv,
  ensureServiceEnvFile,
  getRepoRoot,
  getRuntimePaths,
  getServiceDefinitions,
  getServicePrestartCommands,
  listPortOccupantPids,
  main,
  manageDocker,
  manageServices,
  parseWindowsNetstatPortOccupants,
  parseCliArgs,
  resolveComposeCommand,
  resolveCommandPath,
  runServicePrestartCommands,
  selectServiceKeys,
  shouldManageDocker,
  shouldShowDevDatabaseMaintenanceHint,
  startService,
  stopService,
  waitForPortToClose,
  waitForServicePort,
};
