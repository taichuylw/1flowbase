# GitHub Automation

This directory owns GitHub Actions automation for repository quality gates.

## Files

| Path | Purpose |
| --- | --- |
| `.github/workflows/verify.yml` | Automatic CI for `pull_request` and `push` to `main` / `latest`; runs repo slices, backend consistency, coverage slices, and React Doctor frontend gates in parallel, then publishes one aggregate issue only for `latest` pushes. |
| `.github/workflows/quality-gate.yml` | Manual and nightly quality gate run; full `ci` scope runs component gates in parallel before one aggregate Issue report. |
| `.github/actions/quality-gate/action.yml` | Reusable repository-local action used by CI, manual, and nightly quality gates. |

## Automatic CI

`verify.yml` runs automatically on:

- `pull_request`
- `push` to `main`
- `push` to `latest`

It runs local Quality Gate Action jobs in parallel:

```yaml
scope: repo-tooling
scope: repo-frontend
scope: repo-backend-static
scope: repo-backend-fmt
scope: repo-backend-{clippy,check}-{core-libs,runtime-storage,apps}
scope: repo-backend-test-{core-libs,runtime-storage,control-plane,api-server,plugin-runner}
scope: backend-consistency
scope: coverage-frontend
scope: coverage-backend-{control-plane,storage-postgres,api-server}
```

The `repo-tooling` scope includes `repo-hygiene`, which writes
`tmp/test-governance/repo-hygiene.json` with debt-marker, weak-assertion,
duplicate-test-title, file-size, and directory-pressure findings. Advisory findings
remain warnings; focused tests still fail the repo gate.

It also runs React Doctor as a frontend quality gate against `web/app` changed files:

```yaml
run: npx react-doctor@latest web/app --diff main --offline --fail-on warning --verbose
```

Current React Doctor structural debt is kept in `web/app/react-doctor.config.json`
as narrow per-file rule overrides, so React Doctor still blocks new warnings
outside that explicit baseline.

The final aggregate job downloads the component artifacts and publishes a single
report with:

```yaml
INPUT_PUBLISH_ISSUE: ${{ github.event_name == 'push' && github.ref == 'refs/heads/latest' }}
```

Automatic CI creates a GitHub Issue only for `latest` branch pushes and uploads the
merged `tmp/test-governance` directory as the `test-governance-artifacts` artifact. The
issue body includes the aggregate result summary, component status table, warning status,
coverage percentages, evidence paths, and a failure excerpt when a component gate fails.
Use the artifact for full logs and raw coverage files.
Runs use branch-level concurrency, so a newer push cancels an older in-progress quality gate
for the same branch before stale runs can publish or close quality issues.

## Manual And Nightly Quality Gate

`quality-gate.yml` is triggered from GitHub Actions with `workflow_dispatch` and by a daily
schedule at 18:00 UTC, which is 02:00 Asia/Shanghai.

Recommended first run:

```text
scope: ci
report_type: ci
environment: leave empty
```

For `scope: ci`, manual and scheduled runs mirror the automatic CI shape: repo tooling,
repo frontend, backend static/fmt/package shards, backend app test package shards, backend
consistency, frontend coverage, and backend coverage package shards run as separate jobs.
An aggregate job downloads their artifacts, publishes one Issue report, and uploads
`test-governance-artifacts`.
This keeps wall time close to the slowest component gate instead of the sum of all gates.
Each component job publishes `publish_issue: "false"`; only the aggregate job publishes the
final report with `publish_issue: "true"`.

For narrower dispatch scopes such as `repo-frontend`, `repo-backend`, `backend-consistency`,
or `coverage-backend`,
`quality-gate.yml` runs one targeted job and publishes that single-scope report directly.
Manual runs share the same target-branch concurrency group as automatic quality gates.
Scheduled runs target `latest`, use `scope: ci`, and set `environment: nightly-latest`.

## Scope Options

| Scope | Command |
| --- | --- |
| `ci` | GitHub workflow only: parallel repo slices + `backend-consistency` + coverage slices, then `github-quality-gate-aggregate.js` |
| `repo` | `node scripts/node/verify-repo.js` |
| `repo-tooling` | `node scripts/node/verify-repo.js tooling` |
| `repo-frontend` | `node scripts/node/verify-repo.js frontend` |
| `repo-backend` | `node scripts/node/verify-repo.js backend` |
| `repo-backend-static` | `node scripts/node/verify-backend.js static` |
| `repo-backend-fmt` | `node scripts/node/verify-backend.js fmt` |
| `repo-backend-clippy-core-libs` | `node scripts/node/verify-backend.js clippy core-libs` |
| `repo-backend-clippy-runtime-storage` | `node scripts/node/verify-backend.js clippy runtime-storage` |
| `repo-backend-clippy-apps` | `node scripts/node/verify-backend.js clippy apps` |
| `repo-backend-test-core-libs` | `node scripts/node/verify-backend.js test core-libs` |
| `repo-backend-test-runtime-storage` | `node scripts/node/verify-backend.js test runtime-storage` |
| `repo-backend-test-control-plane` | `node scripts/node/verify-backend.js test control-plane` |
| `repo-backend-test-api-server` | `node scripts/node/verify-backend.js test api-server` |
| `repo-backend-test-plugin-runner` | `node scripts/node/verify-backend.js test plugin-runner` |
| `repo-backend-check-core-libs` | `node scripts/node/verify-backend.js check core-libs` |
| `repo-backend-check-runtime-storage` | `node scripts/node/verify-backend.js check runtime-storage` |
| `repo-backend-check-apps` | `node scripts/node/verify-backend.js check apps` |
| `backend` | `node scripts/node/verify-backend.js` |
| `backend-consistency` | `node scripts/node/cli/verify-backend-consistency.js` |
| `coverage` | `node scripts/node/verify-coverage.js all` |
| `coverage-frontend` | `node scripts/node/verify-coverage.js frontend` |
| `coverage-backend` | `node scripts/node/verify-coverage.js backend` |
| `coverage-backend-control-plane` | `node scripts/node/verify-coverage.js backend control-plane` |
| `coverage-backend-storage-postgres` | `node scripts/node/verify-coverage.js backend storage-postgres` |
| `coverage-backend-api-server` | `node scripts/node/verify-coverage.js backend api-server` |

Use `ci` for the full online repository quality gate. The local `node scripts/node/verify-ci.js`
entry still runs the same gates serially for environments that need one local command, but the
GitHub workflow intentionally splits `ci` across jobs for faster feedback and clearer artifacts.
Use narrower scopes only when debugging or when a faster targeted report is enough. Do not run
the backend consistency scope locally unless explicitly requested, because it exercises
database-backed Rust suites.

## Report Type

| `report_type` | Use When |
| --- | --- |
| `ci` | Code quality, regression, or repository validation. |
| `cd` | Deployment or release validation. |

For `cd` reports, set `environment` to a value such as `staging` or `production`.

## Evidence

The Quality Gate Action writes:

- `tmp/test-governance/quality-gate.latest.log`
- `tmp/test-governance/quality-gate-report.md`
- `tmp/test-governance/quality-gate-report.json`
- `tmp/test-governance/repo-hygiene.json` for `repo`, `repo-tooling`, and `ci`
- `tmp/test-governance/backend-consistency-targets.json` for `ci` and `backend-consistency`

Existing warning, coverage, screenshot, and QA evidence files remain under
`tmp/test-governance/`.
For `ci` and `backend-consistency` scopes, the report also includes the current backend
consistency target results: label, Cargo package, Rust test filter, status, duration,
passed count, and failed count. If the target result artifact is unavailable, the report
falls back to the static target registry with `not_run` status.

## Setup Order

Workflows must install pnpm before enabling `setup-node` pnpm cache:

```yaml
- uses: pnpm/action-setup@v5
- uses: actions/setup-node@v5
  with:
    cache: pnpm
```

Putting `setup-node` before `pnpm/action-setup` can fail with `Unable to locate executable
file: pnpm`.

Workflows opt JavaScript actions into GitHub's Node 24 runtime with:

```yaml
FORCE_JAVASCRIPT_ACTIONS_TO_NODE24: true
```

This keeps hosted-action runtime annotations aligned with the repository's Node 24 test runtime.
