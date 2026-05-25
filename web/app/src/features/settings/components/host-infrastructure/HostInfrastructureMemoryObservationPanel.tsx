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
  Input,
  Space,
  Table,
  Tabs,
  Tag,
  Tree,
  Typography
} from 'antd';
import type { ColumnsType } from 'antd/es/table';
import type { DataNode } from 'antd/es/tree';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import { useAuthStore } from '../../../../state/auth-store';
import { JsonPreviewBlock } from '../../../../shared/ui/json-preview/JsonPreviewBlock';
import {
  fetchSettingsHostInfrastructureMemoryEntries,
  fetchSettingsHostInfrastructureMemoryOverview,
  fetchSettingsHostInfrastructureMemoryTree,
  revealSettingsHostInfrastructureMemoryEntry,
  searchSettingsHostInfrastructureMemoryEntries,
  settingsHostInfrastructureMemoryEntriesQueryKey,
  settingsHostInfrastructureMemoryOverviewQueryKey,
  settingsHostInfrastructureMemorySearchQueryKey,
  settingsHostInfrastructureMemoryTreeQueryKey,
  type SettingsHostInfrastructureMemoryContract,
  type SettingsHostInfrastructureMemoryEntry,
  type SettingsHostInfrastructureMemoryEntryValue,
  type SettingsHostInfrastructureMemoryTreeNode
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

function formatInspectionPath(path: string[]) {
  return path.length ? path.join(' / ') : 'root';
}

type MemoryTreeDataNode = DataNode & {
  inspectionPath: string[];
  children?: MemoryTreeDataNode[];
};

function findTreeKeyByPath(
  nodes: MemoryTreeDataNode[],
  inspectionPath: string[] | null
): string | null {
  if (!inspectionPath) {
    return null;
  }
  const requestedPath = inspectionPath.join('\u001f');
  for (const node of nodes) {
    if (node.inspectionPath.join('\u001f') === requestedPath) {
      return String(node.key);
    }
    const childKey = findTreeKeyByPath(node.children ?? [], inspectionPath);
    if (childKey) {
      return childKey;
    }
  }
  return null;
}

function toTreeData(
  nodes: SettingsHostInfrastructureMemoryTreeNode[],
  loadedChildren: Record<string, SettingsHostInfrastructureMemoryTreeNode[]>
): MemoryTreeDataNode[] {
  return nodes.map((node) => ({
    key: node.node_ref,
    title: (
      <Space size={6}>
        <Typography.Text>{node.label}</Typography.Text>
        <Tag>{node.entry_count}</Tag>
      </Space>
    ),
    isLeaf: !node.has_children,
    inspectionPath: node.inspection_path,
    children: loadedChildren[node.node_ref]
      ? toTreeData(loadedChildren[node.node_ref], loadedChildren)
      : undefined
  }));
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
  const [selectedInspectionPath, setSelectedInspectionPath] = useState<
    string[] | null
  >(null);
  const [entryCursor, setEntryCursor] = useState<string | null>(null);
  const [cursorHistory, setCursorHistory] = useState<string[]>([]);
  const [searchText, setSearchText] = useState('');
  const [submittedSearch, setSubmittedSearch] = useState('');
  const [loadedTreeChildren, setLoadedTreeChildren] = useState<
    Record<string, SettingsHostInfrastructureMemoryTreeNode[]>
  >({});
  const [metadataEntry, setMetadataEntry] =
    useState<SettingsHostInfrastructureMemoryEntry | null>(null);
  const [revealedEntry, setRevealedEntry] =
    useState<SettingsHostInfrastructureMemoryEntryValue | null>(null);
  const queryClient = useQueryClient();

  const overviewQuery = useQuery({
    queryKey: settingsHostInfrastructureMemoryOverviewQueryKey,
    queryFn: fetchSettingsHostInfrastructureMemoryOverview
  });
  const contracts = overviewQuery.data?.contracts ?? [];
  const resolvedActiveContractCode =
    activeContractCode &&
    contracts.some((contract) => contract.contract_code === activeContractCode)
      ? activeContractCode
      : (contracts[0]?.contract_code ?? null);
  const activeContract = contracts.find(
    (contract) => contract.contract_code === resolvedActiveContractCode
  );
  const pageSize = activeContract?.capabilities.default_page_size ?? 50;
  const canListEntries = Boolean(
    activeContract?.supported && activeContract.capabilities.list_entries
  );
  const canListTree = Boolean(
    activeContract?.supported && activeContract.capabilities.list_tree
  );
  const canSearchEntries = Boolean(
    activeContract?.supported && activeContract.capabilities.search_entries
  );
  const entryRequest = selectedInspectionPath
    ? {
        inspection_path: selectedInspectionPath,
        cursor: entryCursor,
        limit: pageSize
      }
    : undefined;
  const entriesQuery = useQuery({
    queryKey: submittedSearch
      ? canSearchEntries
        ? settingsHostInfrastructureMemorySearchQueryKey(
            resolvedActiveContractCode,
            entryRequest
              ? { ...entryRequest, q: submittedSearch }
              : { q: submittedSearch }
          )
        : settingsHostInfrastructureMemoryEntriesQueryKey(
            resolvedActiveContractCode,
            entryRequest
          )
      : settingsHostInfrastructureMemoryEntriesQueryKey(
          resolvedActiveContractCode,
          entryRequest
        ),
    queryFn: () => {
      if (!resolvedActiveContractCode || !entryRequest) {
        return Promise.resolve(null);
      }
      if (submittedSearch && canSearchEntries) {
        return searchSettingsHostInfrastructureMemoryEntries(
          resolvedActiveContractCode,
          { ...entryRequest, q: submittedSearch }
        );
      }
      return fetchSettingsHostInfrastructureMemoryEntries(
        resolvedActiveContractCode,
        entryRequest
      );
    },
    enabled: Boolean(
      resolvedActiveContractCode && canListEntries && entryRequest
    )
  });
  const rootTreeQuery = useQuery({
    queryKey: settingsHostInfrastructureMemoryTreeQueryKey(
      resolvedActiveContractCode,
      {
        inspection_path: [],
        limit: pageSize
      }
    ),
    queryFn: () =>
      resolvedActiveContractCode
        ? fetchSettingsHostInfrastructureMemoryTree(
            resolvedActiveContractCode,
            {
              inspection_path: [],
              limit: pageSize
            }
          )
        : Promise.resolve(null),
    enabled: Boolean(resolvedActiveContractCode && canListTree)
  });
  const entries = entriesQuery.data?.entries ?? [];
  const canReveal = resolveCanReveal(
    canManage,
    overviewQuery.data?.can_manage,
    activeContract
  );

  useEffect(() => {
    setLoadedTreeChildren({});
    setSelectedInspectionPath(null);
    setEntryCursor(null);
    setCursorHistory([]);
    setSubmittedSearch('');
    setSearchText('');
  }, [resolvedActiveContractCode]);

  const refreshMemoryQueries = async (contractCode: string | null) => {
    await Promise.all([
      queryClient.invalidateQueries({
        queryKey: settingsHostInfrastructureMemoryOverviewQueryKey
      }),
      queryClient.invalidateQueries({
        queryKey: [
          'settings',
          'host-infrastructure',
          'memory',
          'contracts',
          contractCode
        ]
      })
    ]);
  };

  const revealMutation = useMutation({
    mutationFn: async ({
      entry,
      revealMode
    }: {
      entry: SettingsHostInfrastructureMemoryEntry;
      revealMode: 'preview' | 'full';
    }) => {
      if (!csrfToken) {
        throw new Error('csrf_missing');
      }
      return revealSettingsHostInfrastructureMemoryEntry(
        entry.contract_code,
        entry.entry_ref,
        csrfToken,
        revealMode
      );
    },
    onSuccess: (value) => {
      setRevealedEntry(value);
    }
  });

  const rootNodes = rootTreeQuery.data?.nodes ?? [];
  const treeData = toTreeData(rootNodes, loadedTreeChildren);
  const selectedTreeKey = findTreeKeyByPath(treeData, selectedInspectionPath);

  const loadTreeChildren = async (treeNode: DataNode) => {
    if (!resolvedActiveContractCode) {
      return;
    }
    const node = treeNode as DataNode & {
      inspectionPath?: string[];
      isLeaf?: boolean;
    };
    if (node.isLeaf || !node.inspectionPath) {
      return;
    }
    const response = await queryClient.fetchQuery({
      queryKey: settingsHostInfrastructureMemoryTreeQueryKey(
        resolvedActiveContractCode,
        { inspection_path: node.inspectionPath, limit: pageSize }
      ),
      queryFn: () =>
        fetchSettingsHostInfrastructureMemoryTree(resolvedActiveContractCode, {
          inspection_path: node.inspectionPath,
          limit: pageSize
        })
    });
    setLoadedTreeChildren((current) => ({
      ...current,
      [String(node.key)]: response.nodes
    }));
  };

  const selectContract = (contractCode: string) => {
    setActiveContractCode(contractCode);
  };

  const selectInspectionPath = (path: string[]) => {
    setSelectedInspectionPath(path);
    setEntryCursor(null);
    setCursorHistory([]);
  };

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
                loading={revealMutation.isPending}
                onClick={() => {
                  revealMutation.mutate({ entry, revealMode: 'preview' });
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
    [canReveal, revealMutation]
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
            void refreshMemoryQueries(resolvedActiveContractCode);
          }}
          loading={
            overviewQuery.isFetching ||
            entriesQuery.isFetching ||
            rootTreeQuery.isFetching
          }
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
          activeKey={resolvedActiveContractCode ?? undefined}
          className="host-memory-panel__tabs"
          items={contracts.map((contract) => ({
            key: contract.contract_code,
            label: (
              <span className="host-memory-panel__tab-label">
                <span>{contract.label}</span>
                <Tag className="host-memory-panel__tab-count">
                  {contract.entry_count}
                </Tag>
              </span>
            ),
            children:
              contract.contract_code === resolvedActiveContractCode ? (
                <div className="host-memory-panel__layout">
                  <Space
                    direction="vertical"
                    size={12}
                    className="host-memory-panel__tree"
                  >
                    {activeContract ? (
                      <>
                        <div className="host-memory-panel__contract-summary">
                          <Typography.Text strong>
                            {activeContract.contract_code}
                          </Typography.Text>
                          <Typography.Text type="secondary">
                            {activeContract.provider_code ?? 'unknown'}
                          </Typography.Text>
                          <Tag color="red">
                            sensitive {activeContract.sensitive_entry_count}
                          </Tag>
                          <Typography.Text type="secondary">
                            {formatBytes(activeContract.total_value_size_bytes)}
                          </Typography.Text>
                        </div>

                        {!activeContract.supported ||
                        !activeContract.capabilities.list_tree ? (
                          <Alert
                            type="warning"
                            showIcon
                            message="当前 contract 不支持 tree inspection。"
                          />
                        ) : rootTreeQuery.isError ? (
                          <Alert
                            type="error"
                            showIcon
                            message="内存树加载失败。"
                          />
                        ) : rootTreeQuery.isSuccess && !rootNodes.length ? (
                          <Empty
                            image={Empty.PRESENTED_IMAGE_SIMPLE}
                            description="暂无内存节点"
                          />
                        ) : (
                          <div className="host-memory-panel__tree-body">
                            <Tree
                              treeData={treeData}
                              loadData={loadTreeChildren}
                              selectedKeys={
                                selectedTreeKey ? [selectedTreeKey] : []
                              }
                              onSelect={(_, info) => {
                                const node = info.node as DataNode & {
                                  inspectionPath?: string[];
                                };
                                if (node.inspectionPath) {
                                  selectInspectionPath(node.inspectionPath);
                                }
                              }}
                            />
                          </div>
                        )}
                      </>
                    ) : (
                      <Empty
                        image={Empty.PRESENTED_IMAGE_SIMPLE}
                        description="请选择内存 contract"
                      />
                    )}
                  </Space>

                  <Space
                    direction="vertical"
                    size={12}
                    className="host-memory-panel__entries"
                  >
                    <div className="host-memory-panel__entries-header">
                      <Space direction="vertical" size={2}>
                        <Typography.Text strong>Entries</Typography.Text>
                        <Typography.Text type="secondary">
                          {selectedInspectionPath
                            ? formatInspectionPath(selectedInspectionPath)
                            : '未选择路径'}
                        </Typography.Text>
                      </Space>
                      <Input.Search
                        allowClear
                        disabled={!canSearchEntries}
                        value={searchText}
                        onChange={(event) => setSearchText(event.target.value)}
                        onSearch={(value) => {
                          if (!canSearchEntries) {
                            return;
                          }
                          setSubmittedSearch(value.trim());
                          setEntryCursor(null);
                          setCursorHistory([]);
                        }}
                        size="small"
                        style={{ maxWidth: 240 }}
                      />
                    </div>

                    {!selectedInspectionPath ? (
                      <Empty
                        image={Empty.PRESENTED_IMAGE_SIMPLE}
                        description="请选择 tree 节点"
                      />
                    ) : entriesQuery.isError ? (
                      <Alert
                        type="error"
                        showIcon
                        message="内存 entry 连接失败。"
                        description="无法读取当前路径的 entries。"
                      />
                    ) : entriesQuery.isSuccess && !entries.length ? (
                      <Empty
                        image={Empty.PRESENTED_IMAGE_SIMPLE}
                        description="暂无内存 entry"
                      />
                    ) : (
                      <>
                        <Table
                          rowKey={(entry) => entry.entry_ref}
                          columns={entryColumns}
                          dataSource={entries}
                          loading={
                            entriesQuery.isLoading || entriesQuery.isFetching
                          }
                          pagination={false}
                          size="small"
                        />
                        <div className="host-memory-panel__entries-header">
                          <Typography.Text type="secondary">
                            {entriesQuery.data
                              ? `${formatBytes(
                                  entriesQuery.data.emitted_bytes
                                )} emitted`
                              : null}
                          </Typography.Text>
                          <Space size={8}>
                            <Button
                              size="small"
                              disabled={!cursorHistory.length}
                              onClick={() => {
                                setCursorHistory((current) => {
                                  const previousCursor = current.at(-1) ?? null;
                                  const nextHistory = current.slice(0, -1);
                                  setEntryCursor(previousCursor || null);
                                  return nextHistory;
                                });
                              }}
                            >
                              上一页
                            </Button>
                            <Button
                              size="small"
                              disabled={!entriesQuery.data?.next_cursor}
                              onClick={() => {
                                const nextCursor =
                                  entriesQuery.data?.next_cursor;
                                if (!nextCursor) {
                                  return;
                                }
                                setCursorHistory((current) => [
                                  ...current,
                                  entryCursor ?? ''
                                ]);
                                setEntryCursor(nextCursor);
                              }}
                            >
                              下一页
                            </Button>
                          </Space>
                        </div>
                      </>
                    )}
                  </Space>
                </div>
              ) : null
          }))}
          onChange={selectContract}
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
              <Descriptions.Item label="Entry ref">
                {metadataEntry.entry_ref}
              </Descriptions.Item>
              <Descriptions.Item label="Path">
                {formatInspectionPath(metadataEntry.inspection_path)}
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
              <Descriptions.Item label="Entry ref">
                {revealedEntry.metadata.entry_ref}
              </Descriptions.Item>
              <Descriptions.Item label="Value state">
                {revealedEntry.value_state}
              </Descriptions.Item>
              <Descriptions.Item label="Reveal mode">
                {revealedEntry.reveal_mode}
              </Descriptions.Item>
              <Descriptions.Item label="Size">
                {formatBytes(revealedEntry.metadata.value_size_bytes)}
              </Descriptions.Item>
            </Descriptions>
            {revealedEntry.value_state === 'available' ? (
              <JsonPreviewBlock
                title="Memory value"
                value={revealedEntry.value}
                collapsible={false}
                height="360px"
                copySuccessMessage="已复制内存 JSON"
              />
            ) : revealedEntry.value_preview ? (
              <Space direction="vertical" size={8}>
                <Alert
                  type="info"
                  showIcon
                  message="preview"
                  description={`${formatBytes(
                    revealedEntry.preview_size_bytes
                  )} of ${formatBytes(revealedEntry.full_value_size_bytes)}`}
                />
                <JsonPreviewBlock
                  title="Memory value preview"
                  value={revealedEntry.value_preview}
                  rawText={revealedEntry.value_preview}
                  collapsible={false}
                  height="320px"
                  copySuccessMessage="已复制内存预览 JSON"
                />
              </Space>
            ) : (
              <Alert
                type="warning"
                showIcon
                message="value_too_large"
                description={`${formatBytes(
                  revealedEntry.full_value_size_bytes
                )} exceeds full reveal limit.`}
              />
            )}
            {canReveal && revealedEntry.value_state === 'preview' ? (
              <Button
                icon={<EyeOutlined />}
                loading={revealMutation.isPending}
                onClick={() =>
                  revealMutation.mutate({
                    entry: revealedEntry.metadata,
                    revealMode: 'full'
                  })
                }
              >
                Full reveal
              </Button>
            ) : null}
          </Space>
        ) : null}
      </Drawer>
    </Space>
  );
}
