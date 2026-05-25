import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';

import { resetAuthStore, useAuthStore } from '../../../../../state/auth-store';

const api = vi.hoisted(() => ({
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
  fetchSettingsHostInfrastructureProviders: vi.fn(),
  saveSettingsHostInfrastructureProviderConfig: vi.fn(),
  fetchSettingsHostInfrastructureMemoryOverview: vi.fn(),
  fetchSettingsHostInfrastructureMemoryEntries: vi.fn(),
  revealSettingsHostInfrastructureMemoryEntry: vi.fn()
}));

vi.mock('../../../api/host-infrastructure', () => api);

import { HostInfrastructurePanel } from '../HostInfrastructurePanel';

function renderPanel(canManage = true) {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } }
  });

  useAuthStore.getState().setAuthenticated({
    csrfToken: 'csrf-123',
    actor: {
      id: 'user-1',
      account: 'root',
      effective_display_role: 'root',
      current_workspace_id: 'workspace-1'
    },
    me: null
  });

  return render(
    <QueryClientProvider client={queryClient}>
      <HostInfrastructurePanel canManage={canManage} />
    </QueryClientProvider>
  );
}

describe('HostInfrastructurePanel', () => {
  afterEach(() => {
    vi.clearAllMocks();
    resetAuthStore();
  });

  beforeEach(() => {
    api.fetchSettingsHostInfrastructureMemoryOverview.mockResolvedValue({
      can_manage: true,
      contracts: []
    });
    api.fetchSettingsHostInfrastructureMemoryEntries.mockResolvedValue({
      contract_code: 'session-store',
      label: 'Sessions',
      provider_code: 'local',
      capabilities: {
        list_entries: true,
        reveal_value: true
      },
      supported: true,
      entries: []
    });
  });

  test('renders installed inactive provider config from manifest schema', async () => {
    api.fetchSettingsHostInfrastructureProviders.mockResolvedValue([
      {
        installation_id: 'installation-1',
        extension_id: 'redis-infra-host',
        provider_code: 'redis',
        display_name: 'Redis',
        description: 'Redis backed host infrastructure.',
        runtime_status: 'inactive',
        desired_state: 'disabled',
        config_ref: 'secret://system/redis-infra-host/config',
        contracts: ['storage-ephemeral', 'cache-store'],
        enabled_contracts: [],
        config_schema: [
          { key: 'host', label: 'Host', type: 'string', required: true },
          { key: 'port', label: 'Port', type: 'number', required: true }
        ],
        config_json: {},
        restart_required: false
      }
    ]);

    renderPanel();

    expect(await screen.findByText('Redis')).toBeInTheDocument();
    expect(screen.getByText('disabled')).toBeInTheDocument();
    expect(screen.getByText('inactive')).toBeInTheDocument();
    expect(screen.getByText('storage-ephemeral')).toBeInTheDocument();
    expect(screen.getByText('cache-store')).toBeInTheDocument();
  });

  test('renders pending restart state without claiming provider is active', async () => {
    api.fetchSettingsHostInfrastructureProviders.mockResolvedValue([
      {
        installation_id: 'installation-1',
        extension_id: 'redis-infra-host',
        provider_code: 'redis',
        display_name: 'Redis',
        description: null,
        runtime_status: 'inactive',
        desired_state: 'pending_restart',
        config_ref: 'secret://system/redis-infra-host/config',
        contracts: ['storage-ephemeral'],
        enabled_contracts: ['storage-ephemeral'],
        config_schema: [],
        config_json: {},
        restart_required: true
      }
    ]);

    renderPanel();

    expect(await screen.findByText('Redis')).toBeInTheDocument();
    expect(screen.getByText('pending_restart')).toBeInTheDocument();
    expect(screen.getByText('inactive')).toBeInTheDocument();
    expect(screen.getByText('重启后生效')).toBeInTheDocument();
  });

  test('saves config and contract selection as one pending restart change', async () => {
    api.fetchSettingsHostInfrastructureProviders.mockResolvedValue([
      {
        installation_id: 'installation-1',
        extension_id: 'redis-infra-host',
        provider_code: 'redis',
        display_name: 'Redis',
        description: null,
        runtime_status: 'inactive',
        desired_state: 'disabled',
        config_ref: 'secret://system/redis-infra-host/config',
        contracts: ['storage-ephemeral', 'cache-store'],
        enabled_contracts: [],
        config_schema: [
          { key: 'host', label: 'Host', type: 'string', required: true },
          {
            key: 'port',
            label: 'Port',
            type: 'number',
            required: true,
            default_value: 6379
          },
          {
            key: 'password_ref',
            label: 'Password Secret Ref',
            type: 'string',
            send_mode: 'secret_ref'
          }
        ],
        config_json: {},
        restart_required: false
      }
    ]);
    api.saveSettingsHostInfrastructureProviderConfig.mockResolvedValue({
      restart_required: true,
      installation_desired_state: 'pending_restart',
      provider_config_status: 'pending_restart'
    });

    renderPanel(true);

    fireEvent.click(await screen.findByRole('button', { name: '配置' }));
    fireEvent.change(screen.getByLabelText('Host'), {
      target: { value: 'localhost' }
    });
    fireEvent.change(screen.getByLabelText('Port'), {
      target: { value: '6379' }
    });
    fireEvent.click(screen.getByLabelText('storage-ephemeral'));
    fireEvent.click(screen.getByRole('button', { name: '保存并等待重启' }));

    await waitFor(() => {
      expect(
        api.saveSettingsHostInfrastructureProviderConfig
      ).toHaveBeenCalledWith(
        'installation-1',
        'redis',
        {
          enabled_contracts: ['storage-ephemeral'],
          config_json: {
            host: 'localhost',
            port: 6379
          }
        },
        'csrf-123'
      );
    });
    expect(
      await screen.findByText('已保存，重启 api-server 一次后生效。')
    ).toBeInTheDocument();
  });

  test('renders memory contract tabs and metadata entries', async () => {
    api.fetchSettingsHostInfrastructureProviders.mockResolvedValue([]);
    api.fetchSettingsHostInfrastructureMemoryOverview.mockResolvedValue({
      can_manage: true,
      contracts: [
        {
          contract_code: 'session-store',
          label: 'Sessions',
          provider_code: 'local',
          capabilities: {
            list_entries: true,
            reveal_value: true
          },
          entry_count: 2,
          sensitive_entry_count: 1,
          total_value_size_bytes: 2048,
          supported: true
        }
      ]
    });
    api.fetchSettingsHostInfrastructureMemoryEntries.mockResolvedValue({
      contract_code: 'session-store',
      label: 'Sessions',
      provider_code: 'local',
      capabilities: {
        list_entries: true,
        reveal_value: true
      },
      supported: true,
      entries: [
        {
          contract_code: 'session-store',
          group_code: 'sessions',
          key: 'session:1',
          entry_kind: 'session',
          status: 'active',
          owner: 'user-1',
          value_size_bytes: 1024,
          ttl_seconds: 60,
          created_at_unix: 1_700_000_000,
          expires_at_unix: 1_700_000_060,
          sensitive: true,
          metadata: { workspace_id: 'workspace-1' }
        }
      ]
    });

    renderPanel(true);

    fireEvent.click(await screen.findByRole('tab', { name: '内存观察' }));

    expect(
      await screen.findByRole('tab', { name: 'Sessions' })
    ).toBeInTheDocument();
    expect(await screen.findByText('session:1')).toBeInTheDocument();
    expect(screen.getByText('1m 0s')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Metadata/ })).toBeEnabled();
    expect(screen.getByRole('button', { name: /Reveal/ })).toBeEnabled();
    expect(
      screen.queryByRole('button', { name: /清理/ })
    ).not.toBeInTheDocument();
  });

  test('explains when memory observation succeeds but there are no contracts', async () => {
    api.fetchSettingsHostInfrastructureProviders.mockResolvedValue([]);
    api.fetchSettingsHostInfrastructureMemoryOverview.mockResolvedValue({
      can_manage: true,
      contracts: []
    });

    renderPanel(true);

    fireEvent.click(await screen.findByRole('tab', { name: '内存观察' }));

    expect(
      await screen.findByText('暂无可观察内存 contract')
    ).toBeInTheDocument();
  });

  test('keeps memory reveal actions unavailable for view-only users', async () => {
    api.fetchSettingsHostInfrastructureProviders.mockResolvedValue([]);
    api.fetchSettingsHostInfrastructureMemoryOverview.mockResolvedValue({
      can_manage: false,
      contracts: [
        {
          contract_code: 'session-store',
          label: 'Sessions',
          provider_code: 'local',
          capabilities: {
            list_entries: true,
            reveal_value: true
          },
          entry_count: 1,
          sensitive_entry_count: 1,
          total_value_size_bytes: 1024,
          supported: true
        }
      ]
    });
    api.fetchSettingsHostInfrastructureMemoryEntries.mockResolvedValue({
      contract_code: 'session-store',
      label: 'Sessions',
      provider_code: 'local',
      capabilities: {
        list_entries: true,
        reveal_value: true
      },
      supported: true,
      entries: [
        {
          contract_code: 'session-store',
          group_code: 'sessions',
          key: 'session:1',
          entry_kind: 'session',
          status: 'active',
          owner: null,
          value_size_bytes: 1024,
          ttl_seconds: null,
          created_at_unix: null,
          expires_at_unix: null,
          sensitive: true,
          metadata: {}
        }
      ]
    });

    renderPanel(false);

    fireEvent.click(await screen.findByRole('tab', { name: '内存观察' }));

    expect(
      await screen.findByText('当前视图只展示 metadata。')
    ).toBeInTheDocument();
    expect(await screen.findByText('session:1')).toBeInTheDocument();
    expect(
      await screen.findByRole('button', { name: /Metadata/ })
    ).toBeEnabled();
    expect(
      screen.queryByRole('button', { name: /Reveal/ })
    ).not.toBeInTheDocument();
    expect(
      screen.queryByRole('button', { name: /清理/ })
    ).not.toBeInTheDocument();
  });

  test('reveals memory value directly for manage users', async () => {
    api.fetchSettingsHostInfrastructureProviders.mockResolvedValue([]);
    api.fetchSettingsHostInfrastructureMemoryOverview.mockResolvedValue({
      can_manage: true,
      contracts: [
        {
          contract_code: 'session-store',
          label: 'Sessions',
          provider_code: 'local',
          capabilities: {
            list_entries: true,
            reveal_value: true
          },
          entry_count: 1,
          sensitive_entry_count: 1,
          total_value_size_bytes: 1024,
          supported: true
        }
      ]
    });
    api.fetchSettingsHostInfrastructureMemoryEntries.mockResolvedValue({
      contract_code: 'session-store',
      label: 'Sessions',
      provider_code: 'local',
      capabilities: {
        list_entries: true,
        reveal_value: true
      },
      supported: true,
      entries: [
        {
          contract_code: 'session-store',
          group_code: 'sessions',
          key: 'session:1',
          entry_kind: 'session',
          status: 'active',
          owner: null,
          value_size_bytes: 1024,
          ttl_seconds: null,
          created_at_unix: null,
          expires_at_unix: null,
          sensitive: true,
          metadata: {}
        }
      ]
    });
    api.revealSettingsHostInfrastructureMemoryEntry.mockResolvedValue({
      metadata: {
        contract_code: 'session-store',
        group_code: 'sessions',
        key: 'session:1',
        entry_kind: 'session',
        status: 'active',
        owner: null,
        value_size_bytes: 1024,
        ttl_seconds: null,
        created_at_unix: null,
        expires_at_unix: null,
        sensitive: true,
        metadata: {}
      },
      value: { flow_run: { status: 'succeeded' } }
    });

    renderPanel(true);

    fireEvent.click(await screen.findByRole('tab', { name: '内存观察' }));
    fireEvent.click(await screen.findByRole('button', { name: /Reveal/ }));

    await waitFor(() => {
      expect(
        api.revealSettingsHostInfrastructureMemoryEntry
      ).toHaveBeenCalledWith('session-store', 'session:1', 'csrf-123');
    });
    expect(
      screen.queryByRole('button', { name: '查看并记录审计' })
    ).not.toBeInTheDocument();
    expect(await screen.findByText('Entry value')).toBeInTheDocument();
    expect(screen.getByLabelText('Memory value JSON')).toHaveTextContent(
      'succeeded'
    );
  });
});
