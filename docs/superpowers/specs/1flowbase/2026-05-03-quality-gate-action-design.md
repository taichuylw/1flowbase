# 1flowbase Quality Gate Action Design

## Background

The repository already owns the quality gate logic through Node scripts:

- `node scripts/node/verify-ci.js`
- `node scripts/node/verify-repo.js`
- `node scripts/node/verify-backend.js`
- `node scripts/node/verify-coverage.js`

The existing GitHub workflow calls `verify-ci` directly and uploads `tmp/test-governance/`.
The missing layer is a reusable GitHub Action boundary that lets CI, CD, and manual
quality checks call the same gate with consistent logs, reports, outputs, and optional
Issue publishing.

## Goal

Add a repository-local Quality Gate Action that:

- Runs repository-owned quality commands instead of duplicating gate logic in workflows.
- Produces stable evidence under `tmp/test-governance/`.
- Can be reused by automatic CI, future CD, and manual quality report workflows.
- Creates a new GitHub Issue only when the caller explicitly enables Issue publishing.

## Non-Goals

- Do not replace the existing `scripts/node/verify-*` gate logic.
- Do not create Issues from automatic `pull_request` or `push` CI.
- Do not maintain one long-lived quality Issue with appended comments.
- Do not move deployment logic into the Quality Gate Action.

## Decision Summary

### 1. Use A Repository-Local Composite Action

Create:

- `.github/actions/quality-gate/action.yml`

The action is repository-local so it can evolve with the project scripts. Workflows call it
with `uses: ./.github/actions/quality-gate`.

The action delegates real execution to a Node script:

- `scripts/node/github-quality-gate.js`

This keeps complex behavior testable in `scripts/node/**` instead of embedding it in YAML.

### 2. Keep Quality Execution And Issue Publishing In One Action Boundary

The action owns both:

- quality gate execution
- optional new Issue creation

Issue publishing is controlled by an explicit input:

- `publish_issue: "true" | "false"`

Automatic CI and CD pass `publish_issue: "false"` unless a workflow intentionally wants a
release report. The manual quality workflow passes `publish_issue: "true"`.

### 3. Manual Reports Always Create A New Issue

When `publish_issue` is true, the action creates a new Issue for that run. It does not search
for an existing Issue and does not append comments to historical reports.

Issue titles follow this shape:

```text
[Quality Gate][CI] 2026-05-03 23:40 main abc1234 failed
[Quality Gate][CD] 2026-05-03 23:45 production abc1234 passed
```

Labels:

- `quality-gate`
- `manual-run`
- `ci-report` or `cd-report`
- `passed` or `failed`

### 4. Workflows Own Triggers, Setup, Permissions, And Artifacts

The action should not hide workflow-level concerns:

- checkout
- Node setup
- pnpm install
- Rust setup
- `cargo-llvm-cov` install
- artifact upload
- GitHub permissions

Workflows stay responsible for those steps. The action assumes required dependencies are
available and focuses on running the selected gate.

## Action Interface

### Inputs

| Input | Required | Default | Description |
| --- | --- | --- | --- |
| `scope` | no | `ci` | Quality gate scope: `ci`, `repo`, `backend`, `coverage`. |
| `report_type` | no | `ci` | Report classification: `ci` or `cd`. |
| `publish_issue` | no | `false` | Whether to create a new GitHub Issue. |
| `github_token` | no | empty | Required only when `publish_issue` is true. |
| `environment` | no | empty | Optional CD environment label, such as `staging` or `production`. |

### Outputs

| Output | Description |
| --- | --- |
| `status` | `passed` or `failed`. |
| `exit_code` | Numeric gate exit code. |
| `report_path` | Markdown report path under `tmp/test-governance/`. |
| `report_json_path` | JSON report path under `tmp/test-governance/`. |
| `issue_url` | Created Issue URL, empty when Issue publishing is disabled. |

## Scope Mapping

| Scope | Command |
| --- | --- |
| `ci` | `node scripts/node/verify-ci.js` |
| `repo` | `node scripts/node/verify-repo.js` |
| `backend` | `node scripts/node/verify-backend.js` |
| `coverage` | `node scripts/node/verify-coverage.js all` |

All command output is captured to:

- `tmp/test-governance/quality-gate.latest.log`

Existing command-specific warning and coverage files remain in their current locations.

## Report Format

The action writes:

- `tmp/test-governance/quality-gate-report.md`
- `tmp/test-governance/quality-gate-report.json`

The Markdown report contains:

- report type
- status
- scope
- branch
- commit
- actor
- workflow name
- run URL
- environment when provided
- main log path
- warning file list
- coverage summary file list
- artifact reminder

The JSON report mirrors the same fields for later automation.

## Workflow Design

### Automatic CI

The existing verify workflow can be migrated to call the action:

```yaml
name: verify

on:
  pull_request:
  push:
    branches:
      - main

permissions:
  contents: read

jobs:
  verify:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: 22
          cache: pnpm
          cache-dependency-path: web/pnpm-lock.yaml
      - uses: pnpm/action-setup@v4
        with:
          version: 10
      - run: pnpm --dir web install --frozen-lockfile
      - uses: dtolnay/rust-toolchain@stable
      - uses: taiki-e/install-action@cargo-llvm-cov
      - uses: ./.github/actions/quality-gate
        with:
          scope: ci
          report_type: ci
          publish_issue: "false"
      - uses: actions/upload-artifact@v4
        if: always()
        with:
          name: test-governance-artifacts
          path: tmp/test-governance
```

### Manual Quality Report

Add a manual workflow:

```yaml
name: manual quality gate

on:
  workflow_dispatch:
    inputs:
      scope:
        type: choice
        default: ci
        options:
          - ci
          - repo
          - backend
          - coverage
      report_type:
        type: choice
        default: ci
        options:
          - ci
          - cd
      environment:
        type: string
        required: false

permissions:
  contents: read
  issues: write
  actions: read

jobs:
  quality-gate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: 22
          cache: pnpm
          cache-dependency-path: web/pnpm-lock.yaml
      - uses: pnpm/action-setup@v4
        with:
          version: 10
      - run: pnpm --dir web install --frozen-lockfile
      - uses: dtolnay/rust-toolchain@stable
      - uses: taiki-e/install-action@cargo-llvm-cov
      - uses: ./.github/actions/quality-gate
        with:
          scope: ${{ inputs.scope }}
          report_type: ${{ inputs.report_type }}
          environment: ${{ inputs.environment }}
          publish_issue: "true"
          github_token: ${{ secrets.GITHUB_TOKEN }}
      - uses: actions/upload-artifact@v4
        if: always()
        with:
          name: test-governance-artifacts
          path: tmp/test-governance
```

The action must still create an Issue when the gate fails. The workflow step should therefore
run the action in a way that lets report publication execute before the final failure is
returned. The Node runner should record the gate exit code, create reports and Issue, then
exit with the original gate status.

### Future CD

CD workflows may call the same action either before deploy or after smoke tests:

- before deploy: `report_type: cd`, `publish_issue: "false"`
- manual release report: `report_type: cd`, `publish_issue: "true"`

Deployment commands, environment protection, and rollback remain outside the action.

## Error Handling

- Invalid `scope` fails before running any gate.
- `publish_issue: "true"` without `github_token` fails after the report files are written.
- GitHub Issue creation failure fails the action because the manual workflow explicitly asked
  for a report Issue.
- Quality gate failure does not skip report generation.
- The final action exit code matches the quality gate exit code unless Issue publication fails.

## Testing

Add tests around the Node runner:

- scope-to-command mapping
- invalid input handling
- report Markdown generation
- report JSON generation
- Issue title and label generation
- no Issue client call when `publish_issue` is false
- Issue client call when `publish_issue` is true
- failure gate still writes report and attempts Issue creation

The workflow YAML itself is not unit-tested, but the action behavior is covered by script tests.

## Acceptance Evidence

Implementation is complete when:

- `.github/actions/quality-gate/action.yml` exists and calls the Node runner.
- A manual workflow can pass `publish_issue: "true"` and create one new Issue per run.
- Automatic CI can pass `publish_issue: "false"` and never create Issues.
- `tmp/test-governance/quality-gate-report.md` and `.json` are produced on pass and fail.
- Existing `tmp/test-governance/` artifact upload still works.
- Script tests cover the runner behavior.

## Stop Conditions

Stop the first implementation before adding deployment-specific logic. The initial change should
only create the reusable quality gate action, migrate or add the manual workflow, and preserve
the existing CI quality semantics.
