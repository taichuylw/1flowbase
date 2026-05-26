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

import { LoadingState } from '../../../shared/ui/loading-state/LoadingState';
import {
  applicationRunMonitoringReportQueryKey,
  fetchApplicationRunMonitoringReport,
  type ApplicationRunMonitoringApiKeyUsage,
  type ApplicationRunMonitoringAuthorizedAccountUsage,
  type ApplicationRunMonitoringBucket,
  type ApplicationRunMonitoringExternalConversationUsage,
  type ApplicationRunMonitoringExternalUserUsage,
  type ApplicationRunMonitoringProtocolBreakdown,
  type ApplicationRunMonitoringReport,
  type ApplicationRunMonitoringRunRank,
  type ApplicationRunMonitoringSourceBreakdown
} from '../api/runtime';
import { ApplicationMonitoringChart } from '../components/monitoring/ApplicationMonitoringChart';
import './application-monitoring-page.css';

type MonitoringTimeRange = 1 | 7 | 28 | 90 | 365;

const TIME_RANGE_OPTIONS: Array<{
  label: string;
  value: MonitoringTimeRange;
}> = [
  { label: '过去 24 小时', value: 1 },
  { label: '过去 7 天', value: 7 },
  { label: '过去 4 周', value: 28 },
  { label: '过去 3 月', value: 90 },
  { label: '过去 12 月', value: 365 }
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
  return new Intl.NumberFormat('zh-CN', { maximumFractionDigits: 0 }).format(
    value
  );
}

function formatDecimal(value: number, digits = 1) {
  return new Intl.NumberFormat('zh-CN', {
    maximumFractionDigits: digits,
    minimumFractionDigits: digits
  }).format(value);
}

function formatPercent(value: number) {
  return `${formatDecimal(value * 100, 1)}%`;
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
  return new Date(value).toLocaleString();
}

function sourceLabel(source: string) {
  return source === 'public_api' ? 'Public API' : '控制台';
}

function statisticValue(value: string | number) {
  return value;
}

const protocolColumns: ColumnsType<ApplicationRunMonitoringProtocolBreakdown> =
  [
    {
      title: '协议',
      dataIndex: 'protocol',
      key: 'protocol'
    },
    {
      title: '请求数',
      dataIndex: 'request_count',
      key: 'request_count',
      align: 'right',
      render: (value: number) => formatInteger(value)
    },
    {
      title: '成功率',
      dataIndex: 'success_rate',
      key: 'success_rate',
      align: 'right',
      render: (value: number) => {
        let color = '#52c41a';
        if (value < 0.9) color = '#ff4d4f';
        else if (value < 0.98) color = '#faad14';
        return (
          <div style={{ display: 'inline-flex', alignItems: 'center', gap: 8 }}>
            <span style={{ minWidth: 42, textAlign: 'right', fontWeight: 550 }}>{formatPercent(value)}</span>
            <div style={{ width: 36, height: 6, background: 'rgba(0,0,0,0.04)', borderRadius: 3, overflow: 'hidden' }}>
              <div style={{ width: `${value * 100}%`, height: '100%', background: color, borderRadius: 3 }} />
            </div>
          </div>
        );
      }
    },
    {
      title: '平均耗时',
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
    title: '来源',
    dataIndex: 'source',
    key: 'source',
    render: sourceLabel
  },
  {
    title: '请求数',
    dataIndex: 'request_count',
    key: 'request_count',
    align: 'right',
    render: (value: number) => formatInteger(value)
  },
  {
    title: '成功率',
    dataIndex: 'success_rate',
    key: 'success_rate',
    align: 'right',
    render: (value: number) => {
      let color = '#52c41a';
      if (value < 0.9) color = '#ff4d4f';
      else if (value < 0.98) color = '#faad14';
      return (
        <div style={{ display: 'inline-flex', alignItems: 'center', gap: 8 }}>
          <span style={{ minWidth: 42, textAlign: 'right', fontWeight: 550 }}>{formatPercent(value)}</span>
          <div style={{ width: 36, height: 6, background: 'rgba(0,0,0,0.04)', borderRadius: 3, overflow: 'hidden' }}>
            <div style={{ width: `${value * 100}%`, height: '100%', background: color, borderRadius: 3 }} />
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
>(label: string, key: keyof T, maxRequests = 0, maxTokens = 0): ColumnsType<T> {
  return [
    {
      title: label,
      dataIndex: key as string,
      key: key as string,
      render: (value: string | null) => value ?? '-'
    },
    {
      title: '请求数',
      dataIndex: 'request_count',
      key: 'request_count',
      align: 'right',
      render: (value: number) => {
        const pct = maxRequests > 0 ? (value / maxRequests) * 100 : 0;
        return (
          <div className="table-cell-progress">
            <span className="table-cell-progress__value">{formatInteger(value)}</span>
            <div className="table-cell-progress__bar-bg">
              <div className="table-cell-progress__bar" style={{ width: `${pct}%`, background: '#e6f4ff', borderRight: '2px solid #1677ff' }} />
            </div>
          </div>
        );
      }
    },
    {
      title: '失败数',
      dataIndex: 'failed_count',
      key: 'failed_count',
      align: 'right',
      render: (value: number) => {
        if (value === 0) return <span style={{ color: 'var(--ant-color-text-secondary)', opacity: 0.6 }}>0</span>;
        return <span style={{ color: '#ff4d4f', fontWeight: 600 }}>{formatInteger(value)}</span>;
      }
    },
    {
      title: '平均耗时',
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
            <span className="table-cell-progress__value">{formatInteger(value)}</span>
            <div className="table-cell-progress__bar-bg">
              <div className="table-cell-progress__bar" style={{ width: `${pct}%`, background: '#f9f0ff', borderRight: '2px solid #722ed1' }} />
            </div>
          </div>
        );
      }
    }
  ];
}

const runRankColumns: ColumnsType<ApplicationRunMonitoringRunRank> = [
  {
    title: '运行',
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
    title: '状态',
    dataIndex: 'status',
    key: 'status'
  },
  {
    title: '耗时',
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
    title: '开始时间',
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
  title: string;
}) {
  return (
    <section className="application-monitoring-panel">
      <Typography.Title level={5}>{title}</Typography.Title>
      {children}
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
        new Date(point.bucket_start).toLocaleDateString()
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
        name: '运行数',
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
        name: '请求数',
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
  const totalRequests = report.sources.reduce((sum, item) => sum + item.request_count, 0);
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
      subtext: '总请求数',
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
        name: '来源',
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
      metricType === 'duration' ? r.duration_ms ?? 0 : r.total_tokens ?? 0
    )
  );

  return (
    <div className="run-rank-list">
      {runs.map((run, index) => {
        const val =
          metricType === 'duration' ? run.duration_ms ?? 0 : run.total_tokens ?? 0;
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
                <span className="run-rank-item__time">{formatTime(run.started_at)}</span>
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
        ? Math.max(1, ...report.authorized_accounts.map((item) => item.total_tokens))
        : 1,
    [report?.authorized_accounts]
  );

  const maxExtUserRequests = useMemo(
    () =>
      report
        ? Math.max(1, ...report.external_users.map((item) => item.request_count))
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
      ?.label ?? '过去 7 天';
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

  if (reportQuery.isPending) {
    return <LoadingState compact />;
  }

  if (reportQuery.isError || !report) {
    return <Result status="error" title="监控报表加载失败" />;
  }

  return (
    <div
      className="application-monitoring-page"
      data-testid="application-monitoring-page"
    >
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
              ? '正在刷新'
              : `当前范围：${activeRangeLabel}`}
          </Typography.Text>
          {!report.overview.running_count_included ? (
            <Tooltip title="仅统计运行已经结束的任务。">
              <Button
                aria-label="运行统计口径"
                className="application-monitoring-page__scope-help"
                icon={<QuestionCircleOutlined aria-hidden="true" />}
                size="small"
                type="text"
              />
            </Tooltip>
          ) : null}
          <Button
            aria-label="刷新监控报表"
            icon={<ReloadOutlined aria-hidden="true" />}
            loading={reportQuery.isFetching}
            onClick={() => {
              void reportQuery.refetch();
            }}
          />
        </Space>
      </div>

      <section className="application-monitoring-page__metrics">
        <div className="application-monitoring-metric application-monitoring-metric--blue">
          <div className="metric-card__icon-wrapper">
            <DashboardOutlined />
          </div>
          <div className="metric-card__content">
            <span className="metric-card__title">运行总数</span>
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
            <span className="metric-card__title">成功率</span>
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
            <span className="metric-card__title">失败数</span>
            <span className="metric-card__value">
              {formatInteger(report.overview.failed_count)}
            </span>
          </div>
        </div>

        <div className="application-monitoring-metric application-monitoring-metric--gold">
          <div className="metric-card__icon-wrapper">
            <ClockCircleOutlined />
          </div>
          <div className="metric-card__content">
            <span className="metric-card__title">慢请求率</span>
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
            <span className="metric-card__title">P95 耗时</span>
            <span className="metric-card__value">
              {formatDuration(report.duration.p95_duration_ms)}
            </span>
          </div>
        </div>

        <div className="application-monitoring-metric application-monitoring-metric--purple">
          <div className="metric-card__icon-wrapper">
            <DatabaseOutlined />
          </div>
          <div className="metric-card__content">
            <span className="metric-card__title">Token 总量</span>
            <span className="metric-card__value">
              {formatInteger(report.tokens.total_tokens_sum)}
            </span>
          </div>
        </div>

        <div className="application-monitoring-metric application-monitoring-metric--orange">
          <div className="metric-card__icon-wrapper">
            <ApiOutlined />
          </div>
          <div className="metric-card__content">
            <span className="metric-card__title">工具回调</span>
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
            <span className="metric-card__title">峰值并发</span>
            <span className="metric-card__value">
              {formatInteger(report.concurrency.peak_concurrency)}
            </span>
          </div>
        </div>
      </section>

      <div className="application-monitoring-page__chart-grid">
        <MonitoringPanel title="Token 趋势">
          {tokenTrendOption ? (
            <ApplicationMonitoringChart
              ariaLabel="Token trend chart"
              option={tokenTrendOption}
            />
          ) : null}
        </MonitoringPanel>
        <MonitoringPanel title="协议分布">
          {protocolOption ? (
            <ApplicationMonitoringChart
              ariaLabel="Protocol distribution chart"
              option={protocolOption}
            />
          ) : null}
        </MonitoringPanel>
        <MonitoringPanel title="来源分布">
          {sourceOption ? (
            <ApplicationMonitoringChart
              ariaLabel="Source distribution chart"
              option={sourceOption}
            />
          ) : null}
        </MonitoringPanel>
      </div>

      <div className="application-monitoring-page__table-grid">
        <MonitoringPanel title="耗时质量">
          <div className="application-monitoring-page__quality-grid">
            <div className="quality-metric-item">
              <span className="quality-metric-item__label">
                <ClockCircleOutlined /> 平均耗时
              </span>
              <span className="quality-metric-item__value">
                {formatDuration(report.duration.avg_duration_ms)}
              </span>
            </div>
            <div className="quality-metric-item">
              <span className="quality-metric-item__label">
                <DashboardOutlined /> P50 耗时
              </span>
              <span className="quality-metric-item__value">
                {formatDuration(report.duration.p50_duration_ms)}
              </span>
            </div>
            <div className="quality-metric-item">
              <span className="quality-metric-item__label">
                <NodeIndexOutlined /> 平均真实节点数
              </span>
              <span className="quality-metric-item__value">
                {formatDecimal(report.nodes.avg_unique_node_count, 1)}
              </span>
            </div>
            <div className="quality-metric-item">
              <span className="quality-metric-item__label">
                <ApiOutlined /> 平均工具回调
              </span>
              <span className="quality-metric-item__value">
                {formatDecimal(report.tool_callbacks.avg_tool_callback_count, 1)}
              </span>
            </div>
          </div>
        </MonitoringPanel>
        <MonitoringPanel title="协议明细">
          <MonitoringTable
            columns={protocolColumns}
            dataSource={report.protocols}
            rowKey="protocol"
          />
        </MonitoringPanel>
        <MonitoringPanel title="来源明细">
          <MonitoringTable
            columns={sourceColumns}
            dataSource={report.sources}
            rowKey="source"
          />
        </MonitoringPanel>
        <MonitoringPanel title="授权账号">
          <MonitoringTable<ApplicationRunMonitoringAuthorizedAccountUsage>
            columns={usageColumns(
              '账号',
              'authorized_account',
              maxAuthRequests,
              maxAuthTokens
            )}
            dataSource={report.authorized_accounts}
            rowKey={(record) => record.authorized_account ?? 'unknown'}
          />
        </MonitoringPanel>
        <MonitoringPanel title="外部用户">
          <MonitoringTable<ApplicationRunMonitoringExternalUserUsage>
            columns={usageColumns(
              '外部用户',
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
              maxApiKeyTokens
            )}
            dataSource={report.api_keys}
            rowKey="api_key_id"
          />
        </MonitoringPanel>
        <MonitoringPanel title="外部会话">
          <MonitoringTable<ApplicationRunMonitoringExternalConversationUsage>
            columns={usageColumns(
              '会话',
              'external_conversation_id',
              maxExtConvRequests,
              maxExtConvTokens
            )}
            dataSource={report.external_conversations}
            rowKey={(record) => record.external_conversation_id ?? 'unknown'}
          />
        </MonitoringPanel>
        <MonitoringPanel title="最慢运行 Top 10">
          <RunRankList runs={report.slowest_runs} metricType="duration" />
        </MonitoringPanel>
        <MonitoringPanel title="高 Token 运行 Top 10">
          <RunRankList runs={report.high_token_runs} metricType="token" />
        </MonitoringPanel>
      </div>
    </div>
  );
}
