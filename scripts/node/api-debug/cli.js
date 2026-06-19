#!/usr/bin/env node

const { main } = require('./core.js');

main(process.argv.slice(2)).then((status) => {
  process.exitCode = status;
}).catch((error) => {
  process.stderr.write(`[1flowbase-api-debug] ${error.message}\n`);
  process.exitCode = 1;
});
