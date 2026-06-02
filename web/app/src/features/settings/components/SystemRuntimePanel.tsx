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
  ExclamationCircleOutlined,
  SyncOutlined
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
        label: i18nText("settings", "auto.deployment_same_machine"),
        icon: CloudServerOutlined
      };
    case 'split_host':
      return {
        color: '#1677ff' as const,
        label: i18nText("settings", "auto.extension_deployment"),
        icon: ClusterOutlined
      };
    case 'runner_unreachable':
      return {
        color: '#ff4d4f' as const,
        label: i18nText("settings", "auto.runner_is_unreachable"),
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
    ? { color: '#00ab73' as const, label: i18nText("settings", "auto.running"), icon: CheckCircleFilled }
    : { color: '#ff4d4f' as const, label: i18nText("settings", "auto.not_reachable"), icon: CloseCircleFilled };
}

function getWorkerStatusMeta(status: string) {
  switch (status) {
    case 'idle':
      return { color: '#00ab73' as const, label: i18nText("settings", "auto.worker_status_idle") };
    case 'polling':
      return { color: '#00ab73' as const, label: i18nText("settings", "auto.worker_status_polling") };
    case 'processing':
      return { color: '#1677ff' as const, label: i18nText("settings", "auto.worker_status_processing") };
    case 'error':
      return { color: '#ff4d4f' as const, label: i18nText("settings", "auto.worker_status_error") };
    case 'stopped':
      return { color: '#86909c' as const, label: i18nText("settings", "auto.worker_status_stopped") };
    case 'not_started':
      return { color: '#86909c' as const, label: i18nText("settings", "auto.worker_status_not_started") };
    default:
      return { color: '#faad14' as const, label: status };
  }
}

function getResumeQueueLabel(queueStatus: string) {
  switch (queueStatus) {
    case 'pending':
      return i18nText("settings", "auto.resume_queue_pending");
    case 'claimed':
      return i18nText("settings", "auto.resume_queue_claimed");
    case 'succeeded':
      return i18nText("settings", "auto.resume_queue_succeeded");
    case 'failed':
      return i18nText("settings", "auto.resume_queue_failed");
    case 'expired':
      return i18nText("settings", "auto.resume_queue_expired");
    default:
      return queueStatus;
  }
}

function formatMemory(value: number) {
  return `${value.toFixed(1)} GB`;
}

function formatOptionalTime(value: string | null | undefined) {
  return value ? new Date(value).toLocaleString() : '—';
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
    cpu: i18nText("settings", "auto.core", { value1: h.cpu.logical_count }),
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
    title: i18nText("settings", "auto.fingerprint"),
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
    title: i18nText("settings", "auto.platform"),
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
    title: i18nText("settings", "auto.memory"),
    key: 'memory',
    width: 200,
    render: (_: unknown, record: HostTableRow) => (
      <Space size={12}>
        <Flex vertical gap={2} style={{ minWidth: 80 }}>
          <Typography.Text type="secondary" style={{ fontSize: 12 }}>
            {i18nText("settings", "auto.total")}{record.memoryTotal}
          </Typography.Text>
          <Typography.Text style={{ fontSize: 12 }}>
            {i18nText("settings", "auto.available")}{record.memoryAvail}
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
    title: i18nText("settings", "auto.hosting_services"),
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
      <SettingsSectionSurface title={i18nText("settings", "auto.system_runtime")} hideHeader heightMode="fill">
        <Flex justify="center" style={{ padding: '64px 0' }}>
          <Spin />
        </Flex>
      </SettingsSectionSurface>
    );
  }

  /* ── error ── */
  if (runtimeQuery.isError) {
    return (
      <SettingsSectionSurface title={i18nText("settings", "auto.system_runtime")} hideHeader heightMode="fill">
        <Alert
          type="error"
          showIcon
          message={i18nText("settings", "auto.runtime_information_loading_failed")}
          description={
            runtimeQuery.error instanceof Error
              ? runtimeQuery.error.message
              : i18nText("settings", "auto.try_again_later")
          }
        />
      </SettingsSectionSurface>
    );
  }

  /* ── no data ── */
  if (!profile) {
    return (
      <SettingsSectionSurface title={i18nText("settings", "auto.system_runtime")} hideHeader heightMode="fill">
        <Empty description={i18nText("settings", "auto.runtime_data_yet")} />
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
  const nativeResumeWorker = profile.native_resume_worker;

  /* ── render ── */
  return (
    <SettingsSectionSurface title={i18nText("settings", "auto.system_runtime")} hideHeader heightMode="fill">
      {/* ════════════════════════════════════════════════
         部署概览
         ════════════════════════════════════════════════ */}
      <div style={{ marginBottom: 32 }}>
        <Flex align="center" gap={8} style={{ marginBottom: 14 }}>
          <InfoCircleOutlined style={{ color: '#00ab73', fontSize: 15 }} />
          <Typography.Text strong style={{ fontSize: 14 }}>
            {i18nText("settings", "auto.deployment_overview")}</Typography.Text>
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
                {i18nText("settings", "auto.deployment_relationship")}</Typography.Text>
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
                {i18nText("settings", "auto.current_language")}</Typography.Text>
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
                {i18nText("settings", "auto.fallback_language")}</Typography.Text>
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
                {i18nText("settings", "auto.supported_languages")}</Typography.Text>
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
            {i18nText("settings", "auto.service_status")}</Typography.Text>
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
                      {i18nText("settings", "auto.version")}</Typography.Text>
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
                      {i18nText("settings", "auto.status")}</Typography.Text>
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
                      {i18nText("settings", "auto.host_fingerprint")}</Typography.Text>
                    <Typography.Text code style={{ fontSize: 12 }}>
                      {svc.data.host_fingerprint?.slice(0, 16) ?? i18nText("settings", "auto.unknown")}
                    </Typography.Text>
                  </div>
                </Flex>
              </div>
            );
          })}
        </Flex>
      </div>

      {nativeResumeWorker ? (
        <div style={{ marginBottom: 32 }}>
          <Flex align="center" gap={8} style={{ marginBottom: 14 }}>
            <SyncOutlined style={{ color: '#1677ff', fontSize: 15 }} />
            <Typography.Text strong style={{ fontSize: 14 }}>
              {i18nText("settings", "auto.native_resume_worker")}</Typography.Text>
          </Flex>

          <Flex gap={16} wrap="wrap">
            <div
              style={{
                flex: '1 1 320px',
                border: '1px solid #f0f0f0',
                borderRadius: 8,
                padding: '18px 20px',
                background: '#fff'
              }}
            >
              <Flex
                align="center"
                justify="space-between"
                style={{ marginBottom: 12 }}
              >
                <Typography.Text strong style={{ fontSize: 14 }}>
                  {i18nText("settings", "auto.worker_runtime")}</Typography.Text>
                <Tag color={getWorkerStatusMeta(nativeResumeWorker.runtime.status).color}>
                  {getWorkerStatusMeta(nativeResumeWorker.runtime.status).label}
                </Tag>
              </Flex>
              <Flex gap={24} wrap="wrap">
                <div>
                  <Typography.Text type="secondary" style={{ fontSize: 11, display: 'block' }}>
                    {i18nText("settings", "auto.worker_id")}</Typography.Text>
                  <Typography.Text code style={{ fontSize: 12 }}>
                    {nativeResumeWorker.runtime.worker_id?.slice(0, 24) ?? '—'}
                  </Typography.Text>
                </div>
                <div>
                  <Typography.Text type="secondary" style={{ fontSize: 11, display: 'block' }}>
                    {i18nText("settings", "auto.last_heartbeat")}</Typography.Text>
                  <Typography.Text style={{ fontSize: 12 }}>
                    {formatOptionalTime(nativeResumeWorker.runtime.last_heartbeat_at)}
                  </Typography.Text>
                </div>
                <div>
                  <Typography.Text type="secondary" style={{ fontSize: 11, display: 'block' }}>
                    {i18nText("settings", "auto.processed")}</Typography.Text>
                  <Typography.Text style={{ fontSize: 12 }}>
                    {nativeResumeWorker.runtime.processed_count}
                  </Typography.Text>
                </div>
                <div>
                  <Typography.Text type="secondary" style={{ fontSize: 11, display: 'block' }}>
                    {i18nText("settings", "auto.last_duration")}</Typography.Text>
                  <Typography.Text style={{ fontSize: 12 }}>
                    {nativeResumeWorker.runtime.last_duration_ms ?? '—'}
                    {nativeResumeWorker.runtime.last_duration_ms == null ? '' : ' ms'}
                  </Typography.Text>
                </div>
              </Flex>
              {nativeResumeWorker.runtime.last_error ? (
                <Alert
                  type="error"
                  showIcon
                  style={{ marginTop: 14 }}
                  message={i18nText("settings", "auto.worker_last_error")}
                  description={nativeResumeWorker.runtime.last_error}
                />
              ) : null}
            </div>

            <div
              style={{
                flex: '1 1 320px',
                border: '1px solid #f0f0f0',
                borderRadius: 8,
                padding: '18px 20px',
                background: '#fff'
              }}
            >
              <Typography.Text strong style={{ display: 'block', fontSize: 14, marginBottom: 12 }}>
                {i18nText("settings", "auto.resume_queue")}</Typography.Text>
              <Flex gap={16} wrap="wrap">
                {[
                  ['pending', nativeResumeWorker.queue.pending_count],
                  ['claimed', nativeResumeWorker.queue.claimed_count],
                  ['succeeded', nativeResumeWorker.queue.succeeded_count],
                  ['failed', nativeResumeWorker.queue.failed_count],
                  ['expired', nativeResumeWorker.queue.expired_claim_count]
                ].map(([queueStatus, count]) => (
                  <div key={queueStatus}>
                    <Typography.Text type="secondary" style={{ fontSize: 11, display: 'block' }}>
                      {getResumeQueueLabel(String(queueStatus))}
                    </Typography.Text>
                    <Typography.Text style={{ fontSize: 16 }}>
                      {count}
                    </Typography.Text>
                  </div>
                ))}
              </Flex>
              <Typography.Text type="secondary" style={{ display: 'block', marginTop: 14, fontSize: 12 }}>
                {i18nText("settings", "auto.oldest_pending_age")}:
                {' '}
                {nativeResumeWorker.queue.oldest_pending_age_seconds ?? '—'}
                {nativeResumeWorker.queue.oldest_pending_age_seconds == null ? '' : ' s'}
              </Typography.Text>
            </div>
          </Flex>
        </div>
      ) : null}

      {/* ════════════════════════════════════════════════
         宿主机
         ════════════════════════════════════════════════ */}
      <div>
        <Flex align="center" gap={8} style={{ marginBottom: 14 }}>
          <ClusterOutlined style={{ color: '#00ab73', fontSize: 15 }} />
          <Typography.Text strong style={{ fontSize: 14 }}>
            {i18nText("settings", "auto.host")}</Typography.Text>
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
          <Empty description={i18nText("settings", "auto.currently_host_information_display")} />
        )}
      </div>
    </SettingsSectionSurface>
  );
}
