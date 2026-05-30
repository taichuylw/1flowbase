import {
  ApiOutlined,
  ClockCircleOutlined,
  CloseCircleOutlined,
  DashboardOutlined,
  DatabaseOutlined,
  HourglassOutlined,
  NodeIndexOutlined,
  QuestionCircleOutlined,
  ReloadOutlined,
  SafetyCertificateOutlined
} from '@ant-design/icons';
import { useQuery } from '@tanstack/react-query';
import {
  Button,
  Empty,
  Radio,
  Result,
  Space,
  Table,
  Tooltip,
  Typography
} from 'antd';
import type { ColumnsType } from 'antd/es/table';
import { useMemo, useState, type ReactNode } from 'react';

import {
  formatDate,
  formatDateTime,
  formatNumber,
  formatTime as formatClockTime
} from '../../../shared/i18n/format';
import { LoadingState } from '../../../shared/ui/loading-state/LoadingState';
import {
  applicationRunMonitoringReportQueryKey,
  applicationRuntimeActivityQueryKey,
  fetchApplicationRunMonitoringReport,
  fetchApplicationRuntimeActivity,
  type ApplicationRunMonitoringApiKeyUsage,
  type ApplicationRunMonitoringAuthorizedAccountUsage,
  type ApplicationRunMonitoringBucket,
  type ApplicationRunMonitoringExternalConversationUsage,
  type ApplicationRunMonitoringExternalUserUsage,
  type ApplicationRunMonitoringProtocolBreakdown,
  type ApplicationRunMonitoringReport,
  type ApplicationRunMonitoringRunRank,
  type ApplicationRunMonitoringSourceBreakdown,
  type ApplicationRuntimeActivity
} from '../api/runtime';
import { ApplicationMonitoringChart } from '../components/monitoring/ApplicationMonitoringChart';
import './application-monitoring-page.css';
import { i18nText } from '../../../shared/i18n/text';

type MonitoringTimeRange = 1 | 7 | 28 | 90 | 365;

const TIME_RANGE_OPTIONS: Array<{
  label: string;
  value: MonitoringTimeRange;
}> = [
  { label: i18nText("applications", "auto.past_twenty_four_hours"), value: 1 },
  { label: i18nText("applications", "auto.past_seven_days"), value: 7 },
  { label: i18nText("applications", "auto.past_four_weeks"), value: 28 },
  { label: i18nText("applications", "auto.past_three_months"), value: 90 },
  { label: i18nText("applications", "auto.past_twelve_months"), value: 365 }
];

function getMonitoringBucket(
  range: MonitoringTimeRange
): ApplicationRunMonitoringBucket {
  if (range <= 1) {
    return 'hour';
  }
  if (range >= 180) {
    return 'month';
  }
  if (range >= 60) {
    return 'week';
  }
  return 'day';
}

function formatInteger(value: number) {
  return formatNumber(value, { maximumFractionDigits: 0 });
}

function formatDecimal(value: number, digits = 1) {
  return formatNumber(value, {
    maximumFractionDigits: digits,
    minimumFractionDigits: digits
  });
}

function formatPercent(value: number) {
  return `${formatDecimal(value * 100, 1)}%`;
}

function formatSignedPercent(value: number) {
  const prefix = value > 0 ? '+' : '';
  return `${prefix}${formatPercent(value)}`;
}

function tokenComparisonMetric(report: ApplicationRunMonitoringReport) {
  if (
    report.tokens_comparison.previous_total_tokens_sum === 0 &&
    report.tokens.total_tokens_sum > 0
  ) {
    return {
      label: i18nText('applications', 'auto.token_increase_from_empty'),
      value: formatInteger(report.tokens.total_tokens_sum)
    };
  }

  return {
    label: i18nText('applications', 'auto.token_change'),
    value: formatSignedPercent(report.tokens_comparison.token_change_rate)
  };
}

function formatDuration(value: number | null | undefined) {
  if (value == null || Number.isNaN(value)) {
    return '-';
  }
  if (value < 1000) {
    return `${formatInteger(value)}ms`;
  }
  return `${formatDecimal(value / 1000, 1)}s`;
}

function formatTime(value: string | null | undefined) {
  if (!value) {
    return '-';
  }
  return formatDateTime(value);
}

function formatTrendBucket(
  value: string,
  bucket: ApplicationRunMonitoringBucket
) {
  if (bucket === 'hour') {
    return formatClockTime(value, {
      hour: '2-digit',
      minute: '2-digit'
    });
  }
  if (bucket === 'month') {
    return formatDate(value, {
      year: 'numeric',
      month: '2-digit'
    });
  }
  return formatDate(value);
}

function sourceLabel(source: string) {
  return source === 'public_api' ? 'Public API' : i18nText("applications", "auto.console");
}

function statisticValue(value: string | number) {
  return value;
}

function runtimeHealthLabel(state: ApplicationRuntimeActivity['health']['state']) {
  switch (state) {
    case 'busy':
      return i18nText('applications', 'auto.runtime_health_busy');
    case 'slow':
      return i18nText('applications', 'auto.runtime_health_slow');
    case 'unstable':
      return i18nText('applications', 'auto.runtime_health_unstable');
    case 'failing':
      return i18nText('applications', 'auto.runtime_health_failing');
    case 'failing_now':
      return i18nText('applications', 'auto.runtime_health_failing_now');
    case 'healthy':
    default:
      return i18nText('applications', 'auto.runtime_health_healthy');
  }
}

function runtimeHealthTone(
  state: ApplicationRuntimeActivity['health']['state']
): 'blue' | 'green' | 'gold' | 'red' | 'purple' | 'cyan' {
  switch (state) {
    case 'healthy':
      return 'green';
    case 'busy':
      return 'cyan';
    case 'slow':
      return 'gold';
    case 'unstable':
      return 'purple';
    case 'failing':
    case 'failing_now':
      return 'red';
    default:
      return 'blue';
  }
}

function runtimeTrendLabel(
  trend: ApplicationRuntimeActivity['health']['throughput_trend']
) {
  switch (trend) {
    case 'rising':
      return i18nText('applications', 'auto.trend_rising');
    case 'falling':
      return i18nText('applications', 'auto.trend_falling');
    case 'steady':
    default:
      return i18nText('applications', 'auto.trend_steady');
  }
}

function RuntimeActivityMetric({
  label,
  value,
  tone = 'blue'
}: {
  label: string;
  value: string | number;
  tone?: 'blue' | 'green' | 'gold' | 'red' | 'purple' | 'cyan';
}) {
  return (
    <div className={`runtime-activity-metric runtime-activity-metric--${tone}`}>
      <span className="runtime-activity-metric__label">{label}</span>
      <span className="runtime-activity-metric__value">{value}</span>
    </div>
  );
}

function RuntimeActivityGroup({
  children,
  title
}: {
  children: ReactNode;
  title: string;
}) {
  return (
    <section className="runtime-activity-group">
      <Typography.Text className="runtime-activity-group__title" type="secondary">
        {title}
      </Typography.Text>
      <div className="runtime-activity-group__metrics">{children}</div>
    </section>
  );
}

function RuntimeActivityTitle() {
  return (
    <span className="runtime-activity-panel__title">
      {i18nText('applications', 'auto.runtime_activity')}
      <Tooltip
        title={
          <span>
            {i18nText('applications', 'auto.current_instance_runtime_data')}
            <br />
            {i18nText('applications', 'auto.runtime_activity_memory_scope')}
          </span>
        }
      >
        <Button
          aria-label={i18nText('applications', 'auto.runtime_activity')}
          className="runtime-activity-panel__help"
          icon={<QuestionCircleOutlined aria-hidden="true" />}
          size="small"
          type="text"
        />
      </Tooltip>
    </span>
  );
}

function RuntimeActivityPanel({
  activity,
  loading,
  error
}: {
  activity?: ApplicationRuntimeActivity;
  loading: boolean;
  error: boolean;
}) {
  if (loading && !activity) {
    return (
      <MonitoringPanel title={<RuntimeActivityTitle />}>
        <LoadingState compact />
      </MonitoringPanel>
    );
  }

  if (error || !activity) {
    return (
      <MonitoringPanel title={<RuntimeActivityTitle />}>
        <Result
          status="warning"
          title={i18nText('applications', 'auto.runtime_activity_load_failed')}
        />
      </MonitoringPanel>
    );
  }

  const active = activity.active;
  const pressure = activity.pressure;
  const health = activity.health;
  const fiveMinutes = activity.windows.five_minutes;

  return (
    <MonitoringPanel title={<RuntimeActivityTitle />}>
      <div className="runtime-activity-panel__groups">
        <RuntimeActivityGroup title={i18nText('applications', 'auto.runtime_group_overview')}>
          <RuntimeActivityMetric
            label={i18nText('applications', 'auto.runtime_health')}
            value={runtimeHealthLabel(health.state)}
            tone={runtimeHealthTone(health.state)}
          />
          <RuntimeActivityMetric
            label={i18nText('applications', 'auto.active_total')}
            value={formatInteger(active.total)}
            tone="blue"
          />
          <RuntimeActivityMetric
            label={i18nText('applications', 'auto.process_peak')}
            value={formatInteger(activity.peaks.process_peak_concurrency)}
            tone="blue"
          />
        </RuntimeActivityGroup>

        <RuntimeActivityGroup title={i18nText('applications', 'auto.runtime_group_protocol')}>
          <RuntimeActivityMetric
            label="HTTP"
            value={formatInteger(active.http_requests)}
            tone="cyan"
          />
          <RuntimeActivityMetric
            label="SSE"
            value={formatInteger(active.sse_connections)}
            tone="green"
          />
          <RuntimeActivityMetric
            label="WebSocket"
            value={formatInteger(active.websocket_connections)}
            tone="purple"
          />
        </RuntimeActivityGroup>

        <RuntimeActivityGroup title={i18nText('applications', 'auto.runtime_group_execution')}>
          <RuntimeActivityMetric
            label={i18nText('applications', 'auto.application_executions_active')}
            value={formatInteger(active.application_executions)}
            tone="gold"
          />
          <RuntimeActivityMetric
            label={i18nText('applications', 'auto.tool_calls_active')}
            value={formatInteger(active.tool_calls)}
            tone="purple"
          />
          <RuntimeActivityMetric
            label={i18nText('applications', 'auto.model_requests_active')}
            value={formatInteger(active.model_requests)}
            tone="blue"
          />
          <RuntimeActivityMetric
            label={i18nText('applications', 'auto.waiting_active')}
            value={active.waiting == null ? '-' : formatInteger(active.waiting)}
            tone="gold"
          />
          <RuntimeActivityMetric
            label={i18nText('applications', 'auto.slow_active_executions')}
            value={formatInteger(pressure.slow_active_executions)}
            tone="gold"
          />
        </RuntimeActivityGroup>

        <RuntimeActivityGroup title={i18nText('applications', 'auto.runtime_group_five_minutes')}>
          <RuntimeActivityMetric
            label={i18nText('applications', 'auto.five_minute_failure_rate')}
            value={formatPercent(health.failure_rate_5m)}
            tone="red"
          />
          <RuntimeActivityMetric
            label={i18nText('applications', 'auto.five_minute_disconnect_rate')}
            value={formatPercent(health.disconnect_rate_5m)}
            tone="purple"
          />
          <RuntimeActivityMetric
            label={i18nText('applications', 'auto.completed_five_minutes')}
            value={formatInteger(fiveMinutes.completed)}
            tone="green"
          />
          <RuntimeActivityMetric
            label={i18nText('applications', 'auto.five_minute_throughput')}
            value={formatDecimal(health.throughput_5m_per_minute, 1)}
            tone="green"
          />
          <RuntimeActivityMetric
            label={i18nText('applications', 'auto.throughput_trend')}
            value={runtimeTrendLabel(health.throughput_trend)}
            tone="cyan"
          />
        </RuntimeActivityGroup>
      </div>
    </MonitoringPanel>
  );
}

const protocolColumns: ColumnsType<ApplicationRunMonitoringProtocolBreakdown> =
  [
    {
      title: i18nText("applications", "auto.protocol"),
      dataIndex: 'protocol',
      key: 'protocol'
    },
    {
      title: i18nText("applications", "auto.request_count"),
      dataIndex: 'request_count',
      key: 'request_count',
      align: 'right',
      render: (value: number) => formatInteger(value)
    },
    {
      title: i18nText("applications", "auto.success_rate"),
      dataIndex: 'success_rate',
      key: 'success_rate',
      align: 'right',
      render: (value: number) => {
        let color = '#52c41a';
        if (value < 0.9) color = '#ff4d4f';
        else if (value < 0.98) color = '#faad14';
        return (
          <div style={{ display: 'inline-flex', alignItems: 'center', gap: 8 }}>
            <span style={{ minWidth: 42, textAlign: 'right', fontWeight: 550 }}>
              {formatPercent(value)}
            </span>
            <div
              style={{
                width: 36,
                height: 6,
                background: 'rgba(0,0,0,0.04)',
                borderRadius: 3,
                overflow: 'hidden'
              }}
            >
              <div
                style={{
                  width: `${value * 100}%`,
                  height: '100%',
                  background: color,
                  borderRadius: 3
                }}
              />
            </div>
          </div>
        );
      }
    },
    {
      title: i18nText("applications", "auto.average_duration"),
      dataIndex: 'avg_duration_ms',
      key: 'avg_duration_ms',
      align: 'right',
      render: (value: number) => {
        let color = 'processing';
        if (value > 1000) color = 'volcano';
        else if (value > 500) color = 'warning';
        return (
          <span className={`duration-tag duration-tag--${color}`}>
            {formatDuration(value)}
          </span>
        );
      }
    },
    {
      title: 'Tokens',
      dataIndex: 'total_tokens',
      key: 'total_tokens',
      align: 'right',
      render: (value: number) => formatInteger(value)
    }
  ];

const sourceColumns: ColumnsType<ApplicationRunMonitoringSourceBreakdown> = [
  {
    title: i18nText("applications", "auto.source"),
    dataIndex: 'source',
    key: 'source',
    render: sourceLabel
  },
  {
    title: i18nText("applications", "auto.request_count"),
    dataIndex: 'request_count',
    key: 'request_count',
    align: 'right',
    render: (value: number) => formatInteger(value)
  },
  {
    title: i18nText("applications", "auto.success_rate"),
    dataIndex: 'success_rate',
    key: 'success_rate',
    align: 'right',
    render: (value: number) => {
      let color = '#52c41a';
      if (value < 0.9) color = '#ff4d4f';
      else if (value < 0.98) color = '#faad14';
      return (
        <div style={{ display: 'inline-flex', alignItems: 'center', gap: 8 }}>
          <span style={{ minWidth: 42, textAlign: 'right', fontWeight: 550 }}>
            {formatPercent(value)}
          </span>
          <div
            style={{
              width: 36,
              height: 6,
              background: 'rgba(0,0,0,0.04)',
              borderRadius: 3,
              overflow: 'hidden'
            }}
          >
            <div
              style={{
                width: `${value * 100}%`,
                height: '100%',
                background: color,
                borderRadius: 3
              }}
            />
          </div>
        </div>
      );
    }
  },
  {
    title: 'Tokens',
    dataIndex: 'total_tokens',
    key: 'total_tokens',
    align: 'right',
    render: (value: number) => formatInteger(value)
  }
];

function usageColumns<
  T extends {
    request_count: number;
    total_tokens: number;
    avg_duration_ms: number;
    failed_count: number;
  }
>(
  label: string,
  key: keyof T,
  maxRequests = 0,
  maxTokens = 0,
  renderDimension?: (value: T[keyof T], record: T) => ReactNode
): ColumnsType<T> {
  return [
    {
      title: label,
      dataIndex: key as string,
      key: key as string,
      render: (value: T[keyof T], record: T) =>
        renderDimension?.(value, record) ?? value ?? '-'
    },
    {
      title: i18nText("applications", "auto.request_count"),
      dataIndex: 'request_count',
      key: 'request_count',
      align: 'right',
      render: (value: number) => {
        const pct = maxRequests > 0 ? (value / maxRequests) * 100 : 0;
        return (
          <div className="table-cell-progress">
            <span className="table-cell-progress__value">
              {formatInteger(value)}
            </span>
            <div className="table-cell-progress__bar-bg">
              <div
                className="table-cell-progress__bar"
                style={{
                  width: `${pct}%`,
                  background: '#e6f4ff',
                  borderRight: '2px solid #1677ff'
                }}
              />
            </div>
          </div>
        );
      }
    },
    {
      title: i18nText("applications", "auto.failure_count"),
      dataIndex: 'failed_count',
      key: 'failed_count',
      align: 'right',
      render: (value: number) => {
        if (value === 0)
          return (
            <span
              style={{ color: 'var(--ant-color-text-secondary)', opacity: 0.6 }}
            >
              0
            </span>
          );
        return (
          <span style={{ color: '#ff4d4f', fontWeight: 600 }}>
            {formatInteger(value)}
          </span>
        );
      }
    },
    {
      title: i18nText("applications", "auto.average_duration"),
      dataIndex: 'avg_duration_ms',
      key: 'avg_duration_ms',
      align: 'right',
      render: (value: number) => {
        let color = 'processing';
        if (value > 1000) color = 'volcano';
        else if (value > 500) color = 'warning';
        return (
          <span className={`duration-tag duration-tag--${color}`}>
            {formatDuration(value)}
          </span>
        );
      }
    },
    {
      title: 'Tokens',
      dataIndex: 'total_tokens',
      key: 'total_tokens',
      align: 'right',
      render: (value: number) => {
        const pct = maxTokens > 0 ? (value / maxTokens) * 100 : 0;
        return (
          <div className="table-cell-progress">
            <span className="table-cell-progress__value">
              {formatInteger(value)}
            </span>
            <div className="table-cell-progress__bar-bg">
              <div
                className="table-cell-progress__bar"
                style={{
                  width: `${pct}%`,
                  background: '#f9f0ff',
                  borderRight: '2px solid #722ed1'
                }}
              />
            </div>
          </div>
        );
      }
    }
  ];
}

const runRankColumns: ColumnsType<ApplicationRunMonitoringRunRank> = [
  {
    title: i18nText("applications", "auto.run"),
    dataIndex: 'title',
    key: 'title',
    render: (value: string, run) => (
      <Space direction="vertical" size={0}>
        <Typography.Text>{value}</Typography.Text>
        <Typography.Text type="secondary">{run.flow_run_id}</Typography.Text>
      </Space>
    )
  },
  {
    title: i18nText("applications", "auto.status"),
    dataIndex: 'status',
    key: 'status'
  },
  {
    title: i18nText("applications", "auto.duration"),
    dataIndex: 'duration_ms',
    key: 'duration_ms',
    align: 'right',
    render: (value: number | null) => formatDuration(value)
  },
  {
    title: 'Tokens',
    dataIndex: 'total_tokens',
    key: 'total_tokens',
    align: 'right',
    render: (value: number | null) =>
      value == null ? '-' : formatInteger(value)
  },
  {
    title: i18nText("applications", "auto.start_time"),
    dataIndex: 'started_at',
    key: 'started_at',
    render: formatTime
  }
];

function MonitoringPanel({
  children,
  title
}: {
  children: ReactNode;
  title: ReactNode;
}) {
  return (
    <section className="application-monitoring-panel">
      <Typography.Title level={5}>{title}</Typography.Title>
      {children}
    </section>
  );
}

function MonitoringMetricGroup({
  children,
  title
}: {
  children: ReactNode;
  title: string;
}) {
  return (
    <section className="application-monitoring-metric-group">
      <Typography.Text
        className="application-monitoring-metric-group__title"
        type="secondary"
      >
        {title}
      </Typography.Text>
      <div className="application-monitoring-metric-group__items">
        {children}
      </div>
    </section>
  );
}

function MonitoringTable<T extends object>({
  columns,
  dataSource,
  rowKey
}: {
  columns: ColumnsType<T>;
  dataSource: T[];
  rowKey: keyof T | ((record: T) => string);
}) {
  return dataSource.length === 0 ? (
    <Empty image={Empty.PRESENTED_IMAGE_SIMPLE} description={null} />
  ) : (
    <Table
      columns={columns}
      dataSource={dataSource}
      pagination={false}
      rowKey={rowKey}
      size="small"
    />
  );
}

function buildTokenTrendOption(report: ApplicationRunMonitoringReport) {
  return {
    color: ['#1677ff'],
    grid: {
      left: 54,
      right: 20,
      top: 28,
      bottom: 34
    },
    tooltip: {
      trigger: 'axis',
      backgroundColor: 'rgba(255, 255, 255, 0.95)',
      borderColor: '#f0f0f0',
      borderWidth: 1,
      textStyle: { color: '#1f1f1f', fontSize: 12 }
    },
    xAxis: {
      type: 'category',
      data: report.tokens_trend.map((point) =>
        formatTrendBucket(point.bucket_start, report.meta.bucket)
      ),
      axisLine: {
        lineStyle: {
          color: '#f0f0f0'
        }
      },
      axisLabel: {
        color: '#8c8c8c'
      }
    },
    yAxis: [
      {
        type: 'value',
        axisLine: { show: false },
        axisLabel: { color: '#8c8c8c' },
        splitLine: {
          lineStyle: {
            color: 'rgba(0, 0, 0, 0.05)',
            type: 'dashed'
          }
        }
      }
    ],
    series: [
      {
        name: 'Tokens',
        type: 'line',
        smooth: true,
        symbol: 'circle',
        symbolSize: 6,
        showSymbol: false,
        areaStyle: {
          color: {
            type: 'linear',
            x: 0,
            y: 0,
            x2: 0,
            y2: 1,
            colorStops: [
              { offset: 0, color: 'rgba(22, 119, 255, 0.25)' },
              { offset: 1, color: 'rgba(22, 119, 255, 0.01)' }
            ]
          }
        },
        data: report.tokens_trend.map((point) => point.total_tokens)
      },
      {
        name: i18nText("applications", "auto.run_count"),
        type: 'bar',
        barMaxWidth: 16,
        itemStyle: {
          borderRadius: [4, 4, 0, 0],
          color: {
            type: 'linear',
            x: 0,
            y: 0,
            x2: 0,
            y2: 1,
            colorStops: [
              { offset: 0, color: '#34d399' },
              { offset: 1, color: '#059669' }
            ]
          }
        },
        data: report.tokens_trend.map((point) => point.run_count)
      }
    ]
  };
}

function buildProtocolOption(report: ApplicationRunMonitoringReport) {
  return {
    grid: {
      left: 54,
      right: 20,
      top: 20,
      bottom: 38
    },
    tooltip: {
      trigger: 'axis',
      backgroundColor: 'rgba(255, 255, 255, 0.95)',
      borderColor: '#f0f0f0',
      borderWidth: 1,
      textStyle: { color: '#1f1f1f', fontSize: 12 }
    },
    xAxis: {
      type: 'category',
      data: report.protocols.map((item) => item.protocol),
      axisLine: {
        lineStyle: {
          color: '#f0f0f0'
        }
      },
      axisLabel: {
        color: '#8c8c8c'
      }
    },
    yAxis: {
      type: 'value',
      axisLine: { show: false },
      axisLabel: { color: '#8c8c8c' },
      splitLine: {
        lineStyle: {
          color: 'rgba(0, 0, 0, 0.05)',
          type: 'dashed'
        }
      }
    },
    series: [
      {
        name: i18nText("applications", "auto.request_count"),
        type: 'bar',
        barMaxWidth: 24,
        itemStyle: {
          borderRadius: [4, 4, 0, 0],
          color: {
            type: 'linear',
            x: 0,
            y: 0,
            x2: 0,
            y2: 1,
            colorStops: [
              { offset: 0, color: '#1677ff' },
              { offset: 1, color: '#85a5ff' }
            ]
          }
        },
        data: report.protocols.map((item) => item.request_count)
      }
    ]
  };
}

function buildSourceOption(report: ApplicationRunMonitoringReport) {
  const totalRequests = report.sources.reduce(
    (sum, item) => sum + item.request_count,
    0
  );
  return {
    color: ['#1677ff', '#52c41a', '#faad14'],
    tooltip: {
      trigger: 'item',
      backgroundColor: 'rgba(255, 255, 255, 0.95)',
      borderColor: '#f0f0f0',
      borderWidth: 1,
      textStyle: { color: '#1f1f1f', fontSize: 12 }
    },
    legend: {
      bottom: 0,
      itemGap: 16
    },
    title: {
      text: formatInteger(totalRequests),
      subtext: i18nText("applications", "auto.total_requests"),
      left: 'center',
      top: '38%',
      textStyle: {
        fontSize: 20,
        fontWeight: 'bold',
        color: '#1f1f1f',
        fontFamily: 'Outfit, Inter, sans-serif'
      },
      subtextStyle: {
        fontSize: 12,
        color: '#8c8c8c'
      }
    },
    series: [
      {
        name: i18nText("applications", "auto.source"),
        type: 'pie',
        radius: ['55%', '75%'],
        center: ['50%', '46%'],
        avoidLabelOverlap: false,
        itemStyle: {
          borderRadius: 4,
          borderColor: '#ffffff',
          borderWidth: 2
        },
        label: {
          show: false
        },
        data: report.sources.map((item) => ({
          name: sourceLabel(item.source),
          value: item.request_count
        }))
      }
    ]
  };
}

interface RunRankListProps {
  runs: ApplicationRunMonitoringRunRank[];
  metricType: 'duration' | 'token';
}

function RunRankList({ runs, metricType }: RunRankListProps) {
  if (runs.length === 0) {
    return <Empty image={Empty.PRESENTED_IMAGE_SIMPLE} description={null} />;
  }

  const maxVal = Math.max(
    1,
    ...runs.map((r) =>
      metricType === 'duration' ? (r.duration_ms ?? 0) : (r.total_tokens ?? 0)
    )
  );

  return (
    <div className="run-rank-list">
      {runs.map((run, index) => {
        const val =
          metricType === 'duration'
            ? (run.duration_ms ?? 0)
            : (run.total_tokens ?? 0);
        const pct = (val / maxVal) * 100;
        const displayVal =
          metricType === 'duration' ? formatDuration(val) : formatInteger(val);

        let statusColor = '#8c8c8c';
        if (run.status === 'succeeded' || run.status === 'success') {
          statusColor = '#52c41a';
        } else if (run.status === 'failed' || run.status === 'fail') {
          statusColor = '#ff4d4f';
        }

        return (
          <div key={run.flow_run_id} className="run-rank-item">
            <div className="run-rank-item__index">#{index + 1}</div>
            <div className="run-rank-item__info">
              <div className="run-rank-item__header">
                <span className="run-rank-item__title">{run.title}</span>
                <span
                  className="run-rank-item__status-dot"
                  style={{ background: statusColor }}
                />
              </div>
              <div className="run-rank-item__sub">
                <span>ID: {run.flow_run_id}</span>
                <span className="run-rank-item__time">
                  {formatTime(run.started_at)}
                </span>
              </div>
            </div>
            <div className="run-rank-item__metric">
              <div className="run-rank-item__metric-value">{displayVal}</div>
              <div className="run-rank-item__track">
                <div
                  className={`run-rank-item__bar run-rank-item__bar--${metricType}`}
                  style={{ width: `${pct}%` }}
                />
              </div>
            </div>
          </div>
        );
      })}
    </div>
  );
}

export function ApplicationMonitoringPage({
  applicationId
}: {
  applicationId: string;
}) {
  const [timeRangeDays, setTimeRangeDays] = useState<MonitoringTimeRange>(7);
  const bucket = getMonitoringBucket(timeRangeDays);
  const reportInput = {
    timeRangeDays,
    bucket
  };
  const reportQuery = useQuery({
    queryKey: applicationRunMonitoringReportQueryKey(
      applicationId,
      reportInput
    ),
    queryFn: () =>
      fetchApplicationRunMonitoringReport(applicationId, reportInput),
    placeholderData: (previousData) => previousData
  });
  const runtimeActivityQuery = useQuery({
    queryKey: applicationRuntimeActivityQueryKey(applicationId),
    queryFn: () => fetchApplicationRuntimeActivity(applicationId),
    refetchInterval: 5000
  });
  const report = reportQuery.data;

  const maxAuthRequests = useMemo(
    () =>
      report
        ? Math.max(
            1,
            ...report.authorized_accounts.map((item) => item.request_count)
          )
        : 1,
    [report?.authorized_accounts]
  );
  const maxAuthTokens = useMemo(
    () =>
      report
        ? Math.max(
            1,
            ...report.authorized_accounts.map((item) => item.total_tokens)
          )
        : 1,
    [report?.authorized_accounts]
  );

  const maxExtUserRequests = useMemo(
    () =>
      report
        ? Math.max(
            1,
            ...report.external_users.map((item) => item.request_count)
          )
        : 1,
    [report?.external_users]
  );
  const maxExtUserTokens = useMemo(
    () =>
      report
        ? Math.max(1, ...report.external_users.map((item) => item.total_tokens))
        : 1,
    [report?.external_users]
  );

  const maxApiKeyRequests = useMemo(
    () =>
      report
        ? Math.max(1, ...report.api_keys.map((item) => item.request_count))
        : 1,
    [report?.api_keys]
  );
  const maxApiKeyTokens = useMemo(
    () =>
      report
        ? Math.max(1, ...report.api_keys.map((item) => item.total_tokens))
        : 1,
    [report?.api_keys]
  );

  const maxExtConvRequests = useMemo(
    () =>
      report
        ? Math.max(
            1,
            ...report.external_conversations.map((item) => item.request_count)
          )
        : 1,
    [report?.external_conversations]
  );
  const maxExtConvTokens = useMemo(
    () =>
      report
        ? Math.max(
            1,
            ...report.external_conversations.map((item) => item.total_tokens)
          )
        : 1,
    [report?.external_conversations]
  );

  const activeRangeLabel =
    TIME_RANGE_OPTIONS.find((option) => option.value === timeRangeDays)
      ?.label ?? i18nText("applications", "auto.past_seven_days");
  const tokenTrendOption = useMemo(
    () => (report ? buildTokenTrendOption(report) : null),
    [report]
  );
  const protocolOption = useMemo(
    () => (report ? buildProtocolOption(report) : null),
    [report]
  );
  const sourceOption = useMemo(
    () => (report ? buildSourceOption(report) : null),
    [report]
  );
  const tokenComparison = report ? tokenComparisonMetric(report) : null;

  if (reportQuery.isPending) {
    return <LoadingState compact />;
  }

  if (reportQuery.isError || !report) {
    return <Result status="error" title={i18nText("applications", "auto.monitoring_report_load_failed")} />;
  }

  return (
    <div
      className="application-monitoring-page"
      data-testid="application-monitoring-page"
    >
      <RuntimeActivityPanel
        activity={runtimeActivityQuery.data}
        loading={runtimeActivityQuery.isPending}
        error={runtimeActivityQuery.isError}
      />

      <div className="application-monitoring-page__toolbar">
        <Radio.Group
          optionType="button"
          options={TIME_RANGE_OPTIONS}
          value={timeRangeDays}
          onChange={(event) => setTimeRangeDays(event.target.value)}
        />
        <Space
          className="application-monitoring-page__toolbar-status"
          size={12}
        >
          <Typography.Text type="secondary">
            {reportQuery.isFetching
              ? i18nText("applications", "auto.refreshing")
              : i18nText("applications", "auto.current_scope", { value1: activeRangeLabel })}
          </Typography.Text>
          {!report.overview.running_count_included ? (
            <Tooltip title={i18nText("applications", "auto.only_finished_runs_counted")}>
              <Button
                aria-label={i18nText("applications", "auto.run_statistics_caliber")}
                className="application-monitoring-page__scope-help"
                icon={<QuestionCircleOutlined aria-hidden="true" />}
                size="small"
                type="text"
              />
            </Tooltip>
          ) : null}
          <Button
            aria-label={i18nText("applications", "auto.refresh_monitoring_report")}
            icon={<ReloadOutlined aria-hidden="true" />}
            loading={reportQuery.isFetching}
            onClick={() => {
              void reportQuery.refetch();
            }}
          />
        </Space>
      </div>

      <section className="application-monitoring-page__metrics">
        <MonitoringMetricGroup title={i18nText("applications", "auto.monitoring_group_outcome")}>
          <div className="application-monitoring-metric application-monitoring-metric--blue">
            <div className="metric-card__icon-wrapper">
              <DashboardOutlined />
            </div>
            <div className="metric-card__content">
              <span className="metric-card__title">{i18nText("applications", "auto.total_runs")}</span>
              <span className="metric-card__value">
                {formatInteger(report.overview.total_count)}
              </span>
            </div>
          </div>

          <div className="application-monitoring-metric application-monitoring-metric--green">
            <div className="metric-card__icon-wrapper">
              <SafetyCertificateOutlined />
            </div>
            <div className="metric-card__content">
              <span className="metric-card__title">{i18nText("applications", "auto.success_rate")}</span>
              <span className="metric-card__value">
                {formatPercent(report.overview.success_rate)}
              </span>
            </div>
          </div>

          <div className="application-monitoring-metric application-monitoring-metric--red">
            <div className="metric-card__icon-wrapper">
              <CloseCircleOutlined />
            </div>
            <div className="metric-card__content">
              <span className="metric-card__title">{i18nText("applications", "auto.failure_count")}</span>
              <span className="metric-card__value">
                {formatInteger(report.overview.failed_count)}
              </span>
            </div>
          </div>
        </MonitoringMetricGroup>

        <MonitoringMetricGroup title={i18nText("applications", "auto.monitoring_group_performance")}>
          <div className="application-monitoring-metric application-monitoring-metric--gold">
            <div className="metric-card__icon-wrapper">
              <ClockCircleOutlined />
            </div>
            <div className="metric-card__content">
              <span className="metric-card__title">{i18nText("applications", "auto.slow_request_rate")}</span>
              <span className="metric-card__value">
                {formatPercent(report.duration.slow_run_rate)}
              </span>
            </div>
          </div>

          <div className="application-monitoring-metric application-monitoring-metric--cyan">
            <div className="metric-card__icon-wrapper">
              <HourglassOutlined />
            </div>
            <div className="metric-card__content">
              <span className="metric-card__title">{i18nText("applications", "auto.percentile_ninety_five_duration")}</span>
              <span className="metric-card__value">
                {formatDuration(report.duration.p95_duration_ms)}
              </span>
            </div>
          </div>
        </MonitoringMetricGroup>

        <MonitoringMetricGroup title={i18nText("applications", "auto.monitoring_group_tokens")}>
          <div className="application-monitoring-metric application-monitoring-metric--purple">
            <div className="metric-card__icon-wrapper">
              <DatabaseOutlined />
            </div>
            <div className="metric-card__content">
              <span className="metric-card__title">{i18nText("applications", "auto.total_tokens_amount")}</span>
              <span className="metric-card__value">
                {formatInteger(report.tokens.total_tokens_sum)}
              </span>
            </div>
          </div>

          <div className="application-monitoring-metric application-monitoring-metric--cyan">
            <div className="metric-card__icon-wrapper">
              <DatabaseOutlined />
            </div>
            <div className="metric-card__content">
              <span className="metric-card__title">{tokenComparison?.label}</span>
              <span className="metric-card__value">
                {tokenComparison?.value}
              </span>
            </div>
          </div>
        </MonitoringMetricGroup>

        <MonitoringMetricGroup title={i18nText("applications", "auto.monitoring_group_execution")}>
          <div className="application-monitoring-metric application-monitoring-metric--orange">
            <div className="metric-card__icon-wrapper">
              <ApiOutlined />
            </div>
            <div className="metric-card__content">
              <span className="metric-card__title">{i18nText("applications", "auto.tool_callback")}</span>
              <span className="metric-card__value">
                {formatInteger(report.tool_callbacks.total_tool_callback_count)}
              </span>
            </div>
          </div>

          <div className="application-monitoring-metric application-monitoring-metric--deep-blue">
            <div className="metric-card__icon-wrapper">
              <NodeIndexOutlined />
            </div>
            <div className="metric-card__content">
              <span className="metric-card__title">{i18nText("applications", "auto.peak_concurrency")}</span>
              <span className="metric-card__value">
                {formatInteger(report.concurrency.peak_concurrency)}
              </span>
            </div>
          </div>
        </MonitoringMetricGroup>
      </section>

      <div className="application-monitoring-page__chart-grid">
        <MonitoringPanel title={i18nText("applications", "auto.token_trend")}>
          {tokenTrendOption ? (
            <ApplicationMonitoringChart
              ariaLabel="Token trend chart"
              option={tokenTrendOption}
            />
          ) : null}
        </MonitoringPanel>
        <MonitoringPanel title={i18nText("applications", "auto.protocol_distribution")}>
          {protocolOption ? (
            <ApplicationMonitoringChart
              ariaLabel="Protocol distribution chart"
              option={protocolOption}
            />
          ) : null}
        </MonitoringPanel>
        <MonitoringPanel title={i18nText("applications", "auto.source_distribution")}>
          {sourceOption ? (
            <ApplicationMonitoringChart
              ariaLabel="Source distribution chart"
              option={sourceOption}
            />
          ) : null}
        </MonitoringPanel>
      </div>

      <div className="application-monitoring-page__table-grid">
        <MonitoringPanel title={i18nText("applications", "auto.duration_quality")}>
          <div className="application-monitoring-page__quality-grid">
            <div className="quality-metric-item">
              <span className="quality-metric-item__label">
                <ClockCircleOutlined /> {i18nText("applications", "auto.average_duration")}</span>
              <span className="quality-metric-item__value">
                {formatDuration(report.duration.avg_duration_ms)}
              </span>
            </div>
            <div className="quality-metric-item">
              <span className="quality-metric-item__label">
                <DashboardOutlined /> {i18nText("applications", "auto.percentile_fifty_duration")}</span>
              <span className="quality-metric-item__value">
                {formatDuration(report.duration.p50_duration_ms)}
              </span>
            </div>
            <div className="quality-metric-item">
              <span className="quality-metric-item__label">
                <NodeIndexOutlined /> {i18nText("applications", "auto.average_real_node_count")}</span>
              <span className="quality-metric-item__value">
                {formatDecimal(report.nodes.avg_unique_node_count, 1)}
              </span>
            </div>
            <div className="quality-metric-item">
              <span className="quality-metric-item__label">
                <ApiOutlined /> {i18nText("applications", "auto.average_tool_callback")}</span>
              <span className="quality-metric-item__value">
                {formatDecimal(
                  report.tool_callbacks.avg_tool_callback_count,
                  1
                )}
              </span>
            </div>
          </div>
        </MonitoringPanel>
        <MonitoringPanel title={i18nText("applications", "auto.protocol_details")}>
          <MonitoringTable
            columns={protocolColumns}
            dataSource={report.protocols}
            rowKey="protocol"
          />
        </MonitoringPanel>
        <MonitoringPanel title={i18nText("applications", "auto.source_details")}>
          <MonitoringTable
            columns={sourceColumns}
            dataSource={report.sources}
            rowKey="source"
          />
        </MonitoringPanel>
        <MonitoringPanel title={i18nText("applications", "auto.authorized_account")}>
          <MonitoringTable<ApplicationRunMonitoringAuthorizedAccountUsage>
            columns={usageColumns(
              i18nText("applications", "auto.account"),
              'authorized_account',
              maxAuthRequests,
              maxAuthTokens
            )}
            dataSource={report.authorized_accounts}
            rowKey={(record) => record.authorized_account ?? 'unknown'}
          />
        </MonitoringPanel>
        <MonitoringPanel title={i18nText("applications", "auto.external_users")}>
          <MonitoringTable<ApplicationRunMonitoringExternalUserUsage>
            columns={usageColumns(
              i18nText("applications", "auto.external_users"),
              'external_user',
              maxExtUserRequests,
              maxExtUserTokens
            )}
            dataSource={report.external_users}
            rowKey={(record) => record.external_user ?? 'unknown'}
          />
        </MonitoringPanel>
        <MonitoringPanel title="API Key">
          <MonitoringTable<ApplicationRunMonitoringApiKeyUsage>
            columns={usageColumns(
              'API Key',
              'api_key_id',
              maxApiKeyRequests,
              maxApiKeyTokens,
              (_value, record) =>
                record.api_key_name_snapshot ?? record.api_key_id
            )}
            dataSource={report.api_keys}
            rowKey="api_key_id"
          />
        </MonitoringPanel>
        <MonitoringPanel title={i18nText("applications", "auto.external_sessions")}>
          <MonitoringTable<ApplicationRunMonitoringExternalConversationUsage>
            columns={usageColumns(
              i18nText("applications", "auto.session"),
              'external_conversation_id',
              maxExtConvRequests,
              maxExtConvTokens
            )}
            dataSource={report.external_conversations}
            rowKey={(record) => record.external_conversation_id ?? 'unknown'}
          />
        </MonitoringPanel>
        <MonitoringPanel title={i18nText("applications", "auto.slowest_runs_top_ten")}>
          <RunRankList runs={report.slowest_runs} metricType="duration" />
        </MonitoringPanel>
        <MonitoringPanel title={i18nText("applications", "auto.high_token_runs_top_ten")}>
          <RunRankList runs={report.high_token_runs} metricType="token" />
        </MonitoringPanel>
      </div>
    </div>
  );
}
