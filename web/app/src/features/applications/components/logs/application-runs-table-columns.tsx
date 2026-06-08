import { Tag } from 'antd';
import type { TFunction } from 'i18next';

import type { DataTableColumn } from '../../../../shared/ui/data-table/DataTable';
import { formatDateTime, formatNumber } from '../../../../shared/i18n/format';
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

  return formatDateTime(value, { hour12: false });
}

function formatRunStatisticNumber(value: number | null | undefined) {
  return typeof value === 'number' && Number.isFinite(value)
    ? formatNumber(value)
    : '-';
}

export function getApplicationRunsTableColumns(
  t: TFunction<'applications'>
): Array<DataTableColumn<ApplicationRunSummary>> {
  return [
  {
    key: 'title',
    title: t('auto.title'),
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
    title: t('auto.authorizer'),
    dataIndex: 'authorized_account',
    width: 160,
    ellipsis: true,
    render: (value) => (value ? `${value}` : '-')
  },
  {
    key: 'id',
    title: t('auto.run_id'),
    dataIndex: 'id',
    width: 180,
    ellipsis: true,
    render: (_value, run) => run.id
  },
  {
    key: 'run_mode',
    title: t('auto.mode'),
    dataIndex: 'run_mode',
    width: 180
  },
  {
    key: 'compatibility_mode',
    title: t('auto.protocol'),
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
    title: t('auto.target_node'),
    dataIndex: 'target_node_id',
    width: 160,
    render: (value) => (typeof value === 'string' && value ? value : t('auto.full_flow'))
  },
  {
    key: 'status',
    title: t('auto.status'),
    width: 120,
    render: (_: unknown, run) => (
      <Tag color={STATUS_COLOR[run.status] ?? 'default'}>{run.status}</Tag>
    )
  },
  {
    key: 'total_tokens',
    title: t('auto.total_tokens'),
    width: 130,
    render: (_value, run) => formatRunStatisticNumber(run.total_tokens)
  },
  {
    key: 'input_tokens',
    title: t('auto.input_tokens'),
    width: 130,
    render: (_value, run) => formatRunStatisticNumber(run.input_tokens)
  },
  {
    key: 'output_tokens',
    title: t('auto.output_tokens'),
    width: 130,
    render: (_value, run) => formatRunStatisticNumber(run.output_tokens)
  },
  {
    key: 'input_cache_hit_tokens',
    title: t('auto.input_cache_hit_tokens'),
    width: 150,
    render: (_value, run) =>
      formatRunStatisticNumber(run.input_cache_hit_tokens)
  },
  {
    key: 'unique_node_count',
    title: t('auto.real_node_count'),
    width: 130,
    render: (_value, run) => formatRunStatisticNumber(run.unique_node_count)
  },
  {
    key: 'tool_callback_count',
    title: t('auto.tool_callback_count'),
    width: 150,
    render: (_value, run) => formatRunStatisticNumber(run.tool_callback_count)
  },
  {
    key: 'started_at',
    title: t('auto.start_time'),
    dataIndex: 'started_at',
    width: 200,
    render: (value) => formatTimestamp(typeof value === 'string' ? value : null)
  },
  {
    key: 'updated_at',
    title: t('auto.updated_at'),
    dataIndex: 'updated_at',
    width: 200,
    render: (value) => formatTimestamp(typeof value === 'string' ? value : null)
  },
  {
    key: 'action',
    title: t('auto.operation'),
    width: 140
  }
  ];
}
