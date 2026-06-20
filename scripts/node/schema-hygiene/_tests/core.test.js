const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');

const {
  collectSchemaInventory,
  evaluateSchemaHygiene,
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

test('evaluateSchemaHygiene checks dynamic_model_table workspace scope and index rules', () => {
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
    create index registered_catalog_scope_updated_idx
      on registered_catalog (scope_id, updated_at desc, id desc);
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
