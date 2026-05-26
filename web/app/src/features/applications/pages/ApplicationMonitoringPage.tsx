import {
  ApiOutlined,
  ClockCircleOutlined,
  DashboardOutlined,
  NodeIndexOutlined,
  ReloadOutlined
} from '@ant-design/icons';
import { useQuery } from '@tanstack/react-query';
import {
  Alert,
  Button,
  Empty,
  Radio,
  Result,
  Space,
  Statistic,
  Table,
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
      render: (value: number) => formatPercent(value)
    },
    {
      title: '平均耗时',
      dataIndex: 'avg_duration_ms',
      key: 'avg_duration_ms',
      align: 'right',
      render: (value: number) => formatDuration(value)
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
    render: (value: number) => formatPercent(value)
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
>(label: string, key: keyof T): ColumnsType<T> {
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
      render: (value: number) => formatInteger(value)
    },
    {
      title: '失败数',
      dataIndex: 'failed_count',
      key: 'failed_count',
      align: 'right',
      render: (value: number) => formatInteger(value)
    },
    {
      title: '平均耗时',
      dataIndex: 'avg_duration_ms',
      key: 'avg_duration_ms',
      align: 'right',
      render: (value: number) => formatDuration(value)
    },
    {
      title: 'Tokens',
      dataIndex: 'total_tokens',
      key: 'total_tokens',
      align: 'right',
      render: (value: number) => formatInteger(value)
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
    color: ['#1677ff', '#00a36c'],
    grid: {
      left: 44,
      right: 20,
      top: 28,
      bottom: 34
    },
    tooltip: {
      trigger: 'axis'
    },
    xAxis: {
      type: 'category',
      data: report.tokens_trend.map((point) =>
        new Date(point.bucket_start).toLocaleDateString()
      )
    },
    yAxis: {
      type: 'value'
    },
    series: [
      {
        name: 'Tokens',
        type: 'line',
        smooth: true,
        symbolSize: 6,
        data: report.tokens_trend.map((point) => point.total_tokens)
      },
      {
        name: '运行数',
        type: 'bar',
        yAxisIndex: 0,
        data: report.tokens_trend.map((point) => point.run_count)
      }
    ]
  };
}

function buildProtocolOption(report: ApplicationRunMonitoringReport) {
  return {
    color: ['#1677ff'],
    grid: {
      left: 44,
      right: 20,
      top: 20,
      bottom: 38
    },
    tooltip: {
      trigger: 'axis'
    },
    xAxis: {
      type: 'category',
      data: report.protocols.map((item) => item.protocol)
    },
    yAxis: {
      type: 'value'
    },
    series: [
      {
        name: '请求数',
        type: 'bar',
        data: report.protocols.map((item) => item.request_count)
      }
    ]
  };
}

function buildSourceOption(report: ApplicationRunMonitoringReport) {
  return {
    color: ['#1677ff', '#00a36c', '#faad14'],
    tooltip: {
      trigger: 'item'
    },
    legend: {
      bottom: 0
    },
    series: [
      {
        name: '来源',
        type: 'pie',
        radius: ['42%', '68%'],
        center: ['50%', '44%'],
        data: report.sources.map((item) => ({
          name: sourceLabel(item.source),
          value: item.request_count
        }))
      }
    ]
  };
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

      {!report.overview.running_count_included ? (
        <Alert showIcon message="运行中数未包含" type="info" />
      ) : null}

      <section className="application-monitoring-page__metrics">
        <div className="application-monitoring-metric">
          <Statistic
            title="运行总数"
            value={statisticValue(report.overview.total_count)}
          />
        </div>
        <div className="application-monitoring-metric">
          <Statistic
            title="成功率"
            value={statisticValue(formatPercent(report.overview.success_rate))}
          />
        </div>
        <div className="application-monitoring-metric">
          <Statistic
            title="失败数"
            value={statisticValue(report.overview.failed_count)}
          />
        </div>
        <div className="application-monitoring-metric">
          <Statistic
            title="慢请求率"
            value={statisticValue(formatPercent(report.duration.slow_run_rate))}
          />
        </div>
        <div className="application-monitoring-metric">
          <Statistic
            title="P95 耗时"
            value={statisticValue(
              formatDuration(report.duration.p95_duration_ms)
            )}
          />
        </div>
        <div className="application-monitoring-metric">
          <Statistic
            title="Token 总量"
            value={statisticValue(
              formatInteger(report.tokens.total_tokens_sum)
            )}
          />
        </div>
        <div className="application-monitoring-metric">
          <Statistic
            title="工具回调"
            value={statisticValue(
              report.tool_callbacks.total_tool_callback_count
            )}
          />
        </div>
        <div className="application-monitoring-metric">
          <Statistic
            title="峰值并发"
            value={statisticValue(report.concurrency.peak_concurrency)}
          />
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
            <span>
              <ClockCircleOutlined /> 平均耗时
            </span>
            <strong>{formatDuration(report.duration.avg_duration_ms)}</strong>
            <span>
              <DashboardOutlined /> P50 耗时
            </span>
            <strong>{formatDuration(report.duration.p50_duration_ms)}</strong>
            <span>
              <NodeIndexOutlined /> 平均真实节点数
            </span>
            <strong>
              {formatDecimal(report.nodes.avg_unique_node_count, 1)}
            </strong>
            <span>
              <ApiOutlined /> 平均工具回调
            </span>
            <strong>
              {formatDecimal(report.tool_callbacks.avg_tool_callback_count, 1)}
            </strong>
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
            columns={usageColumns('账号', 'authorized_account')}
            dataSource={report.authorized_accounts}
            rowKey={(record) => record.authorized_account ?? 'unknown'}
          />
        </MonitoringPanel>
        <MonitoringPanel title="外部用户">
          <MonitoringTable<ApplicationRunMonitoringExternalUserUsage>
            columns={usageColumns('外部用户', 'external_user')}
            dataSource={report.external_users}
            rowKey={(record) => record.external_user ?? 'unknown'}
          />
        </MonitoringPanel>
        <MonitoringPanel title="API Key">
          <MonitoringTable<ApplicationRunMonitoringApiKeyUsage>
            columns={usageColumns('API Key', 'api_key_id')}
            dataSource={report.api_keys}
            rowKey="api_key_id"
          />
        </MonitoringPanel>
        <MonitoringPanel title="外部会话">
          <MonitoringTable<ApplicationRunMonitoringExternalConversationUsage>
            columns={usageColumns('会话', 'external_conversation_id')}
            dataSource={report.external_conversations}
            rowKey={(record) => record.external_conversation_id ?? 'unknown'}
          />
        </MonitoringPanel>
        <MonitoringPanel title="最慢运行 Top 10">
          <MonitoringTable
            columns={runRankColumns}
            dataSource={report.slowest_runs}
            rowKey="flow_run_id"
          />
        </MonitoringPanel>
        <MonitoringPanel title="高 Token 运行 Top 10">
          <MonitoringTable
            columns={runRankColumns}
            dataSource={report.high_token_runs}
            rowKey="flow_run_id"
          />
        </MonitoringPanel>
      </div>
    </div>
  );
}
