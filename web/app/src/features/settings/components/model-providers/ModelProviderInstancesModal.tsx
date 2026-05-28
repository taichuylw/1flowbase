import {
  Alert,
  Empty,
  Modal,
  Space,
  Switch,
  Tag,
  Typography
} from 'antd';

import type {
  SettingsModelProviderCatalogEntry,
  SettingsModelProviderInstance,
  SettingsModelProviderMainInstance,
  SettingsModelProviderOptions
} from '../../api/model-providers';
import { ModelProviderTagList } from './ModelProviderTagList';
import { ModelProviderInstancesTable } from './ModelProviderInstancesTable';
import { i18nText } from '../../../../shared/i18n/text';

type ModelGroup =
  SettingsModelProviderOptions['providers'][number]['model_groups'][number];

export function ModelProviderInstancesModal({
  open,
  catalogEntry,
  providerDisplayName,
  mainInstance,
  modelGroups,
  instances,
  updatingMainInstance,
  updatingInstanceId,
  refreshingCandidates,
  refreshing,
  deleting,
  canManage,
  versionSwitchNotice,
  onClose,
  onEdit,
  onRefreshCandidates,
  onRefreshModels,
  onDelete,
  onToggleAutoIncludeNewInstances,
  onToggleIncludedInMain
}: {
  open: boolean;
  catalogEntry: SettingsModelProviderCatalogEntry | null;
  providerDisplayName: string | null;
  mainInstance: SettingsModelProviderMainInstance | null;
  modelGroups: ModelGroup[];
  instances: SettingsModelProviderInstance[];
  updatingMainInstance: boolean;
  updatingInstanceId?: string | null;
  refreshingCandidates: boolean;
  refreshing: boolean;
  deleting: boolean;
  canManage: boolean;
  versionSwitchNotice: {
    targetVersion: string | null;
    migratedInstanceCount: number | null;
  } | null;
  onClose: () => void;
  onEdit: (instance: SettingsModelProviderInstance) => void;
  onRefreshCandidates: (instance: SettingsModelProviderInstance) => void;
  onRefreshModels: (instance: SettingsModelProviderInstance) => void;
  onDelete: (instance: SettingsModelProviderInstance) => void;
  onToggleAutoIncludeNewInstances: (checked: boolean) => void;
  onToggleIncludedInMain: (
    instance: SettingsModelProviderInstance,
    checked: boolean
  ) => void;
}) {
  const includedCount = instances.filter(
    (instance) => instance.included_in_main
  ).length;
  const aggregatedModelCount = modelGroups.reduce(
    (total, group) => total + group.models.length,
    0
  );
  const displayName = catalogEntry?.display_name ?? providerDisplayName;
  const title = displayName ? i18nText("settings", "auto.k_c13a981c98", { value1: displayName }) : i18nText("settings", "auto.k_9743810bab");

  return (
    <Modal
      open={open}
      width={960}
      title={title}
      aria-label={title}
      onCancel={onClose}
      footer={null}
      destroyOnHidden
    >
      <div className="model-provider-panel__instances-modal">
        {versionSwitchNotice ? (
          <Alert
            type="warning"
            showIcon
            message={i18nText("settings", "auto.k_29dbe8fcae")}
            description={
              versionSwitchNotice.targetVersion
                ? i18nText("settings", "auto.k_e66794c6e4", { value1: versionSwitchNotice.targetVersion, value2: versionSwitchNotice.migratedInstanceCount ?? 0 })
                : undefined
            }
          />
        ) : null}

        <section className="model-provider-panel__main-instance-card">
          <div className="model-provider-panel__main-instance-head">
            <div className="model-provider-panel__main-instance-title-row">
              <Typography.Text strong>{i18nText("settings", "auto.k_4be73cdcc7")}</Typography.Text>
              <div className="model-provider-panel__main-instance-summary">
                <Tag bordered={false} color="blue">
                  {i18nText("settings", "auto.k_b4dbe5471c")}</Tag>
                <Typography.Text type="secondary">
                  {i18nText("settings", "auto.k_c161648890")}{includedCount}
                </Typography.Text>
                <Typography.Text type="secondary">
                  {i18nText("settings", "auto.k_7ac64a2b44")}{aggregatedModelCount}
                </Typography.Text>
              </div>
            </div>
            <Space
              direction="horizontal"
              size={8}
              className="model-provider-panel__main-instance-toggle"
            >
              <Typography.Text type="secondary">
                {i18nText("settings", "auto.k_ce4d3a041d")}</Typography.Text>
              <Switch
                aria-label={i18nText("settings", "auto.k_ce4d3a041d")}
                checked={mainInstance?.auto_include_new_instances ?? false}
                disabled={!canManage || updatingMainInstance}
                onChange={onToggleAutoIncludeNewInstances}
              />
            </Space>
          </div>

          {modelGroups.length === 0 ? (
            <Empty
              image={Empty.PRESENTED_IMAGE_SIMPLE}
              description={i18nText("settings", "auto.k_5d440ac4d7")}
            />
          ) : (
            <div className="model-provider-panel__main-instance-groups">
              {modelGroups.map((group) => (
                <section
                  key={group.source_instance_id}
                  className="model-provider-panel__main-instance-group"
                >
                  <Typography.Text strong>
                    {group.source_instance_display_name}
                  </Typography.Text>
                  <ModelProviderTagList
                    modelIds={group.models.map((model) => model.model_id)}
                    emptyText={i18nText("settings", "auto.k_b14db0a516")}
                  />
                </section>
              ))}
            </div>
          )}
        </section>

        <ModelProviderInstancesTable
          instances={instances}
          canManage={canManage}
          loading={false}
          updatingInstanceId={updatingInstanceId}
          onToggleIncludedInMain={onToggleIncludedInMain}
          onEdit={onEdit}
          onRefreshCandidates={(instance) => {
            if (!refreshingCandidates) {
              onRefreshCandidates(instance);
            }
          }}
          onRefreshModels={(instance) => {
            if (!refreshing) {
              onRefreshModels(instance);
            }
          }}
          onDelete={(instance) => {
            if (!deleting) {
              onDelete(instance);
            }
          }}
        />
      </div>
    </Modal>
  );
}
