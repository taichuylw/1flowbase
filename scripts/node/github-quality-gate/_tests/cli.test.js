const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');

const { main, readInputs } = require('../../github-quality-gate.js');

test('readInputs maps GitHub action environment inputs', () => {
  assert.deepEqual(readInputs({
    INPUT_SCOPE: 'repo',
    INPUT_REPORT_TYPE: 'cd',
    INPUT_PUBLISH_ISSUE: 'true',
    INPUT_GITHUB_TOKEN: 'token',
    INPUT_ENVIRONMENT: 'staging',
  }), {
    scope: 'repo',
    reportType: 'cd',
    publishIssue: true,
    githubToken: 'token',
    environmentName: 'staging',
  });
});

test('main returns the quality gate exit code from the runner', async () => {
  const repoRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'oneflowbase-quality-gate-cli-'));

  const status = await main([], {
    repoRoot,
    env: {
      INPUT_SCOPE: 'backend',
      INPUT_REPORT_TYPE: 'ci',
      INPUT_PUBLISH_ISSUE: 'false',
    },
    spawnSyncImpl() {
      return {
        status: 7,
        stdout: '',
        stderr: '',
      };
    },
    writeStdout() {},
    writeStderr() {},
  });

  assert.equal(status, 7);
});
