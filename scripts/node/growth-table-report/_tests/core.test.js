const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');

const {
  collectGrowthTableReport,
  formatGrowthTableMarkdown,
  loadConfig,
  writeGrowthTableReports,
} = require('../core.js');

function writeFile(repoRoot, relativePath, content) {
  const absolutePath = path.join(repoRoot, relativePath);
  fs.mkdirSync(path.dirname(absolutePath), { recursive: true });
  fs.writeFileSync(absolutePath, content, 'utf8');
}

function createRepoWithMigration(sql, source = '') {
  const repoRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'oneflowbase-growth-report-'));
  writeFile(
    repoRoot,
    'api/crates/storage-durable/postgres/migrations/20260101000000_fixture.sql',
    sql
  );
  if (source.length > 0) {
    writeFile(
      repoRoot,
      'api/crates/storage-durable/postgres/src/runtime_queries.rs',
      source
    );
  }
  return repoRoot;
}

test('collectGrowthTableReport flags missing routing columns, unsafe uniqueness, and missing indexes', () => {
  const repoRoot = createRepoWithMigration(`
    create table runtime_events (
      id uuid primary key,
      flow_run_id uuid not null,
      sequence bigint not null,
      payload jsonb not null default '{}'::jsonb,
      created_at timestamptz not null default now(),
      unique(flow_run_id, sequence)
    );

    create index runtime_events_flow_sequence_idx
      on runtime_events (flow_run_id, sequence asc);
  `, `
    async fn list_runtime_events() {
      sqlx::query("select * from runtime_events where flow_run_id = $1 order by sequence asc");
    }
  `);

  const report = collectGrowthTableReport({
    repoRoot,
    config: {
      sourceSearchDirs: ['api/crates/storage-durable/postgres/src'],
      tables: [
        {
          name: 'runtime_events',
          growthType: 'runtime_event',
          requiredRoutingColumns: ['scope_id', 'flow_run_id'],
          requiredTimeColumns: ['created_at'],
          ownerColumns: ['flow_run_id'],
          uniqueRouteKeys: ['scope_id'],
          recommendedIndexes: [
            {
              columns: ['scope_id', 'flow_run_id', 'sequence', 'id'],
              scenario: 'workspace-scoped runtime event replay by flow run cursor',
              priority: 'must_fix',
            },
          ],
          backfill: {
            source: 'flow_runs.application_id -> applications.workspace_id',
            batchBoundary: 'flow_run_id ranges',
            failureRecovery: 'only update rows where scope_id is null',
          },
        },
      ],
    },
  });

  assert.equal(report.status, 'warning');
  assert.equal(report.summary.mustFix, 1);
  assert.equal(report.summary.later, 0);

  const table = report.tables[0];
  assert.equal(table.status, 'must_fix');
  assert.deepEqual(table.jsonbColumns, ['payload']);
  assert.deepEqual(
    table.findings.map((finding) => finding.rule),
    [
      'missing-routing-column',
      'unique-constraint-routing-key',
      'unique-constraint-routing-key',
      'missing-recommended-index',
      'raw-jsonb-review',
    ]
  );
  assert.equal(table.recommendedIndexes[0].present, false);
  assert.equal(table.recommendedIndexes[0].scenario, 'workspace-scoped runtime event replay by flow run cursor');
  assert.equal(table.queryEntrypoints[0].functionName, 'list_runtime_events');
  assert.equal(table.readEntrypoints[0].functionName, 'list_runtime_events');
  assert.deepEqual(table.writeEntrypoints, []);
  assert.equal(table.backfill.source, 'flow_runs.application_id -> applications.workspace_id');
  assert.equal(table.backfill.followUpMigrationTask, 'required');
  assert.equal(table.downtimeRisk.level, 'high');
  assert.equal(table.constraintReplacementRisk.level, 'high');
});

test('collectGrowthTableReport keeps routed tables without missing recommendations in ok status', () => {
  const repoRoot = createRepoWithMigration(`
    create table application_run_log_summaries (
      flow_run_id uuid primary key,
      scope_id uuid not null,
      application_id uuid not null,
      status text not null,
      created_at timestamptz not null default now(),
      started_at timestamptz not null,
      updated_at timestamptz not null
    );

    create index application_run_log_summaries_scope_application_idx
      on application_run_log_summaries (scope_id, application_id, created_at desc, flow_run_id desc);
  `);

  const report = collectGrowthTableReport({
    repoRoot,
    config: {
      tables: [
        {
          name: 'application_run_log_summaries',
          growthType: 'log_summary',
          requiredRoutingColumns: ['scope_id', 'application_id', 'flow_run_id'],
          requiredTimeColumns: ['created_at', 'started_at'],
          uniqueRouteKeys: ['flow_run_id'],
          recommendedIndexes: [
            {
              columns: ['scope_id', 'application_id', 'created_at', 'flow_run_id'],
              scenario: 'workspace application run log list',
              priority: 'must_fix',
            },
          ],
        },
      ],
    },
  });

  assert.equal(report.status, 'passed');
  assert.equal(report.summary.ok, 1);
  assert.equal(report.tables[0].status, 'ok');
  assert.deepEqual(report.tables[0].findings, []);
  assert.equal(report.tables[0].recommendedIndexes[0].present, true);
  assert.equal(report.tables[0].downtimeRisk.level, 'low');
  assert.equal(report.tables[0].constraintReplacementRisk.level, 'none');
});

test('collectGrowthTableReport splits write and read entrypoints from SQL evidence', () => {
  const repoRoot = createRepoWithMigration(`
    create table runtime_events (
      id uuid primary key,
      scope_id uuid not null,
      flow_run_id uuid not null,
      sequence bigint not null,
      created_at timestamptz not null default now()
    );
  `, `
    async fn append_runtime_event() {
      sqlx::query("insert into runtime_events (id, scope_id, flow_run_id, sequence) values ($1, $2, $3, $4)");
    }

    async fn list_runtime_events() {
      sqlx::query("select * from runtime_events where scope_id = $1 order by sequence asc");
    }
  `);

  const report = collectGrowthTableReport({
    repoRoot,
    config: {
      sourceSearchDirs: ['api/crates/storage-durable/postgres/src'],
      tables: [
        {
          name: 'runtime_events',
          growthType: 'runtime_event',
          requiredRoutingColumns: ['scope_id', 'flow_run_id'],
          requiredTimeColumns: ['created_at'],
          uniqueRouteKeys: ['scope_id'],
          recommendedIndexes: [],
        },
      ],
    },
  });

  const table = report.tables[0];
  assert.equal(table.writeEntrypoints[0].functionName, 'append_runtime_event');
  assert.equal(table.readEntrypoints[0].functionName, 'list_runtime_events');
});

test('writeGrowthTableReports writes JSON and Markdown under tmp/test-governance', () => {
  const repoRoot = createRepoWithMigration(`
    create table runtime_debug_artifacts (
      id uuid primary key,
      workspace_id uuid not null,
      application_id uuid not null,
      flow_run_id uuid,
      artifact_kind text not null,
      created_at timestamptz not null default now()
    );

    create index runtime_debug_artifacts_workspace_created_idx
      on runtime_debug_artifacts (workspace_id, created_at desc, id desc);
  `);

  const result = writeGrowthTableReports({
    repoRoot,
    config: {
      tables: [
        {
          name: 'runtime_debug_artifacts',
          growthType: 'artifact',
          requiredRoutingColumns: ['workspace_id', 'application_id', 'flow_run_id'],
          requiredTimeColumns: ['created_at'],
          uniqueRouteKeys: ['workspace_id'],
          recommendedIndexes: [
            {
              columns: ['workspace_id', 'created_at', 'id'],
              scenario: 'workspace artifact retention and capacity inspection',
              priority: 'must_fix',
            },
          ],
        },
      ],
    },
  });

  assert.equal(result.reportPath, path.join(repoRoot, 'tmp', 'test-governance', 'growth-table-report.json'));
  assert.equal(result.markdownPath, path.join(repoRoot, 'tmp', 'test-governance', 'growth-table-report.md'));
  assert.equal(fs.existsSync(result.reportPath), true);
  assert.equal(fs.existsSync(result.markdownPath), true);

  const markdown = formatGrowthTableMarkdown(result.report);
  assert.match(markdown, /# Growth Table Routing Report/u);
  assert.match(markdown, /runtime_debug_artifacts/u);
  assert.match(markdown, /workspace artifact retention and capacity inspection/u);
  assert.match(markdown, /## Write Path Evidence/u);
  assert.match(markdown, /## Read Path Evidence/u);
  assert.match(markdown, /## Downtime And Constraint Risk/u);
});

test('default config covers issue-required and active legacy high-growth tables', () => {
  const repoRoot = path.resolve(__dirname, '..', '..', '..', '..');
  const config = loadConfig(repoRoot);
  const tableNames = new Set(config.tables.map((table) => table.name));

  for (const tableName of [
    'flow_runs',
    'flow_run_events',
    'runtime_events',
    'runtime_debug_artifacts',
    'application_run_log_summaries',
    'application_public_conversations',
  ]) {
    assert.equal(tableNames.has(tableName), true, `${tableName} must be covered`);
  }
});
