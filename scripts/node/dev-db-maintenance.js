#!/usr/bin/env node

const { parseCliArgs, runDevDatabaseMaintenance } = require('./dev-db-maintenance/core.js');

if (require.main === module) {
  try {
    process.exitCode = runDevDatabaseMaintenance({
      options: parseCliArgs(process.argv.slice(2)),
    });
  } catch (error) {
    process.stderr.write(`[1flowbase-dev-db-maintenance] ${error.message}\n`);
    process.exitCode = 1;
  }
}

module.exports = {
  parseCliArgs,
  runDevDatabaseMaintenance,
};
