const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');

const {
  collectCapacityReport,
  collectPostgresCapacityInspection,
  formatCapacityMarkdown,
  parseCapacityCliArgs,
  writeCapacityReports,
} = require('../core.js');

function tempRepo() {
  return fs.mkdtempSync(path.join(os.tmpdir(), 'oneflowbase-capacity-report-'));
}

function writeJson(filePath, value) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, `${JSON.stringify(value, null, 2)}\n`, 'utf8');
}

test('collectCapacityReport exposes a metadata field matrix with explicit sources', () => {
  const report = collectCapacityReport({
    repoRoot: tempRepo(),
    schemaReport: {
      tables: [],
      findings: [],
      summary: { findings: 0 },
    },
    growthReport: { tables: [], summary: { findings: 0 } },
    rawJsonbReport: { fields: [], summary: { findings: 0, listRawRisks: 0 } },
    logQueryContractReport: { summary: { findings: 0 }, endpoints: [] },
    inspection: { status: 'skipped', source: 'none', metrics: [], errors: [] },
  });

  const fieldNames = report.metadataFields.map((field) => field.field);
  assert.deepEqual(
    [
      'table_name',
      'table_profile',
      'exemption_reason',
      'growth_risk',
      'jsonb_risk',
      'retention_archive_state',
      'total_size_bytes',
      'table_size_bytes',
      'index_size_bytes',
      'row_estimate',
      'collected_at',
    ].every((fieldName) => fieldNames.includes(fieldName)),
    true
  );

  for (const field of report.metadataFields) {
    assert.match(field.owner, /\S/u);
    assert.match(field.sourceOfTruth, /^(scan|live_postgres|manual_reason|inspection_input)$/u);
    assert.equal(typeof field.persisted, 'boolean');
    assert.equal(typeof field.userEditable, 'boolean');
    assert.match(field.historicalImpact, /\S/u);
    assert.match(field.evidenceSource, /\S/u);
    const forbiddenNames = ['ci' + 'tus', 'pg_' + 'partman'];
    assert.doesNotMatch(JSON.stringify(field), new RegExp(forbiddenNames.join('|'), 'iu'));
  }

  const editableFields = report.metadataFields
    .filter((field) => field.userEditable)
    .map((field) => field.field)
    .sort();
  assert.deepEqual(editableFields, ['exemption_reason', 'retention_archive_state']);
});

test('collectPostgresCapacityInspection reads offline metrics for capacity aggregation', () => {
  const repoRoot = tempRepo();
  const inputPath = path.join(repoRoot, 'tmp', 'fixtures', 'capacity-input.json');
  writeJson(inputPath, {
    version: 'postgres-capacity-inspection/v1',
    metrics: [
      {
        schema_name: 'public',
        table_name: 'runtime_events',
        total_size_bytes: 12582912,
        table_size_bytes: 4194304,
        index_size_bytes: 8388608,
        row_estimate: 25000000,
        collected_at: '2026-06-20T00:00:00.000Z',
      },
    ],
  });

  const inspection = collectPostgresCapacityInspection({ repoRoot, inspectionInputPath: inputPath });

  assert.equal(inspection.status, 'loaded');
  assert.equal(inspection.source, 'inspection_input');
  assert.equal(inspection.metrics[0].schemaName, 'public');
  assert.equal(inspection.metrics[0].tableName, 'runtime_events');
  assert.equal(inspection.metrics[0].totalSizeBytes, 12582912);
  assert.equal(inspection.metrics[0].tableSizeBytes, 4194304);
  assert.equal(inspection.metrics[0].indexSizeBytes, 8388608);
  assert.equal(inspection.metrics[0].rowEstimate, 25000000);
  assert.equal(inspection.metrics[0].collectedAt, '2026-06-20T00:00:00.000Z');
});

test('collectPostgresCapacityInspection classifies live connection and permission failures', () => {
  const connectionFailure = collectPostgresCapacityInspection({
    repoRoot: tempRepo(),
    databaseUrl: 'postgres://example.invalid/db',
    spawnSyncImpl() {
      return {
        status: 2,
        stdout: '',
        stderr: 'psql: error: could not connect to server: Connection refused',
      };
    },
  });

  assert.equal(connectionFailure.status, 'failed');
  assert.equal(connectionFailure.exitCode, 1);
  assert.equal(connectionFailure.errors[0].kind, 'connection_failure');
  assert.match(connectionFailure.errors[0].message, /could not connect/u);

  const permissionFailure = collectPostgresCapacityInspection({
    repoRoot: tempRepo(),
    databaseUrl: 'postgres://localhost/db',
    spawnSyncImpl() {
      return {
        status: 1,
        stdout: '',
        stderr: 'ERROR: permission denied for relation pg_class',
      };
    },
  });

  assert.equal(permissionFailure.errors[0].kind, 'permission_denied');
});

test('collectCapacityReport aggregates hygiene, growth, raw JSONB, and capacity metrics by table', () => {
  const report = collectCapacityReport({
    repoRoot: tempRepo(),
    config: {
      thresholds: {
        partitionCandidateRowEstimate: 100000,
        partitionCandidateTotalSizeBytes: 10485760,
        indexToTableSizeRatio: 2,
      },
      retentionArchiveStates: {
        runtime_events: {
          state: 'manual_review_required',
          reason: 'runtime replay records need retention policy before expansion',
        },
      },
    },
    schemaReport: {
      tables: [
        {
          name: 'runtime_events',
          profile: 'managed_table',
          exemption: null,
          findings: [
            {
              severity: 'error',
              rule: 'managed-table-scope-time-index',
              message: 'requires scope + time index',
            },
          ],
        },
      ],
      findings: [
        {
          severity: 'error',
          table: 'runtime_events',
          rule: 'managed-table-scope-time-index',
          message: 'requires scope + time index',
        },
      ],
      summary: { findings: 1 },
    },
    growthReport: {
      tables: [
        {
          name: 'runtime_events',
          status: 'must_fix',
          growthType: 'runtime_event',
          routingColumns: ['flow_run_id'],
          missingRoutingColumns: ['scope_id'],
          findings: [
            {
              priority: 'must_fix',
              rule: 'missing-routing-column',
              message: 'scope_id is required',
            },
            {
              priority: 'must_fix',
              rule: 'missing-recommended-index',
              message: 'scope replay index is missing',
            },
          ],
          recommendedIndexes: [
            {
              present: false,
              priority: 'must_fix',
              columns: ['scope_id', 'flow_run_id', 'sequence', 'id'],
              scenario: 'workspace runtime event replay',
            },
          ],
          backfill: {
            source: 'flow_runs.application_id -> applications.workspace_id',
            followUpMigrationTask: 'required',
          },
          downtimeRisk: { level: 'high', reason: 'backfill required' },
          constraintReplacementRisk: { level: 'high', reason: 'unique key review required' },
        },
      ],
      summary: { findings: 2 },
    },
    rawJsonbReport: {
      fields: [
        {
          table: 'runtime_events',
          column: 'payload',
          payloadKind: 'raw',
          rawListReadRisk: true,
          findings: [
            {
              rule: 'raw-jsonb-list-read',
              message: 'raw payload appears in list entrypoint',
            },
          ],
        },
      ],
      summary: { findings: 1, listRawRisks: 1 },
    },
    logQueryContractReport: {
      summary: { findings: 0 },
      endpoints: [],
    },
    inspection: {
      status: 'loaded',
      source: 'inspection_input',
      metrics: [
        {
          schemaName: 'public',
          tableName: 'runtime_events',
          totalSizeBytes: 12582912,
          tableSizeBytes: 4194304,
          indexSizeBytes: 8388608,
          rowEstimate: 25000000,
          collectedAt: '2026-06-20T00:00:00.000Z',
        },
      ],
      errors: [],
    },
  });

  assert.equal(report.status, 'warning');
  assert.equal(report.summary.tables, 1);
  assert.equal(report.summary.withCapacityMetrics, 1);
  assert.equal(report.summary.partitionCandidates, 1);
  assert.equal(report.summary.indexRisks, 1);
  assert.equal(report.summary.rawPayloadListRisks, 1);
  assert.equal(report.summary.schemaHygieneFindings, 1);
  assert.equal(report.summary.growthFindings, 2);
  assert.equal(report.summary.rawJsonbFindings, 1);

  const table = report.tables[0];
  assert.equal(table.name, 'runtime_events');
  assert.equal(table.profile, 'managed_table');
  assert.equal(table.capacity.totalSizeBytes, 12582912);
  assert.equal(table.capacity.rowEstimate, 25000000);
  assert.equal(table.retentionArchiveState.state, 'manual_review_required');
  assert.equal(table.sources.capacity, 'inspection_input');
  assert.deepEqual(table.flags, {
    partitionCandidate: true,
    indexRisk: true,
    rawPayloadListRisk: true,
  });
  assert.equal(table.findings.length, 4);
});

test('writeCapacityReports writes stable JSON and Markdown under tmp/test-governance', () => {
  const repoRoot = tempRepo();
  const result = writeCapacityReports({
    repoRoot,
    report: {
      version: 'capacity-report/v1',
      generatedAt: '2026-06-20T00:00:00.000Z',
      status: 'passed',
      exitCode: 0,
      metadataFields: [],
      inspection: { status: 'skipped', source: 'none', metrics: [], errors: [] },
      queryContract: { findings: 0 },
      summary: {
        tables: 0,
        withCapacityMetrics: 0,
        partitionCandidates: 0,
        indexRisks: 0,
        rawPayloadListRisks: 0,
        schemaHygieneFindings: 0,
        growthFindings: 0,
        rawJsonbFindings: 0,
        logQueryFindings: 0,
        inspectionErrors: 0,
      },
      tables: [],
    },
  });

  assert.equal(path.relative(repoRoot, result.reportPath), 'tmp/test-governance/capacity-report.json');
  assert.equal(path.relative(repoRoot, result.markdownPath), 'tmp/test-governance/capacity-report.md');
  assert.equal(fs.existsSync(result.reportPath), true);
  assert.equal(fs.existsSync(result.markdownPath), true);
  assert.match(fs.readFileSync(result.markdownPath, 'utf8'), /# Capacity Report/u);
});

test('formatCapacityMarkdown includes capacity metrics and risk flags', () => {
  const markdown = formatCapacityMarkdown({
    status: 'warning',
    summary: {
      tables: 1,
      withCapacityMetrics: 1,
      partitionCandidates: 1,
      indexRisks: 1,
      rawPayloadListRisks: 1,
      schemaHygieneFindings: 0,
      growthFindings: 0,
      rawJsonbFindings: 0,
      logQueryFindings: 0,
      inspectionErrors: 0,
    },
    inspection: { status: 'loaded', source: 'inspection_input', errors: [] },
    metadataFields: [
      {
        field: 'total_size_bytes',
        owner: 'postgres',
        sourceOfTruth: 'live_postgres',
        persisted: false,
        userEditable: false,
        historicalImpact: 'point-in-time metric',
        evidenceSource: 'pg_total_relation_size',
      },
    ],
    tables: [
      {
        name: 'runtime_events',
        schemaName: 'public',
        profile: 'managed_table',
        capacity: {
          totalSizeBytes: 12582912,
          tableSizeBytes: 4194304,
          indexSizeBytes: 8388608,
          rowEstimate: 25000000,
          collectedAt: '2026-06-20T00:00:00.000Z',
        },
        flags: {
          partitionCandidate: true,
          indexRisk: true,
          rawPayloadListRisk: true,
        },
        findings: [],
      },
    ],
  });

  assert.match(markdown, /runtime_events/u);
  assert.match(markdown, /12582912/u);
  assert.match(markdown, /2026-06-20T00:00:00\.000Z/u);
  assert.match(markdown, /partition_candidate/u);
  assert.match(markdown, /index_risk/u);
  assert.match(markdown, /raw_payload_list_risk/u);
  assert.match(markdown, /total_size_bytes/u);
});

test('parseCapacityCliArgs accepts inspection input and live database options', () => {
  assert.deepEqual(parseCapacityCliArgs(['--inspection-input', 'tmp/size.json']), {
    help: false,
    inspectionInputPath: 'tmp/size.json',
    databaseUrl: null,
    psqlPath: 'psql',
    schema: 'public',
    configPath: 'scripts/node/capacity-report/config.json',
  });

  assert.deepEqual(parseCapacityCliArgs(['--database-url', 'postgres://localhost/db', '--psql', '/bin/psql', '--schema', 'audit']), {
    help: false,
    inspectionInputPath: null,
    databaseUrl: 'postgres://localhost/db',
    psqlPath: '/bin/psql',
    schema: 'audit',
    configPath: 'scripts/node/capacity-report/config.json',
  });

  assert.throws(
    () => parseCapacityCliArgs(['--inspection-input', 'tmp/size.json', '--database-url', 'postgres://localhost/db']),
    /cannot be used together/u
  );
});
