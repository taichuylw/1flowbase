const EXPANSION_SCOPE_COLUMN = 'scope_id';
const EXPANSION_TIME_COLUMN = 'created_at';
const EXPANSION_TIE_BREAKER_COLUMN = 'id';
const PLATFORM_FIELDS = [
  'id',
  'scope_id',
  'created_at',
  'updated_at',
  'created_by',
  'updated_by',
];

function findColumn(table, columnName) {
  return table.columns.find((column) => column.name === columnName);
}

function hasColumn(table, columnName) {
  return table.columns.some((column) => column.name === columnName);
}

function hasScopeColumn(table) {
  return hasColumn(table, EXPANSION_SCOPE_COLUMN);
}

function hasScopeTimeIndex(table) {
  return table.indexes.some((index) => {
    const [scopeColumn, timeColumn, tieBreakerColumn] = index.columns;
    return scopeColumn === EXPANSION_SCOPE_COLUMN
      && timeColumn === EXPANSION_TIME_COLUMN
      && tieBreakerColumn === EXPANSION_TIE_BREAKER_COLUMN;
  });
}

function columnReadiness(table, columnName) {
  const column = findColumn(table, columnName);
  return {
    present: Boolean(column),
    nullable: column ? column.nullable : null,
    default: column ? column.default : null,
    type: column ? column.type : null,
  };
}

function pushAction(actions, action) {
  if (!actions.includes(action)) {
    actions.push(action);
  }
}

function categoryForTable({ profile, appendOnly, systemScope, declaration }) {
  if (declaration.category) {
    return declaration.category;
  }
  if (profile === 'dynamic_model_table') {
    return 'dynamic_runtime_table';
  }
  if (profile === 'registered_system_table') {
    return 'registered_system_table';
  }
  if (systemScope) {
    return 'system_global';
  }
  if (appendOnly) {
    return 'join_or_child';
  }
  return 'unknown_needs_review';
}

function declaredSource(value) {
  if (typeof value === 'string' && value.trim().length > 0) {
    return value.trim();
  }
  return null;
}

function scopeGenerationSourceForTable({ table, systemScope, declaration }) {
  const declared = declaredSource(declaration.scopeGenerationSource || declaration.scopeSource);
  if (declared) {
    return {
      status: 'declared',
      source: declared,
    };
  }

  if (systemScope) {
    return {
      status: 'declared',
      source: 'SYSTEM_SCOPE_ID',
    };
  }

  if (hasColumn(table, 'workspace_id')) {
    return {
      status: 'inferred',
      source: 'workspace_id',
    };
  }

  return {
    status: 'needs_owner_review',
    source: null,
  };
}

function backfillSourceForTable({ table, declaration, scopeGenerationSource }) {
  const declared = declaredSource(declaration.backfillSource);
  if (declared) {
    return declared;
  }
  if (!hasScopeColumn(table) && scopeGenerationSource.status === 'inferred') {
    return scopeGenerationSource.source;
  }
  return null;
}

function generationStatusForColumn({ columnName, table, declaration }) {
  const key = `${columnName}Generation`;
  const source = declaredSource(declaration[key]);
  if (source) {
    return {
      status: 'declared',
      source,
    };
  }
  const column = findColumn(table, columnName);
  if (columnName === 'id') {
    return {
      status: column ? 'declare_generation_rule' : 'missing',
      source: null,
    };
  }
  if ((columnName === 'created_at' || columnName === 'updated_at') && column && column.default) {
    return {
      status: 'schema_default',
      source: 'default now()',
    };
  }
  if (column) {
    return {
      status: 'declare_generation_rule',
      source: null,
    };
  }
  return {
    status: 'missing',
    source: null,
  };
}

function recommendedActionsForTable({
  table,
  appendOnly,
  systemScope,
  hasReadinessIndex,
  scopeGenerationSource,
  backfillSource,
  declaration,
}) {
  const actions = [];
  if (!hasColumn(table, 'id')) {
    pushAction(actions, 'add_id');
  }
  if (!hasColumn(table, 'created_at')) {
    pushAction(actions, 'add_created_at');
  }
  if (!appendOnly && !hasColumn(table, 'updated_at')) {
    pushAction(actions, 'add_updated_at');
  }
  if (systemScope && !hasScopeColumn(table)) {
    pushAction(actions, 'mark_system_scope');
  }
  if (!systemScope && !hasScopeColumn(table)) {
    if (backfillSource) {
      pushAction(actions, 'add_scope_id');
      pushAction(actions, 'backfill_scope_id');
    } else {
      pushAction(actions, 'needs_owner_review');
    }
  }
  if (!systemScope && (hasScopeColumn(table) || backfillSource) && !hasReadinessIndex) {
    pushAction(actions, 'add_scope_time_index');
  }

  const writePathSource = declaredSource(declaration.writePathSource);
  const hasMissingAuditColumns = !hasColumn(table, 'created_by') || !hasColumn(table, 'updated_by');
  const generationNeedsDeclaration = !writePathSource
    || generationStatusForColumn({ columnName: 'id', table, declaration }).status === 'declare_generation_rule'
    || generationStatusForColumn({ columnName: 'created_by', table, declaration }).status === 'declare_generation_rule'
    || generationStatusForColumn({ columnName: 'updated_by', table, declaration }).status === 'declare_generation_rule'
    || hasMissingAuditColumns;
  if (generationNeedsDeclaration && !actions.includes('needs_owner_review')) {
    pushAction(actions, 'declare_generation_rule');
  }

  if (actions.length === 0) {
    pushAction(actions, 'no_action');
  }
  return actions;
}

function platformReadinessForTable({ table, profile, config, tableFindings }) {
  const appendOnly = config.appendOnlyTables.has(table.name);
  const systemScope = config.systemScopeTables.has(table.name);
  const declaration = config.tableReadiness[table.name] || {};
  const fields = Object.fromEntries(
    [...PLATFORM_FIELDS, 'workspace_id'].map((columnName) => [
      columnName,
      {
        ...columnReadiness(table, columnName),
        generation: PLATFORM_FIELDS.includes(columnName)
          ? generationStatusForColumn({ columnName, table, declaration })
          : null,
      },
    ])
  );
  const hasReadinessIndex = hasScopeTimeIndex(table);
  const scopeGenerationSource = scopeGenerationSourceForTable({
    table,
    systemScope,
    declaration,
  });
  const backfillSource = backfillSourceForTable({
    table,
    declaration,
    scopeGenerationSource,
  });
  const recommendedActions = recommendedActionsForTable({
    table,
    appendOnly,
    systemScope,
    hasReadinessIndex,
    scopeGenerationSource,
    backfillSource,
    declaration,
  });
  const severity = tableFindings.some((item) => item.severity === 'error')
    ? 'error'
    : recommendedActions.includes('no_action') ? 'ok' : 'warning';

  return {
    category: categoryForTable({ profile, appendOnly, systemScope, declaration }),
    timeKey: EXPANSION_TIME_COLUMN,
    tieBreaker: EXPANSION_TIE_BREAKER_COLUMN,
    fields,
    missingFields: PLATFORM_FIELDS.filter((columnName) => !hasColumn(table, columnName)),
    requiredScopeId: !systemScope,
    routingKeyStatus: hasScopeColumn(table) ? 'present' : 'missing',
    scopeGenerationSource,
    backfillSource,
    writePathSource: declaredSource(declaration.writePathSource),
    hasScopeTimeIdIndex: hasReadinessIndex,
    recommendedActions,
    severity,
    reason: recommendedActions.includes('no_action')
      ? 'platform field and expansion readiness checks passed'
      : 'platform field or generation source action required',
  };
}

module.exports = {
  EXPANSION_SCOPE_COLUMN,
  hasColumn,
  hasScopeColumn,
  hasScopeTimeIndex,
  platformReadinessForTable,
};
