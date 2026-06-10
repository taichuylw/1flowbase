#!/usr/bin/env node

const { main } = require('../merge-current-to-main-latest/core.js');

if (require.main === module) {
  try {
    process.exitCode = main(process.argv.slice(2));
  } catch (error) {
    process.stderr.write(`[1flowbase-merge-current-to-main-latest] ${error.message}\n`);
    process.exitCode = 1;
  }
}

module.exports = {
  main,
};
