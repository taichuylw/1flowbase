const test = require("node:test");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");

const repoRoot = path.resolve(__dirname, "..", "..", "..", "..");

function readVerifyWorkflow() {
  return fs.readFileSync(
    path.join(repoRoot, ".github", "workflows", "verify.yml"),
    "utf8",
  );
}

function readQualityGateWorkflow() {
  return fs.readFileSync(
    path.join(repoRoot, ".github", "workflows", "quality-gate.yml"),
    "utf8",
  );
}

function readQualityGateAction() {
  return fs.readFileSync(
    path.join(repoRoot, ".github", "actions", "quality-gate", "action.yml"),
    "utf8",
  );
}

function readGitHubAutomationDocs() {
  return fs.readFileSync(
    path.join(repoRoot, ".github", "GITHUB_AUTOMATION.md"),
    "utf8",
  );
}

function readReactDoctorConfig() {
  return JSON.parse(
    fs.readFileSync(
      path.join(repoRoot, "web", "app", "doctor.config.json"),
      "utf8",
    ),
  );
}

function extractPushBranches(workflow) {
  const match = workflow.match(
    /push:\n\s+branches:\n(?<branches>(?:\s+- .+\n)+)/u,
  );
  assert.ok(match, "verify workflow must declare push branches");

  return match.groups.branches
    .split(/\r?\n/u)
    .map((line) => line.trim().replace(/^- /u, ""))
    .filter(Boolean);
}

test("verify workflow runs on main and latest but only publishes quality reports on latest pushes", () => {
  const workflow = readVerifyWorkflow();

  assert.deepEqual(extractPushBranches(workflow), ["main", "latest"]);
  assert.match(
    workflow,
    /concurrency:\n\s+group: verify-\$\{\{ github\.ref_name \}\}\n\s+cancel-in-progress: true/u,
  );
  assert.match(
    workflow,
    /INPUT_PUBLISH_ISSUE: \$\{\{ github\.event_name == 'push' && github\.ref == 'refs\/heads\/latest' \}\}/u,
  );
  assert.match(
    workflow,
    /INPUT_PUBLISH_PR_COMMENT: \$\{\{ github\.event_name == 'pull_request' && github\.event\.pull_request\.head\.repo\.full_name == github\.repository \}\}/u,
  );
  assert.match(workflow, /INPUT_PR_NUMBER: \$\{\{ github\.event\.pull_request\.number \}\}/u);
  assert.doesNotMatch(workflow, /INPUT_PUBLISH_ISSUE: .+refs\/heads\/main/u);
});

test("verify workflow runs quality gate scopes in parallel before one aggregate report", () => {
  const workflow = readVerifyWorkflow();

  assert.match(workflow, /repo-tooling-gate:\n\s+runs-on: ubuntu-latest/u);
  assert.match(workflow, /repo-frontend-gate:\n\s+runs-on: ubuntu-latest/u);
  assert.match(workflow, /repo-backend-gate:\n\s+runs-on: ubuntu-latest/u);
  assert.match(workflow, /fail-fast: false/u);
  assert.match(workflow, /- repo-backend-static/u);
  assert.match(workflow, /- repo-backend-clippy-core-libs/u);
  assert.match(workflow, /- repo-backend-test-control-plane/u);
  assert.match(workflow, /- repo-backend-test-api-server/u);
  assert.match(workflow, /- repo-backend-test-plugin-runner/u);
  assert.doesNotMatch(workflow, /- repo-backend-test-apps/u);
  assert.match(workflow, /- repo-backend-check-runtime-storage/u);
  assert.match(
    workflow,
    /backend-consistency-gate:\n\s+runs-on: ubuntu-latest/u,
  );
  assert.match(workflow, /coverage-frontend-gate:\n\s+runs-on: ubuntu-latest/u);
  assert.match(workflow, /coverage-backend-gate:\n\s+runs-on: ubuntu-latest/u);
  assert.match(workflow, /- coverage-backend-control-plane/u);
  assert.match(workflow, /- coverage-backend-storage-postgres/u);
  assert.match(workflow, /- coverage-backend-api-server/u);
  assert.match(
    workflow,
    /verify:\n\s+needs:\n\s+- repo-tooling-gate\n\s+- repo-frontend-gate\n\s+- repo-backend-gate\n\s+- backend-consistency-gate\n\s+- coverage-frontend-gate\n\s+- coverage-backend-gate/u,
  );
  assert.match(workflow, /scope: repo-tooling/u);
  assert.match(workflow, /scope: repo-frontend/u);
  assert.match(workflow, /scope: \$\{\{ matrix\.scope \}\}/u);
  assert.match(
    workflow,
    /start_postgres: \$\{\{ startsWith\(matrix\.scope, 'repo-backend-test-'\) \}\}/u,
  );
  assert.match(workflow, /scope: backend-consistency/u);
  assert.match(workflow, /scope: coverage-frontend/u);
  assert.match(workflow, /name: test-governance-repo-tooling/u);
  assert.match(workflow, /name: test-governance-repo-frontend/u);
  assert.match(workflow, /name: test-governance-\$\{\{ matrix\.scope \}\}/u);
  assert.match(workflow, /name: test-governance-backend-consistency/u);
  assert.match(workflow, /name: test-governance-coverage-frontend/u);
  assert.match(
    workflow,
    /coverage-backend-gate:[\s\S]*?scope: \$\{\{ matrix\.scope \}\}[\s\S]*?start_postgres: \$\{\{ matrix\.scope == 'coverage-backend-storage-postgres' \|\| matrix\.scope == 'coverage-backend-api-server' \}\}/u,
  );
  assert.match(workflow, /merge-multiple: false/u);
  assert.match(
    workflow,
    /node scripts\/node\/cli\/github-quality-gate-aggregate\.js/u,
  );
});

test("verify workflow runs React Doctor as a frontend quality gate", () => {
  const workflow = readVerifyWorkflow();

  assert.match(workflow, /react-doctor-gate:\n\s+runs-on: ubuntu-latest/u);
  assert.match(workflow, /fetch-depth: 0/u);
  assert.match(
    workflow,
    /git fetch --no-tags --prune --depth=1 origin main:refs\/remotes\/origin\/main/u,
  );
  assert.match(workflow, /uses: actions\/setup-node@v5/u);
  assert.match(workflow, /node-version: 24/u);
  assert.match(
    workflow,
    /npx react-doctor@0\.2\.16 web\/app --diff origin\/main --offline --fail-on warning --verbose/u,
  );
  assert.doesNotMatch(workflow, /uses: millionco\/react-doctor@main/u);
  assert.doesNotMatch(
    workflow,
    /github-token: \$\{\{ secrets\.GITHUB_TOKEN \}\}/u,
  );
  assert.match(
    workflow,
    /verify:\n\s+needs:\n\s+- repo-tooling-gate\n\s+- repo-frontend-gate\n\s+- repo-backend-gate\n\s+- backend-consistency-gate\n\s+- coverage-frontend-gate\n\s+- coverage-backend-gate\n\s+- react-doctor-gate/u,
  );
});

test("React Doctor keeps current debt as a narrow baseline", () => {
  const config = readReactDoctorConfig();

  assert.deepEqual(config.ignore.overrides, [
    {
      files: ["src/features/frontstage/pages/FrontStagePage.tsx"],
      rules: [
        "react-doctor/no-cascading-set-state",
        "react-doctor/no-effect-chain",
        "react-doctor/no-prop-callback-in-effect",
        "react-doctor/no-inline-exhaustive-style",
        "react-doctor/no-giant-component",
        "react-doctor/no-many-boolean-props",
        "react-doctor/prefer-useReducer",
        "react-doctor/no-derived-state-effect",
      ],
    },
    {
      files: [
        "src/features/agent-flow/_tests/editor/agent-flow-canvas-interactions.test.tsx",
      ],
      rules: ["react-doctor/no-prop-callback-in-effect"],
    },
    {
      files: ["src/features/agent-flow/_tests/node-inspector/support.tsx"],
      rules: [
        "react-doctor/no-pass-data-to-parent",
        "react-doctor/no-prop-callback-in-effect",
        "react-doctor/only-export-components",
      ],
    },
    {
      files: ["src/features/agent-flow/components/editor/AgentFlowCanvas.tsx"],
      rules: [
        "react-doctor/no-pass-data-to-parent",
        "react-doctor/no-prop-callback-in-effect",
      ],
    },
    {
      files: ["src/features/agent-flow/components/nodes/AgentFlowNodeCard.tsx"],
      rules: [
        "react-doctor/no-giant-component",
        "react-doctor/prefer-tag-over-role",
      ],
    },
    {
      files: [
        "src/features/applications/components/api/ApplicationApiKeysPanel.tsx",
      ],
      rules: ["react-doctor/prefer-module-scope-pure-function"],
    },
    {
      files: ["src/features/applications/pages/ApplicationLogsPage.tsx"],
      rules: [
        "react-doctor/no-adjust-state-on-prop-change",
        "react-doctor/no-cascading-set-state",
        "react-doctor/no-chain-state-updates",
        "react-doctor/no-derived-state-effect",
        "react-doctor/no-giant-component",
        "react-doctor/prefer-tag-over-role",
        "react-doctor/prefer-useReducer",
      ],
    },
    {
      files: ["src/features/applications/pages/ApplicationMonitoringPage.tsx"],
      rules: ["react-doctor/no-giant-component"],
    },
    {
      files: [
        "src/features/frontstage/components/FrontStagePageTreeSidebar.tsx",
      ],
      rules: [
        "react-doctor/click-events-have-key-events",
        "react-doctor/client-localstorage-no-version",
        "react-doctor/no-noninteractive-element-interactions",
        "react-doctor/no-render-in-render",
        "react-doctor/no-static-element-interactions",
      ],
    },
    {
      files: [
        "src/features/settings/components/host-infrastructure/HostInfrastructureCachePanel.tsx",
      ],
      rules: [
        "react-doctor/no-chain-state-updates",
        "react-doctor/no-giant-component",
        "react-doctor/query-mutation-missing-invalidation",
      ],
    },
    {
      files: [
        "src/features/settings/components/host-infrastructure/HostInfrastructureMemoryObservationPanel.tsx",
      ],
      rules: [
        "react-doctor/exhaustive-deps",
        "react-doctor/js-combine-iterations",
        "react-doctor/no-cascading-set-state",
        "react-doctor/no-chain-state-updates",
        "react-doctor/no-derived-state-effect",
        "react-doctor/no-giant-component",
        "react-doctor/no-static-element-interactions",
        "react-doctor/no-tiny-text",
        "react-doctor/prefer-tag-over-role",
        "react-doctor/prefer-use-effect-event",
        "react-doctor/prefer-useReducer",
        "react-doctor/query-mutation-missing-invalidation",
      ],
    },
    {
      files: [
        "src/features/settings/components/model-providers/ModelProviderInstanceDrawer.tsx",
      ],
      rules: [
        "react-doctor/no-adjust-state-on-prop-change",
        "react-doctor/no-cascading-set-state",
        "react-doctor/no-chain-state-updates",
        "react-doctor/no-derived-state",
        "react-doctor/no-event-handler",
        "react-doctor/no-giant-component",
        "react-doctor/prefer-useReducer",
        "react-doctor/rerender-state-only-in-handlers",
      ],
    },
    {
      files: ["src/shared/ui/api-docs/ApiDocsExplorer.tsx"],
      rules: [
        "react-doctor/no-adjust-state-on-prop-change",
        "react-doctor/no-derived-state",
        "react-doctor/no-event-handler",
        "react-doctor/no-giant-component",
        "react-doctor/no-pass-data-to-parent",
        "react-doctor/no-prop-callback-in-effect",
        "react-doctor/no-render-in-render",
        "react-doctor/prefer-module-scope-pure-function",
      ],
    },
    {
      files: ["src/shared/ui/api-docs/_tests/ApiDocsExplorer.test.tsx"],
      rules: ["react-doctor/anchor-is-valid"],
    },
  ]);
});

test("GitHub automation docs describe latest-only issue publishing", () => {
  const readme = readGitHubAutomationDocs();

  assert.match(readme, /push` to `latest`/u);
  assert.match(
    readme,
    /INPUT_PUBLISH_ISSUE: \$\{\{ github\.event_name == 'push' && github\.ref == 'refs\/heads\/latest' \}\}/u,
  );
  assert.match(
    readme,
    /creates a GitHub Issue only for `latest` branch pushes/u,
  );
  assert.doesNotMatch(readme, /main branch push failures/u);
  assert.doesNotMatch(readme, /refs\/heads\/main/u);
});

test("GitHub automation docs describe the React Doctor frontend gate", () => {
  const readme = readGitHubAutomationDocs();

  assert.match(readme, /React Doctor frontend gates/u);
  assert.match(
    readme,
    /npx react-doctor@0\.2\.16 web\/app --diff origin\/main --offline --fail-on warning --verbose/u,
  );
  assert.match(readme, /web\/app\/doctor\.config\.json/u);
  assert.match(readme, /explicit baseline/u);
});

test("quality gate workflow supports dispatch targets and nightly latest CI defaults", () => {
  const workflow = readQualityGateWorkflow();

  assert.match(workflow, /name: quality gate/u);
  assert.match(
    workflow,
    /target_branch:\n\s+description: Target branch\n\s+type: string\n\s+default: latest/u,
  );
  assert.match(
    workflow,
    /concurrency:\n\s+group: quality-gate-\$\{\{ github\.event_name \}\}-\$\{\{ github\.event_name == 'workflow_dispatch' && inputs\.target_branch \|\| 'latest' \}\}\n\s+cancel-in-progress: true/u,
  );
  assert.match(
    workflow,
    /schedule:\n(?:\s+# .+\n)?\s+- cron: ["']0 18 \* \* \*["']/u,
  );
  assert.match(
    workflow,
    /QUALITY_GATE_TARGET_BRANCH: \$\{\{ github\.event_name == 'workflow_dispatch' && inputs\.target_branch \|\| 'latest' \}\}/u,
  );
  assert.match(workflow, /FORCE_JAVASCRIPT_ACTIONS_TO_NODE24: true/u);
  assert.match(
    workflow,
    /QUALITY_GATE_SCOPE: \$\{\{ github\.event_name == 'workflow_dispatch' && inputs\.scope \|\| 'ci' \}\}/u,
  );
  assert.match(
    workflow,
    /QUALITY_GATE_REPORT_TYPE: \$\{\{ github\.event_name == 'workflow_dispatch' && inputs\.report_type \|\| 'ci' \}\}/u,
  );
  assert.match(workflow, /QUALITY_GATE_SCHEDULED_ENVIRONMENT: nightly-latest/u);
  assert.match(workflow, /- container-images/u);
  assert.match(
    workflow,
    /GITHUB_REF_NAME: \$\{\{ env\.QUALITY_GATE_TARGET_BRANCH \}\}/u,
  );
  assert.match(
    workflow,
    /GITHUB_SHA: \$\{\{ env\.QUALITY_GATE_TARGET_SHA \}\}/u,
  );
  assert.match(
    workflow,
    /environment: \$\{\{ github\.event_name == 'schedule' && env\.QUALITY_GATE_SCHEDULED_ENVIRONMENT \|\| inputs\.environment \}\}/u,
  );
});

test("quality gate workflow runs ci scope as parallel component gates before one published aggregate report", () => {
  const workflow = readQualityGateWorkflow();

  assert.match(
    workflow,
    /repo-tooling-gate:\n\s+if: \$\{\{ github\.event_name == 'schedule' \|\| \(github\.event_name == 'workflow_dispatch' && inputs\.scope == 'ci'\) \}\}/u,
  );
  assert.match(
    workflow,
    /repo-frontend-gate:\n\s+if: \$\{\{ github\.event_name == 'schedule' \|\| \(github\.event_name == 'workflow_dispatch' && inputs\.scope == 'ci'\) \}\}/u,
  );
  assert.match(
    workflow,
    /repo-backend-gate:\n\s+if: \$\{\{ github\.event_name == 'schedule' \|\| \(github\.event_name == 'workflow_dispatch' && inputs\.scope == 'ci'\) \}\}/u,
  );
  assert.match(workflow, /- repo-backend-static/u);
  assert.match(workflow, /- repo-backend-clippy-runtime-storage/u);
  assert.match(workflow, /- repo-backend-test-control-plane/u);
  assert.match(workflow, /- repo-backend-test-api-server/u);
  assert.match(workflow, /- repo-backend-test-plugin-runner/u);
  assert.doesNotMatch(workflow, /- repo-backend-test-apps/u);
  assert.match(workflow, /- repo-backend-check-apps/u);
  assert.match(
    workflow,
    /backend-consistency-gate:\n\s+if: \$\{\{ github\.event_name == 'schedule' \|\| \(github\.event_name == 'workflow_dispatch' && inputs\.scope == 'ci'\) \}\}/u,
  );
  assert.match(
    workflow,
    /coverage-frontend-gate:\n\s+if: \$\{\{ github\.event_name == 'schedule' \|\| \(github\.event_name == 'workflow_dispatch' && inputs\.scope == 'ci'\) \}\}/u,
  );
  assert.match(
    workflow,
    /coverage-backend-gate:\n\s+if: \$\{\{ github\.event_name == 'schedule' \|\| \(github\.event_name == 'workflow_dispatch' && inputs\.scope == 'ci'\) \}\}/u,
  );
  assert.match(
    workflow,
    /container-images-gate:\n\s+if: \$\{\{ github\.event_name == 'schedule' \|\| \(github\.event_name == 'workflow_dispatch' && \(inputs\.scope == 'ci' \|\| inputs\.scope == 'container-images'\)\) \}\}/u,
  );
  assert.match(workflow, /- coverage-backend-control-plane/u);
  assert.match(workflow, /- coverage-backend-storage-postgres/u);
  assert.match(workflow, /- coverage-backend-api-server/u);
  assert.match(
    workflow,
    /aggregate:\n\s+if: \$\{\{ always\(\) && \(github\.event_name == 'schedule' \|\| \(github\.event_name == 'workflow_dispatch' && inputs\.scope == 'ci'\)\) \}\}/u,
  );
  assert.match(
    workflow,
    /aggregate:\n(?:.*\n)*?\s+needs:\n\s+- repo-tooling-gate\n\s+- repo-frontend-gate\n\s+- repo-backend-gate\n\s+- backend-consistency-gate\n\s+- coverage-frontend-gate\n\s+- coverage-backend-gate/u,
  );
  assert.match(workflow, /- container-images-gate/u);
  assert.match(workflow, /scope: repo-tooling/u);
  assert.match(workflow, /scope: repo-frontend/u);
  assert.match(workflow, /scope: \$\{\{ matrix\.scope \}\}/u);
  assert.match(
    workflow,
    /start_postgres: \$\{\{ startsWith\(matrix\.scope, 'repo-backend-test-'\) \}\}/u,
  );
  assert.match(workflow, /scope: backend-consistency/u);
  assert.match(workflow, /scope: coverage-frontend/u);
  assert.match(workflow, /scope: container-images/u);
  assert.match(workflow, /publish_issue: "false"/u);
  assert.match(workflow, /INPUT_PUBLISH_ISSUE: "true"/u);
  assert.match(workflow, /INPUT_EXPECTED_SCOPES: .*container-images/u);
  assert.match(
    workflow,
    /node scripts\/node\/cli\/github-quality-gate-aggregate\.js/u,
  );
  assert.match(workflow, /name: test-governance-repo-tooling/u);
  assert.match(workflow, /name: test-governance-repo-frontend/u);
  assert.match(workflow, /name: test-governance-\$\{\{ matrix\.scope \}\}/u);
  assert.match(workflow, /name: test-governance-backend-consistency/u);
  assert.match(workflow, /name: test-governance-coverage-frontend/u);
  assert.match(workflow, /name: test-governance-container-images/u);
  assert.match(workflow, /name: test-governance-artifacts/u);
});

test("quality gate workflow keeps non-ci dispatch scopes on a single targeted job", () => {
  const workflow = readQualityGateWorkflow();

  assert.match(
    workflow,
    /single-scope-gate:\n\s+if: \$\{\{ github\.event_name == 'workflow_dispatch' && inputs\.scope != 'ci' && inputs\.scope != 'container-images' \}\}/u,
  );
  assert.match(workflow, /scope: \$\{\{ env\.QUALITY_GATE_SCOPE \}\}/u);
  assert.match(
    workflow,
    /start_postgres: \$\{\{ inputs\.scope == 'backend' \|\| inputs\.scope == 'backend-consistency' \|\| inputs\.scope == 'repo-backend' \|\| startsWith\(inputs\.scope, 'repo-backend-test-'\) \|\| inputs\.scope == 'coverage' \|\| inputs\.scope == 'coverage-backend' \|\| startsWith\(inputs\.scope, 'coverage-backend-'\) \}\}/u,
  );
  assert.match(workflow, /publish_issue: "true"/u);
});

test("quality gate action clears stale middleware containers before starting postgres", () => {
  const action = readQualityGateAction();

  assert.match(
    action,
    /docker compose -f docker\/docker-compose\.middleware\.yaml down --remove-orphans/u,
  );
  assert.match(
    action,
    /docker-compose -f docker\/docker-compose\.middleware\.yaml down --remove-orphans/u,
  );
});
