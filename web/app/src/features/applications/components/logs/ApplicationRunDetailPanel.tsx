import { ArrowLeftOutlined } from '@ant-design/icons';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import {
  Button,
  Descriptions,
  Empty,
  Result,
  Space,
  Tag,
  Timeline,
  Typography
} from 'antd';
import { useEffect, useState } from 'react';

import { JsonPreviewBlock } from '../../../../shared/ui/json-preview/JsonPreviewBlock';
import { useAuthStore } from '../../../../state/auth-store';
import {
  applicationRunDetailQueryKey,
  completeCallbackTask,
  fetchApplicationRunDetail,
  resumeFlowRun,
  type ApplicationRunDetail
} from '../../api/runtime';
import { ApplicationRunResumeCard } from './ApplicationRunResumeCard';
import './application-run-detail-panel.css';

const STATUS_COLOR: Record<string, string> = {
  succeeded: 'green',
  failed: 'red',
  running: 'blue',
  waiting_human: 'gold',
  waiting_callback: 'orange',
  cancelled: 'default'
};

function formatTimestamp(value: string | null | undefined) {
  if (!value) {
    return '未结束';
  }

  return new Date(value).toLocaleString('zh-CN', { hour12: false });
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return Boolean(value && typeof value === 'object' && !Array.isArray(value));
}

function summarizeValue(value: unknown): string {
  if (value === null || value === undefined) {
    return '无';
  }

  if (typeof value === 'string') {
    return value;
  }

  if (typeof value === 'number' || typeof value === 'boolean') {
    return String(value);
  }

  if (Array.isArray(value)) {
    return value.length === 0
      ? '空列表'
      : value.map((entry) => summarizeValue(entry)).join('、');
  }

  if (isRecord(value)) {
    const entries = Object.entries(value);

    if (entries.length === 0) {
      return '空对象';
    }

    return entries
      .map(([key, entryValue]) => `${key}: ${summarizeValue(entryValue)}`)
      .join(' · ');
  }

  return String(value);
}

function findFirstString(value: unknown): string | null {
  if (typeof value === 'string' && value.trim().length > 0) {
    return value;
  }

  if (Array.isArray(value)) {
    for (const entry of value) {
      const nestedValue = findFirstString(entry);

      if (nestedValue) {
        return nestedValue;
      }
    }

    return null;
  }

  if (isRecord(value)) {
    for (const nestedValue of Object.values(value)) {
      const firstString = findFirstString(nestedValue);

      if (firstString) {
        return firstString;
      }
    }
  }

  return null;
}

function findNamedString(
  value: unknown,
  preferredKeys: readonly string[]
): string | null {
  if (Array.isArray(value)) {
    for (const entry of value) {
      const nestedValue = findNamedString(entry, preferredKeys);

      if (nestedValue) {
        return nestedValue;
      }
    }

    return null;
  }

  if (!isRecord(value)) {
    return null;
  }

  for (const [key, entryValue] of Object.entries(value)) {
    if (
      typeof entryValue === 'string' &&
      preferredKeys.some((preferredKey) => key.includes(preferredKey)) &&
      entryValue.trim().length > 0
    ) {
      return entryValue;
    }
  }

  for (const entryValue of Object.values(value)) {
    const nestedValue = findNamedString(entryValue, preferredKeys);

    if (nestedValue) {
      return nestedValue;
    }
  }

  return null;
}

function extractEventText(payload: unknown) {
  if (!isRecord(payload)) {
    return '';
  }

  for (const key of ['text', 'delta']) {
    const value = payload[key];

    if (typeof value === 'string') {
      return value;
    }
  }

  return '';
}

function collectTextDeltas(detail: ApplicationRunDetail) {
  const text = detail.events
    .filter((event) => event.event_type === 'text_delta')
    .sort((left, right) => left.sequence - right.sequence)
    .map((event) => extractEventText(event.payload))
    .join('');

  return text.trim().length > 0 ? text : null;
}

function findPreferredOutputText(payload: unknown): string | null {
  if (!isRecord(payload)) {
    return null;
  }

  for (const key of ['answer', 'text', 'content', 'message']) {
    const value = payload[key];

    if (typeof value === 'string' && value.trim().length > 0) {
      return value;
    }
  }

  return null;
}

function buildConversation(detail: ApplicationRunDetail) {
  const userContent =
    findNamedString(detail.flow_run.input_payload, [
      'query',
      'question',
      'prompt',
      'message',
      'input'
    ]) ?? summarizeValue(detail.flow_run.input_payload);
  const preferredNodeOutput =
    [...detail.node_runs]
      .reverse()
      .map((nodeRun) => findFirstString(nodeRun.output_payload))
      .find((value): value is string => Boolean(value)) ?? null;
  const assistantContent =
    collectTextDeltas(detail) ??
    findPreferredOutputText(detail.flow_run.output_payload) ??
    findFirstString(detail.flow_run.output_payload) ??
    preferredNodeOutput ??
    (detail.flow_run.error_payload
      ? summarizeValue(detail.flow_run.error_payload)
      : '暂无输出');

  return { userContent, assistantContent };
}

function StatusTag({ status }: { status: string }) {
  return <Tag color={STATUS_COLOR[status] ?? 'default'}>{status}</Tag>;
}

function RunConversation({ detail }: { detail: ApplicationRunDetail }) {
  const { userContent, assistantContent } = buildConversation(detail);

  return (
    <section aria-label="AI 对话" className="application-run-detail__section">
      <div className="application-run-detail__section-header">
        <Typography.Title level={5}>AI 对话</Typography.Title>
      </div>
      <div className="application-run-detail__conversation">
        <article className="application-run-detail__message application-run-detail__message--user">
          <Typography.Text strong>User</Typography.Text>
          <Typography.Paragraph>{userContent}</Typography.Paragraph>
        </article>
        <article className="application-run-detail__message application-run-detail__message--ai">
          <Typography.Text strong>AI</Typography.Text>
          <Typography.Paragraph>{assistantContent}</Typography.Paragraph>
        </article>
      </div>
    </section>
  );
}

function NodeRunList({ detail }: { detail: ApplicationRunDetail }) {
  const [expandedNodeRunId, setExpandedNodeRunId] = useState<string | null>(
    null
  );

  useEffect(() => {
    setExpandedNodeRunId(null);
  }, [detail.flow_run.id]);

  return (
    <section className="application-run-detail__section">
      <div className="application-run-detail__section-header">
        <Typography.Title level={5}>工作流节点</Typography.Title>
        <Typography.Text type="secondary">
          {detail.node_runs.length} 个节点
        </Typography.Text>
      </div>
      {detail.node_runs.length === 0 ? (
        <Empty
          description="本次运行没有节点记录"
          image={Empty.PRESENTED_IMAGE_SIMPLE}
        />
      ) : (
        <div className="application-run-detail__node-list">
          {detail.node_runs.map((nodeRun) => {
            const expanded = expandedNodeRunId === nodeRun.id;

            return (
              <div
                className="application-run-detail__node-item"
                key={nodeRun.id}
              >
                <button
                  aria-expanded={expanded}
                  className="application-run-detail__node-button"
                  onClick={() =>
                    setExpandedNodeRunId((current) =>
                      current === nodeRun.id ? null : nodeRun.id
                    )
                  }
                  type="button"
                >
                  <span className="application-run-detail__node-main">
                    <span className="application-run-detail__node-title">
                      {nodeRun.node_alias}
                    </span>
                    <span className="application-run-detail__node-meta">
                      {nodeRun.node_type} · {nodeRun.node_id}
                    </span>
                  </span>
                  <span className="application-run-detail__node-side">
                    <StatusTag status={nodeRun.status} />
                    <span className="application-run-detail__node-time">
                      {formatTimestamp(nodeRun.started_at)}
                    </span>
                  </span>
                </button>
                {expanded ? (
                  <section
                    aria-label={`${nodeRun.node_alias} 节点输入输出`}
                    className="application-run-detail__node-detail"
                  >
                    <JsonPreviewBlock
                      height="180px"
                      title="输入"
                      value={nodeRun.input_payload}
                    />
                    <JsonPreviewBlock
                      height="180px"
                      title="输出"
                      value={nodeRun.output_payload}
                    />
                    {nodeRun.error_payload ? (
                      <JsonPreviewBlock
                        height="160px"
                        title="错误"
                        value={nodeRun.error_payload}
                      />
                    ) : null}
                    {Object.keys(nodeRun.metrics_payload).length > 0 ? (
                      <JsonPreviewBlock
                        defaultCollapsed
                        height="160px"
                        title="指标"
                        value={nodeRun.metrics_payload}
                      />
                    ) : null}
                  </section>
                ) : null}
              </div>
            );
          })}
        </div>
      )}
    </section>
  );
}

function RunMetadata({ detail }: { detail: ApplicationRunDetail }) {
  return (
    <section className="application-run-detail__section">
      <div className="application-run-detail__section-header">
        <Typography.Title level={5}>运行摘要</Typography.Title>
      </div>
      <Descriptions
        column={{ xs: 1, sm: 1, md: 2 }}
        items={[
          {
            key: 'run_id',
            label: '运行 ID',
            children: detail.flow_run.id
          },
          {
            key: 'status',
            label: '运行状态',
            children: <StatusTag status={detail.flow_run.status} />
          },
          {
            key: 'mode',
            label: '运行模式',
            children: detail.flow_run.run_mode
          },
          {
            key: 'target',
            label: '目标节点',
            children: detail.flow_run.target_node_id ?? '全流'
          },
          {
            key: 'started_at',
            label: '开始时间',
            children: formatTimestamp(detail.flow_run.started_at)
          },
          {
            key: 'finished_at',
            label: '结束时间',
            children: formatTimestamp(detail.flow_run.finished_at)
          }
        ]}
        size="small"
      />
    </section>
  );
}

function RunArtifacts({ detail }: { detail: ApplicationRunDetail }) {
  return (
    <section className="application-run-detail__section">
      <div className="application-run-detail__section-header">
        <Typography.Title level={5}>运行输入输出</Typography.Title>
      </div>
      <div className="application-run-detail__json-grid">
        <JsonPreviewBlock
          defaultCollapsed
          height="180px"
          title="运行输入"
          value={detail.flow_run.input_payload}
        />
        <JsonPreviewBlock
          defaultCollapsed
          height="180px"
          title="运行输出"
          value={detail.flow_run.output_payload}
        />
      </div>
    </section>
  );
}

function RunTimeline({ detail }: { detail: ApplicationRunDetail }) {
  return (
    <section className="application-run-detail__section">
      <div className="application-run-detail__section-header">
        <Typography.Title level={5}>事件时间线</Typography.Title>
      </div>
      {detail.events.length === 0 ? (
        <Empty
          description="本次运行没有事件"
          image={Empty.PRESENTED_IMAGE_SIMPLE}
        />
      ) : (
        <Timeline
          items={detail.events.map((event) => ({
            children: (
              <Space direction="vertical" size={2}>
                <Typography.Text strong>{event.event_type}</Typography.Text>
                <Typography.Text type="secondary">
                  {formatTimestamp(event.created_at)}
                </Typography.Text>
                <Typography.Text type="secondary">
                  {summarizeValue(event.payload)}
                </Typography.Text>
              </Space>
            )
          }))}
        />
      )}
    </section>
  );
}

function renderDetail(detail: ApplicationRunDetail) {
  return (
    <div className="application-run-detail__content">
      <RunConversation detail={detail} />
      <NodeRunList detail={detail} />
      <RunMetadata detail={detail} />
      <RunArtifacts detail={detail} />
      <RunTimeline detail={detail} />
    </div>
  );
}

export function ApplicationRunDetailPanel({
  applicationId,
  runId,
  onBack
}: {
  applicationId: string;
  runId: string;
  onBack: () => void;
}) {
  const queryClient = useQueryClient();
  const csrfToken = useAuthStore((state) => state.csrfToken);
  const detailQuery = useQuery({
    queryKey: applicationRunDetailQueryKey(applicationId, runId),
    queryFn: () => fetchApplicationRunDetail(applicationId, runId)
  });
  const resumeMutation = useMutation({
    mutationFn: async ({
      checkpointId,
      inputPayload
    }: {
      checkpointId: string;
      inputPayload: Record<string, unknown>;
    }) => {
      if (!csrfToken) {
        throw new Error('missing runtime resume context');
      }

      return resumeFlowRun(
        applicationId,
        runId,
        checkpointId,
        inputPayload,
        csrfToken
      );
    },
    onSuccess: async (detail) => {
      queryClient.setQueryData(
        applicationRunDetailQueryKey(applicationId, runId),
        detail
      );
      await queryClient.invalidateQueries({
        queryKey: ['applications', applicationId, 'runtime']
      });
    }
  });
  const callbackMutation = useMutation({
    mutationFn: async ({
      callbackTaskId,
      responsePayload
    }: {
      callbackTaskId: string;
      responsePayload: Record<string, unknown>;
    }) => {
      if (!csrfToken) {
        throw new Error('missing callback context');
      }

      return completeCallbackTask(
        applicationId,
        callbackTaskId,
        responsePayload,
        csrfToken
      );
    },
    onSuccess: async (detail) => {
      queryClient.setQueryData(
        applicationRunDetailQueryKey(applicationId, runId),
        detail
      );
      await queryClient.invalidateQueries({
        queryKey: ['applications', applicationId, 'runtime']
      });
    }
  });

  let content = <Result status="info" title="正在加载运行详情" />;

  if (detailQuery.isError) {
    content = <Result status="error" title="运行详情加载失败" />;
  } else if (detailQuery.data) {
    content = (
      <Space direction="vertical" size="large" style={{ width: '100%' }}>
        {renderDetail(detailQuery.data)}
        <ApplicationRunResumeCard
          detail={detailQuery.data}
          onCompleteCallback={(callbackTaskId, responsePayload) =>
            callbackMutation.mutateAsync({ callbackTaskId, responsePayload })
          }
          onResume={(checkpointId, inputPayload) =>
            resumeMutation.mutateAsync({ checkpointId, inputPayload })
          }
        />
      </Space>
    );
  }

  return (
    <div className="application-run-detail">
      <div className="application-run-detail__header">
        <Button icon={<ArrowLeftOutlined />} onClick={onBack}>
          返回日志
        </Button>
        <div>
          <Typography.Title level={4}>运行详情</Typography.Title>
          <Typography.Text type="secondary">{runId}</Typography.Text>
        </div>
      </div>
      {content}
    </div>
  );
}
