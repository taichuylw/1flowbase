#!/usr/bin/env node

const {
  parseCliArgs,
  runVersionBump,
} = require('./bump-version/core.js');

try {
  process.exitCode = runVersionBump({
    options: parseCliArgs(process.argv.slice(2)),
  });
} catch (error) {
  process.stderr.write(`[1flowbase-bump-version] ${error.message}\n`);
  process.exitCode = 1;
}
