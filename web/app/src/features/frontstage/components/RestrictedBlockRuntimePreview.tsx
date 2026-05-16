import {
  BlockUiRenderer,
  type BlockRendererActionEvent
} from '@1flowbase/block-renderer';
import { Alert, Descriptions, Empty, Space, Tag, Typography } from 'antd';

import type { RestrictedBlockRuntimeHostSnapshot } from '../lib/restricted-block-runtime-host';

export interface RestrictedBlockRuntimePreviewProps {
  snapshot: RestrictedBlockRuntimeHostSnapshot;
  onAction?: (event: BlockRendererActionEvent) => void;
}

export type RestrictedBlockRuntimeActionEvent = BlockRendererActionEvent;

const statusView: Record<
  RestrictedBlockRuntimeHostSnapshot['status'],
  { message: string; type: 'info' | 'success' | 'warning' | 'error' }
> = {
  idle: { message: '尚未运行', type: 'info' },
  running: { message: '运行中', type: 'info' },
  ready: { message: '运行结果', type: 'success' },
  failed: { message: '运行失败', type: 'error' },
  timed_out: { message: '运行超时', type: 'warning' },
  disposed: { message: '已释放', type: 'info' }
};

export function RestrictedBlockRuntimePreview({
  snapshot,
  onAction
}: RestrictedBlockRuntimePreviewProps) {
  const view = statusView[snapshot.status];

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
            <Empty image={Empty.PRESENTED_IMAGE_SIMPLE} description="暂无 UI schema" />
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
      title="错误摘要"
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
            {snapshot.logs.length} 条
          </Typography.Text>
          {snapshot.logs.map((log, index) => (
            <Typography.Text key={`${log.level}-${index}`}>
              <Tag>{log.level}</Tag>
              {log.message}
            </Typography.Text>
          ))}
        </Space>
      ) : (
        <Typography.Text type="secondary">无</Typography.Text>
      )}

      <Typography.Text strong>Effects</Typography.Text>
      {snapshot.effects.length > 0 ? (
        <Space direction="vertical" size={4} style={{ width: '100%' }}>
          <Typography.Text type="secondary">
            {snapshot.effects.length} 个 effect
          </Typography.Text>
          {snapshot.effects.map((effect, index) => (
            <Typography.Text key={`${effect.type}-${index}`}>
              {formatEffect(effect)}
            </Typography.Text>
          ))}
        </Space>
      ) : (
        <Typography.Text type="secondary">无</Typography.Text>
      )}

      <Typography.Text strong>Rejections</Typography.Text>
      {snapshot.rejections.length > 0 ? (
        <Space direction="vertical" size={4} style={{ width: '100%' }}>
          <Typography.Text type="secondary">
            {snapshot.rejections.length} 个 rejection
          </Typography.Text>
          {snapshot.rejections.map((rejection, index) => (
            <Typography.Text key={`${rejection.code}-${index}`}>
              <Tag color="warning">{rejection.code}</Tag>
              {rejection.path}: {rejection.message}
            </Typography.Text>
          ))}
        </Space>
      ) : (
        <Typography.Text type="secondary">无</Typography.Text>
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
