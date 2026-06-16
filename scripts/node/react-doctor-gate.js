#!/usr/bin/env node

const { runReactDoctorGate } = require('./react-doctor-gate/core.js');

if (require.main === module) {
  Promise.resolve()
    .then(() => runReactDoctorGate())
    .then((status) => {
      process.exitCode = status;
    })
    .catch((error) => {
      process.stderr.write(`[1flowbase-react-doctor-gate] ${error.message}\n`);
      process.exitCode = 1;
    });
}

module.exports = {
  runReactDoctorGate,
};
