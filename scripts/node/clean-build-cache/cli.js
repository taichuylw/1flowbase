#!/usr/bin/env node

const {
  parseCliArgs,
  runBuildCacheCleanup: main,
} = require('./core.js');

if (require.main === module) {
  main({
    options: parseCliArgs(process.argv.slice(2)),
  }).then((status) => {
    process.exitCode = status;
  }).catch((error) => {
    process.stderr.write(`[1flowbase-clean-build-cache] ${error.message}\n`);
    process.exitCode = 1;
  });
}

module.exports = {
  parseCliArgs,
  main,
};
