import {
  BlockUiRenderer,
  type BlockRendererActionEvent
} from '@1flowbase/block-renderer';
import { Alert, Descriptions, Empty, Space, Tag, Typography } from 'antd';

import type { RestrictedBlockRuntimeHostSnapshot } from '../lib/restricted-block-runtime-host';
import { i18nText } from '../../../shared/i18n/text';

export interface RestrictedBlockRuntimePreviewProps {
  snapshot: RestrictedBlockRuntimeHostSnapshot;
  onAction?: (event: BlockRendererActionEvent) => void;
}

export type RestrictedBlockRuntimeActionEvent = BlockRendererActionEvent;

function getStatusView(status: RestrictedBlockRuntimeHostSnapshot['status']): {
  message: string;
  type: 'info' | 'success' | 'warning' | 'error';
} {
  switch (status) {
    case 'idle':
      return { message: i18nText("frontstage", "auto.k_5afa7a9851"), type: 'info' };
    case 'running':
      return { message: i18nText("frontstage", "auto.k_5942497005"), type: 'info' };
    case 'ready':
      return { message: i18nText("frontstage", "auto.k_618d581979"), type: 'success' };
    case 'failed':
      return { message: i18nText("frontstage", "auto.k_f5f12f1d7a"), type: 'error' };
    case 'timed_out':
      return { message: i18nText("frontstage", "auto.k_5debddc330"), type: 'warning' };
    case 'disposed':
      return { message: i18nText("frontstage", "auto.k_0fc2814fea"), type: 'info' };
  }
}

export function RestrictedBlockRuntimePreview({
  snapshot,
  onAction
}: RestrictedBlockRuntimePreviewProps) {
  const view = getStatusView(snapshot.status);

  return (
    <Space
      data-testid="restricted-block-runtime-preview"
      direction="vertical"
      size="small"
      style={{ width: '100%' }}
    >
      <Alert type={view.type} showIcon message={view.message} />

      {snapshot.status === 'ready' ? (
        <Space direction="vertical" size="small" style={{ width: '100%' }}>
          {snapshot.schema === undefined ? (
            <Empty image={Empty.PRESENTED_IMAGE_SIMPLE} description={i18nText("frontstage", "auto.k_d62f5a0a4e")} />
          ) : (
            <BlockUiRenderer
              schema={snapshot.schema}
              validationOptions={snapshot.schemaValidationOptions}
              onAction={onAction}
            />
          )}
        </Space>
      ) : null}

      {snapshot.status === 'failed' || snapshot.status === 'timed_out' ? (
        <RuntimeErrorSummary snapshot={snapshot} />
      ) : null}

      <RuntimeActivitySummary snapshot={snapshot} />
    </Space>
  );
}

function RuntimeErrorSummary({
  snapshot
}: {
  snapshot: RestrictedBlockRuntimeHostSnapshot;
}) {
  const firstError = snapshot.error?.errors[0];

  return (
    <Descriptions
      bordered
      size="small"
      column={1}
      title={i18nText("frontstage", "auto.k_e127739219")}
      items={[
        {
          key: 'kind',
          label: 'Kind',
          children: snapshot.error?.kind ?? 'unknown'
        },
        {
          key: 'message',
          label: 'Message',
          children: snapshot.error?.message ?? 'Runtime failed.'
        },
        {
          key: 'code',
          label: 'Code',
          children: firstError?.code ?? snapshot.error?.kind ?? 'runtime_error'
        },
        {
          key: 'path',
          label: 'Path',
          children: firstError?.path ?? 'runtime'
        }
      ]}
    />
  );
}

function RuntimeActivitySummary({
  snapshot
}: {
  snapshot: RestrictedBlockRuntimeHostSnapshot;
}) {
  return (
    <Space direction="vertical" size="small" style={{ width: '100%' }}>
      <Typography.Text strong>Logs</Typography.Text>
      {snapshot.logs.length > 0 ? (
        <Space direction="vertical" size={4} style={{ width: '100%' }}>
          <Typography.Text type="secondary">
            {snapshot.logs.length} {i18nText("frontstage", "auto.k_bce2ef6151")}</Typography.Text>
          {snapshot.logs.map((log, index) => (
            <Typography.Text key={`${log.level}-${index}`}>
              <Tag>{log.level}</Tag>
              {log.message}
            </Typography.Text>
          ))}
        </Space>
      ) : (
        <Typography.Text type="secondary">{i18nText("frontstage", "auto.k_72077749f7")}</Typography.Text>
      )}

      <Typography.Text strong>Effects</Typography.Text>
      {snapshot.effects.length > 0 ? (
        <Space direction="vertical" size={4} style={{ width: '100%' }}>
          <Typography.Text type="secondary">
            {snapshot.effects.length} {i18nText("frontstage", "auto.k_d98770d860")}</Typography.Text>
          {snapshot.effects.map((effect, index) => (
            <Typography.Text key={`${effect.type}-${index}`}>
              {formatEffect(effect)}
            </Typography.Text>
          ))}
        </Space>
      ) : (
        <Typography.Text type="secondary">{i18nText("frontstage", "auto.k_72077749f7")}</Typography.Text>
      )}

      <Typography.Text strong>Rejections</Typography.Text>
      {snapshot.rejections.length > 0 ? (
        <Space direction="vertical" size={4} style={{ width: '100%' }}>
          <Typography.Text type="secondary">
            {snapshot.rejections.length} {i18nText("frontstage", "auto.k_402cffd864")}</Typography.Text>
          {snapshot.rejections.map((rejection, index) => (
            <Typography.Text key={`${rejection.code}-${index}`}>
              <Tag color="warning">{rejection.code}</Tag>
              {rejection.path}: {rejection.message}
            </Typography.Text>
          ))}
        </Space>
      ) : (
        <Typography.Text type="secondary">{i18nText("frontstage", "auto.k_72077749f7")}</Typography.Text>
      )}
    </Space>
  );
}

function formatEffect(
  effect: RestrictedBlockRuntimeHostSnapshot['effects'][number]
): string {
  switch (effect.type) {
    case 'action':
      return `action: ${effect.actionId}`;
    case 'data':
      return `data: ${effect.operation}`;
    case 'event':
      return `event: ${effect.name}`;
  }
}
