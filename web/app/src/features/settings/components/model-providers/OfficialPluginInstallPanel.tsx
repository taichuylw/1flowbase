import { useMemo, useState } from 'react';

import { QuestionCircleOutlined } from '@ant-design/icons';
import { Button, Empty, Modal, Select, Tag, Tooltip, Typography } from 'antd';

import { ScrollableSurface } from '../../../../shared/ui/scrollable-surface/ScrollableSurface';
import type {
  SettingsOfficialPluginCatalogEntry,
  SettingsPluginFamilyEntry
} from '../../api/plugins';
import { i18nText } from '../../../../shared/i18n/text';

type InstallState = 'idle' | 'installing' | 'success' | 'failed';

function getInstallButtonLabel(
  entry: SettingsOfficialPluginCatalogEntry,
  installState: InstallState,
  activePluginId: string | null
) {
  if (activePluginId === entry.plugin_id && installState === 'installing') {
    return i18nText("settings", "auto.k_411642f1d0");
  }

  if (
    entry.install_status === 'assigned' ||
    (activePluginId === entry.plugin_id && installState === 'success')
  ) {
    return i18nText("settings", "auto.k_f0d2fb13dd");
  }

  if (activePluginId === entry.plugin_id && installState === 'failed') {
    return i18nText("settings", "auto.k_87c0fd2d68");
  }

  return i18nText("settings", "auto.k_662ccf68d0");
}

function compareOfficialVersion(left: string, right: string) {
  return left.localeCompare(right, undefined, {
    numeric: true,
    sensitivity: 'base'
  });
}

function pickPreferredOfficialEntry(
  current: SettingsOfficialPluginCatalogEntry,
  candidate: SettingsOfficialPluginCatalogEntry,
  family: SettingsPluginFamilyEntry | undefined
) {
  if (family?.latest_version) {
    const currentMatchesFamilyLatest =
      current.latest_version === family.latest_version;
    const candidateMatchesFamilyLatest =
      candidate.latest_version === family.latest_version;

    if (currentMatchesFamilyLatest !== candidateMatchesFamilyLatest) {
      return candidateMatchesFamilyLatest ? candidate : current;
    }
  }

  const versionComparison = compareOfficialVersion(
    candidate.latest_version,
    current.latest_version
  );
  if (versionComparison !== 0) {
    return versionComparison > 0 ? candidate : current;
  }

  const statusScore = {
    assigned: 2,
    installed: 1,
    not_installed: 0
  } as const;
  const currentStatusScore = statusScore[current.install_status];
  const candidateStatusScore = statusScore[candidate.install_status];

  if (currentStatusScore !== candidateStatusScore) {
    return candidateStatusScore > currentStatusScore ? candidate : current;
  }

  return compareOfficialVersion(candidate.plugin_id, current.plugin_id) > 0
    ? candidate
    : current;
}

const OFFICIAL_PLUGIN_RELEASES_URL =
  'https://github.com/taichuy/1flowbase-official-plugins/releases';
const DEFAULT_PROVIDER_ICON_SRC = '/icon.svg';

function getOfficialPluginIconSrc(entry: SettingsOfficialPluginCatalogEntry) {
  return entry.icon?.trim() || DEFAULT_PROVIDER_ICON_SRC;
}

function getTagColor(tag: string) {
  switch (tag) {
    case 'latest':
      return 'gold';
    case 'active':
      return 'green';
    case 'installed':
      return 'blue';
    case 'installing':
      return 'processing';
    case 'failed':
      return 'red';
    case 'hybrid':
      return 'purple';
    case 'dynamic':
      return 'cyan';
    case 'static':
      return 'default';
    default:
      return 'default';
  }
}

function renderTagLabel(tag: string) {
  if (tag === 'latest') {
    return (
      <span className="model-provider-panel__tag-label">
        latest
        <Tooltip title={i18nText("settings", "auto.k_9b281e3e0a")}>
          <QuestionCircleOutlined className="model-provider-panel__tag-help" />
        </Tooltip>
      </span>
    );
  }

  if (tag === 'hybrid' || tag === 'dynamic' || tag === 'static') {
    return (
      <span className="model-provider-panel__tag-label">
        {tag}
        <Tooltip
          title={
            tag === 'hybrid'
              ? i18nText("settings", "auto.k_b99bc4228a")
              : tag === 'dynamic'
                ? i18nText("settings", "auto.k_557571148f")
                : i18nText("settings", "auto.k_be8ea55915")
          }
        >
          <QuestionCircleOutlined className="model-provider-panel__tag-help" />
        </Tooltip>
      </span>
    );
  }

  return tag;
}

function getStatusTags(
  entry: SettingsOfficialPluginCatalogEntry,
  family: SettingsPluginFamilyEntry | undefined,
  installState: InstallState,
  activePluginId: string | null
) {
  const tags: string[] = [];

  if (family) {
    tags.push(family.current_version);
    tags.push(
      family.has_update && family.latest_version
        ? family.latest_version
        : 'latest'
    );
    tags.push(entry.model_discovery_mode);
    return tags;
  }

  tags.push(entry.latest_version);

  if (activePluginId === entry.plugin_id && installState === 'installing') {
    tags.push('installing');
  } else if (entry.install_status === 'assigned') {
    tags.push('active');
  } else if (entry.install_status === 'installed') {
    tags.push('installed');
  } else if (activePluginId === entry.plugin_id && installState === 'failed') {
    tags.push('failed');
  } else {
    tags.push('latest');
  }

  tags.push(entry.model_discovery_mode);
  return tags;
}

export function OfficialPluginInstallPanel({
  sourceMeta,
  entries,
  familiesByProviderCode,
  loading,
  canManage,
  activePluginId,
  installState,
  upgradingProviderCode,
  onInstall,
  onOpenUpload,
  onUpgradeLatest
}: {
  sourceMeta: {
    sourceKind: string;
    sourceLabel: string;
    registryUrl: string;
  } | null;
  entries: SettingsOfficialPluginCatalogEntry[];
  familiesByProviderCode: Record<string, SettingsPluginFamilyEntry | undefined>;
  loading?: boolean;
  canManage: boolean;
  activePluginId: string | null;
  installState: InstallState;
  upgradingProviderCode: string | null;
  onInstall: (entry: SettingsOfficialPluginCatalogEntry) => void;
  onOpenUpload: () => void;
  onUpgradeLatest: (entry: SettingsOfficialPluginCatalogEntry) => void;
}) {
  const [modal, contextHolder] = Modal.useModal();
  const [selectedPluginId, setSelectedPluginId] = useState<string | null>(null);
  const normalizedEntries = useMemo(() => {
    const grouped = new Map<string, SettingsOfficialPluginCatalogEntry>();

    for (const entry of entries) {
      const existing = grouped.get(entry.provider_code);
      if (!existing) {
        grouped.set(entry.provider_code, entry);
        continue;
      }

      grouped.set(
        entry.provider_code,
        pickPreferredOfficialEntry(
          existing,
          entry,
          familiesByProviderCode[entry.provider_code]
        )
      );
    }

    return Array.from(grouped.values());
  }, [entries, familiesByProviderCode]);
  const visibleEntries = useMemo(() => {
    if (!selectedPluginId) {
      return normalizedEntries;
    }

    return normalizedEntries.filter(
      (entry) => entry.plugin_id === selectedPluginId
    );
  }, [normalizedEntries, selectedPluginId]);
  const openExternal = (url: string) => {
    window.open(url, '_blank', 'noopener,noreferrer');
  };

  return (
    <ScrollableSurface className="model-provider-panel__official">
      {contextHolder}
      <div className="model-provider-panel__section-head">
        <div>
          <Typography.Title level={5}>{i18nText("settings", "auto.model_providers")}</Typography.Title>
          {sourceMeta ? (
            <Typography.Text type="secondary">
              {i18nText("settings", "auto.k_47a3d3a019")}{sourceMeta.sourceLabel}
              {i18nText("settings", "auto.k_4540c3f939")}</Typography.Text>
          ) : null}
          <div className="model-provider-panel__official-toolbar">
            {canManage ? (
              <Button onClick={onOpenUpload}>{i18nText("settings", "auto.k_31f407d6e8")}</Button>
            ) : null}
            {sourceMeta ? (
              <Button onClick={() => openExternal(sourceMeta.registryUrl)}>
                {i18nText("settings", "auto.k_c63f79e636")}</Button>
            ) : null}
            <Button onClick={() => openExternal(OFFICIAL_PLUGIN_RELEASES_URL)}>
              {i18nText("settings", "auto.k_9e0c8a3d10")}</Button>
          </div>
        </div>
      </div>
      <Select
        allowClear
        showSearch
        className="model-provider-panel__official-select"
        placeholder={i18nText("settings", "auto.k_9cdcc87be5")}
        optionFilterProp="label"
        value={selectedPluginId}
        onChange={(value) => setSelectedPluginId(value ?? null)}
        options={normalizedEntries.map((entry) => ({
          value: entry.plugin_id,
          label: `${entry.display_name} / ${entry.protocol}`
        }))}
      />

      {normalizedEntries.length === 0 ? (
        <div className="model-provider-panel__empty">
          <Empty
            image={Empty.PRESENTED_IMAGE_SIMPLE}
            description={
              loading ? i18nText("settings", "auto.k_6133694e11") : i18nText("settings", "auto.k_de2cfb189b")
            }
          />
        </div>
      ) : (
        <div className="model-provider-panel__official-grid">
          {visibleEntries.map((entry) => {
            const family = familiesByProviderCode[entry.provider_code];
            const installing =
              activePluginId === entry.plugin_id &&
              installState === 'installing';
            const installed =
              entry.install_status === 'assigned' ||
              (activePluginId === entry.plugin_id &&
                installState === 'success');
            const upgrading = upgradingProviderCode === entry.provider_code;
            const buttonLabel = family
              ? family.has_update
                ? upgrading
                  ? i18nText("settings", "auto.k_fdb8b1f420")
                  : i18nText("settings", "auto.k_eed748acf2")
                : i18nText("settings", "auto.k_db620f1761")
              : getInstallButtonLabel(entry, installState, activePluginId);
            const buttonDisabled = family ? !family.has_update : installed;

            return (
              <article
                key={entry.plugin_id}
                className="model-provider-panel__official-card"
              >
                <div className="model-provider-panel__catalog-item-main">
                  <div className="model-provider-panel__catalog-item-title-row">
                    <img
                      className="model-provider-panel__provider-icon"
                      src={getOfficialPluginIconSrc(entry)}
                      alt=""
                      aria-hidden="true"
                      loading="lazy"
                      onError={(event) => {
                        const image = event.currentTarget;
                        if (image.src.endsWith(DEFAULT_PROVIDER_ICON_SRC)) {
                          return;
                        }
                        image.src = DEFAULT_PROVIDER_ICON_SRC;
                      }}
                    />
                    <Typography.Title level={5}>
                      {entry.display_name}
                    </Typography.Title>
                  </div>
                  <div className="model-provider-panel__catalog-item-tag-row">
                    {getStatusTags(
                      entry,
                      family,
                      installState,
                      activePluginId
                    ).map((tag) => (
                      <Tag
                        key={`${entry.plugin_id}-${tag}`}
                        color={getTagColor(tag)}
                      >
                        {renderTagLabel(tag)}
                      </Tag>
                    ))}
                  </div>
                  {entry.description ? (
                    <Typography.Paragraph className="model-provider-panel__official-card-description">
                      {entry.description}
                    </Typography.Paragraph>
                  ) : null}
                </div>

                {canManage ? (
                  <div className="model-provider-panel__catalog-item-actions">
                    {entry.help_url ? (
                      <Button onClick={() => openExternal(entry.help_url!)}>
                        {i18nText("settings", "auto.k_1069127253")}</Button>
                    ) : null}
                    <Button
                      type={buttonDisabled ? 'default' : 'primary'}
                      loading={installing || upgrading}
                      disabled={buttonDisabled}
                      onClick={() => {
                        void modal.confirm({
                          title: family ? i18nText("settings", "auto.k_aa22ba464e") : i18nText("settings", "auto.k_76a38f0a4c"),
                          icon: null,
                          centered: true,
                          okText: buttonLabel,
                          cancelText: i18nText("settings", "auto.cancel"),
                          okButtonProps: {
                            loading: installing || upgrading,
                            disabled: buttonDisabled
                          },
                          content: (
                            <div className="model-provider-panel__install-confirm">
                              <div className="model-provider-panel__install-confirm-card">
                                <Typography.Title level={5}>
                                  {entry.display_name}
                                </Typography.Title>
                                <Typography.Paragraph type="secondary">
                                  {family
                                    ? i18nText("settings", "auto.k_75aedfd084", { value1: entry.display_name, value2: entry.latest_version })
                                    : i18nText("settings", "auto.k_8585bc2c84", { value1: entry.latest_version })}
                                </Typography.Paragraph>
                                <div className="model-provider-panel__catalog-item-meta">
                                  <span>{i18nText("settings", "auto.k_47f7f369a7")}{entry.protocol}</span>
                                  <span>
                                    {i18nText("settings", "auto.k_0d22afe6d2")}{entry.model_discovery_mode}
                                  </span>
                                </div>
                              </div>
                            </div>
                          ),
                          onOk: async () => {
                            if (family) {
                              onUpgradeLatest(entry);
                              return;
                            }

                            onInstall(entry);
                          }
                        });
                      }}
                    >
                      {buttonLabel}
                    </Button>
                  </div>
                ) : entry.help_url ? (
                  <div className="model-provider-panel__catalog-item-actions">
                    <Button onClick={() => openExternal(entry.help_url!)}>
                      {i18nText("settings", "auto.k_1069127253")}</Button>
                  </div>
                ) : null}
              </article>
            );
          })}
        </div>
      )}
    </ScrollableSurface>
  );
}
