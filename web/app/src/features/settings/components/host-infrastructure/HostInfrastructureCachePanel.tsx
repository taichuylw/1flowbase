import { useEffect, useMemo, useState } from 'react';

import {
  CheckOutlined,
  DeleteOutlined,
  EyeOutlined,
  ReloadOutlined
} from '@ant-design/icons';
import {
  Alert,
  Button,
  Descriptions,
  Drawer,
  Empty,
  Modal,
  Space,
  Table,
  Tag,
  Typography,
  message
} from 'antd';
import type { ColumnsType } from 'antd/es/table';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import { useAuthStore } from '../../../../state/auth-store';
import { JsonPreviewBlock } from '../../../../shared/ui/json-preview/JsonPreviewBlock';
import {
  clearSettingsHostInfrastructureCacheDomain,
  clearSettingsHostInfrastructureCacheEntry,
  fetchSettingsHostInfrastructureCacheEntries,
  fetchSettingsHostInfrastructureCacheOverview,
  revealSettingsHostInfrastructureCacheEntry,
  settingsHostInfrastructureCacheEntriesQueryKey,
  settingsHostInfrastructureCacheOverviewQueryKey,
  type SettingsHostInfrastructureCacheDomain,
  type SettingsHostInfrastructureCacheEntry,
  type SettingsHostInfrastructureCacheEntryValue
} from '../../api/host-infrastructure';
import { i18nText } from '../../../../shared/i18n/text';

function formatBytes(value: number) {
  if (value < 1024) {
    return `${value} B`;
  }
  if (value < 1024 * 1024) {
    return `${(value / 1024).toFixed(1)} KB`;
  }
  return `${(value / 1024 / 1024).toFixed(1)} MB`;
}

function formatTtl(value: number | null) {
  if (value == null) {
    return i18nText("settings", "auto.k_ff139dbaf3");
  }
  if (value < 60) {
    return `${value}s`;
  }
  if (value < 3600) {
    return `${Math.floor(value / 60)}m ${value % 60}s`;
  }
  return `${Math.floor(value / 3600)}h ${Math.floor((value % 3600) / 60)}m`;
}

function formatUnixTimestamp(value: number | null) {
  if (value == null) {
    return 'unknown';
  }
  return new Date(value * 1000).toLocaleString();
}

function formatUpdatedAt(value: number) {
  if (!value) {
    return i18nText("settings", "auto.k_b6b13cc32c");
  }
  return new Date(value).toLocaleTimeString();
}

export function HostInfrastructureCachePanel({
  canManage
}: {
  canManage: boolean;
}) {
  const csrfToken = useAuthStore((state) => state.csrfToken);
  const [activeDomain, setActiveDomain] = useState<string | null>(null);
  const [revealedEntry, setRevealedEntry] =
    useState<SettingsHostInfrastructureCacheEntryValue | null>(null);
  const [modal, modalContextHolder] = Modal.useModal();
  const [messageApi, messageContextHolder] = message.useMessage();
  const queryClient = useQueryClient();
  const overviewQuery = useQuery({
    queryKey: settingsHostInfrastructureCacheOverviewQueryKey,
    queryFn: fetchSettingsHostInfrastructureCacheOverview
  });
  const domains = overviewQuery.data?.domains ?? [];
  const capabilities = overviewQuery.data?.capabilities;
  const entriesQuery = useQuery({
    queryKey: settingsHostInfrastructureCacheEntriesQueryKey(activeDomain),
    queryFn: () =>
      activeDomain
        ? fetchSettingsHostInfrastructureCacheEntries(activeDomain)
        : Promise.resolve(null),
    enabled: Boolean(activeDomain && capabilities?.list_entries)
  });
  const entries = entriesQuery.data?.entries ?? [];

  useEffect(() => {
    if (
      activeDomain &&
      domains.some((domain) => domain.domain_code === activeDomain)
    ) {
      return;
    }
    setActiveDomain(domains[0]?.domain_code ?? null);
  }, [activeDomain, domains]);

  const refreshCacheQueries = async (domainCode: string | null) => {
    await Promise.all([
      queryClient.invalidateQueries({
        queryKey: settingsHostInfrastructureCacheOverviewQueryKey
      }),
      queryClient.invalidateQueries({
        queryKey: settingsHostInfrastructureCacheEntriesQueryKey(domainCode)
      })
    ]);
  };

  const revealMutation = useMutation({
    mutationFn: async (entry: SettingsHostInfrastructureCacheEntry) => {
      if (!csrfToken) {
        throw new Error('csrf_missing');
      }
      return revealSettingsHostInfrastructureCacheEntry(
        entry.domain_code,
        entry.key,
        csrfToken
      );
    },
    onSuccess: (value) => {
      setRevealedEntry(value);
    }
  });
  const clearEntryMutation = useMutation({
    mutationFn: async (entry: SettingsHostInfrastructureCacheEntry) => {
      if (!csrfToken) {
        throw new Error('csrf_missing');
      }
      return clearSettingsHostInfrastructureCacheEntry(
        entry.domain_code,
        entry.key,
        csrfToken
      );
    },
    onSuccess: async (_, entry) => {
      await refreshCacheQueries(entry.domain_code);
      messageApi.success(i18nText("settings", "auto.k_3a49d24203"));
    }
  });
  const clearDomainMutation = useMutation({
    mutationFn: async (domainCode: string) => {
      if (!csrfToken) {
        throw new Error('csrf_missing');
      }
      return clearSettingsHostInfrastructureCacheDomain(domainCode, csrfToken);
    },
    onSuccess: async (_, domainCode) => {
      await refreshCacheQueries(domainCode);
      messageApi.success(i18nText("settings", "auto.k_4a8f5abd96"));
    }
  });

  const canReveal = Boolean(canManage && capabilities?.reveal_value);
  const canClearEntry = Boolean(canManage && capabilities?.clear_entry);
  const canClearDomain = Boolean(canManage && capabilities?.clear_domain);

  const entryColumns = useMemo<
    ColumnsType<SettingsHostInfrastructureCacheEntry>
  >(
    () => [
      {
        title: 'Key',
        dataIndex: 'key',
        key: 'key',
        render: (key: string) => (
          <Typography.Text copyable className="host-cache-panel__key">
            {key}
          </Typography.Text>
        )
      },
      {
        title: 'TTL',
        dataIndex: 'ttl_seconds',
        key: 'ttl',
        width: 120,
        render: (ttl: number | null) => formatTtl(ttl)
      },
      {
        title: 'Size',
        dataIndex: 'value_size_bytes',
        key: 'size',
        width: 110,
        render: (size: number) => formatBytes(size)
      },
      {
        title: '',
        key: 'actions',
        width: 190,
        render: (_, entry) =>
          canManage ? (
            <Space size={4}>
              <Button
                icon={<EyeOutlined />}
                disabled={!canReveal || revealMutation.isPending}
                onClick={() => {
                  modal.confirm({
                    title: i18nText("settings", "auto.k_81f04f1ca1"),
                    content:
                      i18nText("settings", "auto.k_17f57796d8"),
                    okText: i18nText("settings", "auto.k_190bf928b1"),
                    cancelText: i18nText("settings", "auto.cancel"),
                    onOk: () => revealMutation.mutateAsync(entry)
                  });
                }}
                size="small"
              >
                {i18nText("settings", "auto.k_ea8b000633")}</Button>
              <Button
                danger
                icon={<DeleteOutlined />}
                disabled={!canClearEntry || clearEntryMutation.isPending}
                onClick={() => {
                  modal.confirm({
                    title: i18nText("settings", "auto.k_073c7b8b23"),
                    content: i18nText("settings", "auto.k_30cbb97b6a", { value1: entry.key }),
                    okText: i18nText("settings", "auto.k_f1c3ef7622"),
                    okButtonProps: { danger: true },
                    cancelText: i18nText("settings", "auto.cancel"),
                    onOk: () => clearEntryMutation.mutateAsync(entry)
                  });
                }}
                size="small"
              >
                {i18nText("settings", "auto.k_907c3945d8")}</Button>
            </Space>
          ) : (
            <Typography.Text type="secondary">{i18nText("settings", "auto.k_c27081c863")}</Typography.Text>
          )
      }
    ],
    [
      canClearEntry,
      canManage,
      canReveal,
      clearEntryMutation,
      modal,
      revealMutation
    ]
  );

  const activeDomainSummary = domains.find(
    (domain) => domain.domain_code === activeDomain
  );

  if (overviewQuery.isSuccess && capabilities && !capabilities.list_domains) {
    return (
      <Space direction="vertical" size={16} className="host-cache-panel">
        {modalContextHolder}
        {messageContextHolder}
        <Alert
          type="warning"
          showIcon
          message={i18nText("settings", "auto.k_0f6be2f3d5")}
          description={i18nText("settings", "auto.k_15d732c71c")}
        />
        <Button
          icon={<ReloadOutlined />}
          onClick={() => overviewQuery.refetch()}
          loading={overviewQuery.isFetching}
        >
          {i18nText("settings", "auto.k_dc6755b83a")}</Button>
      </Space>
    );
  }

  return (
    <Space direction="vertical" size={16} className="host-cache-panel">
      {modalContextHolder}
      {messageContextHolder}
      <div className="host-cache-panel__toolbar">
        <Space size={[8, 8]} wrap>
          <Tag color="blue">
            Provider: {overviewQuery.data?.provider_code ?? 'unknown'}
          </Tag>
          <Tag
            icon={capabilities?.reveal_value ? <CheckOutlined /> : undefined}
          >
            Value inspection {capabilities?.reveal_value ? 'on' : 'off'}
          </Tag>
          <Typography.Text type="secondary">
            {i18nText("settings", "auto.k_8827f80e7b")}{formatUpdatedAt(overviewQuery.dataUpdatedAt)}
          </Typography.Text>
        </Space>
        <Button
          icon={<ReloadOutlined />}
          onClick={() => {
            void overviewQuery.refetch();
            if (activeDomain) {
              void entriesQuery.refetch();
            }
          }}
          loading={overviewQuery.isFetching || entriesQuery.isFetching}
        >
          {i18nText("settings", "auto.refresh")}</Button>
      </div>

      {!canManage ? (
        <Alert
          type="info"
          showIcon
          message={i18nText("settings", "auto.k_db605406d8")}
          description={i18nText("settings", "auto.k_061e7014e0")}
        />
      ) : null}

      {overviewQuery.isSuccess && capabilities?.list_domains && !domains.length ? (
        <Alert
          type="info"
          showIcon
          message={i18nText("settings", "auto.k_b486563b7c")}
          description={i18nText("settings", "auto.k_5c6f5a6b5a")}
        />
      ) : null}

      <div className="host-cache-panel__layout">
        <aside className="host-cache-panel__domains">
          <div className="host-cache-panel__section-title">{i18nText("settings", "auto.k_d4766fc965")}</div>
          {domains.length ? (
            <Space
              direction="vertical"
              size={8}
              className="host-cache-panel__domain-list"
            >
              {domains.map((domain) => (
                <button
                  className={[
                    'host-cache-panel__domain-button',
                    domain.domain_code === activeDomain
                      ? 'host-cache-panel__domain-button--active'
                      : ''
                  ]
                    .filter(Boolean)
                    .join(' ')}
                  key={domain.domain_code}
                  onClick={() => setActiveDomain(domain.domain_code)}
                  type="button"
                >
                  <span>{domain.domain_code}</span>
                  <Typography.Text type="secondary">
                    {domain.entry_count} entries ·{' '}
                    {formatBytes(domain.total_value_size_bytes)}
                  </Typography.Text>
                </button>
              ))}
            </Space>
          ) : (
            <Empty
              image={Empty.PRESENTED_IMAGE_SIMPLE}
              description={i18nText("settings", "auto.k_9e6e8a15d8")}
            />
          )}
        </aside>

        <section className="host-cache-panel__entries">
          <div className="host-cache-panel__entries-header">
            <Space direction="vertical" size={2}>
              <Typography.Text strong>
                {activeDomain ?? i18nText("settings", "auto.k_bfbe272414")}
              </Typography.Text>
              {activeDomainSummary ? (
                <Typography.Text type="secondary">
                  {activeDomainSummary.entry_count} entries ·{' '}
                  {formatBytes(activeDomainSummary.total_value_size_bytes)}
                </Typography.Text>
              ) : null}
            </Space>
            {activeDomain && canManage ? (
              <Button
                danger
                icon={<DeleteOutlined />}
                disabled={!canClearDomain || clearDomainMutation.isPending}
                onClick={() => {
                  modal.confirm({
                    title: i18nText("settings", "auto.k_39dc9b475e"),
                    content: i18nText("settings", "auto.k_c509239c72", { value1: activeDomain }),
                    okText: i18nText("settings", "auto.k_f1c3ef7622"),
                    okButtonProps: { danger: true },
                    cancelText: i18nText("settings", "auto.cancel"),
                    onOk: () => clearDomainMutation.mutateAsync(activeDomain)
                  });
                }}
              >
                {i18nText("settings", "auto.k_39dc9b475e")}</Button>
            ) : null}
          </div>
          <Table
            rowKey="key"
            columns={entryColumns}
            dataSource={entries}
            loading={entriesQuery.isLoading || overviewQuery.isLoading}
            pagination={false}
            size="small"
          />
        </section>
      </div>

      <Drawer
        title="Entry value"
        width={640}
        open={Boolean(revealedEntry)}
        onClose={() => setRevealedEntry(null)}
        destroyOnClose
      >
        {revealedEntry ? (
          <Space
            direction="vertical"
            size={16}
            className="host-cache-panel__drawer"
          >
            <Descriptions column={1} size="small">
              <Descriptions.Item label="Domain">
                {revealedEntry.metadata.domain_code}
              </Descriptions.Item>
              <Descriptions.Item label="Key">
                {revealedEntry.metadata.key}
              </Descriptions.Item>
              <Descriptions.Item label="TTL">
                {formatTtl(revealedEntry.metadata.ttl_seconds)}
              </Descriptions.Item>
              <Descriptions.Item label="Size">
                {formatBytes(revealedEntry.metadata.value_size_bytes)}
              </Descriptions.Item>
              <Descriptions.Item label="Created">
                {formatUnixTimestamp(revealedEntry.metadata.created_at_unix)}
              </Descriptions.Item>
              <Descriptions.Item label="Expires">
                {formatUnixTimestamp(revealedEntry.metadata.expires_at_unix)}
              </Descriptions.Item>
            </Descriptions>
            <JsonPreviewBlock
              title="Cache value"
              value={revealedEntry.value}
              collapsible={false}
              height="360px"
              copySuccessMessage={i18nText("settings", "auto.k_4e4a4870f7")}
            />
          </Space>
        ) : null}
      </Drawer>
    </Space>
  );
}
