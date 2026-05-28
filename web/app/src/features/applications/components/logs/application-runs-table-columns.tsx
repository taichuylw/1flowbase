import { Tag } from 'antd';

import type { DataTableColumn } from '../../../../shared/ui/data-table/DataTable';
import type { ApplicationRunSummary } from '../../api/runtime';
import { formatApplicationRunCompatibilityMode } from '../../lib/run-compatibility-mode';
import { i18nText } from '../../../../shared/i18n/text';

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
    title: i18nText("applications", "auto.k_748d7dc7e3"),
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
    title: i18nText("applications", "auto.k_1c31fbd81e"),
    dataIndex: 'authorized_account',
    width: 160,
    ellipsis: true,
    render: (value) => (value ? `${value}` : '-')
  },
  {
    key: 'id',
    title: i18nText("applications", "auto.k_c1189023fb"),
    dataIndex: 'id',
    width: 180,
    ellipsis: true
  },
  {
    key: 'run_mode',
    title: i18nText("applications", "auto.k_ed0eea8f20"),
    dataIndex: 'run_mode',
    width: 180
  },
  {
    key: 'compatibility_mode',
    title: i18nText("applications", "auto.k_b0c431675b"),
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
    title: i18nText("applications", "auto.k_eba69a0678"),
    dataIndex: 'target_node_id',
    width: 160,
    render: (value) => (typeof value === 'string' && value ? value : i18nText("applications", "auto.k_538c417265"))
  },
  {
    key: 'status',
    title: i18nText("applications", "auto.k_62e951a692"),
    width: 120,
    render: (_: unknown, run) => (
      <Tag color={STATUS_COLOR[run.status] ?? 'default'}>{run.status}</Tag>
    )
  },
  {
    key: 'total_tokens',
    title: i18nText("applications", "auto.k_151dec7e9d"),
    width: 130,
    render: (_value, run) =>
      formatRunStatisticNumber(run.statistics?.total_tokens)
  },
  {
    key: 'unique_node_count',
    title: i18nText("applications", "auto.k_7f1c2ccf01"),
    width: 130,
    render: (_value, run) =>
      formatRunStatisticNumber(run.statistics?.unique_node_count)
  },
  {
    key: 'tool_callback_count',
    title: i18nText("applications", "auto.k_bf55cc6a69"),
    width: 150,
    render: (_value, run) =>
      formatRunStatisticNumber(run.statistics?.tool_callback_count)
  },
  {
    key: 'started_at',
    title: i18nText("applications", "auto.k_e8868af6eb"),
    dataIndex: 'started_at',
    width: 200,
    render: (value) => formatTimestamp(typeof value === 'string' ? value : null)
  },
  {
    key: 'updated_at',
    title: i18nText("applications", "auto.k_093dea88c9"),
    dataIndex: 'updated_at',
    width: 200,
    render: (value) => formatTimestamp(typeof value === 'string' ? value : null)
  },
  {
    key: 'action',
    title: i18nText("applications", "auto.k_f3ea6d345e"),
    width: 140
  }
];
