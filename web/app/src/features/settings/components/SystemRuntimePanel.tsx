import { useQuery } from '@tanstack/react-query';
import {
  Alert,
  Table,
  Tag,
  Typography,
  Space,
  Spin,
  Empty,
  Flex
} from 'antd';
import type { ColumnsType } from 'antd/es/table';
import {
  CheckCircleFilled,
  CloseCircleFilled,
  InfoCircleOutlined,
  EnvironmentOutlined,
  GlobalOutlined,
  CloudServerOutlined,
  ClusterOutlined,
  ExclamationCircleOutlined
} from '@ant-design/icons';

import {
  fetchSettingsSystemRuntimeProfile,
  settingsSystemRuntimeQueryKey
} from '../api/system-runtime';
import type { SettingsSystemRuntimeProfile } from '../api/system-runtime';
import { SettingsSectionSurface } from './SettingsSectionSurface';
import { i18nText } from '../../../shared/i18n/text';

/* ── helpers ────────────────────────────────────── */

function getRelationshipLabel(relationship: string) {
  switch (relationship) {
    case 'same_host':
      return {
        color: '#00ab73' as const,
        label: i18nText("settings", "auto.k_bf1af0fc7c"),
        icon: CloudServerOutlined
      };
    case 'split_host':
      return {
        color: '#1677ff' as const,
        label: i18nText("settings", "auto.k_5e25829373"),
        icon: ClusterOutlined
      };
    case 'runner_unreachable':
      return {
        color: '#ff4d4f' as const,
        label: i18nText("settings", "auto.k_5daaf917ae"),
        icon: ExclamationCircleOutlined
      };
    default:
      return {
        color: '#86909c' as const,
        label: relationship,
        icon: InfoCircleOutlined
      };
  }
}

function getReachabilityMeta(reachable: boolean) {
  return reachable
    ? { color: '#00ab73' as const, label: i18nText("settings", "auto.k_5942497005"), icon: CheckCircleFilled }
    : { color: '#ff4d4f' as const, label: i18nText("settings", "auto.k_358801edaa"), icon: CloseCircleFilled };
}

function formatMemory(value: number) {
  return `${value.toFixed(1)} GB`;
}

/* ── data shapes for the host table ──────────────── */

interface HostTableRow {
  key: string;
  fingerprint: string;
  platform: string;
  cpu: string;
  memoryTotal: string;
  memoryAvail: string;
  memoryUsage: number; // 0-1 ratio
  services: string[];
}

function buildHostRows(profile: SettingsSystemRuntimeProfile): HostTableRow[] {
  return profile.hosts.map((h) => ({
    key: h.host_fingerprint,
    fingerprint: h.host_fingerprint,
    platform: `${h.platform.os}/${h.platform.arch}${h.platform.libc ? `/${h.platform.libc}` : ''}`,
    cpu: i18nText("settings", "auto.k_2b1862a39d", { value1: h.cpu.logical_count }),
    memoryTotal: formatMemory(h.memory.total_gb),
    memoryAvail: formatMemory(h.memory.available_gb),
    memoryUsage:
      h.memory.total_gb > 0 ? 1 - h.memory.available_gb / h.memory.total_gb : 0,
    services: h.services
  }));
}

/* ── columns ────────────────────────────────────── */

const hostColumns: ColumnsType<HostTableRow> = [
  {
    title: i18nText("settings", "auto.k_3852a0ca84"),
    dataIndex: 'fingerprint',
    key: 'fingerprint',
    width: 140,
    render: (v: string) => (
      <Typography.Text code copyable style={{ fontSize: 12 }}>
        {v.slice(0, 12)}…
      </Typography.Text>
    )
  },
  {
    title: i18nText("settings", "auto.k_e4b9d69486"),
    dataIndex: 'platform',
    key: 'platform',
    width: 180
  },
  {
    title: 'CPU',
    dataIndex: 'cpu',
    key: 'cpu',
    width: 80
  },
  {
    title: i18nText("settings", "auto.k_d014ab7703"),
    key: 'memory',
    width: 200,
    render: (_: unknown, record: HostTableRow) => (
      <Space size={12}>
        <Flex vertical gap={2} style={{ minWidth: 80 }}>
          <Typography.Text type="secondary" style={{ fontSize: 12 }}>
            {i18nText("settings", "auto.k_3af1ac5b4e")}{record.memoryTotal}
          </Typography.Text>
          <Typography.Text style={{ fontSize: 12 }}>
            {i18nText("settings", "auto.k_e91365cf9e")}{record.memoryAvail}
          </Typography.Text>
        </Flex>
        <div
          style={{
            width: 60,
            height: 4,
            background: '#f0f0f0',
            borderRadius: 2,
            overflow: 'hidden'
          }}
        >
          <div
            style={{
              width: `${Math.round(record.memoryUsage * 100)}%`,
              height: '100%',
              background:
                record.memoryUsage > 0.85
                  ? '#ff4d4f'
                  : record.memoryUsage > 0.65
                    ? '#faad14'
                    : '#00ab73',
              borderRadius: 2,
              transition: 'width 0.3s'
            }}
          />
        </div>
      </Space>
    )
  },
  {
    title: i18nText("settings", "auto.k_015b410625"),
    key: 'services',
    width: 180,
    render: (_: unknown, record: HostTableRow) => (
      <Space size={4} wrap>
        {record.services.map((s) => (
          <Tag key={s} color="default" style={{ fontSize: 11 }}>
            {s}
          </Tag>
        ))}
        {record.services.length === 0 && (
          <Typography.Text type="secondary" style={{ fontSize: 12 }}>
            —
          </Typography.Text>
        )}
      </Space>
    )
  }
];

/* ── component ──────────────────────────────────── */

export function SystemRuntimePanel() {
  const runtimeQuery = useQuery({
    queryKey: settingsSystemRuntimeQueryKey,
    queryFn: fetchSettingsSystemRuntimeProfile
  });

  const profile = runtimeQuery.data;
  const relationshipMeta = profile
    ? getRelationshipLabel(profile.topology.relationship)
    : null;
  /* ── loading ── */
  if (runtimeQuery.isLoading) {
    return (
      <SettingsSectionSurface title={i18nText("settings", "auto.k_5027fd1718")} hideHeader heightMode="fill">
        <Flex justify="center" style={{ padding: '64px 0' }}>
          <Spin />
        </Flex>
      </SettingsSectionSurface>
    );
  }

  /* ── error ── */
  if (runtimeQuery.isError) {
    return (
      <SettingsSectionSurface title={i18nText("settings", "auto.k_5027fd1718")} hideHeader heightMode="fill">
        <Alert
          type="error"
          showIcon
          message={i18nText("settings", "auto.k_3d8ece8d38")}
          description={
            runtimeQuery.error instanceof Error
              ? runtimeQuery.error.message
              : i18nText("settings", "auto.k_662437e7e6")
          }
        />
      </SettingsSectionSurface>
    );
  }

  /* ── no data ── */
  if (!profile) {
    return (
      <SettingsSectionSurface title={i18nText("settings", "auto.k_5027fd1718")} hideHeader heightMode="fill">
        <Empty description={i18nText("settings", "auto.k_10b9da25f8")} />
      </SettingsSectionSurface>
    );
  }

  /* ── services ── */
  const servicesToRender = [
    {
      key: 'api_server',
      label: 'API Server',
      data: profile.services.api_server
    },
    {
      key: 'plugin_runner',
      label: 'Plugin Runner',
      data: profile.services.plugin_runner
    }
  ];

  const hostRows = buildHostRows(profile);

  /* ── render ── */
  return (
    <SettingsSectionSurface title={i18nText("settings", "auto.k_5027fd1718")} hideHeader heightMode="fill">
      {/* ════════════════════════════════════════════════
         部署概览
         ════════════════════════════════════════════════ */}
      <div style={{ marginBottom: 32 }}>
        <Flex align="center" gap={8} style={{ marginBottom: 14 }}>
          <InfoCircleOutlined style={{ color: '#00ab73', fontSize: 15 }} />
          <Typography.Text strong style={{ fontSize: 14 }}>
            {i18nText("settings", "auto.k_7579673d06")}</Typography.Text>
        </Flex>

        <Flex
          wrap="wrap"
          style={{
            background: '#fafafa',
            borderRadius: 8,
            border: '1px solid #f0f0f0',
            padding: '20px 24px'
          }}
        >
          {/* 部署关系 */}
          <Flex
            align="flex-start"
            gap={10}
            style={{
              minWidth: 160,
              paddingRight: 32,
              borderRight: '1px solid #f0f0f0'
            }}
          >
            <EnvironmentOutlined
              style={{ color: '#86909c', fontSize: 14, marginTop: 2 }}
            />
            <div>
              <Typography.Text
                type="secondary"
                style={{ fontSize: 12, display: 'block', marginBottom: 4 }}
              >
                {i18nText("settings", "auto.k_cf93420ca4")}</Typography.Text>
              {relationshipMeta ? (
                <Space size={6}>
                  <relationshipMeta.icon
                    style={{ color: relationshipMeta.color, fontSize: 13 }}
                  />
                  <Typography.Text
                    style={{ fontSize: 13, color: relationshipMeta.color }}
                  >
                    {relationshipMeta.label}
                  </Typography.Text>
                </Space>
              ) : (
                <Typography.Text>—</Typography.Text>
              )}
            </div>
          </Flex>

          {/* 当前语言 */}
          <Flex
            align="flex-start"
            gap={10}
            style={{
              minWidth: 140,
              padding: '0 32px',
              borderRight: '1px solid #f0f0f0'
            }}
          >
            <GlobalOutlined
              style={{ color: '#86909c', fontSize: 14, marginTop: 2 }}
            />
            <div>
              <Typography.Text
                type="secondary"
                style={{ fontSize: 12, display: 'block', marginBottom: 4 }}
              >
                {i18nText("settings", "auto.k_b13af74fb0")}</Typography.Text>
              <Typography.Text style={{ fontSize: 13 }}>
                {profile.locale_meta.resolved_locale}
              </Typography.Text>
            </div>
          </Flex>

          {/* 回退语言 */}
          <Flex
            align="flex-start"
            gap={10}
            style={{
              minWidth: 120,
              padding: '0 32px',
              borderRight: '1px solid #f0f0f0'
            }}
          >
            <GlobalOutlined
              style={{ color: '#86909c', fontSize: 14, marginTop: 2 }}
            />
            <div>
              <Typography.Text
                type="secondary"
                style={{ fontSize: 12, display: 'block', marginBottom: 4 }}
              >
                {i18nText("settings", "auto.k_dd904b14d0")}</Typography.Text>
              <Typography.Text style={{ fontSize: 13 }}>
                {profile.locale_meta.fallback_locale}
              </Typography.Text>
            </div>
          </Flex>

          {/* 支持语言 */}
          <Flex
            align="flex-start"
            gap={10}
            style={{ minWidth: 180, paddingLeft: 32 }}
          >
            <GlobalOutlined
              style={{ color: '#86909c', fontSize: 14, marginTop: 2 }}
            />
            <div>
              <Typography.Text
                type="secondary"
                style={{ fontSize: 12, display: 'block', marginBottom: 4 }}
              >
                {i18nText("settings", "auto.k_83b604e91d")}</Typography.Text>
              <Typography.Text style={{ fontSize: 13 }}>
                {profile.locale_meta.supported_locales.join(', ')}
              </Typography.Text>
            </div>
          </Flex>
        </Flex>
      </div>

      {/* ════════════════════════════════════════════════
         服务状态
         ════════════════════════════════════════════════ */}
      <div style={{ marginBottom: 32 }}>
        <Flex align="center" gap={8} style={{ marginBottom: 14 }}>
          <CloudServerOutlined style={{ color: '#00ab73', fontSize: 15 }} />
          <Typography.Text strong style={{ fontSize: 14 }}>
            {i18nText("settings", "auto.k_d42da9e2ac")}</Typography.Text>
        </Flex>

        <Flex gap={16} wrap="wrap">
          {servicesToRender.map((svc) => {
            const reachMeta = getReachabilityMeta(svc.data.reachable);
            const leftBorder = svc.data.reachable ? '#00ab73' : '#ff4d4f';
            return (
              <div
                key={svc.key}
                style={{
                  flex: '1 1 300px',
                  border: '1px solid #f0f0f0',
                  borderRadius: 8,
                  padding: '18px 20px',
                  borderLeft: `3px solid ${leftBorder}`,
                  background: '#fff'
                }}
              >
                <Flex
                  align="center"
                  justify="space-between"
                  style={{ marginBottom: 12 }}
                >
                  <Typography.Text strong style={{ fontSize: 14 }}>
                    {svc.label}
                  </Typography.Text>
                  <Space size={6}>
                    <reachMeta.icon
                      style={{ color: reachMeta.color, fontSize: 13 }}
                    />
                    <Typography.Text
                      style={{ color: reachMeta.color, fontSize: 12 }}
                    >
                      {reachMeta.label}
                    </Typography.Text>
                  </Space>
                </Flex>

                <Flex gap={24} wrap="wrap">
                  <div>
                    <Typography.Text
                      type="secondary"
                      style={{
                        fontSize: 11,
                        display: 'block',
                        marginBottom: 2
                      }}
                    >
                      {i18nText("settings", "auto.k_989d1affa0")}</Typography.Text>
                    <Typography.Text style={{ fontSize: 13 }}>
                      {svc.data.version ?? '—'}
                    </Typography.Text>
                  </div>
                  <div>
                    <Typography.Text
                      type="secondary"
                      style={{
                        fontSize: 11,
                        display: 'block',
                        marginBottom: 2
                      }}
                    >
                      {i18nText("settings", "auto.k_62e951a692")}</Typography.Text>
                    <Typography.Text style={{ fontSize: 13 }}>
                      {svc.data.status ?? '—'}
                    </Typography.Text>
                  </div>
                  <div>
                    <Typography.Text
                      type="secondary"
                      style={{
                        fontSize: 11,
                        display: 'block',
                        marginBottom: 2
                      }}
                    >
                      {i18nText("settings", "auto.k_1346ed6e4f")}</Typography.Text>
                    <Typography.Text code style={{ fontSize: 12 }}>
                      {svc.data.host_fingerprint?.slice(0, 16) ?? i18nText("settings", "auto.k_d9c32a4c3d")}
                    </Typography.Text>
                  </div>
                </Flex>
              </div>
            );
          })}
        </Flex>
      </div>

      {/* ════════════════════════════════════════════════
         宿主机
         ════════════════════════════════════════════════ */}
      <div>
        <Flex align="center" gap={8} style={{ marginBottom: 14 }}>
          <ClusterOutlined style={{ color: '#00ab73', fontSize: 15 }} />
          <Typography.Text strong style={{ fontSize: 14 }}>
            {i18nText("settings", "auto.k_5051a125e2")}</Typography.Text>
          <Tag style={{ marginLeft: 4, fontSize: 11, lineHeight: '20px' }}>
            {hostRows.length}
          </Tag>
        </Flex>

        {hostRows.length > 0 ? (
          <Table<HostTableRow>
            columns={hostColumns}
            dataSource={hostRows}
            pagination={false}
            size="small"
            bordered
            style={{ fontSize: 13 }}
          />
        ) : (
          <Empty description={i18nText("settings", "auto.k_3ff996f432")} />
        )}
      </div>
    </SettingsSectionSurface>
  );
}
