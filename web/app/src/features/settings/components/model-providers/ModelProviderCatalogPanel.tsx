import { Button, Empty, Select, Space, Table, Tag, Typography } from 'antd';

import { ScrollableSurface } from '../../../../shared/ui/scrollable-surface/ScrollableSurface';
import type { SettingsPluginFamilyEntry } from '../../api/plugins';
import type { SettingsModelProviderCatalogEntry } from '../../api/model-providers';
import { formatPluginAvailabilityStatus } from './plugin-installation-status';
import { ModelProviderOverviewSummary } from '../../pages/settings-page/model-providers/ModelProviderOverviewSummary';


function getCatalogDescription(
  family: SettingsPluginFamilyEntry,
  currentCatalogEntry: SettingsModelProviderCatalogEntry | null | undefined
) {
  return (
    family.description?.trim() ||
    currentCatalogEntry?.description_key?.trim() ||
    '未提供说明'
  );
}

function compareVersions(left: string, right: string) {
  return right.localeCompare(left, undefined, {
    numeric: true,
    sensitivity: 'base'
  });
}


export function ModelProviderCatalogPanel({
  overviewRows,
  entries,
  currentCatalogEntries,
  loading,
  canManage,
  deletingProviderCode,
  switchingProviderCode,
  upgradingProviderCode,
  onCreate,
  onViewInstances,
  onUpgradeLatest,
  onSwitchVersion,
  onDelete
}: {
  overviewRows: { key: string; label: string; value: string }[];
  entries: SettingsPluginFamilyEntry[];
  currentCatalogEntries: Record<
    string,
    SettingsModelProviderCatalogEntry | null
  >;
  loading?: boolean;
  canManage: boolean;
  deletingProviderCode?: string | null;
  switchingProviderCode?: string | null;
  upgradingProviderCode?: string | null;
  onCreate: (entry: SettingsPluginFamilyEntry) => void;
  onViewInstances: (entry: SettingsPluginFamilyEntry) => void;
  onUpgradeLatest: (entry: SettingsPluginFamilyEntry) => void;
  onSwitchVersion: (
    entry: SettingsPluginFamilyEntry,
    installationId: string
  ) => void;
  onDelete: (entry: SettingsPluginFamilyEntry) => void;
}) {
  return (
    <ScrollableSurface className="model-provider-panel__catalog">
      <div className="model-provider-panel__section-head">
        <ModelProviderOverviewSummary rows={overviewRows} />
      </div>

      <Table<SettingsPluginFamilyEntry>
        className="model-provider-panel__catalog-table"
        rowKey="provider_code"
        size="small"
        loading={loading}
        pagination={false}
        dataSource={entries}
        scroll={{ x: 780 }}
        locale={{
          emptyText: (
            <Empty
              image={Empty.PRESENTED_IMAGE_SIMPLE}
              description={loading ? '正在加载供应商目录...' : '暂无可用供应商'}
            />
          )
        }}
        columns={[
          ...(canManage
            ? [
              {
                title: '操作',
                key: 'actions',
                width: 120,
                render: (_: unknown, entry: SettingsPluginFamilyEntry) => (
                  <Space
                    size={4}
                    className="model-provider-panel__catalog-actions"
                  >
                    <Button
                      type="link"
                      onClick={() => onViewInstances(entry)}
                    >
                      配置
                    </Button>
                    <Button type="link" onClick={() => onCreate(entry)}>
                      添加
                    </Button>
                    <Button
                      danger
                      type="link"
                      loading={deletingProviderCode === entry.provider_code}
                      onClick={() => onDelete(entry)}
                    >
                      删除
                    </Button>
                  </Space>
                )
              }
            ]
            : []),
          {
            title: '名称',
            key: 'provider',
            width: 180,
            render: (_, entry) => (
              <div className="model-provider-panel__catalog-name">
                <Typography.Text strong>{entry.display_name}</Typography.Text>
              </div>
            )
          },
          {
            title: '状态',
            key: 'status',
            width: 130,
            render: (_, entry) => {
              const currentCatalogEntry =
                currentCatalogEntries[entry.provider_code];
              const status = formatPluginAvailabilityStatus(
                currentCatalogEntry?.availability_status ?? 'disabled'
              );

              return (
                <Space
                  wrap
                  size={[6, 6]}
                  className="model-provider-panel__catalog-status"
                >
                  <Tag color={status.color}>{status.label}</Tag>
                  <Tag>{entry.model_discovery_mode}</Tag>
                  {entry.has_update ? <Tag color="gold">有可用更新</Tag> : null}
                </Space>
              );
            }
          },
          {
            title: '版本',
            key: 'version',
            width: 120,
            render: (_, entry) => {
              const versionOptions = [...entry.installed_versions]
                .sort((left, right) =>
                  compareVersions(left.plugin_version, right.plugin_version)
                )
                .map((version) => ({
                  value: version.installation_id,
                  label: version.plugin_version
                }));

              return (
                <div className="model-provider-panel__catalog-version">
                  {canManage ? (
                    <Space size={8} wrap className="model-provider-panel__version-inline">
                      <Select
                        size="small"
                        value={entry.current_installation_id}
                        className="model-provider-panel__version-select"
                        classNames={{
                          popup: {
                            root: 'model-provider-panel__version-dropdown'
                          }
                        }}
                        aria-label={`切换 ${entry.display_name} 版本`}
                        loading={switchingProviderCode === entry.provider_code}
                        options={versionOptions}
                        onChange={(installationId) => {
                          if (installationId === entry.current_installation_id) {
                            return;
                          }

                          onSwitchVersion(entry, installationId);
                        }}
                      />
                      {entry.has_update ? (
                        <Button
                          size="small"
                          type="default"
                          loading={upgradingProviderCode === entry.provider_code}
                          onClick={() => onUpgradeLatest(entry)}
                        >
                          更新
                        </Button>
                      ) : null}
                    </Space>
                  ) : (
                    <Typography.Text strong>{entry.current_version}</Typography.Text>
                  )}
                </div>
              );
            }
          },
          {
            title: '说明',
            key: 'summary',
            width: 200,
            render: (_, entry) => {
              const currentCatalogEntry =
                currentCatalogEntries[entry.provider_code];
              const description = getCatalogDescription(
                entry,
                currentCatalogEntry
              );

              return (
                <div className="model-provider-panel__catalog-description">
                  <Typography.Paragraph
                    className="model-provider-panel__catalog-description-text"
                    ellipsis={{ rows: 2, tooltip: description }}
                  >
                    {description}
                  </Typography.Paragraph>
                </div>
              );
            }
          }
        ]}
      />
    </ScrollableSurface>
  );
}
