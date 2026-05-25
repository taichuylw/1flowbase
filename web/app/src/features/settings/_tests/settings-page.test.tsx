/* eslint-disable testing-library/no-node-access */
import {
  fireEvent,
  render,
  screen,
  waitFor,
  within
} from '@testing-library/react';
import { Grid } from 'antd';
import { beforeEach, describe, expect, test, vi } from 'vitest';

const membersApi = vi.hoisted(() => ({
  settingsMembersQueryKey: ['settings', 'members'],
  fetchSettingsMembers: vi.fn(),
  createSettingsMember: vi.fn(),
  disableSettingsMember: vi.fn(),
  resetSettingsMemberPassword: vi.fn(),
  replaceSettingsMemberRoles: vi.fn()
}));

const rolesApi = vi.hoisted(() => ({
  settingsRolesQueryKey: ['settings', 'roles'],
  settingsRolePermissionsQueryKey: vi.fn((roleCode: string) => [
    'settings',
    'roles',
    roleCode,
    'permissions'
  ]),
  fetchSettingsRoles: vi.fn(),
  createSettingsRole: vi.fn(),
  updateSettingsRole: vi.fn(),
  deleteSettingsRole: vi.fn(),
  fetchSettingsRolePermissions: vi.fn(),
  replaceSettingsRolePermissions: vi.fn()
}));

const permissionsApi = vi.hoisted(() => ({
  settingsPermissionsQueryKey: ['settings', 'permissions'],
  fetchSettingsPermissions: vi.fn()
}));

const docsApi = vi.hoisted(() => ({
  settingsApiDocsCatalogQueryKey: ['settings', 'docs', 'catalog'],
  settingsApiDocsCategoryOperationsQueryKey: vi.fn((categoryId: string) => [
    'settings',
    'docs',
    'category',
    categoryId,
    'operations'
  ]),
  settingsApiDocsOperationSpecQueryKey: vi.fn((operationId: string) => [
    'settings',
    'docs',
    'operation',
    operationId,
    'openapi'
  ]),
  fetchSettingsApiDocsCatalog: vi.fn(),
  fetchSettingsApiDocsCategoryOperations: vi.fn(),
  fetchSettingsApiDocsOperationSpec: vi.fn()
}));

const modelProvidersApi = vi.hoisted(() => ({
  settingsModelProviderCatalogQueryKey: [
    'settings',
    'model-providers',
    'catalog'
  ],
  settingsModelProviderInstancesQueryKey: [
    'settings',
    'model-providers',
    'instances'
  ],
  settingsModelProviderOptionsQueryKey: [
    'settings',
    'model-providers',
    'options'
  ],
  settingsModelProviderModelsQueryKey: vi.fn((instanceId: string) => [
    'settings',
    'model-providers',
    'models',
    instanceId
  ]),
  fetchSettingsModelProviderCatalog: vi.fn(),
  fetchSettingsModelProviderInstances: vi.fn(),
  fetchSettingsModelProviderOptions: vi.fn(),
  fetchSettingsModelProviderMainInstance: vi.fn(),
  fetchSettingsModelProviderModels: vi.fn(),
  previewSettingsModelProviderModels: vi.fn(),
  createSettingsModelProviderInstance: vi.fn(),
  updateSettingsModelProviderInstance: vi.fn(),
  updateSettingsModelProviderMainInstance: vi.fn(),
  revealSettingsModelProviderSecret: vi.fn(),
  validateSettingsModelProviderInstance: vi.fn(),
  refreshSettingsModelProviderModels: vi.fn(),
  deleteSettingsModelProviderInstance: vi.fn()
}));

const pluginsApi = vi.hoisted(() => ({
  settingsOfficialPluginsQueryKey: ['settings', 'plugins', 'official-catalog'],
  settingsPluginFamiliesQueryKey: ['settings', 'plugins', 'families'],
  fetchSettingsPluginFamilies: vi.fn(),
  fetchSettingsOfficialPluginCatalog: vi.fn(),
  installSettingsOfficialPlugin: vi.fn(),
  uploadSettingsPluginPackage: vi.fn(),
  upgradeSettingsPluginFamilyLatest: vi.fn(),
  switchSettingsPluginFamilyVersion: vi.fn(),
  fetchSettingsPluginTask: vi.fn()
}));

const systemRuntimeApi = vi.hoisted(() => ({
  settingsSystemRuntimeQueryKey: ['settings', 'system-runtime'],
  fetchSettingsSystemRuntimeProfile: vi.fn()
}));

const fileManagementApi = vi.hoisted(() => ({
  settingsFileStoragesQueryKey: ['settings', 'files', 'storages'],
  settingsFileTablesQueryKey: ['settings', 'files', 'tables'],
  fetchSettingsFileStorages: vi.fn(),
  createSettingsFileStorage: vi.fn(),
  fetchSettingsFileTables: vi.fn(),
  createSettingsFileTable: vi.fn(),
  updateSettingsFileTableBinding: vi.fn()
}));

const hostInfrastructureApi = vi.hoisted(() => ({
  settingsHostInfrastructureProvidersQueryKey: [
    'settings',
    'host-infrastructure',
    'providers'
  ],
  settingsHostInfrastructureMemoryOverviewQueryKey: [
    'settings',
    'host-infrastructure',
    'memory'
  ],
  settingsHostInfrastructureMemoryEntriesQueryKey: vi.fn(
    (contractCode: string | null) => [
      'settings',
      'host-infrastructure',
      'memory',
      'contracts',
      contractCode,
      'entries'
    ]
  ),
  settingsHostInfrastructureMemoryTreeQueryKey: vi.fn(
    (contractCode: string | null) => [
      'settings',
      'host-infrastructure',
      'memory',
      'contracts',
      contractCode,
      'tree'
    ]
  ),
  settingsHostInfrastructureMemoryStatsQueryKey: vi.fn(
    (contractCode: string | null) => [
      'settings',
      'host-infrastructure',
      'memory',
      'contracts',
      contractCode,
      'stats'
    ]
  ),
  settingsHostInfrastructureMemorySearchQueryKey: vi.fn(
    (contractCode: string | null) => [
      'settings',
      'host-infrastructure',
      'memory',
      'contracts',
      contractCode,
      'search'
    ]
  ),
  fetchSettingsHostInfrastructureProviders: vi.fn(),
  saveSettingsHostInfrastructureProviderConfig: vi.fn(),
  fetchSettingsHostInfrastructureMemoryOverview: vi.fn(),
  fetchSettingsHostInfrastructureMemoryStats: vi.fn(),
  fetchSettingsHostInfrastructureMemoryEntries: vi.fn(),
  fetchSettingsHostInfrastructureMemoryTree: vi.fn(),
  searchSettingsHostInfrastructureMemoryEntries: vi.fn(),
  revealSettingsHostInfrastructureMemoryEntry: vi.fn()
}));

const dataModelsApi = vi.hoisted(() => ({
  settingsDataSourcesQueryKey: ['settings', 'data-models', 'sources'],
  settingsDataModelsQueryKey: vi.fn((sourceId: string) => [
    'settings',
    'data-models',
    'models',
    sourceId
  ]),
  settingsDataModelScopeGrantsQueryKey: vi.fn((modelId: string) => [
    'settings',
    'data-models',
    'scope-grants',
    modelId
  ]),
  settingsDataModelAdvisorFindingsQueryKey: vi.fn((modelId: string) => [
    'settings',
    'data-models',
    'advisor',
    modelId
  ]),
  settingsDataModelRecordPreviewQueryKey: vi.fn((modelCode: string) => [
    'settings',
    'data-models',
    'record-preview',
    modelCode
  ]),
  fetchSettingsDataSourceInstances: vi.fn(),
  updateSettingsDataSourceDefaults: vi.fn(),
  fetchSettingsDataModels: vi.fn(),
  createSettingsDataModel: vi.fn(),
  updateSettingsDataModel: vi.fn(),
  deleteSettingsDataModel: vi.fn(),
  updateSettingsDataModelApiExposure: vi.fn(),
  fetchSettingsDataModelScopeGrants: vi.fn(),
  createSettingsDataModelField: vi.fn(),
  updateSettingsDataModelField: vi.fn(),
  deleteSettingsDataModelField: vi.fn(),
  createSettingsDataModelScopeGrant: vi.fn(),
  updateSettingsDataModelScopeGrant: vi.fn(),
  fetchSettingsDataModelAdvisorFindings: vi.fn(),
  fetchSettingsDataModelRecordPreview: vi.fn()
}));

vi.mock('../api/members', () => membersApi);
vi.mock('../api/roles', () => rolesApi);
vi.mock('../api/permissions', () => permissionsApi);
vi.mock('../api/api-docs', () => docsApi);
vi.mock('../api/model-providers', () => modelProvidersApi);
vi.mock('../api/plugins', () => pluginsApi);
vi.mock('../api/system-runtime', () => systemRuntimeApi);
vi.mock('../api/file-management', () => fileManagementApi);
vi.mock('../api/host-infrastructure', () => hostInfrastructureApi);
vi.mock('../api/data-models', () => dataModelsApi);
vi.mock('@scalar/api-reference-react', () => ({
  ApiReferenceReact: () => <div data-testid="settings-page-scalar">Scalar</div>
}));
vi.mock('@1flowbase/api-client', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@1flowbase/api-client')>();

  return {
    ...actual,
    listConsoleMembers: membersApi.fetchSettingsMembers,
    createConsoleMember: membersApi.createSettingsMember,
    disableConsoleMember: membersApi.disableSettingsMember,
    resetConsoleMemberPassword: membersApi.resetSettingsMemberPassword,
    replaceConsoleMemberRoles: membersApi.replaceSettingsMemberRoles,
    listConsoleRoles: rolesApi.fetchSettingsRoles,
    fetchConsoleRolePermissions: rolesApi.fetchSettingsRolePermissions,
    createConsoleRole: rolesApi.createSettingsRole,
    updateConsoleRole: rolesApi.updateSettingsRole,
    deleteConsoleRole: rolesApi.deleteSettingsRole,
    replaceConsoleRolePermissions: rolesApi.replaceSettingsRolePermissions,
    fetchConsolePermissions: permissionsApi.fetchSettingsPermissions,
    fetchConsoleSystemRuntimeProfile:
      systemRuntimeApi.fetchSettingsSystemRuntimeProfile
  };
});

import { AppProviders } from '../../../app/AppProviders';
import { AppRouterProvider } from '../../../app/router';
import { resetAuthStore, useAuthStore } from '../../../state/auth-store';

const useBreakpointSpy = vi.spyOn(Grid, 'useBreakpoint');

function authenticateWithPermissions(
  permissions: string[],
  effectiveDisplayRole: 'manager' | 'root' = 'manager'
) {
  useAuthStore.getState().setAuthenticated({
    csrfToken: 'csrf-123',
    actor: {
      id: 'user-1',
      account: effectiveDisplayRole,
      effective_display_role: effectiveDisplayRole,
      current_workspace_id: 'workspace-1'
    },
    me: {
      id: 'user-1',
      account: effectiveDisplayRole,
      email: `${effectiveDisplayRole}@example.com`,
      phone: null,
      nickname: effectiveDisplayRole,
      name: effectiveDisplayRole,
      avatar_url: null,
      introduction: '',
      effective_display_role: effectiveDisplayRole,
      permissions
    }
  });
}

function renderApp(pathname: string) {
  window.history.pushState({}, '', pathname);

  return render(
    <AppProviders>
      <AppRouterProvider />
    </AppProviders>
  );
}

describe('SettingsPage', () => {
  beforeEach(() => {
    resetAuthStore();
    useBreakpointSpy.mockReturnValue({
      xs: true,
      sm: true,
      md: true,
      lg: true,
      xl: false,
      xxl: false
    });
    membersApi.fetchSettingsMembers.mockResolvedValue([]);
    rolesApi.fetchSettingsRoles.mockResolvedValue([]);
    rolesApi.fetchSettingsRolePermissions.mockResolvedValue({
      role_code: 'manager',
      permission_codes: []
    });
    permissionsApi.fetchSettingsPermissions.mockResolvedValue([]);
    docsApi.fetchSettingsApiDocsCatalog.mockResolvedValue({
      title: '1flowbase API',
      version: '0.1.0',
      categories: [
        {
          id: 'console',
          label: '控制面',
          operation_count: 0
        }
      ]
    });
    docsApi.fetchSettingsApiDocsCategoryOperations.mockResolvedValue({
      id: 'console',
      label: '控制面',
      operations: []
    });
    docsApi.fetchSettingsApiDocsOperationSpec.mockResolvedValue({
      openapi: '3.1.0',
      info: { title: '1flowbase API', version: '0.1.0' },
      paths: {},
      components: {}
    });
    modelProvidersApi.fetchSettingsModelProviderCatalog.mockResolvedValue([]);
    modelProvidersApi.fetchSettingsModelProviderInstances.mockResolvedValue([]);
    modelProvidersApi.fetchSettingsModelProviderOptions.mockResolvedValue({
      locale_meta: {
        requested_locale: 'zh_Hans',
        resolved_locale: 'zh_Hans',
        fallback_locale: 'en_US',
        supported_locales: ['zh_Hans', 'en_US']
      },
      i18n_catalog: {},
      providers: []
    });
    modelProvidersApi.fetchSettingsModelProviderMainInstance.mockResolvedValue({
      provider_code: 'openai_compatible',
      auto_include_new_instances: true
    });
    pluginsApi.fetchSettingsPluginFamilies.mockResolvedValue([]);
    pluginsApi.fetchSettingsOfficialPluginCatalog.mockResolvedValue({
      source_kind: 'official_registry',
      source_label: '官方源',
      registry_url: 'https://official.example.com/official-registry.json',
      entries: []
    });
    pluginsApi.installSettingsOfficialPlugin.mockResolvedValue({
      installation: {
        id: 'installation-1',
        provider_code: 'openai_compatible',
        plugin_id: 'openai_compatible@0.1.0',
        plugin_version: '0.1.0',
        contract_version: '1flowbase.provider/v1',
        protocol: 'openai_compatible',
        display_name: 'OpenAI Compatible',
        source_kind: 'official_registry',
        trust_level: 'verified_official',
        verification_status: 'valid',
        enabled: true,
        install_path: '/tmp/openai-compatible',
        checksum: 'sha256:abc123',
        signature_status: 'unsigned',
        signature_algorithm: null,
        signing_key_id: null,
        metadata_json: {},
        created_at: '2026-04-18T21:00:00Z',
        updated_at: '2026-04-18T21:00:00Z'
      },
      task: {
        id: 'task-1',
        installation_id: 'installation-1',
        workspace_id: 'workspace-1',
        provider_code: 'openai_compatible',
        task_kind: 'assign',
        status: 'success',
        status_message: 'assigned',
        detail_json: {},
        created_at: '2026-04-18T21:00:00Z',
        updated_at: '2026-04-18T21:00:00Z',
        finished_at: '2026-04-18T21:00:00Z'
      }
    });
    pluginsApi.fetchSettingsPluginTask.mockResolvedValue({
      id: 'task-1',
      installation_id: 'installation-1',
      workspace_id: 'workspace-1',
      provider_code: 'openai_compatible',
      task_kind: 'assign',
      status: 'success',
      status_message: 'assigned',
      detail_json: {},
      created_at: '2026-04-18T21:00:00Z',
      updated_at: '2026-04-18T21:00:00Z',
      finished_at: '2026-04-18T21:00:00Z'
    });
    systemRuntimeApi.fetchSettingsSystemRuntimeProfile.mockResolvedValue({
      provider_install_root: '/home/taichu/git/1flowbase/api/plugins',
      host_extension_dropin_root:
        '/home/taichu/git/1flowbase/api/plugins/host-extension/dropins',
      locale_meta: {
        requested_locale: null,
        resolved_locale: 'zh_Hans',
        source: 'fallback',
        fallback_locale: 'en_US',
        supported_locales: ['zh_Hans', 'en_US']
      },
      topology: {
        relationship: 'same_host'
      },
      services: {
        api_server: {
          reachable: true,
          service: 'api-server',
          status: 'ok',
          version: '0.1.0',
          host_fingerprint: 'host-1'
        },
        plugin_runner: {
          reachable: true,
          service: 'plugin-runner',
          status: 'ok',
          version: '0.1.0',
          host_fingerprint: 'host-1'
        }
      },
      hosts: [
        {
          host_fingerprint: 'host-1',
          platform: {
            os: 'linux',
            arch: 'amd64',
            libc: 'musl',
            rust_target_triple: 'x86_64-unknown-linux-musl'
          },
          cpu: {
            logical_count: 8
          },
          memory: {
            total_bytes: 17179869184,
            total_gb: 16,
            available_bytes: 8589934592,
            available_gb: 8,
            process_bytes: 1073741824,
            process_gb: 1
          },
          services: ['api-server', 'plugin-runner']
        }
      ]
    });
    fileManagementApi.fetchSettingsFileStorages.mockResolvedValue([
      {
        id: 'storage-1',
        code: 'local-default',
        title: 'Local Default',
        driver_type: 'local',
        enabled: true,
        is_default: true,
        health_status: 'ready',
        last_health_error: null,
        config_json: {
          root_path: '/srv/files'
        },
        rule_json: {}
      }
    ]);
    fileManagementApi.fetchSettingsFileTables.mockResolvedValue([
      {
        id: 'table-1',
        code: 'attachments',
        title: 'Attachments',
        scope_kind: 'workspace',
        scope_id: 'workspace-1',
        model_definition_id: 'model-1',
        bound_storage_id: 'storage-1',
        bound_storage_title: 'Local Default',
        is_builtin: true,
        is_default: true,
        status: 'active'
      }
    ]);
    hostInfrastructureApi.fetchSettingsHostInfrastructureProviders.mockResolvedValue(
      []
    );
    hostInfrastructureApi.fetchSettingsHostInfrastructureMemoryOverview.mockResolvedValue(
      {
        can_manage: true,
        contracts: []
      }
    );
    hostInfrastructureApi.fetchSettingsHostInfrastructureMemoryStats.mockResolvedValue(
      {
        contract_code: 'session-store',
        label: 'Sessions',
        provider_code: 'local',
        supported: true,
        inspection_path: [],
        entry_count: 0,
        sensitive_entry_count: 0,
        total_value_size_bytes: 0
      }
    );
    hostInfrastructureApi.fetchSettingsHostInfrastructureMemoryTree.mockResolvedValue(
      {
        contract_code: 'session-store',
        label: 'Sessions',
        provider_code: 'local',
        supported: true,
        inspection_path: [],
        nodes: [],
        next_cursor: null,
        limit: 50,
        byte_limit: 65536,
        emitted_bytes: 0,
        truncated_by_byte_limit: false
      }
    );
    hostInfrastructureApi.fetchSettingsHostInfrastructureMemoryEntries.mockResolvedValue(
      {
        contract_code: 'session-store',
        label: 'Sessions',
        provider_code: 'local',
        capabilities: {
          list_entries: true,
          reveal_value: true
        },
        supported: true,
        entries: []
      }
    );
    dataModelsApi.fetchSettingsDataSourceInstances.mockResolvedValue([
      {
        id: 'main_source',
        source_kind: 'main_source',
        installation_id: 'main_source',
        source_code: 'main_source',
        display_name: '主数据源',
        status: 'ready',
        default_data_model_status: 'published',
        default_api_exposure_status: 'published_not_exposed',
        config_json: {},
        secret_ref: null,
        secret_version: null,
        catalog_refresh_status: null,
        catalog_last_error_message: null,
        catalog_refreshed_at: null
      }
    ]);
    dataModelsApi.fetchSettingsDataModels.mockResolvedValue([]);
    dataModelsApi.fetchSettingsDataModelScopeGrants.mockResolvedValue([]);
    dataModelsApi.fetchSettingsDataModelAdvisorFindings.mockResolvedValue([]);
    dataModelsApi.fetchSettingsDataModelRecordPreview.mockResolvedValue(null);
  });

  test('shows API 文档 only for root or api_reference.view.all', async () => {
    const rootView = (() => {
      authenticateWithPermissions([], 'root');
      return renderApp('/settings');
    })();

    await waitFor(
      () => {
        expect(window.location.pathname).toBe('/settings/docs');
      },
      { timeout: 5000 }
    );
    await waitFor(() => {
      expect(docsApi.fetchSettingsApiDocsCatalog).toHaveBeenCalled();
    });
    rootView.unmount();

    resetAuthStore();
    docsApi.fetchSettingsApiDocsCatalog.mockClear();
    authenticateWithPermissions([
      'route_page.view.all',
      'api_reference.view.all'
    ]);
    const view = renderApp('/settings');

    await waitFor(
      () => {
        expect(window.location.pathname).toBe('/settings/docs');
      },
      { timeout: 5000 }
    );
    await waitFor(() => {
      expect(docsApi.fetchSettingsApiDocsCatalog).toHaveBeenCalled();
    });
    view.unmount();

    resetAuthStore();
    authenticateWithPermissions(['route_page.view.all', 'user.view.all']);
    renderApp('/settings');

    await waitFor(() => {
      expect(window.location.pathname).toBe('/settings/members');
    });
    expect(
      screen.queryByRole('heading', { name: 'API 文档', level: 3 })
    ).not.toBeInTheDocument();
  }, 10000);

  test('renders /settings/members when user.view.all is present', async () => {
    authenticateWithPermissions(['route_page.view.all', 'user.view.all']);

    renderApp('/settings/members');

    await waitFor(() => {
      expect(window.location.pathname).toBe('/settings/members');
    });
    expect(screen.getByTestId('section-page-layout')).toHaveClass(
      'section-page-layout--wide',
      'section-page-layout--viewport'
    );
    expect(
      await screen.findByText(
        '重置密码会将目标账号密码重置为默认临时密码，并要求用户登录后立即修改。'
      )
    ).toBeInTheDocument();
    expect(
      screen.queryByRole('button', { name: '新建用户' })
    ).not.toBeInTheDocument();
  });

  test('disables root member write actions while leaving normal members operable', async () => {
    authenticateWithPermissions(
      ['route_page.view.all', 'user.view.all', 'user.manage.all'],
      'root'
    );
    membersApi.fetchSettingsMembers.mockResolvedValue([
      {
        id: 'root-user',
        account: 'root',
        email: 'root@example.com',
        phone: null,
        name: 'Root',
        nickname: 'Root',
        introduction: '',
        default_display_role: 'root',
        email_login_enabled: true,
        phone_login_enabled: false,
        status: 'active',
        role_codes: ['root']
      },
      {
        id: 'manager-1',
        account: 'manager-1',
        email: 'manager-1@example.com',
        phone: null,
        name: 'Manager 1',
        nickname: 'Manager 1',
        introduction: '',
        default_display_role: 'manager',
        email_login_enabled: true,
        phone_login_enabled: false,
        status: 'active',
        role_codes: ['manager']
      }
    ]);

    renderApp('/settings/members');

    await waitFor(() => {
      expect(membersApi.fetchSettingsMembers).toHaveBeenCalled();
    });

    const rootRow = (
      await screen.findByText('Root', {}, { timeout: 10000 })
    ).closest('tr') as HTMLElement;
    const managerRow = (
      await screen.findByText('Manager 1', {}, { timeout: 10000 })
    ).closest('tr') as HTMLElement;

    expect(rootRow).not.toBeNull();
    expect(managerRow).not.toBeNull();

    expect(
      within(rootRow).getByRole('button', { name: /编辑$/ })
    ).toBeDisabled();
    expect(
      within(rootRow).getByRole('button', { name: /停用$/ })
    ).toBeDisabled();
    expect(
      within(rootRow).getByRole('button', { name: /重置密码$/ })
    ).toBeDisabled();
    expect(
      within(managerRow).getByRole('button', { name: /编辑$/ })
    ).toBeEnabled();
    expect(
      within(managerRow).getByRole('button', { name: /停用$/ })
    ).toBeEnabled();
    expect(
      within(managerRow).getByRole('button', { name: /重置密码$/ })
    ).toBeEnabled();
  }, 10000);

  test('redirects /settings/docs to /settings/members when docs is hidden but members is visible', async () => {
    authenticateWithPermissions(['route_page.view.all', 'user.view.all']);

    renderApp('/settings/docs');

    await waitFor(() => {
      expect(window.location.pathname).toBe('/settings/members');
    });
    expect(screen.getByTestId('section-page-layout')).toHaveClass(
      'section-page-layout--viewport'
    );
    expect(
      await screen.findByText(
        '重置密码会将目标账号密码重置为默认临时密码，并要求用户登录后立即修改。'
      )
    ).toBeInTheDocument();
  });

  test('shows 数据源 when state_model.view.all is the only visible settings section', async () => {
    authenticateWithPermissions([
      'route_page.view.all',
      'state_model.view.all'
    ]);

    renderApp('/settings');

    await waitFor(() => {
      expect(window.location.pathname).toBe('/settings/data-models');
    });
    expect(await screen.findByRole('link', { name: '数据源' })).toHaveAttribute(
      'href',
      '/settings/data-models'
    );
    expect(dataModelsApi.fetchSettingsDataSourceInstances).toHaveBeenCalled();
    expect(
      await screen.findByText('主数据源', {}, { timeout: 10000 })
    ).toBeInTheDocument();
  });

  test('shows 系统运行 when system_runtime.view.all is the only visible settings section', async () => {
    authenticateWithPermissions([
      'route_page.view.all',
      'system_runtime.view.all'
    ]);

    renderApp('/settings');

    await waitFor(() => {
      expect(window.location.pathname).toBe('/settings/system-runtime');
    });
    expect(await screen.findByText('部署概览')).toBeInTheDocument();
    expect(screen.getByText('同机部署')).toBeInTheDocument();
    expect(screen.getByText('zh_Hans')).toBeInTheDocument();
    expect(screen.getByText('API Server')).toBeInTheDocument();
    expect(screen.getByText('Plugin Runner')).toBeInTheDocument();
    expect(
      systemRuntimeApi.fetchSettingsSystemRuntimeProfile
    ).toHaveBeenCalled();
  });

  test('shows 基础设施 and 内存观察 when plugin_config.view.all is present', async () => {
    authenticateWithPermissions([
      'route_page.view.all',
      'plugin_config.view.all'
    ]);

    renderApp('/settings');

    await waitFor(() => {
      expect(window.location.pathname).toBe('/settings/host-infrastructure');
    });
    expect(
      await screen.findByRole('link', { name: '内存观察' }, { timeout: 10000 })
    ).toHaveAttribute('href', '/settings/memory-observation');
    expect(
      await screen.findByText(
        '安装、配置和启用会保存为待应用变更，重启 api-server 一次后生效。'
      )
    ).toBeInTheDocument();
    expect(
      screen.queryByRole('tab', { name: '内存观察' })
    ).not.toBeInTheDocument();
  });

  test('renders memory observation as a settings section route', async () => {
    authenticateWithPermissions([
      'route_page.view.all',
      'plugin_config.view.all'
    ]);
    hostInfrastructureApi.fetchSettingsHostInfrastructureMemoryOverview.mockResolvedValue(
      {
        can_manage: true,
        contracts: [
          {
            contract_code: 'session-store',
            label: 'Sessions',
            provider_code: 'local',
            capabilities: {
              list_entries: true,
              list_tree: true,
              search_entries: true,
              reveal_value: true
            },
            supported: true
          }
        ]
      }
    );
    hostInfrastructureApi.fetchSettingsHostInfrastructureMemoryStats.mockResolvedValue(
      {
        contract_code: 'session-store',
        label: 'Sessions',
        provider_code: 'local',
        capabilities: {
          list_entries: true,
          list_tree: true,
          search_entries: true,
          reveal_value: true
        },
        supported: true,
        inspection_path: [],
        entry_count: 1,
        sensitive_entry_count: 1,
        total_value_size_bytes: 317
      }
    );
    hostInfrastructureApi.fetchSettingsHostInfrastructureMemoryTree.mockResolvedValue(
      {
        contract_code: 'session-store',
        label: 'Sessions',
        provider_code: 'local',
        capabilities: {
          list_entries: true,
          list_tree: true,
          search_entries: true,
          reveal_value: true
        },
        supported: true,
        inspection_path: [],
        nodes: [
          {
            node_ref: 'root-session-node',
            label: '00000000-0000-0000-0000-000000000001',
            inspection_path: ['00000000-0000-0000-0000-000000000001'],
            depth: 1,
            has_children: false
          }
        ],
        next_cursor: null,
        limit: 50,
        byte_limit: 65536,
        emitted_bytes: 0,
        truncated_by_byte_limit: false
      }
    );
    hostInfrastructureApi.fetchSettingsHostInfrastructureMemoryEntries.mockResolvedValue(
      {
        contract_code: 'session-store',
        label: 'Sessions',
        provider_code: 'local',
        capabilities: {
          list_entries: true,
          list_tree: true,
          search_entries: true,
          reveal_value: true
        },
        supported: true,
        inspection_path: ['00000000-0000-0000-0000-000000000001'],
        entries: [
          {
            contract_code: 'session-store',
            group_code: '00000000-0000-0000-0000-000000000001',
            entry_ref: 'session:1',
            key: 'session:1',
            inspection_path: [
              '00000000-0000-0000-0000-000000000001',
              'session:1'
            ],
            entry_kind: 'session',
            status: 'active',
            owner: 'user-1',
            value_size_bytes: 317,
            metadata_size_bytes: 2,
            ttl_seconds: 600,
            created_at_unix: 1_700_000_000,
            expires_at_unix: 1_700_000_600,
            sensitive: true,
            metadata: {}
          }
        ],
        next_cursor: null,
        limit: 50,
        byte_limit: 65536,
        emitted_bytes: 128,
        truncated_by_byte_limit: false
      }
    );

    renderApp('/settings/memory-observation');

    await waitFor(() => {
      expect(window.location.pathname).toBe('/settings/memory-observation');
    });
    expect(
      await screen.findByRole('link', { name: '内存观察' }, { timeout: 10000 })
    ).toHaveAttribute('href', '/settings/memory-observation');
    expect(
      await screen.findByRole('tab', { name: 'Sessions' }, { timeout: 10000 })
    ).toBeInTheDocument();
    fireEvent.click(
      await screen.findByText('00000000-0000-0000-0000-000000000001')
    );
    expect(await screen.findByText('session:1')).toBeInTheDocument();
    expect(
      screen.queryByRole('tab', { name: 'Provider 配置' })
    ).not.toBeInTheDocument();
  });

  test('shows 文件管理 when file_table.view.own is the only visible settings section', async () => {
    authenticateWithPermissions(['route_page.view.all', 'file_table.view.own']);

    renderApp('/settings');

    await waitFor(() => {
      expect(window.location.pathname).toBe('/settings/files');
    });
    expect(
      await screen.findByRole('tab', { name: '文件表' })
    ).toBeInTheDocument();
  });

  test('renders the empty settings state when no section is visible', async () => {
    authenticateWithPermissions(['route_page.view.all']);

    renderApp('/settings');

    await waitFor(() => {
      expect(window.location.pathname).toBe('/settings');
    });
    expect(
      await screen.findByText(/当前账号暂无可访问内容/)
    ).toBeInTheDocument();
    expect(
      screen.queryByRole('navigation', { name: 'Section navigation' })
    ).not.toBeInTheDocument();
  });
});
