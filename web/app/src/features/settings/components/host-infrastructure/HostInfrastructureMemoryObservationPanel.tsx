import { useEffect, useMemo, useState } from 'react';

import {
  EyeOutlined,
  FileSearchOutlined,
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
  Tabs,
  Tag,
  Typography
} from 'antd';
import type { ColumnsType } from 'antd/es/table';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import { useAuthStore } from '../../../../state/auth-store';
import { JsonPreviewBlock } from '../../../../shared/ui/json-preview/JsonPreviewBlock';
import {
  fetchSettingsHostInfrastructureMemoryEntries,
  fetchSettingsHostInfrastructureMemoryOverview,
  revealSettingsHostInfrastructureMemoryEntry,
  settingsHostInfrastructureMemoryEntriesQueryKey,
  settingsHostInfrastructureMemoryOverviewQueryKey,
  type SettingsHostInfrastructureMemoryContract,
  type SettingsHostInfrastructureMemoryEntry,
  type SettingsHostInfrastructureMemoryEntryValue
} from '../../api/host-infrastructure';

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
    return '无过期';
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
    return '尚未刷新';
  }
  return new Date(value).toLocaleTimeString();
}

function resolveCanReveal(
  pageCanManage: boolean,
  overviewCanManage: boolean | undefined,
  contract: SettingsHostInfrastructureMemoryContract | undefined
) {
  return Boolean(
    pageCanManage &&
    overviewCanManage &&
    contract?.supported &&
    contract.capabilities.reveal_value
  );
}

export function HostInfrastructureMemoryObservationPanel({
  canManage
}: {
  canManage: boolean;
}) {
  const csrfToken = useAuthStore((state) => state.csrfToken);
  const [activeContractCode, setActiveContractCode] = useState<string | null>(
    null
  );
  const [metadataEntry, setMetadataEntry] =
    useState<SettingsHostInfrastructureMemoryEntry | null>(null);
  const [revealedEntry, setRevealedEntry] =
    useState<SettingsHostInfrastructureMemoryEntryValue | null>(null);
  const [modal, modalContextHolder] = Modal.useModal();
  const queryClient = useQueryClient();

  const overviewQuery = useQuery({
    queryKey: settingsHostInfrastructureMemoryOverviewQueryKey,
    queryFn: fetchSettingsHostInfrastructureMemoryOverview
  });
  const contracts = overviewQuery.data?.contracts ?? [];
  const activeContract = contracts.find(
    (contract) => contract.contract_code === activeContractCode
  );
  const canListEntries = Boolean(
    activeContract?.supported && activeContract.capabilities.list_entries
  );

  const entriesQuery = useQuery({
    queryKey:
      settingsHostInfrastructureMemoryEntriesQueryKey(activeContractCode),
    queryFn: () =>
      activeContractCode
        ? fetchSettingsHostInfrastructureMemoryEntries(activeContractCode)
        : Promise.resolve(null),
    enabled: Boolean(activeContractCode && canListEntries)
  });
  const entries = entriesQuery.data?.entries ?? [];
  const canReveal = resolveCanReveal(
    canManage,
    overviewQuery.data?.can_manage,
    activeContract
  );

  useEffect(() => {
    if (
      activeContractCode &&
      contracts.some(
        (contract) => contract.contract_code === activeContractCode
      )
    ) {
      return;
    }
    setActiveContractCode(contracts[0]?.contract_code ?? null);
  }, [activeContractCode, contracts]);

  const refreshMemoryQueries = async (contractCode: string | null) => {
    await Promise.all([
      queryClient.invalidateQueries({
        queryKey: settingsHostInfrastructureMemoryOverviewQueryKey
      }),
      queryClient.invalidateQueries({
        queryKey: settingsHostInfrastructureMemoryEntriesQueryKey(contractCode)
      })
    ]);
  };

  const revealMutation = useMutation({
    mutationFn: async (entry: SettingsHostInfrastructureMemoryEntry) => {
      if (!csrfToken) {
        throw new Error('csrf_missing');
      }
      return revealSettingsHostInfrastructureMemoryEntry(
        entry.contract_code,
        entry.key,
        csrfToken
      );
    },
    onSuccess: (value) => {
      setRevealedEntry(value);
    }
  });

  const entryColumns = useMemo<
    ColumnsType<SettingsHostInfrastructureMemoryEntry>
  >(
    () => [
      {
        title: 'Key',
        dataIndex: 'key',
        key: 'key',
        render: (key: string) => (
          <Typography.Text copyable className="host-memory-panel__key">
            {key}
          </Typography.Text>
        )
      },
      {
        title: 'Group',
        dataIndex: 'group_code',
        key: 'group_code',
        width: 140
      },
      {
        title: 'Kind',
        dataIndex: 'entry_kind',
        key: 'entry_kind',
        width: 130
      },
      {
        title: 'Status',
        dataIndex: 'status',
        key: 'status',
        width: 110,
        render: (status: string) => <Tag>{status}</Tag>
      },
      {
        title: 'Sensitive',
        dataIndex: 'sensitive',
        key: 'sensitive',
        width: 110,
        render: (sensitive: boolean) => (
          <Tag color={sensitive ? 'red' : 'default'}>
            {sensitive ? 'yes' : 'no'}
          </Tag>
        )
      },
      {
        title: 'TTL',
        dataIndex: 'ttl_seconds',
        key: 'ttl_seconds',
        width: 120,
        render: (ttl: number | null) => formatTtl(ttl)
      },
      {
        title: 'Size',
        dataIndex: 'value_size_bytes',
        key: 'value_size_bytes',
        width: 110,
        render: (size: number) => formatBytes(size)
      },
      {
        title: '',
        key: 'actions',
        width: canReveal ? 220 : 120,
        render: (_, entry) => (
          <Space size={4}>
            <Button
              icon={<FileSearchOutlined />}
              onClick={() => setMetadataEntry(entry)}
              size="small"
            >
              Metadata
            </Button>
            {canReveal ? (
              <Button
                icon={<EyeOutlined />}
                disabled={revealMutation.isPending}
                onClick={() => {
                  modal.confirm({
                    title: '查看内存 value',
                    content:
                      '这个操作可能展示用户输入、运行日志、模型输出或业务记录，并会写入审计日志。',
                    okText: '查看并记录审计',
                    cancelText: '取消',
                    onOk: () => revealMutation.mutateAsync(entry)
                  });
                }}
                size="small"
              >
                Reveal
              </Button>
            ) : null}
          </Space>
        )
      }
    ],
    [canReveal, modal, revealMutation]
  );

  if (overviewQuery.isError) {
    return (
      <Space direction="vertical" size={16} className="host-memory-panel">
        <Alert
          type="error"
          showIcon
          message="内存观察连接失败。"
          description="无法读取当前 api-server 的 host infrastructure memory observation API。"
        />
        <Button
          icon={<ReloadOutlined />}
          onClick={() => overviewQuery.refetch()}
          loading={overviewQuery.isFetching}
        >
          刷新
        </Button>
      </Space>
    );
  }

  return (
    <Space direction="vertical" size={16} className="host-memory-panel">
      {modalContextHolder}
      <div className="host-memory-panel__toolbar">
        <Space size={[8, 8]} wrap>
          <Tag color="blue">{contracts.length} contracts</Tag>
          <Tag>
            Reveal {overviewQuery.data?.can_manage ? 'available' : 'off'}
          </Tag>
          <Typography.Text type="secondary">
            最近刷新: {formatUpdatedAt(overviewQuery.dataUpdatedAt)}
          </Typography.Text>
        </Space>
        <Button
          icon={<ReloadOutlined />}
          onClick={() => {
            void refreshMemoryQueries(activeContractCode);
          }}
          loading={overviewQuery.isFetching || entriesQuery.isFetching}
        >
          刷新
        </Button>
      </div>

      {!canReveal ? (
        <Alert
          type="info"
          showIcon
          message="当前视图只展示 metadata。"
          description="Reveal value 需要基础设施 manage 权限和当前 contract 的 reveal_value 能力。"
        />
      ) : null}

      {overviewQuery.isSuccess && !contracts.length ? (
        <Empty
          image={Empty.PRESENTED_IMAGE_SIMPLE}
          description="暂无可观察内存 contract"
        />
      ) : null}

      {contracts.length ? (
        <Tabs
          activeKey={activeContractCode ?? undefined}
          onChange={setActiveContractCode}
          items={contracts.map((contract) => ({
            key: contract.contract_code,
            label: contract.label,
            children: (
              <Space
                direction="vertical"
                size={16}
                className="host-memory-panel__contract"
              >
                <Descriptions bordered size="small" column={{ xs: 1, md: 3 }}>
                  <Descriptions.Item label="Contract">
                    {contract.contract_code}
                  </Descriptions.Item>
                  <Descriptions.Item label="Provider">
                    {contract.provider_code ?? 'unknown'}
                  </Descriptions.Item>
                  <Descriptions.Item label="Supported">
                    {contract.supported ? 'yes' : 'no'}
                  </Descriptions.Item>
                  <Descriptions.Item label="Entries">
                    {contract.entry_count}
                  </Descriptions.Item>
                  <Descriptions.Item label="Sensitive">
                    {contract.sensitive_entry_count}
                  </Descriptions.Item>
                  <Descriptions.Item label="Value size">
                    {formatBytes(contract.total_value_size_bytes)}
                  </Descriptions.Item>
                </Descriptions>

                {!contract.supported || !contract.capabilities.list_entries ? (
                  <Alert
                    type="warning"
                    showIcon
                    message="当前 contract 不支持 entry inspection。"
                    description="可用能力会随 provider 暴露；当前无法列出这个 contract 的内存 entry。"
                  />
                ) : entriesQuery.isError ? (
                  <Alert
                    type="error"
                    showIcon
                    message="内存 entry 连接失败。"
                    description="无法读取当前 contract 的 entries。"
                  />
                ) : entriesQuery.isSuccess && !entries.length ? (
                  <Empty
                    image={Empty.PRESENTED_IMAGE_SIMPLE}
                    description="暂无内存 entry"
                  />
                ) : (
                  <Table
                    rowKey={(entry) => `${entry.group_code}:${entry.key}`}
                    columns={entryColumns}
                    dataSource={entries}
                    loading={entriesQuery.isLoading || overviewQuery.isLoading}
                    pagination={false}
                    size="small"
                  />
                )}
              </Space>
            )
          }))}
        />
      ) : null}

      <Drawer
        title="Entry metadata"
        width={640}
        open={Boolean(metadataEntry)}
        onClose={() => setMetadataEntry(null)}
        destroyOnClose
      >
        {metadataEntry ? (
          <Space
            direction="vertical"
            size={16}
            className="host-memory-panel__drawer"
          >
            <Descriptions column={1} size="small">
              <Descriptions.Item label="Contract">
                {metadataEntry.contract_code}
              </Descriptions.Item>
              <Descriptions.Item label="Group">
                {metadataEntry.group_code}
              </Descriptions.Item>
              <Descriptions.Item label="Key">
                {metadataEntry.key}
              </Descriptions.Item>
              <Descriptions.Item label="Owner">
                {metadataEntry.owner ?? 'unknown'}
              </Descriptions.Item>
              <Descriptions.Item label="Created">
                {formatUnixTimestamp(metadataEntry.created_at_unix)}
              </Descriptions.Item>
              <Descriptions.Item label="Expires">
                {formatUnixTimestamp(metadataEntry.expires_at_unix)}
              </Descriptions.Item>
            </Descriptions>
            <JsonPreviewBlock
              title="Metadata"
              value={metadataEntry.metadata}
              collapsible={false}
              height="320px"
              copySuccessMessage="已复制 metadata JSON"
            />
          </Space>
        ) : null}
      </Drawer>

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
            className="host-memory-panel__drawer"
          >
            <Descriptions column={1} size="small">
              <Descriptions.Item label="Contract">
                {revealedEntry.metadata.contract_code}
              </Descriptions.Item>
              <Descriptions.Item label="Group">
                {revealedEntry.metadata.group_code}
              </Descriptions.Item>
              <Descriptions.Item label="Key">
                {revealedEntry.metadata.key}
              </Descriptions.Item>
              <Descriptions.Item label="Size">
                {formatBytes(revealedEntry.metadata.value_size_bytes)}
              </Descriptions.Item>
            </Descriptions>
            <JsonPreviewBlock
              title="Memory value"
              value={revealedEntry.value}
              collapsible={false}
              height="360px"
              copySuccessMessage="已复制内存 JSON"
            />
          </Space>
        ) : null}
      </Drawer>
    </Space>
  );
}
