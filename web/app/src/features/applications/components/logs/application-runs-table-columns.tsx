import { Tag } from 'antd';

import type { DataTableColumn } from '../../../../shared/ui/data-table/DataTable';
import type { ApplicationRunSummary } from '../../api/runtime';
import { formatApplicationRunCompatibilityMode } from '../../lib/run-compatibility-mode';

const STATUS_COLOR: Record<string, string> = {
  succeeded: 'green',
  failed: 'red',
  running: 'blue',
  waiting_human: 'gold',
  waiting_callback: 'orange'
};

function formatTimestamp(value: string | null | undefined) {
  if (!value) {
    return '-';
  }

  return new Date(value).toLocaleString('zh-CN', { hour12: false });
}

function formatRunStatisticNumber(value: number | null | undefined) {
  return typeof value === 'number' && Number.isFinite(value)
    ? value.toLocaleString('zh-CN')
    : '-';
}

export const APPLICATION_RUNS_TABLE_COLUMNS: Array<
  DataTableColumn<ApplicationRunSummary>
> = [
  {
    key: 'title',
    title: '标题',
    dataIndex: 'title',
    width: 240,
    ellipsis: true,
    render: (value) => (value ? `${value}` : '-')
  },
  {
    key: 'expand_id',
    title: 'expand_id',
    dataIndex: 'expand_id',
    width: 180,
    ellipsis: true,
    render: (value) => (value ? `${value}` : '-')
  },
  {
    key: 'authorized_account',
    title: '授权人',
    dataIndex: 'authorized_account',
    width: 160,
    ellipsis: true,
    render: (value) => (value ? `${value}` : '-')
  },
  {
    key: 'id',
    title: '运行 ID',
    dataIndex: 'id',
    width: 180,
    ellipsis: true
  },
  {
    key: 'run_mode',
    title: '模式',
    dataIndex: 'run_mode',
    width: 180
  },
  {
    key: 'compatibility_mode',
    title: '协议',
    dataIndex: 'compatibility_mode',
    width: 170,
    ellipsis: true,
    render: (value) =>
      formatApplicationRunCompatibilityMode(
        typeof value === 'string' ? value : null
      )
  },
  {
    key: 'target_node_id',
    title: '目标节点',
    dataIndex: 'target_node_id',
    width: 160,
    render: (value) => (typeof value === 'string' && value ? value : '全流')
  },
  {
    key: 'status',
    title: '状态',
    width: 120,
    render: (_: unknown, run) => (
      <Tag color={STATUS_COLOR[run.status] ?? 'default'}>{run.status}</Tag>
    )
  },
  {
    key: 'total_tokens',
    title: '总 tokens',
    width: 130,
    render: (_value, run) =>
      formatRunStatisticNumber(run.statistics?.total_tokens)
  },
  {
    key: 'unique_node_count',
    title: '真实节点数',
    width: 130,
    render: (_value, run) =>
      formatRunStatisticNumber(run.statistics?.unique_node_count)
  },
  {
    key: 'tool_callback_count',
    title: '工具回调次数',
    width: 150,
    render: (_value, run) =>
      formatRunStatisticNumber(run.statistics?.tool_callback_count)
  },
  {
    key: 'started_at',
    title: '开始时间',
    dataIndex: 'started_at',
    width: 200,
    render: (value) => formatTimestamp(typeof value === 'string' ? value : null)
  },
  {
    key: 'updated_at',
    title: '更新时间',
    dataIndex: 'updated_at',
    width: 200,
    render: (value) => formatTimestamp(typeof value === 'string' ? value : null)
  },
  {
    key: 'action',
    title: '操作',
    width: 140
  }
];
