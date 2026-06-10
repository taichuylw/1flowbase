const BACKEND_SHARDS = [
  {
    key: 'core-libs',
    packages: ['domain', 'access-control', 'observability', 'runtime-profile', 'plugin-framework'],
  },
  {
    key: 'runtime-storage',
    packages: [
      'runtime-core',
      'orchestration-runtime',
      'publish-gateway',
      'storage-durable',
      'storage-ephemeral',
      'storage-object',
      'storage-postgres',
    ],
  },
  {
    key: 'apps',
    packages: ['control-plane', 'api-server', 'plugin-runner'],
  },
];
const BACKEND_APP_TEST_SHARDS = [
  {
    key: 'control-plane',
    packages: ['control-plane'],
  },
  {
    key: 'api-server',
    packages: ['api-server'],
  },
  {
    key: 'plugin-runner',
    packages: ['plugin-runner'],
  },
];
const BACKEND_TEST_SHARDS = [
  ...BACKEND_SHARDS,
  ...BACKEND_APP_TEST_SHARDS,
];
const BACKEND_CI_TEST_SHARDS = [
  BACKEND_SHARDS.find((shard) => shard.key === 'core-libs'),
  BACKEND_SHARDS.find((shard) => shard.key === 'runtime-storage'),
  ...BACKEND_APP_TEST_SHARDS,
];
const BACKEND_CONSISTENCY_TARGETS = [
  {
    label: 'consistency-control-plane-state-transitions',
    packageName: 'control-plane',
    filter: 'state_transition_tests',
  },
  {
    label: 'consistency-control-plane-workspace-session',
    packageName: 'control-plane',
    filter: 'workspace_session',
  },
  {
    label: 'consistency-control-plane-model-definition-service',
    packageName: 'control-plane',
    filter: 'model_definition_service_tests',
  },
  {
    label: 'consistency-control-plane-model-definition-runtime-sync',
    packageName: 'control-plane',
    filter: 'model_definition_runtime_sync_tests',
  },
  {
    label: 'consistency-control-plane-resource-action-kernel',
    packageName: 'control-plane',
    filter: 'resource_action_tests',
  },
  {
    label: 'consistency-runtime-acl',
    packageName: 'runtime-core',
    filter: 'runtime_acl_tests',
  },
  {
    label: 'consistency-runtime-engine',
    packageName: 'runtime-core',
    filter: 'runtime_engine_tests',
  },
  {
    label: 'consistency-storage-migration-smoke',
    packageName: 'storage-postgres',
    filter: 'migration_smoke',
  },
  {
    label: 'consistency-storage-model-definition-repository',
    packageName: 'storage-postgres',
    filter: 'model_definition_repository_tests',
  },
  {
    label: 'consistency-storage-runtime-record-repository',
    packageName: 'storage-postgres',
    filter: 'runtime_record_repository_tests',
  },
  {
    label: 'consistency-storage-orchestration-runtime-repository',
    packageName: 'storage-postgres',
    filter: 'orchestration_runtime_repository_tests',
  },
  {
    label: 'consistency-storage-physical-schema-repository',
    packageName: 'storage-postgres',
    filter: 'physical_schema_repository_tests',
  },
  {
    label: 'consistency-storage-workspace-scope',
    packageName: 'storage-postgres',
    filter: 'workspace_scope_tests',
  },
  {
    label: 'consistency-api-model-definition-routes',
    packageName: 'api-server',
    filter: 'model_definition_routes',
  },
  {
    label: 'consistency-api-runtime-model-routes',
    packageName: 'api-server',
    filter: 'runtime_model_routes',
  },
  {
    label: 'consistency-api-workspace-routes',
    packageName: 'api-server',
    filter: 'workspace_routes',
  },
  {
    label: 'consistency-api-file-management-routes',
    packageName: 'api-server',
    filter: 'file_management_routes',
  },
];

module.exports = {
  BACKEND_CONSISTENCY_TARGETS,
  BACKEND_CI_TEST_SHARDS,
  BACKEND_SHARDS,
  BACKEND_TEST_SHARDS,
};
