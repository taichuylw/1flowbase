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
  settingsHostInfrastructureCacheOverviewQueryKey: [
    'settings',
    'host-infrastructure',
    'cache'
  ],
  settingsHostInfrastructureCacheEntriesQueryKey: vi.fn(
    (domainCode: string | null) => [
      'settings',
      'host-infrastructure',
      'cache',
      'domains',
      domainCode,
      'entries'
    ]
  ),
  fetchSettingsHostInfrastructureProviders: vi.fn(),
  saveSettingsHostInfrastructureProviderConfig: vi.fn(),
  fetchSettingsHostInfrastructureCacheOverview: vi.fn(),
  fetchSettingsHostInfrastructureCacheEntries: vi.fn(),
  revealSettingsHostInfrastructureCacheEntry: vi.fn(),
  clearSettingsHostInfrastructureCacheEntry: vi.fn(),
  clearSettingsHostInfrastructureCacheDomain: vi.fn()
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
    api.fetchSettingsHostInfrastructureCacheOverview.mockResolvedValue({
      provider_code: 'local',
      can_manage: true,
      capabilities: {
        list_domains: true,
        list_entries: true,
        reveal_value: true,
        clear_entry: true,
        clear_domain: true
      },
      domains: []
    });
    api.fetchSettingsHostInfrastructureCacheEntries.mockResolvedValue({
      domain_code: 'application-logs',
      capabilities: {
        list_domains: true,
        list_entries: true,
        reveal_value: true,
        clear_entry: true,
        clear_domain: true
      },
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

  test('renders cache domains and metadata entries', async () => {
    api.fetchSettingsHostInfrastructureProviders.mockResolvedValue([]);
    api.fetchSettingsHostInfrastructureCacheOverview.mockResolvedValue({
      provider_code: 'local',
      can_manage: true,
      capabilities: {
        list_domains: true,
        list_entries: true,
        reveal_value: true,
        clear_entry: true,
        clear_domain: true
      },
      domains: [
        {
          domain_code: 'application-logs',
          entry_count: 2,
          total_value_size_bytes: 2048
        }
      ]
    });
    api.fetchSettingsHostInfrastructureCacheEntries.mockResolvedValue({
      domain_code: 'application-logs',
      capabilities: {
        list_domains: true,
        list_entries: true,
        reveal_value: true,
        clear_entry: true,
        clear_domain: true
      },
      entries: [
        {
          domain_code: 'application-logs',
          key: 'application-logs:run:1',
          value_size_bytes: 1024,
          ttl_seconds: 60,
          created_at_unix: 1_700_000_000,
          expires_at_unix: 1_700_000_060
        }
      ]
    });

    renderPanel(true);

    fireEvent.click(await screen.findByRole('tab', { name: '缓存观察' }));

    expect(
      (await screen.findAllByText('application-logs')).length
    ).toBeGreaterThan(0);
    expect(
      await screen.findByText('application-logs:run:1')
    ).toBeInTheDocument();
    expect(screen.getByText('1m 0s')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /查看 value/ })).toBeEnabled();
    expect(screen.getAllByRole('button', { name: /清理/ })[0]).toBeEnabled();
  });

  test('explains when cache inspection succeeds but the current store is empty', async () => {
    api.fetchSettingsHostInfrastructureProviders.mockResolvedValue([]);
    api.fetchSettingsHostInfrastructureCacheOverview.mockResolvedValue({
      provider_code: 'local',
      can_manage: true,
      capabilities: {
        list_domains: true,
        list_entries: true,
        reveal_value: true,
        clear_entry: true,
        clear_domain: true
      },
      domains: []
    });

    renderPanel(true);

    fireEvent.click(await screen.findByRole('tab', { name: '缓存观察' }));

    expect(
      await screen.findByText('当前 cache-store 没有可观察 entry。')
    ).toBeInTheDocument();
    expect(
      screen.getByText(
        'API 已连接到当前 api-server 进程的 local Moka cache-store；没有缓存域表示当前进程里暂时没有 entry，或重启后内存缓存已清空。'
      )
    ).toBeInTheDocument();
  });

  test('keeps cache value and clear actions unavailable for view-only users', async () => {
    api.fetchSettingsHostInfrastructureProviders.mockResolvedValue([]);
    api.fetchSettingsHostInfrastructureCacheOverview.mockResolvedValue({
      provider_code: 'local',
      can_manage: false,
      capabilities: {
        list_domains: true,
        list_entries: true,
        reveal_value: true,
        clear_entry: true,
        clear_domain: true
      },
      domains: [
        {
          domain_code: 'application-logs',
          entry_count: 1,
          total_value_size_bytes: 1024
        }
      ]
    });
    api.fetchSettingsHostInfrastructureCacheEntries.mockResolvedValue({
      domain_code: 'application-logs',
      capabilities: {
        list_domains: true,
        list_entries: true,
        reveal_value: true,
        clear_entry: true,
        clear_domain: true
      },
      entries: [
        {
          domain_code: 'application-logs',
          key: 'application-logs:run:1',
          value_size_bytes: 1024,
          ttl_seconds: null,
          created_at_unix: null,
          expires_at_unix: null
        }
      ]
    });

    renderPanel(false);

    fireEvent.click(await screen.findByRole('tab', { name: '缓存观察' }));

    expect(await screen.findByText('仅 metadata')).toBeInTheDocument();
    expect(
      screen.queryByRole('button', { name: /查看 value/ })
    ).not.toBeInTheDocument();
    expect(
      screen.queryByRole('button', { name: '清理缓存域' })
    ).not.toBeInTheDocument();
  });

  test('reveals cache value after explicit confirmation', async () => {
    api.fetchSettingsHostInfrastructureProviders.mockResolvedValue([]);
    api.fetchSettingsHostInfrastructureCacheOverview.mockResolvedValue({
      provider_code: 'local',
      can_manage: true,
      capabilities: {
        list_domains: true,
        list_entries: true,
        reveal_value: true,
        clear_entry: true,
        clear_domain: true
      },
      domains: [
        {
          domain_code: 'application-logs',
          entry_count: 1,
          total_value_size_bytes: 1024
        }
      ]
    });
    api.fetchSettingsHostInfrastructureCacheEntries.mockResolvedValue({
      domain_code: 'application-logs',
      capabilities: {
        list_domains: true,
        list_entries: true,
        reveal_value: true,
        clear_entry: true,
        clear_domain: true
      },
      entries: [
        {
          domain_code: 'application-logs',
          key: 'application-logs:run:1',
          value_size_bytes: 1024,
          ttl_seconds: null,
          created_at_unix: null,
          expires_at_unix: null
        }
      ]
    });
    api.revealSettingsHostInfrastructureCacheEntry.mockResolvedValue({
      metadata: {
        domain_code: 'application-logs',
        key: 'application-logs:run:1',
        value_size_bytes: 1024,
        ttl_seconds: null,
        created_at_unix: null,
        expires_at_unix: null
      },
      value: { flow_run: { status: 'succeeded' } }
    });

    renderPanel(true);

    fireEvent.click(await screen.findByRole('tab', { name: '缓存观察' }));
    fireEvent.click(await screen.findByRole('button', { name: /查看 value/ }));
    fireEvent.click(
      await screen.findByRole('button', { name: '查看并记录审计' })
    );

    await waitFor(() => {
      expect(
        api.revealSettingsHostInfrastructureCacheEntry
      ).toHaveBeenCalledWith(
        'application-logs',
        'application-logs:run:1',
        'csrf-123'
      );
    });
    expect(await screen.findByText('Entry value')).toBeInTheDocument();
    expect(screen.getByLabelText('Cache value JSON')).toHaveTextContent(
      'succeeded'
    );
  });
});
