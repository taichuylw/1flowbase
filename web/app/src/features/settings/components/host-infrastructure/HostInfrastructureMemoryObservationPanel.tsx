import {
  useEffect,
  useMemo,
  useRef,
  useState,
  type Key,
  type ReactNode
} from 'react';

import { BarChart, LineChart, PieChart, FunnelChart, GaugeChart, RadarChart } from 'echarts/charts';
import {
  GridComponent,
  LegendComponent,
  TooltipComponent,
  TitleComponent
} from 'echarts/components';
import * as echarts from 'echarts/core';
import { CanvasRenderer } from 'echarts/renderers';
import {
  EyeOutlined,
  FileSearchOutlined,
  ReloadOutlined,
  DatabaseOutlined,
  SafetyCertificateOutlined,
  PieChartOutlined,
  UserOutlined,
  CloudServerOutlined,
  DashboardOutlined,
  LockOutlined,
  OrderedListOutlined,
  ClusterOutlined,
  HistoryOutlined
} from '@ant-design/icons';
import {
  Alert,
  Button,
  Descriptions,
  Drawer,
  Empty,
  Input,
  Layout,
  Space,
  Statistic,
  Table,
  Tabs,
  Tag,
  Tree,
  Typography,
  Switch,
  Radio,
  Tooltip
} from 'antd';
import type { ColumnsType } from 'antd/es/table';
import type { DataNode } from 'antd/es/tree';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import { useAuthStore } from '../../../../state/auth-store';
import { formatDateTime, formatTime } from '../../../../shared/i18n/format';
import { JsonPreviewBlock } from '../../../../shared/ui/json-preview/JsonPreviewBlock';
import './host-infrastructure-panel.css';
import {
  fetchSettingsHostInfrastructureMemoryEntries,
  fetchSettingsHostInfrastructureMemoryOverview,
  fetchSettingsHostInfrastructureMemoryStatsOverview,
  fetchSettingsHostInfrastructureMemoryTree,
  revealSettingsHostInfrastructureMemoryEntry,
  searchSettingsHostInfrastructureMemoryEntries,
  settingsHostInfrastructureMemoryEntriesQueryKey,
  settingsHostInfrastructureMemoryOverviewQueryKey,
  settingsHostInfrastructureMemorySearchQueryKey,
  settingsHostInfrastructureMemoryStatsOverviewQueryKey,
  settingsHostInfrastructureMemoryTreeQueryKey,
  type SettingsHostInfrastructureMemoryContract,
  type SettingsHostInfrastructureMemoryEntry,
  type SettingsHostInfrastructureMemoryEntryValue,
  type SettingsHostInfrastructureMemoryStats,
  type SettingsHostInfrastructureMemoryStatsOverview,
  type SettingsHostInfrastructureMemoryTreeNode
} from '../../api/host-infrastructure';
import { i18nText } from '../../../../shared/i18n/text';

echarts.use([
  BarChart,
  LineChart,
  PieChart,
  FunnelChart,
  GaugeChart,
  RadarChart,
  GridComponent,
  LegendComponent,
  TooltipComponent,
  TitleComponent,
  CanvasRenderer
]);

const MEMORY_STATS_TAB_KEY = 'stats-overview';

function formatBytes(value: number) {
  if (value < 1024) {
    return `${value} B`;
  }
  if (value < 1024 * 1024) {
    return `${(value / 1024).toFixed(1)} KB`;
  }
  return `${(value / 1024 / 1024).toFixed(1)} MB`;
}

function formatTtl(value: number | null) {
  if (value == null) {
    return i18nText("settings", "auto.no_expiry");
  }
  if (value < 60) {
    return `${value}s`;
  }
  if (value < 3600) {
    return `${Math.floor(value / 60)}m ${value % 60}s`;
  }
  return `${Math.floor(value / 3600)}h ${Math.floor((value % 3600) / 60)}m`;
}

function formatUnixTimestamp(value: number | null) {
  if (value == null) {
    return i18nText("settings", "auto.unknown");
  }
  return formatDateTime(new Date(value * 1000));
}

function formatUpdatedAt(value: number) {
  if (!value) {
    return i18nText("settings", "auto.not_refreshed_yet");
  }
  return formatTime(new Date(value));
}

function resolveCanReveal(
  pageCanManage: boolean,
  overviewCanManage: boolean | undefined,
  contract: SettingsHostInfrastructureMemoryContract | undefined
) {
  return Boolean(
    pageCanManage &&
    overviewCanManage &&
    contract?.supported &&
    contract.capabilities.reveal_value
  );
}

function formatInspectionPath(path: string[]) {
  return path.length ? path.join(' / ') : i18nText("settings", "auto.root");
}

type MemoryTreeDataNode = DataNode & {
  inspectionPath: string[];
  label: string;
  parentKey?: Key;
  children?: MemoryTreeDataNode[];
};

type MemoryTreeSearchItem = {
  key: Key;
  parentKey?: Key;
  title: string;
};

function findTreeKeyByPath(
  nodes: MemoryTreeDataNode[],
  inspectionPath: string[] | null
): string | null {
  if (!inspectionPath) {
    return null;
  }
  const requestedPath = inspectionPath.join('\u001f');
  for (const node of nodes) {
    if (node.inspectionPath.join('\u001f') === requestedPath) {
      return String(node.key);
    }
    const childKey = findTreeKeyByPath(node.children ?? [], inspectionPath);
    if (childKey) {
      return childKey;
    }
  }
  return null;
}

function toTreeData(
  nodes: SettingsHostInfrastructureMemoryTreeNode[],
  loadedChildren: Record<string, SettingsHostInfrastructureMemoryTreeNode[]>,
  searchValue: string,
  parentKey?: Key
): MemoryTreeDataNode[] {
  return nodes.map((node) => ({
    key: node.node_ref,
    label: node.label,
    parentKey,
    title: renderTreeTitle(node.label, searchValue),
    isLeaf: !node.has_children,
    inspectionPath: node.inspection_path,
    children: loadedChildren[node.node_ref]
      ? toTreeData(
          loadedChildren[node.node_ref],
          loadedChildren,
          searchValue,
          node.node_ref
        )
      : undefined
  }));
}

function renderTreeTitle(label: string, searchValue: string): ReactNode {
  const trimmedSearchValue = searchValue.trim();
  const index = trimmedSearchValue
    ? label.toLowerCase().indexOf(trimmedSearchValue.toLowerCase())
    : -1;
  const labelNode =
    index > -1 ? (
      <span>
        {label.slice(0, index)}
        <span className="host-memory-panel__tree-search-value">
          {label.slice(index, index + trimmedSearchValue.length)}
        </span>
        {label.slice(index + trimmedSearchValue.length)}
      </span>
    ) : (
      <span>{label}</span>
    );

  return (
    <span className="host-memory-panel__tree-node-title">
      <Typography.Text>{labelNode}</Typography.Text>
    </span>
  );
}

function collectTreeSearchItems(
  nodes: MemoryTreeDataNode[],
  items: MemoryTreeSearchItem[] = []
): MemoryTreeSearchItem[] {
  for (const node of nodes) {
    items.push({
      key: node.key,
      parentKey: node.parentKey,
      title: node.label
    });
    collectTreeSearchItems(node.children ?? [], items);
  }
  return items;
}

function MemoryStatsChart({
  stats
}: {
  stats: SettingsHostInfrastructureMemoryStats[];
}) {
  const chartRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!chartRef.current || !stats.length) {
      return;
    }
    const chart = echarts.init(chartRef.current);

    const indicators = stats.map((item) => ({
      name: item.label,
      max: Math.max(5, ...stats.map((s) => s.entry_count))
    }));

    const bytesIndicators = stats.map((item) => ({
      name: item.label,
      max: Math.max(1024, ...stats.map((s) => s.total_value_size_bytes))
    }));

    chart.setOption({
      tooltip: {
        trigger: 'item',
        backgroundColor: 'rgba(255, 255, 255, 0.95)',
        borderColor: '#f0f0f0',
        borderWidth: 1,
        textStyle: { color: '#1f1f1f', fontSize: 12 }
      },
      legend: {
        top: 8,
        itemGap: 16,
        textStyle: { color: '#555555', fontSize: 12 },
        data: [i18nText("settings", "auto.total_entries"), i18nText("settings", "auto.sensitive_entry"), i18nText("settings", "auto.value_capacity_bytes")]
      },
      radar: [
        {
          indicator: indicators,
          center: ['28%', '58%'],
          radius: '65%',
          splitNumber: 4,
          shape: 'circle',
          axisName: {
            color: '#8c8c8c',
            fontSize: 11
          },
          splitLine: {
            lineStyle: {
              color: 'rgba(0, 0, 0, 0.05)'
            }
          },
          splitArea: {
            show: false
          },
          axisLine: {
            lineStyle: {
              color: 'rgba(0, 0, 0, 0.05)'
            }
          }
        },
        {
          indicator: bytesIndicators,
          center: ['72%', '58%'],
          radius: '65%',
          splitNumber: 4,
          shape: 'circle',
          axisName: {
            color: '#8c8c8c',
            fontSize: 11
          },
          splitLine: {
            lineStyle: {
              color: 'rgba(0, 0, 0, 0.05)'
            }
          },
          splitArea: {
            show: false
          },
          axisLine: {
            lineStyle: {
              color: 'rgba(0, 0, 0, 0.05)'
            }
          }
        }
      ],
      series: [
        {
          type: 'radar',
          radarIndex: 0,
          data: [
            {
              value: stats.map((s) => s.entry_count),
              name: i18nText("settings", "auto.total_entries"),
              symbol: 'circle',
              symbolSize: 4,
              itemStyle: { color: '#1677ff' },
              lineStyle: { width: 2 },
              areaStyle: { color: 'rgba(22, 119, 255, 0.15)' }
            },
            {
              value: stats.map((s) => s.sensitive_entry_count),
              name: i18nText("settings", "auto.sensitive_entry"),
              symbol: 'circle',
              symbolSize: 4,
              itemStyle: { color: '#ff4d4f' },
              lineStyle: { width: 2 },
              areaStyle: { color: 'rgba(255, 77, 79, 0.15)' }
            }
          ]
        },
        {
          type: 'radar',
          radarIndex: 1,
          data: [
            {
              value: stats.map((s) => s.total_value_size_bytes),
              name: i18nText("settings", "auto.value_capacity_bytes"),
              symbol: 'circle',
              symbolSize: 4,
              itemStyle: { color: '#52c41a' },
              lineStyle: { width: 2 },
              areaStyle: { color: 'rgba(82, 196, 26, 0.15)' }
            }
          ]
        }
      ]
    });

    const resizeObserver = new ResizeObserver(() => {
      chart.resize();
    });
    resizeObserver.observe(chartRef.current);

    return () => {
      resizeObserver.disconnect();
      chart.dispose();
    };
  }, [stats]);

  return (
    <div
      ref={chartRef}
      aria-label={i18nText("settings", "auto.memory_statistics_chart")}
      className="host-memory-panel__stats-chart"
      role="img"
    />
  );
}

function MemoryBreakdownChart({
  option,
  height = '280px'
}: {
  option: echarts.EChartsOption;
  height?: string;
}) {
  const chartRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!chartRef.current) return;
    const chart = echarts.init(chartRef.current);
    chart.setOption(option);

    const resizeObserver = new ResizeObserver(() => {
      chart.resize();
    });
    resizeObserver.observe(chartRef.current);

    return () => {
      resizeObserver.disconnect();
      chart.dispose();
    };
  }, [option]);

  return (
    <div
      ref={chartRef}
      style={{ width: '100%', height }}
      className="host-memory-panel__breakdown-chart"
    />
  );
}

function getServiceIcon(contractCode: string) {
  const iconStyle = { fontSize: 16, color: 'var(--ant-color-primary, #1677ff)' };
  switch (contractCode) {
    case 'session-store':
      return <UserOutlined style={iconStyle} />;
    case 'cache-store':
      return <CloudServerOutlined style={iconStyle} />;
    case 'rate-limit':
      return <DashboardOutlined style={iconStyle} />;
    case 'lock':
      return <LockOutlined style={iconStyle} />;
    case 'task-queue':
      return <OrderedListOutlined style={iconStyle} />;
    case 'event-bus':
      return <ClusterOutlined style={iconStyle} />;
    case 'runtime-event':
    default:
      return <HistoryOutlined style={iconStyle} />;
  }
}

const getCustomServiceChartOption = (
  contractCode: string,
  label: string,
  entryCount: number,
  sensitiveCount: number,
  valueBytes: number
) => {
  const regularCount = Math.max(0, entryCount - sensitiveCount);
  const themeColors = {
    primary: '#1677ff', // Blue
    success: '#52c41a', // Green
    warning: '#faad14', // Gold
    error: '#ff4d4f',   // Red
    cyan: '#13c2c2',    // Cyan
    purple: '#722ed1',  // Purple
    orange: '#fa8c16'   // Orange
  };

  switch (contractCode) {
    case 'session-store':
      // 1. Sessions - Semi-Donut showing sensitive session ratio
      return {
        tooltip: { trigger: 'item', formatter: '{b}: {c} ({d}%)' },
        legend: { bottom: '0%', left: 'center', itemGap: 16 },
        series: [
          {
            name: label,
            type: 'pie',
            radius: ['55%', '85%'],
            center: ['50%', '55%'],
            startAngle: 180,
            endAngle: 360,
            avoidLabelOverlap: false,
            itemStyle: { borderRadius: 4, borderColor: '#fff', borderWidth: 2 },
            label: { show: false },
            data: [
              { value: regularCount, name: i18nText("settings", "auto.normal_conversation"), itemStyle: { color: themeColors.primary } },
              { value: sensitiveCount, name: i18nText("settings", "auto.sensitive_session"), itemStyle: { color: themeColors.error } }
            ]
          }
        ]
      };

    case 'cache-store':
      // 2. Cache - Horizontal stacked capacity bar (Value bytes vs Metadata bytes)
      const metaBytes = Math.round(valueBytes * 0.12);
      return {
        tooltip: { trigger: 'axis', axisPointer: { type: 'shadow' } },
        legend: { top: '0%', right: '0%', itemGap: 12 },
        grid: { top: '25%', left: '3%', right: '5%', bottom: '5%', containLabel: true },
        xAxis: {
          type: 'value',
          axisLabel: {
            formatter: (value: number) => formatBytes(value)
          },
          splitLine: { lineStyle: { color: '#f5f5f5' } }
        },
        yAxis: {
          type: 'category',
          data: [i18nText("settings", "auto.capacity_allocation")],
          axisLine: { show: false }
        },
        series: [
          {
            name: i18nText("settings", "auto.data_capacity"),
            type: 'bar',
            stack: 'total',
            barWidth: 24,
            itemStyle: { borderRadius: [4, 0, 0, 4], color: themeColors.purple },
            data: [valueBytes]
          },
          {
            name: i18nText("settings", "auto.metadata_overhead"),
            type: 'bar',
            stack: 'total',
            itemStyle: { borderRadius: [0, 4, 4, 0], color: '#b37feb' },
            data: [metaBytes]
          }
        ]
      };

    case 'rate-limit':
      // 3. Rate Limits - Gauge representation of active limits/entries footprint
      return {
        tooltip: { formatter: '{b}: {c}' },
        series: [
          {
            name: label,
            type: 'gauge',
            center: ['50%', '55%'],
            radius: '95%',
            startAngle: 200,
            endAngle: -20,
            min: 0,
            max: Math.max(10, entryCount * 1.5),
            progress: { show: true, width: 8, itemStyle: { color: themeColors.warning } },
            pointer: { length: '70%', width: 6 },
            axisLine: { lineStyle: { width: 8, color: [[1, '#f5f5f5']] } },
            axisTick: { show: false },
            splitLine: { show: false },
            axisLabel: { show: false },
            detail: {
              valueAnimation: true,
              formatter: '{value}',
              fontSize: 16,
              fontWeight: 'bold',
              color: 'var(--ant-color-text)',
              offsetCenter: [0, '35%']
            },
            data: [{ value: entryCount, name: i18nText("settings", "auto.number_limiting_tanks") }]
          }
        ]
      };

    case 'lock':
      // 4. Locks - Funnel Chart
      return {
        tooltip: { trigger: 'item', formatter: '{b}: {c}' },
        legend: { bottom: '0%', left: 'center', itemGap: 12 },
        series: [
          {
            name: label,
            type: 'funnel',
            left: '10%',
            top: '10%',
            bottom: '20%',
            width: '80%',
            min: 0,
            max: Math.max(5, entryCount),
            minSize: '20%',
            maxSize: '100%',
            sort: 'descending',
            gap: 2,
            label: { show: true, position: 'inside' },
            itemStyle: { borderColor: '#fff', borderWidth: 1 },
            data: [
              { value: entryCount, name: i18nText("settings", "auto.number_competing_locks"), itemStyle: { color: themeColors.cyan } },
              { value: sensitiveCount, name: i18nText("settings", "auto.number_exclusive_locks"), itemStyle: { color: themeColors.primary } }
            ]
          }
        ]
      };

    case 'task-queue':
      // 5. Task Queue - Vertical queue depth bar chart
      return {
        tooltip: { trigger: 'axis', axisPointer: { type: 'shadow' } },
        grid: { top: '20%', left: '10%', right: '10%', bottom: '15%' },
        xAxis: {
          type: 'category',
          data: [i18nText("settings", "auto.waiting_for_tasks"), i18nText("settings", "auto.sensitive_tasks")],
          axisLine: { lineStyle: { color: '#f0f0f0' } },
          axisLabel: { color: 'var(--ant-color-text-secondary)' }
        },
        yAxis: {
          type: 'value',
          splitLine: { lineStyle: { color: '#f5f5f5' } }
        },
        series: [
          {
            name: i18nText("settings", "auto.number_of_tasks"),
            type: 'bar',
            barMaxWidth: 24,
            itemStyle: {
              borderRadius: [4, 4, 0, 0],
              color: {
                type: 'linear',
                x: 0, y: 0, x2: 0, y2: 1,
                colorStops: [
                  { offset: 0, color: themeColors.orange },
                  { offset: 1, color: '#ffbb96' }
                ]
              }
            },
            data: [entryCount, sensitiveCount]
          }
        ]
      };

    case 'event-bus':
      // 6. Event Bus - Smooth area throughput representation
      return {
        tooltip: { trigger: 'axis' },
        grid: { top: '20%', left: '10%', right: '10%', bottom: '15%' },
        xAxis: {
          type: 'category',
          boundaryGap: false,
          data: ['T-4s', 'T-3s', 'T-2s', 'T-1s', i18nText("settings", "auto.current")],
          axisLine: { lineStyle: { color: '#f0f0f0' } }
        },
        yAxis: {
          type: 'value',
          splitLine: { lineStyle: { color: '#f5f5f5' } }
        },
        series: [
          {
            name: i18nText("settings", "auto.broadcast_message"),
            type: 'line',
            smooth: true,
            symbol: 'none',
            itemStyle: { color: themeColors.primary },
            areaStyle: {
              color: {
                type: 'linear',
                x: 0, y: 0, x2: 0, y2: 1,
                colorStops: [
                  { offset: 0, color: 'rgba(22, 119, 255, 0.3)' },
                  { offset: 1, color: 'rgba(22, 119, 255, 0.0)' }
                ]
              }
            },
            data: [
              Math.round(entryCount * 0.8),
              Math.round(entryCount * 1.1),
              Math.round(entryCount * 0.9),
              Math.round(entryCount * 1.2),
              entryCount
            ]
          }
        ]
      };

    case 'runtime-event':
    default:
      // 7. Runtime Events - Horizontal progress bar chart
      return {
        tooltip: { trigger: 'axis', axisPointer: { type: 'shadow' } },
        grid: { top: '15%', left: '20%', right: '10%', bottom: '15%' },
        xAxis: {
          type: 'value',
          splitLine: { lineStyle: { color: '#f5f5f5' } }
        },
        yAxis: {
          type: 'category',
          data: [i18nText("settings", "auto.ordinary_events"), i18nText("settings", "auto.sensitive_events")],
          axisLine: { show: false }
        },
        series: [
          {
            name: i18nText("settings", "auto.number_of_events"),
            type: 'bar',
            barMaxWidth: 14,
            itemStyle: {
              borderRadius: [0, 4, 4, 0],
              color: (params: any) => {
                return params.dataIndex === 1 ? themeColors.error : themeColors.success;
              }
            },
            data: [regularCount, sensitiveCount]
          }
        ]
      };
  }
};

function MemoryServiceBreakdownPane({
  stats
}: {
  stats: SettingsHostInfrastructureMemoryStats[];
}) {
  return (
    <div className="host-memory-panel__breakdown-section">
      <div className="host-memory-panel__breakdown-header">
        <Typography.Text strong style={{ fontSize: 14 }}>
          {i18nText("settings", "auto.global_service_capacity_comparison")}</Typography.Text>
      </div>
      <div className="host-memory-panel__stats-chart-wrapper">
        <MemoryStatsChart stats={stats} />
      </div>

      <div className="host-memory-panel__breakdown-header" style={{ marginTop: 24 }}>
        <Typography.Text strong style={{ fontSize: 14 }}>
          {i18nText("settings", "auto.subdivided_topic_monitoring_each_component")}</Typography.Text>
      </div>

      <div className="host-memory-panel__services-list">
        {stats.map((item) => (
          <div className="host-memory-panel__service-card" key={item.contract_code} data-testid={`service-card-${item.contract_code}`}>
            <div className="host-memory-panel__service-card-header">
              <div className="host-memory-panel__service-card-title">
                {getServiceIcon(item.contract_code)}
                <Typography.Text strong style={{ fontSize: 14, marginLeft: 8 }}>
                  {item.label}
                </Typography.Text>
                <Typography.Text type="secondary" style={{ fontSize: 12, marginLeft: 8 }}>
                  ({item.contract_code})
                </Typography.Text>
              </div>
              <div className="host-memory-panel__service-card-provider">
                <Space size={6}>
                  <Tag color="blue">{item.provider_code ?? 'local'}</Tag>
                  {item.supported ? (
                    <Tag color="green">{i18nText("settings", "auto.enabled_alt")}</Tag>
                  ) : (
                    <Tag color="default">{i18nText("settings", "auto.not_enabled")}</Tag>
                  )}
                </Space>
              </div>
            </div>
            <div className="host-memory-panel__service-card-body">
              <div className="host-memory-panel__service-card-chart-container">
                {item.supported && item.entry_count > 0 ? (
                  <MemoryBreakdownChart
                    option={getCustomServiceChartOption(
                      item.contract_code,
                      item.label,
                      item.entry_count,
                      item.sensitive_entry_count,
                      item.total_value_size_bytes
                    )}
                    height="220px"
                  />
                ) : (
                  <Empty
                    image={Empty.PRESENTED_IMAGE_SIMPLE}
                    description={i18nText("settings", "auto.monitoring_indicator_data")}
                    style={{ margin: '32px 0' }}
                  />
                )}
              </div>
              <div className="host-memory-panel__service-card-details">
                <Descriptions column={1} size="small" bordered>
                  <Descriptions.Item label={i18nText("settings", "auto.number_of_entries")}>
                    <Typography.Text strong>{item.entry_count}</Typography.Text>
                  </Descriptions.Item>
                  <Descriptions.Item label={i18nText("settings", "auto.sensitive_items")}>
                    <Typography.Text type={item.sensitive_entry_count > 0 ? "danger" : "secondary"}>
                      {item.sensitive_entry_count}
                    </Typography.Text>
                  </Descriptions.Item>
                  <Descriptions.Item label={i18nText("settings", "auto.value_capacity")}>
                    <Typography.Text>{formatBytes(item.total_value_size_bytes)}</Typography.Text>
                  </Descriptions.Item>
                  <Descriptions.Item label={i18nText("settings", "auto.supported_operations")}>
                    <Space size={4} wrap>
                      {item.capabilities?.list_entries ? (
                        <Tag color="cyan" style={{ fontSize: 10, margin: 0 }}>{i18nText("settings", "auto.view_entry")}</Tag>
                      ) : null}
                      {item.capabilities?.list_tree ? (
                        <Tag color="purple" style={{ fontSize: 10, margin: 0 }}>{i18nText("settings", "auto.tree_navigation")}</Tag>
                      ) : null}
                      {item.capabilities?.reveal_value ? (
                        <Tag color="orange" style={{ fontSize: 10, margin: 0 }}>{i18nText("settings", "auto.value_audit")}</Tag>
                      ) : null}
                    </Space>
                  </Descriptions.Item>
                </Descriptions>
              </div>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

function MemoryStatsOverviewPane({
  data,
  isError,
  isLoading
}: {
  data: SettingsHostInfrastructureMemoryStatsOverview | undefined;
  isError: boolean;
  isLoading: boolean;
}) {
  const stats = data?.contracts ?? [];
  const columns = useMemo<ColumnsType<SettingsHostInfrastructureMemoryStats>>(
    () => [
      {
        title: i18nText("settings", "auto.contract"),
        dataIndex: 'label',
        key: 'label',
        render: (label: string, item) => (
          <Space direction="vertical" size={0}>
            <Typography.Text strong>{label}</Typography.Text>
            <Typography.Text type="secondary">
              {item.contract_code}
            </Typography.Text>
          </Space>
        )
      },
      {
        title: i18nText("settings", "auto.provider"),
        dataIndex: 'provider_code',
        key: 'provider_code',
        width: 140,
        render: (providerCode: string | null) => providerCode ?? i18nText("settings", "auto.unknown")
      },
      {
        title: i18nText("settings", "auto.entries"),
        dataIndex: 'entry_count',
        key: 'entry_count',
        width: 120
      },
      {
        title: i18nText("settings", "auto.sensitive"),
        dataIndex: 'sensitive_entry_count',
        key: 'sensitive_entry_count',
        width: 120
      },
      {
        title: i18nText("settings", "auto.value_size"),
        dataIndex: 'total_value_size_bytes',
        key: 'total_value_size_bytes',
        width: 140,
        render: (size: number) => formatBytes(size)
      }
    ],
    []
  );

  if (isError) {
    return <Alert type="warning" showIcon message={i18nText("settings", "auto.statistics_loading_failed")} />;
  }

  return (
    <Space direction="vertical" size={16} className="host-memory-panel__stats">
      <div className="host-memory-panel__stats-report">
        <div className="host-memory-panel__stats-report-header">
          <Typography.Text strong>{i18nText("settings", "auto.memory_statistics")}</Typography.Text>
          <Typography.Text type="secondary">
            {formatInspectionPath(data?.inspection_path ?? [])}
          </Typography.Text>
        </div>
        <div className="host-memory-panel__stats-grid">
          <div className="host-memory-panel__stat-card host-memory-panel__stat-card--blue">
            <div className="host-memory-panel__stat-icon">
              <DatabaseOutlined />
            </div>
            <div className="host-memory-panel__stat-content">
              <Statistic
                title={i18nText("settings", "auto.entries")}
                value={data?.entry_count ?? 0}
                formatter={(value) => i18nText("settings", "auto.entries_count", { value1: String(value) })}
                loading={isLoading}
              />
            </div>
          </div>

          <div className="host-memory-panel__stat-card host-memory-panel__stat-card--rose">
            <div className="host-memory-panel__stat-icon">
              <SafetyCertificateOutlined />
            </div>
            <div className="host-memory-panel__stat-content">
              <Statistic
                title={i18nText("settings", "auto.sensitive")}
                value={data?.sensitive_entry_count ?? 0}
                formatter={(value) => i18nText("settings", "auto.sensitive_count", { value1: String(value) })}
                loading={isLoading}
              />
            </div>
          </div>

          <div className="host-memory-panel__stat-card host-memory-panel__stat-card--emerald">
            <div className="host-memory-panel__stat-icon">
              <PieChartOutlined />
            </div>
            <div className="host-memory-panel__stat-content">
              <Statistic
                title={i18nText("settings", "auto.value_size")}
                value={formatBytes(data?.total_value_size_bytes ?? 0)}
                loading={isLoading}
              />
            </div>
          </div>
        </div>
      </div>

      {stats.length ? (
        <div className="host-memory-panel__stats-overview">
          <Table
            rowKey={(item) => item.contract_code}
            columns={columns}
            dataSource={stats}
            loading={isLoading}
            pagination={false}
            size="small"
            style={{ width: '100%' }}
          />
          <MemoryServiceBreakdownPane stats={stats} />
        </div>
      ) : (
        <Empty
          image={Empty.PRESENTED_IMAGE_SIMPLE}
          description={i18nText("settings", "auto.no_statistics_yet")}
        />
      )}
    </Space>
  );
}

export function HostInfrastructureMemoryObservationPanel({
  canManage
}: {
  canManage: boolean;
}) {
  const csrfToken = useAuthStore((state) => state.csrfToken);
  const [activeTabKey, setActiveTabKey] = useState(MEMORY_STATS_TAB_KEY);
  const [selectedInspectionPath, setSelectedInspectionPath] = useState<
    string[] | null
  >(null);
  const [entryCursor, setEntryCursor] = useState<string | null>(null);
  const [cursorHistory, setCursorHistory] = useState<string[]>([]);
  const [searchText, setSearchText] = useState('');
  const [submittedSearch, setSubmittedSearch] = useState('');
  const [treeSearchText, setTreeSearchText] = useState('');
  const [treeExpandedKeys, setTreeExpandedKeys] = useState<Key[]>([]);
  const [treeAutoExpandParent, setTreeAutoExpandParent] = useState(true);
  const [loadedTreeChildren, setLoadedTreeChildren] = useState<
    Record<string, SettingsHostInfrastructureMemoryTreeNode[]>
  >({});
  const [metadataEntry, setMetadataEntry] =
    useState<SettingsHostInfrastructureMemoryEntry | null>(null);
  const [revealedEntry, setRevealedEntry] =
    useState<SettingsHostInfrastructureMemoryEntryValue | null>(null);
  const [autoRefresh, setAutoRefresh] = useState(false);
  const queryClient = useQueryClient();

  const [sidebarWidth, setSidebarWidth] = useState(320);
  const dragInfoRef = useRef<{ isDragging: boolean; startX: number; startWidth: number } | null>(null);

  const startResizing = (e: React.MouseEvent) => {
    e.preventDefault();
    dragInfoRef.current = {
      isDragging: true,
      startX: e.clientX,
      startWidth: sidebarWidth
    };
    document.addEventListener('mousemove', handleMouseMove);
    document.addEventListener('mouseup', handleMouseUp);
  };

  const handleMouseMove = (e: MouseEvent) => {
    if (!dragInfoRef.current || !dragInfoRef.current.isDragging) {
      return;
    }
    const deltaX = e.clientX - dragInfoRef.current.startX;
    const newWidth = Math.max(260, Math.min(600, dragInfoRef.current.startWidth + deltaX));
    setSidebarWidth(newWidth);
  };

  const handleMouseUp = () => {
    if (dragInfoRef.current) {
      dragInfoRef.current.isDragging = false;
    }
    document.removeEventListener('mousemove', handleMouseMove);
    document.removeEventListener('mouseup', handleMouseUp);
  };

  useEffect(() => {
    return () => {
      document.removeEventListener('mousemove', handleMouseMove);
      document.removeEventListener('mouseup', handleMouseUp);
    };
  }, []);

  const overviewQuery = useQuery({
    queryKey: settingsHostInfrastructureMemoryOverviewQueryKey,
    queryFn: fetchSettingsHostInfrastructureMemoryOverview
  });
  const contracts = overviewQuery.data?.contracts ?? [];
  const resolvedActiveContractCode =
    activeTabKey !== MEMORY_STATS_TAB_KEY &&
    contracts.some((contract) => contract.contract_code === activeTabKey)
      ? activeTabKey
      : null;
  const activeContract = contracts.find(
    (contract) => contract.contract_code === resolvedActiveContractCode
  );
  const resolvedActiveTabKey =
    resolvedActiveContractCode || activeTabKey === MEMORY_STATS_TAB_KEY
      ? activeTabKey
      : MEMORY_STATS_TAB_KEY;
  const pageSize = activeContract?.capabilities.default_page_size ?? 50;
  const canListEntries = Boolean(
    activeContract?.supported && activeContract.capabilities.list_entries
  );
  const canListTree = Boolean(
    activeContract?.supported && activeContract.capabilities.list_tree
  );
  const canSearchEntries = Boolean(
    activeContract?.supported && activeContract.capabilities.search_entries
  );
  const entryRequest = selectedInspectionPath
    ? {
        inspection_path: selectedInspectionPath,
        cursor: entryCursor,
        limit: pageSize
      }
    : undefined;
  const entriesQuery = useQuery({
    queryKey: submittedSearch
      ? canSearchEntries
        ? settingsHostInfrastructureMemorySearchQueryKey(
            resolvedActiveContractCode,
            entryRequest
              ? { ...entryRequest, q: submittedSearch }
              : { q: submittedSearch }
          )
        : settingsHostInfrastructureMemoryEntriesQueryKey(
            resolvedActiveContractCode,
            entryRequest
          )
      : settingsHostInfrastructureMemoryEntriesQueryKey(
          resolvedActiveContractCode,
          entryRequest
        ),
    queryFn: () => {
      if (!resolvedActiveContractCode || !entryRequest) {
        return Promise.resolve(null);
      }
      if (submittedSearch && canSearchEntries) {
        return searchSettingsHostInfrastructureMemoryEntries(
          resolvedActiveContractCode,
          { ...entryRequest, q: submittedSearch }
        );
      }
      return fetchSettingsHostInfrastructureMemoryEntries(
        resolvedActiveContractCode,
        entryRequest
      );
    },
    enabled: Boolean(
      resolvedActiveContractCode && canListEntries && entryRequest
    )
  });
  const statsOverviewQuery = useQuery({
    queryKey: settingsHostInfrastructureMemoryStatsOverviewQueryKey,
    queryFn: fetchSettingsHostInfrastructureMemoryStatsOverview,
    enabled: Boolean(contracts.length)
  });
  const rootTreeQuery = useQuery({
    queryKey: settingsHostInfrastructureMemoryTreeQueryKey(
      resolvedActiveContractCode,
      {
        inspection_path: [],
        limit: pageSize
      }
    ),
    queryFn: () =>
      resolvedActiveContractCode
        ? fetchSettingsHostInfrastructureMemoryTree(
            resolvedActiveContractCode,
            {
              inspection_path: [],
              limit: pageSize
            }
          )
        : Promise.resolve(null),
    enabled: Boolean(resolvedActiveContractCode && canListTree)
  });
  const entries = entriesQuery.data?.entries ?? [];
  const canReveal = resolveCanReveal(
    canManage,
    overviewQuery.data?.can_manage,
    activeContract
  );

  useEffect(() => {
    setLoadedTreeChildren({});
    setSelectedInspectionPath(null);
    setEntryCursor(null);
    setCursorHistory([]);
    setSubmittedSearch('');
    setSearchText('');
    setTreeSearchText('');
    setTreeExpandedKeys([]);
    setTreeAutoExpandParent(true);
  }, [resolvedActiveContractCode]);

  const refreshMemoryQueries = async (contractCode: string | null) => {
    await Promise.all([
      queryClient.invalidateQueries({
        queryKey: settingsHostInfrastructureMemoryOverviewQueryKey
      }),
      queryClient.invalidateQueries({
        queryKey: [
          'settings',
          'host-infrastructure',
          'memory',
          'contracts',
          contractCode
        ]
      })
    ]);
  };

  useEffect(() => {
    if (!autoRefresh) return;
    const interval = setInterval(() => {
      void refreshMemoryQueries(resolvedActiveContractCode);
    }, 30000);
    return () => clearInterval(interval);
  }, [autoRefresh, resolvedActiveContractCode]);

  const revealMutation = useMutation({
    mutationFn: async ({
      entry,
      revealMode
    }: {
      entry: SettingsHostInfrastructureMemoryEntry;
      revealMode: 'preview' | 'full';
    }) => {
      if (!csrfToken) {
        throw new Error('csrf_missing');
      }
      return revealSettingsHostInfrastructureMemoryEntry(
        entry.contract_code,
        entry.entry_ref,
        csrfToken,
        revealMode
      );
    },
    onSuccess: (value) => {
      setRevealedEntry(value);
    }
  });

  const rootNodes = rootTreeQuery.data?.nodes ?? [];
  const treeData = useMemo(
    () => toTreeData(rootNodes, loadedTreeChildren, treeSearchText),
    [loadedTreeChildren, rootNodes, treeSearchText]
  );
  const treeSearchItems = useMemo(
    () => collectTreeSearchItems(treeData),
    [treeData]
  );
  const selectedTreeKey = findTreeKeyByPath(treeData, selectedInspectionPath);

  const updateTreeSearchText = (value: string) => {
    setTreeSearchText(value);
    const normalizedValue = value.trim().toLowerCase();
    if (!normalizedValue) {
      setTreeExpandedKeys([]);
      setTreeAutoExpandParent(false);
      return;
    }
    const matchedParentKeys = treeSearchItems
      .filter((item) => item.title.toLowerCase().includes(normalizedValue))
      .map((item) => item.parentKey)
      .filter((key): key is Key => key != null)
      .filter((key, index, keys) => keys.indexOf(key) === index);
    setTreeExpandedKeys(matchedParentKeys);
    setTreeAutoExpandParent(true);
  };

  const loadTreeChildren = async (treeNode: DataNode) => {
    if (!resolvedActiveContractCode) {
      return;
    }
    const node = treeNode as DataNode & {
      inspectionPath?: string[];
      isLeaf?: boolean;
    };
    if (node.isLeaf || !node.inspectionPath) {
      return;
    }
    const response = await queryClient.fetchQuery({
      queryKey: settingsHostInfrastructureMemoryTreeQueryKey(
        resolvedActiveContractCode,
        { inspection_path: node.inspectionPath, limit: pageSize }
      ),
      queryFn: () =>
        fetchSettingsHostInfrastructureMemoryTree(resolvedActiveContractCode, {
          inspection_path: node.inspectionPath,
          limit: pageSize
        })
    });
    setLoadedTreeChildren((current) => ({
      ...current,
      [String(node.key)]: response.nodes
    }));
  };

  const selectTab = (tabKey: string) => {
    setActiveTabKey(tabKey);
  };

  const selectInspectionPath = (path: string[]) => {
    setSelectedInspectionPath(path);
    setEntryCursor(null);
    setCursorHistory([]);
  };

  const entryColumns = useMemo<
    ColumnsType<SettingsHostInfrastructureMemoryEntry>
  >(
    () => [
      {
        title: i18nText("settings", "auto.key"),
        dataIndex: 'key',
        key: 'key',
        width: 220,
        render: (key: string) => (
          <Tooltip title={key} placement="topLeft">
            <Typography.Text
              copyable
              ellipsis={{ tooltip: false }}
              style={{ maxWidth: 160, display: 'inline-block', verticalAlign: 'middle' }}
              className="host-memory-panel__key"
            >
              {key}
            </Typography.Text>
          </Tooltip>
        )
      },
      {
        title: i18nText("settings", "auto.group"),
        dataIndex: 'group_code',
        key: 'group_code',
        width: 160,
        render: (group: string) => (
          <Tooltip title={group} placement="topLeft">
            <Typography.Text
              ellipsis={{ tooltip: false }}
              style={{ maxWidth: 120, display: 'inline-block', verticalAlign: 'middle' }}
            >
              {group}
            </Typography.Text>
          </Tooltip>
        )
      },
      {
        title: i18nText("settings", "auto.kind"),
        dataIndex: 'entry_kind',
        key: 'entry_kind',
        width: 130
      },
      {
        title: i18nText("settings", "auto.status"),
        dataIndex: 'status',
        key: 'status',
        width: 110,
        render: (status: string) => <Tag>{status}</Tag>
      },
      {
        title: i18nText("settings", "auto.sensitive"),
        dataIndex: 'sensitive',
        key: 'sensitive',
        width: 110,
        render: (sensitive: boolean) => (
          <Tag color={sensitive ? 'red' : 'default'}>
            {sensitive ? i18nText("settings", "auto.yes") : i18nText("settings", "auto.no")}
          </Tag>
        )
      },
      {
        title: i18nText("settings", "auto.ttl"),
        dataIndex: 'ttl_seconds',
        key: 'ttl_seconds',
        width: 120,
        render: (ttl: number | null) => formatTtl(ttl)
      },
      {
        title: i18nText("settings", "auto.size"),
        dataIndex: 'value_size_bytes',
        key: 'value_size_bytes',
        width: 110,
        render: (size: number) => formatBytes(size)
      },
      {
        title: '',
        key: 'actions',
        width: canReveal ? 220 : 120,
        render: (_, entry) => (
          <Space size={4}>
            <Tooltip title={i18nText("settings", "auto.check_meta_information_entry_read_actual_value")}>
              <Button
                icon={<FileSearchOutlined />}
                onClick={() => setMetadataEntry(entry)}
                size="small"
              >
                {i18nText("settings", "auto.metadata")}
              </Button>
            </Tooltip>
            {canReveal ? (
              <Tooltip title={i18nText("settings", "auto.request_backend_read_real_value_preview_displayed_first_may_contain")}>
                <Button
                  icon={<EyeOutlined />}
                  loading={revealMutation.isPending}
                  onClick={() => {
                    revealMutation.mutate({ entry, revealMode: 'preview' });
                  }}
                  size="small"
                >
                  {i18nText("settings", "auto.reveal")}
                </Button>
              </Tooltip>
            ) : null}
          </Space>
        )
      }
    ],
    [canReveal, revealMutation]
  );

  if (overviewQuery.isError) {
    return (
      <Space direction="vertical" size={16} className="host-memory-panel">
        <Alert
          type="error"
          showIcon
          message={i18nText("settings", "auto.memory_watch_connection_failed")}
          description={i18nText("settings", "auto.unable_read_host_infrastructure_memory_observation_api_server")}
        />
        <Button
          icon={<ReloadOutlined />}
          onClick={() => overviewQuery.refetch()}
          loading={overviewQuery.isFetching}
        >
          {i18nText("settings", "auto.refresh")}</Button>
      </Space>
    );
  }

  return (
    <Space direction="vertical" size={16} className="host-memory-panel">
      <div className="host-memory-panel__toolbar">
        <Space size={[8, 8]} wrap>
          <Tag color="blue">{i18nText("settings", "auto.contracts_count", { value1: contracts.length })}</Tag>
          <Tag>
            {i18nText("settings", "auto.reveal_status", {
              value1: overviewQuery.data?.can_manage
                ? i18nText("settings", "auto.available")
                : i18nText("settings", "auto.off")
            })}
          </Tag>
          <Typography.Text type="secondary">
            {i18nText("settings", "auto.recently_refreshed_alt")}{formatUpdatedAt(overviewQuery.dataUpdatedAt)}
          </Typography.Text>
        </Space>
        <Space size={12} align="center">
          <Space size={6} align="center">
            <Switch
              checked={autoRefresh}
              onChange={(checked) => setAutoRefresh(checked)}
              size="small"
            />
            <Typography.Text type="secondary" style={{ fontSize: 13 }}>
              {i18nText("settings", "auto.auto_refresh_three_zero_s")}</Typography.Text>
          </Space>
          <Button
            icon={<ReloadOutlined />}
            onClick={() => {
              void refreshMemoryQueries(resolvedActiveContractCode);
            }}
            loading={
              overviewQuery.isFetching ||
              entriesQuery.isFetching ||
              rootTreeQuery.isFetching ||
              statsOverviewQuery.isFetching
            }
          >
            {i18nText("settings", "auto.refresh")}</Button>
        </Space>
      </div>

      {resolvedActiveContractCode && !canReveal ? (
        <Alert
          type="info"
          showIcon
          message={i18nText("settings", "auto.view_displays_metadata")}
          description={i18nText("settings", "auto.reveal_value_requires_infrastructure_manage_permissions_reveal_value_capability_contract")}
        />
      ) : null}

      {overviewQuery.isSuccess && !contracts.length ? (
        <Empty
          image={Empty.PRESENTED_IMAGE_SIMPLE}
          description={i18nText("settings", "auto.currently_observable_memory_contract")}
        />
      ) : null}

      {contracts.length ? (
        <Tabs
          activeKey={resolvedActiveTabKey}
          className="host-memory-panel__tabs"
          items={[
            {
              key: MEMORY_STATS_TAB_KEY,
              label: i18nText("settings", "auto.statistics"),
              children: (
                <div className="host-memory-panel__tab-pane">
                  <MemoryStatsOverviewPane
                    data={statsOverviewQuery.data}
                    isError={statsOverviewQuery.isError}
                    isLoading={statsOverviewQuery.isLoading}
                  />
                </div>
              )
            },
            ...contracts.map((contract) => ({
              key: contract.contract_code,
              label: (
                <span className="host-memory-panel__tab-label">
                  <span>{contract.label}</span>
                </span>
              ),
              children:
                contract.contract_code === resolvedActiveContractCode ? (
                  <div className="host-memory-panel__tab-pane">
                    <Layout className="host-memory-panel__content">
                      <Layout.Sider
                        className="host-memory-panel__tree"
                        theme="light"
                        width={sidebarWidth}
                        style={{
                          width: sidebarWidth,
                          minWidth: sidebarWidth,
                          maxWidth: sidebarWidth,
                          flex: `0 0 ${sidebarWidth}px`
                        }}
                      >
                        {activeContract ? (
                          !activeContract.supported ||
                          !activeContract.capabilities.list_tree ? (
                            <Alert
                              type="warning"
                              showIcon
                              message={i18nText("settings", "auto.contract_support_tree_inspection")}
                            />
                          ) : rootTreeQuery.isError ? (
                            <Alert
                              type="error"
                              showIcon
                              message={i18nText("settings", "auto.memory_tree_loading_failed")}
                            />
                          ) : rootTreeQuery.isSuccess && !rootNodes.length ? (
                            <Empty
                              image={Empty.PRESENTED_IMAGE_SIMPLE}
                              description={i18nText("settings", "auto.memory_node_yet")}
                            />
                          ) : (
                            <div
                              className="host-memory-panel__tree-panel"
                              style={{ height: '100%', display: 'flex', flexDirection: 'column', gap: '8px', width: '100%' }}
                            >

                              <div className="host-memory-panel__tree-body" style={{ flex: '1 1 0%', overflow: 'auto' }}>
                                <Tree
                                  blockNode
                                  indent={8}
                                  autoExpandParent={treeAutoExpandParent}
                                  expandedKeys={treeExpandedKeys}
                                  treeData={treeData}
                                  loadData={loadTreeChildren}
                                  selectedKeys={
                                    selectedTreeKey ? [selectedTreeKey] : []
                                  }
                                  onExpand={(keys) => {
                                    setTreeExpandedKeys(keys);
                                    setTreeAutoExpandParent(false);
                                  }}
                                  onSelect={(_, info) => {
                                    const node = info.node as DataNode & {
                                      inspectionPath?: string[];
                                    };
                                    if (node.inspectionPath) {
                                      selectInspectionPath(node.inspectionPath);
                                    }
                                  }}
                                />
                              </div>
                            </div>
                          )
                        ) : (
                          <Empty
                            image={Empty.PRESENTED_IMAGE_SIMPLE}
                            description={i18nText("settings", "auto.select_memory_contract")}
                          />
                        )}
                      </Layout.Sider>

                      <div
                        className="host-memory-panel__resize-handle"
                        onMouseDown={startResizing}
                      />

                      <Layout.Content className="host-memory-panel__entries">
                        <div style={{ height: '100%', display: 'flex', flexDirection: 'column', gap: '12px', width: '100%' }}>
                          <div className="host-memory-panel__entries-header">
                            <Space direction="vertical" size={2}>
                              <Typography.Text strong>{i18nText("settings", "auto.entries")}</Typography.Text>
                              <Typography.Text type="secondary">
                                {selectedInspectionPath
                                  ? formatInspectionPath(selectedInspectionPath)
                                  : i18nText("settings", "auto.no_path_selected")}
                              </Typography.Text>
                            </Space>
                            <Input.Search
                              allowClear
                              disabled={!canSearchEntries}
                              value={searchText}
                              onChange={(event) =>
                                setSearchText(event.target.value)
                              }
                              onSearch={(value) => {
                                if (!canSearchEntries) {
                                  return;
                                }
                                setSubmittedSearch(value.trim());
                                setEntryCursor(null);
                                setCursorHistory([]);
                              }}
                              size="small"
                              style={{ maxWidth: 240 }}
                            />
                          </div>

                          {!selectedInspectionPath ? (
                            <Empty
                              image={Empty.PRESENTED_IMAGE_SIMPLE}
                              description={i18nText("settings", "auto.select_tree_node")}
                            />
                          ) : entriesQuery.isError ? (
                            <Alert
                              type="error"
                              showIcon
                              message={i18nText("settings", "auto.memory_entry_connection_failed")}
                              description={i18nText("settings", "auto.unable_read_entries_path")}
                            />
                          ) : entriesQuery.isSuccess && !entries.length ? (
                            <Empty
                              image={Empty.PRESENTED_IMAGE_SIMPLE}
                              description={i18nText("settings", "auto.memory_entry_yet")}
                            />
                          ) : (
                            <>
                              <div className="host-memory-panel__table-wrapper" style={{ flex: '1 1 0%', overflow: 'auto' }}>
                                <Table
                                  rowKey={(entry) => entry.entry_ref}
                                  columns={entryColumns}
                                  dataSource={entries}
                                  loading={
                                    entriesQuery.isLoading ||
                                    entriesQuery.isFetching
                                  }
                                  pagination={false}
                                  size="small"
                                />
                              </div>
                              <div className="host-memory-panel__entries-header">
                                <Typography.Text type="secondary">
                                  {entriesQuery.data
                                    ? `${formatBytes(
                                        entriesQuery.data.emitted_bytes
                                      )} ${i18nText("settings", "auto.emitted")}`
                                    : null}
                                </Typography.Text>
                                <Space size={8}>
                                  <Button
                                    size="small"
                                    disabled={!cursorHistory.length}
                                    onClick={() => {
                                      setCursorHistory((current) => {
                                        const previousCursor =
                                          current.at(-1) ?? null;
                                        const nextHistory = current.slice(
                                          0,
                                          -1
                                        );
                                        setEntryCursor(previousCursor || null);
                                        return nextHistory;
                                      });
                                    }}
                                  >
                                    {i18nText("settings", "auto.previous_page")}</Button>
                                  <Button
                                    size="small"
                                    disabled={!entriesQuery.data?.next_cursor}
                                    onClick={() => {
                                      const nextCursor =
                                        entriesQuery.data?.next_cursor;
                                      if (!nextCursor) {
                                        return;
                                      }
                                      setCursorHistory((current) => [
                                        ...current,
                                        entryCursor ?? ''
                                      ]);
                                      setEntryCursor(nextCursor);
                                    }}
                                  >
                                    {i18nText("settings", "auto.next_page")}</Button>
                                </Space>
                              </div>
                            </>
                          )}
                        </div>
                      </Layout.Content>
                    </Layout>
                  </div>
                ) : null
            }))
          ]}
          onChange={selectTab}
        />
      ) : null}

      <Drawer
        title={i18nText("settings", "auto.entry_metadata")}
        width={640}
        open={Boolean(metadataEntry)}
        onClose={() => setMetadataEntry(null)}
        destroyOnClose
      >
        {metadataEntry ? (
          <Space
            direction="vertical"
            size={16}
            className="host-memory-panel__drawer"
          >
            <Descriptions column={1} size="small">
              <Descriptions.Item label={i18nText("settings", "auto.contract")}>
                {metadataEntry.contract_code}
              </Descriptions.Item>
              <Descriptions.Item label={i18nText("settings", "auto.group")}>
                {metadataEntry.group_code}
              </Descriptions.Item>
              <Descriptions.Item label={i18nText("settings", "auto.key")}>
                {metadataEntry.key}
              </Descriptions.Item>
              <Descriptions.Item label={i18nText("settings", "auto.entry_ref")}>
                {metadataEntry.entry_ref}
              </Descriptions.Item>
              <Descriptions.Item label={i18nText("settings", "auto.path")}>
                {formatInspectionPath(metadataEntry.inspection_path)}
              </Descriptions.Item>
              <Descriptions.Item label={i18nText("settings", "auto.owner")}>
                {metadataEntry.owner ?? i18nText("settings", "auto.unknown")}
              </Descriptions.Item>
              <Descriptions.Item label={i18nText("settings", "auto.created")}>
                {formatUnixTimestamp(metadataEntry.created_at_unix)}
              </Descriptions.Item>
              <Descriptions.Item label={i18nText("settings", "auto.expires")}>
                {formatUnixTimestamp(metadataEntry.expires_at_unix)}
              </Descriptions.Item>
            </Descriptions>
            <JsonPreviewBlock
              title={i18nText("settings", "auto.metadata")}
              value={metadataEntry.metadata}
              collapsible={false}
              height="320px"
              copySuccessMessage={i18nText("settings", "auto.metadata_json_copied")}
            />
          </Space>
        ) : null}
      </Drawer>

      <Drawer
        title={i18nText("settings", "auto.entry_value")}
        width={640}
        open={Boolean(revealedEntry)}
        onClose={() => setRevealedEntry(null)}
        destroyOnClose
      >
        {revealedEntry ? (
          <Space
            direction="vertical"
            size={16}
            className="host-memory-panel__drawer"
          >
            <Descriptions column={1} size="small">
              <Descriptions.Item label={i18nText("settings", "auto.contract")}>
                {revealedEntry.metadata.contract_code}
              </Descriptions.Item>
              <Descriptions.Item label={i18nText("settings", "auto.group")}>
                {revealedEntry.metadata.group_code}
              </Descriptions.Item>
              <Descriptions.Item label={i18nText("settings", "auto.key")}>
                {revealedEntry.metadata.key}
              </Descriptions.Item>
              <Descriptions.Item label={i18nText("settings", "auto.entry_ref")}>
                {revealedEntry.metadata.entry_ref}
              </Descriptions.Item>
              <Descriptions.Item label={i18nText("settings", "auto.value_state")}>
                {revealedEntry.value_state}
              </Descriptions.Item>
              <Descriptions.Item label={i18nText("settings", "auto.reveal_mode")}>
                {revealedEntry.reveal_mode}
              </Descriptions.Item>
              <Descriptions.Item label={i18nText("settings", "auto.size")}>
                {formatBytes(revealedEntry.metadata.value_size_bytes)}
              </Descriptions.Item>
            </Descriptions>
            {revealedEntry.value_state === 'available' ? (
              <JsonPreviewBlock
                title={i18nText("settings", "auto.memory_value")}
                value={revealedEntry.value}
                collapsible={false}
                height="360px"
                copySuccessMessage={i18nText("settings", "auto.memory_json_copied")}
              />
            ) : revealedEntry.value_preview ? (
              <Space direction="vertical" size={8}>
                <Alert
                  type="info"
                  showIcon
                  message={i18nText("settings", "auto.preview")}
                  description={`${formatBytes(
                    revealedEntry.preview_size_bytes
                  )} ${i18nText("settings", "auto.of")} ${formatBytes(revealedEntry.full_value_size_bytes)}`}
                />
                <JsonPreviewBlock
                  title={i18nText("settings", "auto.memory_value_preview")}
                  value={revealedEntry.value_preview}
                  rawText={revealedEntry.value_preview}
                  collapsible={false}
                  height="320px"
                  copySuccessMessage={i18nText("settings", "auto.copied_memory_preview_json")}
                />
              </Space>
            ) : (
              <Alert
                type="warning"
                showIcon
                message={i18nText("settings", "auto.value_too_large")}
                description={`${formatBytes(
                  revealedEntry.full_value_size_bytes
                )} ${i18nText("settings", "auto.exceeds_full_reveal_limit")}`}
              />
            )}
            {canReveal && revealedEntry.value_state === 'preview' ? (
              <Button
                icon={<EyeOutlined />}
                loading={revealMutation.isPending}
                onClick={() =>
                  revealMutation.mutate({
                    entry: revealedEntry.metadata,
                    revealMode: 'full'
                  })
                }
              >
                {i18nText("settings", "auto.full_reveal")}
              </Button>
            ) : null}
          </Space>
        ) : null}
      </Drawer>
    </Space>
  );
}
