import { Card, Table, Tag } from 'antd';

import type { NodeLastRun } from '../../../api/runtime';
import { i18nText } from '../../../../../shared/i18n/text';

const STATUS_COLOR: Record<string, string> = {
  succeeded: 'green',
  failed: 'red',
  running: 'blue'
};

function formatDuration(startedAt: string, finishedAt: string | null) {
  if (!finishedAt) {
    return i18nText("agentFlow", "auto.in_progress");
  }

  const durationMs =
    new Date(finishedAt).getTime() - new Date(startedAt).getTime();

  return `${Math.max(durationMs, 0)}`;
}

function summarizeTokenUsage(lastRun: NodeLastRun) {
  const metricsPayload = lastRun.node_run.metrics_payload;
  if (
    !metricsPayload ||
    typeof metricsPayload !== 'object' ||
    Array.isArray(metricsPayload)
  ) {
    return '—';
  }

  const metricsRecord = metricsPayload as Record<string, unknown>;
  const usage =
    metricsRecord.usage &&
    typeof metricsRecord.usage === 'object' &&
    !Array.isArray(metricsRecord.usage)
      ? (metricsRecord.usage as Record<string, unknown>)
      : metricsRecord;
  const value =
    typeof usage.total_tokens === 'number' || typeof usage.total_tokens === 'string'
      ? usage.total_tokens
      : undefined;

  if (typeof value === 'number') {
    return `${value}`;
  }

  if (typeof value === 'string' && value.trim()) {
    return value.trim();
  }

  return '—';
}

export function NodeRunSummaryCard({
  lastRun
}: {
  lastRun: NodeLastRun;
}) {
  const row = {
    key: 'summary',
    status: lastRun.flow_run.status,
    duration: formatDuration(
      lastRun.flow_run.started_at,
      lastRun.flow_run.finished_at
    ),
    tokens: summarizeTokenUsage(lastRun)
  };

  const columns = [
    {
      title: i18nText("agentFlow", "auto.status"),
      dataIndex: 'status',
      key: 'status',
      render: (status: string) => (
        <Tag color={STATUS_COLOR[status] ?? 'default'}>{status}</Tag>
      )
    },
    {
      title: i18nText("agentFlow", "auto.time_taken_ms"),
      dataIndex: 'duration',
      key: 'duration'
    },
    {
      title: 'token',
      dataIndex: 'tokens',
      key: 'tokens'
    }
  ];

  return (
    <Card title={i18nText("agentFlow", "auto.running_summary")}>
      <Table
        columns={columns}
        dataSource={[row]}
        pagination={false}
        size="small"
      />
    </Card>
  );
}
