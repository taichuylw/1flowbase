import { Card, Descriptions } from 'antd';

import type { NodeLastRun } from '../../../api/runtime';
import { i18nText } from '../../../../../shared/i18n/text';

function formatTimestamp(value: string | null) {
  if (!value) {
    return i18nText("agentFlow", "auto.k_081043f899");
  }

  return new Date(value).toLocaleString('zh-CN', { hour12: false });
}

export function NodeRunMetadataCard({
  lastRun
}: {
  lastRun: NodeLastRun;
}) {
  return (
    <Card title={i18nText("agentFlow", "auto.k_db9e375556")}>
      <Descriptions
        column={1}
        size="small"
        items={[
          {
            key: 'node_alias',
            label: i18nText("agentFlow", "auto.k_e840cd6f1e"),
            children: `${lastRun.node_run.node_alias} (${lastRun.node_run.node_id})`
          },
          {
            key: 'node_type',
            label: i18nText("agentFlow", "auto.k_4ef7fe524c"),
            children: lastRun.node_run.node_type
          },
          {
            key: 'started_at',
            label: i18nText("agentFlow", "auto.k_e8868af6eb"),
            children: formatTimestamp(lastRun.node_run.started_at)
          },
          {
            key: 'finished_at',
            label: i18nText("agentFlow", "auto.k_a0bb9f49ab"),
            children: formatTimestamp(lastRun.node_run.finished_at)
          }
        ]}
      />
    </Card>
  );
}
