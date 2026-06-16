#!/usr/bin/env node

const { main } = require('../acp-claude-smoke/core.js');

main(process.argv.slice(2)).then((status) => {
  process.exitCode = status;
}).catch((error) => {
  process.stderr.write(`[1flowbase-acp-claude-smoke] ${error.message}\n`);
  process.exitCode = 1;
});
