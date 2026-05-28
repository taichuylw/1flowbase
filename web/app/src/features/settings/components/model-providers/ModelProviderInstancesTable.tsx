import { Descriptions, Empty, Switch, Tag, Typography } from 'antd';

import { CollapseShell } from '../../../../shared/ui/collapse-shell/CollapseShell';
import type { SettingsModelProviderInstance } from '../../api/model-providers';
import { ModelProviderTagList } from './ModelProviderTagList';
import { i18nText } from '../../../../shared/i18n/text';

function renderStatusTag(status: string) {
  switch (status) {
    case 'ready':
      return (
        <Tag
          className="model-provider-panel__instance-status-tag"
          color="green"
          bordered={false}
        >
          ready
        </Tag>
      );
    case 'invalid':
      return (
        <Tag
          className="model-provider-panel__instance-status-tag"
          color="red"
          bordered={false}
        >
          invalid
        </Tag>
      );
    case 'disabled':
      return (
        <Tag className="model-provider-panel__instance-status-tag" bordered={false}>
          disabled
        </Tag>
      );
    default:
      return (
        <Tag
          className="model-provider-panel__instance-status-tag"
          color="gold"
          bordered={false}
        >
          {status}
        </Tag>
      );
  }
}

function formatCatalogRefreshedAt(value: string | null) {
  if (!value) {
    return i18nText("settings", "auto.key_mlpmfjhanp");
  }

  const matched = value.match(
    /^(\d{4}-\d{2}-\d{2})[T\s](\d{2}:\d{2}:\d{2})/
  );

  if (!matched) {
    return value;
  }

  return `${matched[1]} ${matched[2]}`;
}

export function ModelProviderInstancesTable({
  instances,
  loading,
  canManage,
  updatingInstanceId,
  onToggleIncludedInMain,
  onEdit,
  onRefreshCandidates,
  onRefreshModels,
  onDelete
}: {
  instances: SettingsModelProviderInstance[];
  loading?: boolean;
  canManage: boolean;
  updatingInstanceId?: string | null;
  onToggleIncludedInMain: (
    instance: SettingsModelProviderInstance,
    checked: boolean
  ) => void;
  onEdit: (instance: SettingsModelProviderInstance) => void;
  onRefreshCandidates: (instance: SettingsModelProviderInstance) => void;
  onRefreshModels: (instance: SettingsModelProviderInstance) => void;
  onDelete: (instance: SettingsModelProviderInstance) => void;
}) {
  if (instances.length === 0) {
    return (
      <section className="model-provider-panel__instances">
        <Empty
          image={Empty.PRESENTED_IMAGE_SIMPLE}
          description={loading ? i18nText("settings", "auto.key_oddbbbgloj") : i18nText("settings", "auto.key_bdcbaafgpo")}
        />
      </section>
    );
  }

  return (
    <section className="model-provider-panel__instances">
      <CollapseShell
        accordion
        className="model-provider-panel__instances-collapse"
        defaultActiveKey={instances[0] ? [instances[0].id] : undefined}
        items={instances.map((instance) => ({
          key: instance.id,
          header: (
            <div className="model-provider-panel__instance-header">
              <div className="model-provider-panel__instance-header-main">
                <div className="model-provider-panel__instance-title-row">
                  <span className="model-provider-panel__instance-title">
                    {instance.display_name}
                  </span>
                  {renderStatusTag(instance.status)}
                </div>
              </div>

              <div className="model-provider-panel__instance-header-side">
                <div className="model-provider-panel__instance-inclusion-card">
                  <span className="model-provider-panel__instance-stat-label">
                    {i18nText("settings", "auto.key_edfhjamomm")}</span>
                  <div className="model-provider-panel__instance-inclusion-row">
                    <Switch
                      aria-label={i18nText("settings", "auto.key_cmfjbnbmlk", { value1: instance.display_name })}
                      checked={instance.included_in_main}
                      disabled={!canManage || updatingInstanceId === instance.id}
                      onClick={(_, event) => {
                        event?.stopPropagation();
                      }}
                      onChange={(checked) => {
                        onToggleIncludedInMain(instance, checked);
                      }}
                    />
                    <Typography.Text type="secondary">
                      {instance.included_in_main ? i18nText("settings", "auto.key_nkhccefnfa") : i18nText("settings", "auto.key_diajdnigel")}
                    </Typography.Text>
                  </div>
                </div>

                <div className="model-provider-panel__instance-stats">
                  <div className="model-provider-panel__instance-stat">
                    <span className="model-provider-panel__instance-stat-label">
                      {i18nText("settings", "auto.key_mdolggmmla")}</span>
                    <span className="model-provider-panel__instance-stat-value">
                      {instance.enabled_model_ids.length}
                    </span>
                  </div>
                  <div className="model-provider-panel__instance-stat">
                    <span className="model-provider-panel__instance-stat-label">
                      {i18nText("settings", "auto.key_bnbppfhgjf")}</span>
                    <span className="model-provider-panel__instance-stat-value">
                      {instance.model_count}
                    </span>
                  </div>
                </div>
              </div>
            </div>
          ),
          children: (
            <>
              <Descriptions
                className="model-provider-panel__instance-descriptions"
                size="small"
                column={1}
                items={[
                  {
                    key: 'base-url',
                    label: 'Base URL',
                    children: (
                      <Typography.Paragraph
                        className="model-provider-panel__instance-baseurl-value"
                        ellipsis={{ rows: 2, expandable: true, symbol: i18nText("settings", "auto.key_laoceiddph") }}
                        style={{ marginBottom: 0 }}
                      >
                        {String(instance.config_json.base_url ?? i18nText("settings", "auto.key_gdfjfojflh"))}
                      </Typography.Paragraph>
                    )
                  },
                  {
                    key: 'enabled-models',
                    label: i18nText("settings", "auto.key_mdolggmmla"),
                    children: (
                      <ModelProviderTagList
                        modelIds={instance.enabled_model_ids}
                        emptyText={i18nText("settings", "auto.key_ffkaelfimn")}
                      />
                    )
                  },
                  {
                    key: 'refreshed-at',
                    label: i18nText("settings", "auto.key_bgakfchkha"),
                    children: (
                      <Typography.Text type="secondary">
                        {formatCatalogRefreshedAt(instance.catalog_refreshed_at)}
                      </Typography.Text>
                    )
                  }
                ]}
              />

              {canManage ? (
                <div className="model-provider-panel__instance-actions">
                  <button
                    type="button"
                    className="model-provider-panel__instance-action-btn"
                    onClick={() => onEdit(instance)}
                    aria-label={i18nText("settings", "auto.key_bjebpdgllf", { value1: instance.display_name })}
                  >
                    {i18nText("settings", "auto.key_chjcjaikmg")}</button>
                  <button
                    type="button"
                    className="model-provider-panel__instance-action-btn"
                    onClick={() => onRefreshCandidates(instance)}
                    aria-label={i18nText("settings", "auto.key_ehmpfjadma", { value1: instance.display_name })}
                  >
                    {i18nText("settings", "auto.key_efiphoajfn")}</button>
                  <button
                    type="button"
                    className="model-provider-panel__instance-action-btn"
                    onClick={() => onRefreshModels(instance)}
                    aria-label={i18nText("settings", "auto.key_lojpmnlkkk", { value1: instance.display_name })}
                  >
                    {i18nText("settings", "auto.key_ijcpicmnbg")}</button>
                  <button
                    type="button"
                    className="model-provider-panel__instance-action-btn model-provider-panel__instance-action-btn--danger"
                    onClick={() => onDelete(instance)}
                    aria-label={i18nText("settings", "auto.key_nnfakolcpn", { value1: instance.display_name })}
                  >
                    {i18nText("settings", "auto.key_gohdkfppkf")}</button>
                </div>
              ) : null}
            </>
          )
        }))}
      />
    </section>
  );
}
