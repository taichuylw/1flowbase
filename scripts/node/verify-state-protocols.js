#!/usr/bin/env node

const {
  buildStateProtocolCommands: buildCommands,
  parseStateProtocolsCliArgs,
  runStateProtocols: main,
} = require('./verify/index.js');

if (require.main === module) {
  Promise.resolve()
    .then(() => main(process.argv.slice(2)))
    .then((status) => {
      process.exitCode = status;
    })
    .catch((error) => {
      process.stderr.write(`[1flowbase-verify-state-protocols] ${error.message}\n`);
      process.exitCode = 1;
    });
}

module.exports = {
  buildCommands,
  main,
  parseStateProtocolsCliArgs,
};
