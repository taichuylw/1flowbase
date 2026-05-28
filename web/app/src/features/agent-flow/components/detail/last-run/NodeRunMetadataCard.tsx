import { Card, Descriptions } from 'antd';

import type { NodeLastRun } from '../../../api/runtime';
import { i18nText } from '../../../../../shared/i18n/text';
import { formatDateTime } from '../../../../../shared/i18n/format';

function formatTimestamp(value: string | null) {
  if (!value) {
    return i18nText("agentFlow", "auto.key_aibaedpijj");
  }

  return formatDateTime(value, { hour12: false });
}

export function NodeRunMetadataCard({
  lastRun
}: {
  lastRun: NodeLastRun;
}) {
  return (
    <Card title={i18nText("agentFlow", "auto.key_nljodhfffg")}>
      <Descriptions
        column={1}
        size="small"
        items={[
          {
            key: 'node_alias',
            label: i18nText("agentFlow", "auto.fallback_node_label"),
            children: `${lastRun.node_run.node_alias} (${lastRun.node_run.node_id})`
          },
          {
            key: 'node_type',
            label: i18nText("agentFlow", "auto.key_eophpofcem"),
            children: lastRun.node_run.node_type
          },
          {
            key: 'started_at',
            label: i18nText("agentFlow", "auto.key_oiigikpgol"),
            children: formatTimestamp(lastRun.node_run.started_at)
          },
          {
            key: 'finished_at',
            label: i18nText("agentFlow", "auto.key_kalljpejkl"),
            children: formatTimestamp(lastRun.node_run.finished_at)
          }
        ]}
      />
    </Card>
  );
}
