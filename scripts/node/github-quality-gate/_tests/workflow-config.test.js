const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');

const repoRoot = path.resolve(__dirname, '..', '..', '..', '..');

function readVerifyWorkflow() {
  return fs.readFileSync(path.join(repoRoot, '.github', 'workflows', 'verify.yml'), 'utf8');
}

function readManualQualityGateWorkflow() {
  return fs.readFileSync(path.join(repoRoot, '.github', 'workflows', 'quality-gate.yml'), 'utf8');
}

function readGitHubReadme() {
  return fs.readFileSync(path.join(repoRoot, '.github', 'README.md'), 'utf8');
}

function extractPushBranches(workflow) {
  const match = workflow.match(/push:\n\s+branches:\n(?<branches>(?:\s+- .+\n)+)/u);
  assert.ok(match, 'verify workflow must declare push branches');

  return match.groups.branches
    .split(/\r?\n/u)
    .map((line) => line.trim().replace(/^- /u, ''))
    .filter(Boolean);
}

test('verify workflow runs on main and latest but only publishes quality reports on latest pushes', () => {
  const workflow = readVerifyWorkflow();

  assert.deepEqual(extractPushBranches(workflow), ['main', 'latest']);
  assert.match(workflow, /concurrency:\n\s+group: quality-gate-\$\{\{ github\.ref_name \}\}\n\s+cancel-in-progress: true/u);
  assert.match(
    workflow,
    /publish_issue: \$\{\{ github\.event_name == 'push' && github\.ref == 'refs\/heads\/latest' \}\}/u
  );
  assert.doesNotMatch(workflow, /publish_issue: .+refs\/heads\/main/u);
});

test('GitHub automation docs describe latest-only issue publishing', () => {
  const readme = readGitHubReadme();

  assert.match(readme, /push` to `latest`/u);
  assert.match(
    readme,
    /publish_issue: \$\{\{ github\.event_name == 'push' && github\.ref == 'refs\/heads\/latest' \}\}/u
  );
  assert.match(readme, /creates a GitHub Issue only for `latest` branch pushes/u);
  assert.doesNotMatch(readme, /main branch push failures/u);
  assert.doesNotMatch(readme, /refs\/heads\/main/u);
});

test('manual quality gate defaults to latest and can target supported branches', () => {
  const workflow = readManualQualityGateWorkflow();

  assert.match(workflow, /target_branch:\n\s+description: Target branch\n\s+type: choice\n\s+default: latest\n\s+options:\n\s+- latest\n\s+- main/u);
  assert.match(workflow, /concurrency:\n\s+group: quality-gate-\$\{\{ inputs\.target_branch \}\}\n\s+cancel-in-progress: true/u);
  assert.match(workflow, /uses: actions\/checkout@v5\n\s+with:\n\s+ref: \$\{\{ inputs\.target_branch \}\}/u);
  assert.match(workflow, /GITHUB_REF_NAME: \$\{\{ inputs\.target_branch \}\}/u);
  assert.match(workflow, /GITHUB_SHA: \$\{\{ env\.QUALITY_GATE_TARGET_SHA \}\}/u);
});
