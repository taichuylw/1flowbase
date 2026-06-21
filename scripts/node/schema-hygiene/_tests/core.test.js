const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');

const {
  collectSchemaInventory,
  evaluateSchemaHygiene,
  loadConfig,
  main,
} = require('../core.js');

function writeFile(repoRoot, relativePath, content) {
  const absolutePath = path.join(repoRoot, relativePath);
  fs.mkdirSync(path.dirname(absolutePath), { recursive: true });
  fs.writeFileSync(absolutePath, content, 'utf8');
}

function createRepoWithMigration(sql) {
  const repoRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'oneflowbase-schema-hygiene-'));
  writeFile(
    repoRoot,
    'api/crates/storage-durable/postgres/migrations/20260101000000_fixture.sql',
    sql
  );
  return repoRoot;
}

test('collectSchemaInventory reads tables, columns, constraints, indexes, FKs, and JSONB fields from migrations', () => {
  const repoRoot = createRepoWithMigration(`
    create table workspaces (
      id uuid primary key,
      created_at timestamptz not null default now(),
      updated_at timestamptz not null default now()
    );

    create table example_events (
      id uuid primary key,
      workspace_id uuid not null references workspaces(id) on delete cascade,
      payload jsonb not null default '{}'::jsonb,
      sequence bigint not null,
      created_at timestamptz not null default now(),
      unique(workspace_id, sequence)
    );

    create index example_events_workspace_created_idx
      on example_events (workspace_id, created_at desc, id desc);
  `);

  const inventory = collectSchemaInventory({ repoRoot });
  const table = inventory.tables.find((candidate) => candidate.name === 'example_events');

  assert.ok(table);
  assert.deepEqual(table.primaryKey.columns, ['id']);
  assert.deepEqual(table.jsonbColumns, ['payload']);
  assert.deepEqual(
    table.columns.map((column) => column.name),
    ['id', 'workspace_id', 'payload', 'sequence', 'created_at']
  );
  assert.equal(table.foreignKeys[0].columns[0], 'workspace_id');
  assert.equal(table.foreignKeys[0].references.table, 'workspaces');
  assert.equal(table.uniqueConstraints[0].columns[0], 'workspace_id');
  assert.deepEqual(table.indexes[0].columns, ['workspace_id', 'created_at', 'id']);
});

test('evaluateSchemaHygiene treats unmarked tables as managed_table and reports fail rules', () => {
  const repoRoot = createRepoWithMigration(`
    create table audit_events (
      id uuid primary key,
      payload jsonb not null default '{}'::jsonb,
      created_at timestamptz not null default now()
    );
  `);

  const inventory = collectSchemaInventory({ repoRoot });
  const report = evaluateSchemaHygiene({ inventory });

  assert.equal(report.tables[0].profile, 'managed_table');
  assert.deepEqual(
    report.findings.map((finding) => finding.rule),
    [
      'managed-table-updated-at-or-append-only',
      'managed-table-scope-column',
      'managed-table-scope-time-index',
    ]
  );
  assert.equal(report.summary.errors, 3);
});

test('evaluateSchemaHygiene requires scope_id even when workspace_id is present', () => {
  const repoRoot = createRepoWithMigration(`
    create table workspace_events (
      id uuid primary key,
      workspace_id uuid not null,
      payload jsonb not null default '{}'::jsonb,
      created_at timestamptz not null default now(),
      updated_at timestamptz not null default now()
    );

    create index workspace_events_workspace_created_idx
      on workspace_events (workspace_id, created_at desc, id desc);
  `);

  const inventory = collectSchemaInventory({ repoRoot });
  const report = evaluateSchemaHygiene({ inventory });

  assert.deepEqual(
    report.findings.map((finding) => finding.rule),
    [
      'managed-table-scope-column',
      'managed-table-scope-time-index',
    ]
  );
});

test('evaluateSchemaHygiene requires scope_id created_at id index for expansion readiness', () => {
  const repoRoot = createRepoWithMigration(`
    create table scoped_events (
      id uuid primary key,
      scope_id uuid not null,
      payload jsonb not null default '{}'::jsonb,
      created_at timestamptz not null default now(),
      updated_at timestamptz not null default now()
    );

    create index scoped_events_scope_updated_idx
      on scoped_events (scope_id, updated_at desc, id desc);
  `);

  const inventory = collectSchemaInventory({ repoRoot });
  const report = evaluateSchemaHygiene({ inventory });

  assert.deepEqual(
    report.findings.map((finding) => finding.rule),
    ['managed-table-scope-time-index']
  );
});

test('evaluateSchemaHygiene passes managed table with required expansion fields and index', () => {
  const repoRoot = createRepoWithMigration(`
    create table scoped_events (
      id uuid primary key,
      scope_id uuid not null,
      payload jsonb not null default '{}'::jsonb,
      created_at timestamptz not null default now(),
      updated_at timestamptz not null default now()
    );

    create index scoped_events_scope_created_idx
      on scoped_events (scope_id, created_at desc, id desc);
  `);

  const inventory = collectSchemaInventory({ repoRoot });
  const report = evaluateSchemaHygiene({ inventory });

  assert.deepEqual(report.findings, []);
  assert.equal(report.summary.errors, 0);
});

test('evaluateSchemaHygiene reports platform readiness matrix and stable actions', () => {
  const repoRoot = createRepoWithMigration(`
    create table workspace_events (
      id uuid primary key,
      workspace_id uuid not null,
      payload jsonb not null default '{}'::jsonb,
      created_at timestamptz not null default now()
    );
  `);

  const inventory = collectSchemaInventory({ repoRoot });
  const report = evaluateSchemaHygiene({ inventory });
  const readiness = report.tables[0].platformReadiness;

  assert.equal(readiness.category, 'unknown_needs_review');
  assert.equal(readiness.fields.id.present, true);
  assert.equal(readiness.fields.scope_id.present, false);
  assert.equal(readiness.fields.workspace_id.present, true);
  assert.equal(readiness.fields.created_at.present, true);
  assert.equal(readiness.fields.updated_at.present, false);
  assert.equal(readiness.scopeGenerationSource.status, 'inferred');
  assert.equal(readiness.scopeGenerationSource.source, 'workspace_id');
  assert.deepEqual(readiness.missingFields, ['scope_id', 'updated_at', 'created_by', 'updated_by']);
  assert.deepEqual(readiness.recommendedActions, [
    'add_updated_at',
    'add_scope_id',
    'backfill_scope_id',
    'add_scope_time_index',
    'declare_generation_rule',
  ]);
});

test('evaluateSchemaHygiene marks missing scope source as needs_owner_review', () => {
  const repoRoot = createRepoWithMigration(`
    create table ownerless_events (
      id uuid primary key,
      payload jsonb not null default '{}'::jsonb,
      created_at timestamptz not null default now()
    );
  `);

  const inventory = collectSchemaInventory({ repoRoot });
  const report = evaluateSchemaHygiene({ inventory });
  const readiness = report.tables[0].platformReadiness;

  assert.equal(readiness.scopeGenerationSource.status, 'needs_owner_review');
  assert.equal(readiness.scopeGenerationSource.source, null);
  assert.equal(readiness.backfillSource, null);
  assert.deepEqual(readiness.recommendedActions, ['needs_owner_review']);
  assert.equal(readiness.recommendedActions.includes('backfill_scope_id'), false);
});

test('evaluateSchemaHygiene stops migration actions when id or created_at needs owner review', () => {
  const repoRoot = createRepoWithMigration(`
    create table workspace_events (
      workspace_id uuid not null,
      payload jsonb not null default '{}'::jsonb,
      updated_at timestamptz not null default now()
    );
  `);

  const inventory = collectSchemaInventory({ repoRoot });
  const report = evaluateSchemaHygiene({ inventory });
  const readiness = report.tables[0].platformReadiness;

  assert.deepEqual(readiness.recommendedActions, [
    'add_id',
    'needs_owner_review',
  ]);
  assert.equal(readiness.recommendedActions.includes('backfill_scope_id'), false);
  assert.equal(readiness.recommendedActions.includes('add_scope_time_index'), false);
});

test('evaluateSchemaHygiene can explicitly stop a table at needs_owner_review warning', () => {
  const repoRoot = createRepoWithMigration(`
    create table ownerless_events (
      id uuid primary key,
      payload jsonb not null default '{}'::jsonb,
      created_at timestamptz not null default now()
    );
  `);

  const inventory = collectSchemaInventory({ repoRoot });
  const report = evaluateSchemaHygiene({
    inventory,
    config: {
      needsOwnerReviewTables: {
        ownerless_events: 'owner relation is not declared enough for automated migration',
      },
    },
  });

  assert.equal(report.summary.errors, 0);
  assert.equal(report.summary.warnings, 1);
  assert.equal(report.findings[0].rule, 'managed-table-needs-owner-review');
  assert.deepEqual(report.tables[0].platformReadiness.recommendedActions, ['needs_owner_review']);
});

test('evaluateSchemaHygiene lets bounded plugin projections use explicit exemptions', () => {
  const catalogReason = 'bounded plugin catalog projection keyed by plugin installation';
  const artifactReason = 'bounded plugin artifact projection keyed by installation and node';
  const repoRoot = createRepoWithMigration(`
    create table plugin_package_catalog_projection (
      installation_id uuid primary key,
      package_code text not null,
      package_version text not null,
      catalog_snapshot_json jsonb not null default '{}'::jsonb,
      projection_status text not null,
      refreshed_at timestamptz,
      updated_at timestamptz not null default now()
    );

    create table plugin_artifact_instances (
      node_id text not null,
      installation_id uuid not null,
      artifact_status text not null,
      runtime_status text not null default 'inactive',
      checked_at timestamptz not null default now(),
      primary key (node_id, installation_id)
    );
  `);

  const inventory = collectSchemaInventory({ repoRoot });
  const report = evaluateSchemaHygiene({
    inventory,
    config: {
      exemptions: {
        plugin_package_catalog_projection: {
          kind: 'bounded_projection',
          reason: catalogReason,
          skip: [
            'managed-table-id',
            'managed-table-created-at',
            'managed-table-scope-column',
            'managed-table-scope-time-index',
          ],
        },
        plugin_artifact_instances: {
          kind: 'bounded_projection',
          reason: artifactReason,
          skip: [
            'managed-table-id',
            'managed-table-created-at',
            'managed-table-updated-at-or-append-only',
            'managed-table-scope-column',
            'managed-table-scope-time-index',
          ],
        },
      },
    },
  });

  assert.equal(report.summary.errors, 0);

  for (const [tableName, reason] of [
    ['plugin_package_catalog_projection', catalogReason],
    ['plugin_artifact_instances', artifactReason],
  ]) {
    const table = report.tables.find((candidate) => candidate.name === tableName);
    assert.equal(table.exemption.kind, 'bounded_projection');
    assert.equal(table.exemption.reason, reason);
    assert.deepEqual(table.findings, []);
    assert.equal(table.platformReadiness.category, 'bounded_projection');
    assert.equal(table.platformReadiness.severity, 'ok');
    assert.equal(table.platformReadiness.reason, reason);
    assert.deepEqual(table.platformReadiness.recommendedActions, ['bounded_projection_exempt']);
  }
});

test('evaluateSchemaHygiene uses default generation declarations for complete tables', () => {
  const repoRoot = createRepoWithMigration(`
    create table scoped_events (
      id uuid primary key,
      scope_id uuid not null,
      payload jsonb not null default '{}'::jsonb,
      created_by uuid,
      updated_by uuid,
      created_at timestamptz not null default now(),
      updated_at timestamptz not null default now()
    );

    create index scoped_events_scope_created_idx
      on scoped_events (scope_id, created_at desc, id desc);
  `);

  const inventory = collectSchemaInventory({ repoRoot });
  const report = evaluateSchemaHygiene({
    inventory,
    config: {
      defaultTableReadiness: {
        idGeneration: 'application write path supplies uuidv7 ids',
        scopeGenerationSource: 'repository write path binds scope_id',
        created_byGeneration: 'actor context or nullable system path',
        updated_byGeneration: 'actor context or nullable system path',
        writePathSource: 'repository write path owns platform fields',
      },
    },
  });

  assert.deepEqual(report.tables[0].platformReadiness.recommendedActions, ['no_action']);
});

test('collectSchemaInventory rejects empty schema instead of silently passing', () => {
  const repoRoot = createRepoWithMigration('select 1;');

  assert.throws(
    () => collectSchemaInventory({ repoRoot }),
    /No PostgreSQL tables discovered/u
  );
});

test('collectSchemaInventory reports unsupported table elements and alter actions', () => {
  const repoRoot = createRepoWithMigration(`
    create table copied_table (
      like base_table including all
    );
    create table editable_table (
      id uuid primary key
    );
    alter table editable_table alter column id type text;
  `);

  const inventory = collectSchemaInventory({ repoRoot });

  assert.deepEqual(
    inventory.parseErrors.map((parseError) => parseError.rule),
    ['unsupported-table-element', 'unsupported-alter-table-action']
  );
});

test('collectSchemaInventory applies supported alter column nullability and default actions', () => {
  const repoRoot = createRepoWithMigration(`
    create table editable_table (
      id uuid primary key,
      label text
    );
    alter table editable_table
      alter column label set not null,
      alter column label set default 'untitled';
  `);

  const inventory = collectSchemaInventory({ repoRoot });
  const table = inventory.tables.find((candidate) => candidate.name === 'editable_table');
  const label = table.columns.find((column) => column.name === 'label');

  assert.equal(inventory.parseErrors.length, 0);
  assert.equal(label.nullable, false);
  assert.equal(label.default, true);
});

test('evaluateSchemaHygiene fails exemptions without a reason', () => {
  const repoRoot = createRepoWithMigration(`
    create table tiny_catalog (
      code text primary key,
      label text not null
    );
  `);

  const inventory = collectSchemaInventory({ repoRoot });
  const report = evaluateSchemaHygiene({
    inventory,
    config: {
      exemptions: {
        tiny_catalog: {
          skip: ['managed_table'],
        },
      },
    },
  });

  assert.equal(report.findings[0].rule, 'exemption-reason-required');
  assert.equal(report.findings[0].severity, 'error');
});

test('evaluateSchemaHygiene fails vague reasons and broad exemption skips', () => {
  const repoRoot = createRepoWithMigration(`
    create table tiny_catalog (
      code text primary key,
      label text not null
    );
  `);

  const inventory = collectSchemaInventory({ repoRoot });
  const report = evaluateSchemaHygiene({
    inventory,
    config: {
      exemptions: {
        tiny_catalog: {
          reason: 'misc',
          skip: ['managed_table'],
        },
      },
    },
  });

  assert.deepEqual(
    report.findings.map((finding) => finding.rule).slice(0, 2),
    ['exemption-reason-format', 'exemption-skip-too-broad']
  );
  assert.equal(report.summary.errors > 2, true);
});

test('evaluateSchemaHygiene rejects forbidden reason words at boundaries and punctuation', () => {
  const repoRoot = createRepoWithMigration(`
    create table tiny_catalog (
      code text primary key,
      label text not null
    );
  `);
  const inventory = collectSchemaInventory({ repoRoot });

  for (const reason of [
    'misc schema ledger exception',
    'schema ledger exception misc',
    'misc: schema ledger exception',
    'legacy schema ledger exception',
    'todo schema ledger exception',
  ]) {
    const report = evaluateSchemaHygiene({
      inventory,
      config: {
        exemptions: {
          tiny_catalog: {
            reason,
            skip: ['id'],
          },
        },
      },
    });

    assert.equal(report.findings[0].rule, 'exemption-reason-format');
  }
});

test('evaluateSchemaHygiene checks dynamic_model_table scope_id and index rules', () => {
  const repoRoot = createRepoWithMigration(`
    create table modeled_rows (
      id uuid primary key,
      created_at timestamptz not null default now(),
      updated_at timestamptz not null default now()
    );
  `);

  const inventory = collectSchemaInventory({ repoRoot });
  const report = evaluateSchemaHygiene({
    inventory,
    config: {
      tableProfiles: {
        modeled_rows: 'dynamic_model_table',
      },
    },
  });

  assert.deepEqual(
    report.findings.map((finding) => finding.rule),
    [
      'dynamic-model-scope-column',
      'dynamic-model-scope-time-index',
    ]
  );
});

test('evaluateSchemaHygiene does not let managed_table skip bypass dynamic_model_table rules', () => {
  const repoRoot = createRepoWithMigration(`
    create table modeled_rows (
      id uuid primary key,
      created_at timestamptz not null default now(),
      updated_at timestamptz not null default now()
    );
  `);

  const inventory = collectSchemaInventory({ repoRoot });
  const report = evaluateSchemaHygiene({
    inventory,
    config: {
      tableProfiles: {
        modeled_rows: 'dynamic_model_table',
      },
      exemptions: {
        modeled_rows: {
          reason: 'bounded fixture table for exemption validation',
          skip: ['managed_table'],
        },
      },
    },
  });

  assert.deepEqual(
    report.findings.map((finding) => finding.rule),
    [
      'exemption-skip-too-broad',
      'dynamic-model-scope-column',
      'dynamic-model-scope-time-index',
    ]
  );
});

test('evaluateSchemaHygiene keeps registered_system_table profile visible and scanned', () => {
  const repoRoot = createRepoWithMigration(`
    create table registered_catalog (
      id uuid primary key,
      created_at timestamptz not null default now()
    );
  `);

  const inventory = collectSchemaInventory({ repoRoot });
  const report = evaluateSchemaHygiene({
    inventory,
    config: {
      registeredSystemTables: ['registered_catalog'],
      registeredSystemTableTemplates: {
        registered_catalog: {
          requiredColumns: ['id', 'created_at', 'fixed_code'],
        },
      },
    },
  });

  assert.equal(report.tables[0].profile, 'registered_system_table');
  assert.deepEqual(
    report.findings.map((finding) => finding.rule),
    [
      'registered-system-table-required-column',
      'managed-table-updated-at-or-append-only',
      'managed-table-scope-column',
      'managed-table-scope-time-index',
    ]
  );
});

test('evaluateSchemaHygiene fails registered_system_table without fixed template declaration', () => {
  const repoRoot = createRepoWithMigration(`
    create table registered_catalog (
      id uuid primary key,
      scope_id uuid not null,
      fixed_code text not null,
      created_at timestamptz not null default now(),
      updated_at timestamptz not null default now()
    );
    create index registered_catalog_scope_created_idx
      on registered_catalog (scope_id, created_at desc, id desc);
  `);

  const inventory = collectSchemaInventory({ repoRoot });
  const report = evaluateSchemaHygiene({
    inventory,
    config: {
      registeredSystemTables: ['registered_catalog'],
    },
  });

  assert.equal(report.tables[0].profile, 'registered_system_table');
  assert.deepEqual(
    report.findings.map((finding) => finding.rule),
    ['registered-system-table-template-missing']
  );
});

test('default schema hygiene config exempts only bounded plugin projection tables', () => {
  const repoRoot = path.resolve(__dirname, '..', '..', '..', '..');
  const inventory = collectSchemaInventory({ repoRoot });
  const report = evaluateSchemaHygiene({
    inventory,
    config: loadConfig(repoRoot),
  });

  for (const tableName of ['plugin_package_catalog_projection', 'plugin_artifact_instances']) {
    const table = report.tables.find((candidate) => candidate.name === tableName);
    assert.equal(table.exemption.kind, 'bounded_projection');
    assert.equal(table.findings.some((finding) => finding.rule === 'managed-table-needs-owner-review'), false);
    assert.equal(table.platformReadiness.recommendedActions.includes('needs_owner_review'), false);
    assert.deepEqual(table.platformReadiness.recommendedActions, ['bounded_projection_exempt']);
  }

  for (const tableName of ['plugin_tasks', 'plugin_installations', 'plugin_worker_leases']) {
    const table = report.tables.find((candidate) => candidate.name === tableName);
    assert.ok(table.findings.some((finding) => finding.rule === 'managed-table-needs-owner-review'));
    assert.deepEqual(table.platformReadiness.recommendedActions, ['needs_owner_review']);
  }
});

test('default schema hygiene config declares trace projection owner chain', () => {
  const repoRoot = path.resolve(__dirname, '..', '..', '..', '..');
  const inventory = collectSchemaInventory({ repoRoot });
  const report = evaluateSchemaHygiene({
    inventory,
    config: loadConfig(repoRoot),
  });

  for (const tableName of [
    'application_run_trace_projection_statuses',
    'application_run_trace_nodes',
    'application_run_trace_node_contents',
  ]) {
    const table = report.tables.find((candidate) => candidate.name === tableName);
    assert.equal(
      table.findings.some((finding) => finding.rule === 'managed-table-needs-owner-review'),
      false
    );
    assert.equal(table.platformReadiness.recommendedActions.includes('needs_owner_review'), false);
    assert.equal(table.platformReadiness.category, 'join_or_child');
    assert.equal(table.platformReadiness.scopeGenerationSource.status, 'declared');
    assert.match(table.platformReadiness.scopeGenerationSource.source, /flow_runs\.scope_id/u);
  }
});

test('default schema hygiene config declares issue 1074 owner-confirmed scoped readiness tables', () => {
  const repoRoot = path.resolve(__dirname, '..', '..', '..', '..');
  const inventory = collectSchemaInventory({ repoRoot });
  const report = evaluateSchemaHygiene({
    inventory,
    config: loadConfig(repoRoot),
  });

  for (const tableName of [
    'external_agent_telemetry_events',
    'flow_compiled_plans',
    'flows',
    'mcp_groups',
    'mcp_tool_bindings',
    'model_fields',
  ]) {
    const table = report.tables.find((candidate) => candidate.name === tableName);
    assert.equal(
      table.findings.some((finding) => finding.rule === 'managed-table-needs-owner-review'),
      false
    );
    assert.equal(table.platformReadiness.fields.id.present, true);
    assert.equal(table.platformReadiness.fields.scope_id.present, true);
    assert.equal(table.platformReadiness.fields.created_at.present, true);
    assert.equal(table.platformReadiness.hasScopeTimeIdIndex, true);
    assert.equal(table.platformReadiness.scopeGenerationSource.status, 'declared');
    assert.equal(table.platformReadiness.recommendedActions.includes('needs_owner_review'), false);
  }

  for (const tableName of [
    'plugin_installations',
    'plugin_worker_leases',
    'user_auth_identities',
  ]) {
    const table = report.tables.find((candidate) => candidate.name === tableName);
    assert.ok(table.findings.some((finding) => finding.rule === 'managed-table-needs-owner-review'));
    assert.deepEqual(table.platformReadiness.recommendedActions, ['needs_owner_review']);
  }
});

test('default schema hygiene config declares issue 1073 lifecycle scoped readiness tables', () => {
  const repoRoot = path.resolve(__dirname, '..', '..', '..', '..');
  const inventory = collectSchemaInventory({ repoRoot });
  const report = evaluateSchemaHygiene({
    inventory,
    config: loadConfig(repoRoot),
  });

  for (const tableName of [
    'application_publication_versions',
    'flow_versions',
    'model_failover_queue_snapshots',
  ]) {
    const table = report.tables.find((candidate) => candidate.name === tableName);
    assert.equal(
      table.findings.some((finding) => finding.rule === 'managed-table-needs-owner-review'),
      false
    );
    assert.equal(table.platformReadiness.fields.id.present, true);
    assert.equal(table.platformReadiness.fields.scope_id.present, true);
    assert.equal(table.platformReadiness.fields.created_at.present, true);
    if (tableName === 'flow_versions') {
      assert.equal(table.platformReadiness.fields.updated_at.present, true);
    }
    assert.equal(table.platformReadiness.hasScopeTimeIdIndex, true);
    assert.equal(table.platformReadiness.scopeGenerationSource.status, 'declared');
    assert.equal(table.platformReadiness.recommendedActions.includes('needs_owner_review'), false);
  }

  for (const tableName of [
    'flow_drafts',
    'model_catalog_sync_runs',
    'model_change_logs',
    'model_failover_queue_items',
    'model_provider_catalog_entries',
    'data_source_catalog_caches',
    'data_source_secrets',
  ]) {
    const table = report.tables.find((candidate) => candidate.name === tableName);
    assert.ok(table.findings.some((finding) => finding.rule === 'managed-table-needs-owner-review'));
    assert.deepEqual(table.platformReadiness.recommendedActions, ['needs_owner_review']);
  }
});

test('main writes JSON and Markdown reports under tmp/test-governance and exits non-zero on fail findings', async () => {
  const repoRoot = createRepoWithMigration(`
    create table audit_events (
      id uuid primary key,
      created_at timestamptz not null default now()
    );
  `);
  const stderr = [];
  const stdout = [];

  const status = await main([], {
    repoRoot,
    writeStdout(text) {
      stdout.push(text);
    },
    writeStderr(text) {
      stderr.push(text);
    },
  });

  assert.equal(status, 1);
  assert.match(stdout.join(''), /schema-hygiene\.json/u);
  assert.match(stderr.join(''), /managed-table-scope-column/u);
  const jsonPath = path.join(repoRoot, 'tmp/test-governance/schema-hygiene.json');
  const markdownPath = path.join(repoRoot, 'tmp/test-governance/schema-hygiene.md');
  assert.equal(fs.existsSync(jsonPath), true);
  assert.equal(fs.existsSync(markdownPath), true);
  const report = JSON.parse(fs.readFileSync(jsonPath, 'utf8'));
  assert.equal(typeof report.findings[0].reason, 'string');
  assert.equal(typeof report.findings[0].action, 'string');
  assert.match(fs.readFileSync(markdownPath, 'utf8'), /Suggested action/u);
});
