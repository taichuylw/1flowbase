import { useEffect, useMemo, useState } from 'react';
import {
  Button,
  Descriptions,
  Empty,
  Space,
  Tabs,
  Tag,
  Typography
} from 'antd';

import type {
  AgentFlowDebugMessage,
  AgentFlowTraceItem
} from '../../api/runtime';
import { getAgentFlowNodeTypeIcon } from '../../lib/node-type-icons';
import { AgentFlowDockPanel } from '../editor/AgentFlowDockPanel';
import { NodeRunPayloadSections } from '../detail/last-run/NodeRunIOCard';
import './conversation-log-panel.css';

function isRecord(value: unknown): value is Record<string, unknown> {
  return Boolean(value && typeof value === 'object' && !Array.isArray(value));
}

function nodeDisplayName(item: AgentFlowTraceItem) {
  if (item.nodeType === 'start') {
    return '用户输入';
  }

  if (item.nodeType === 'answer') {
    return '直接回复';
  }

  return item.nodeAlias;
}

function metricText(item: AgentFlowTraceItem) {
  const duration = item.durationMs == null ? null : `${item.durationMs} ms`;
  const metricsPayload = item.metricsPayload;
  const tokenValue =
    isRecord(metricsPayload) &&
    (typeof metricsPayload.total_tokens === 'number' ||
      typeof metricsPayload.total_tokens === 'string')
      ? metricsPayload.total_tokens
      : null;

  if (tokenValue && duration) {
    return `${tokenValue} tokens · ${duration}`;
  }

  if (tokenValue) {
    return `${tokenValue} tokens`;
  }

  return duration ?? '进行中';
}

function buildDetailInput(message: AgentFlowDebugMessage) {
  const firstTraceItem = message.traceSummary[0];

  if (firstTraceItem && Object.keys(firstTraceItem.inputPayload).length > 0) {
    return firstTraceItem.inputPayload;
  }

  if (firstTraceItem && Object.keys(firstTraceItem.outputPayload).length > 0) {
    return firstTraceItem.outputPayload;
  }

  return {};
}

function buildDetailOutput(message: AgentFlowDebugMessage) {
  if (message.rawOutput) {
    return message.rawOutput;
  }

  const lastTraceItem = message.traceSummary.at(-1);

  if (lastTraceItem && Object.keys(lastTraceItem.outputPayload).length > 0) {
    return lastTraceItem.outputPayload;
  }

  return {
    answer: message.content
  };
}

function formatTimestamp(value: string | null | undefined) {
  if (!value) {
    return '—';
  }

  return new Date(value).toLocaleString('zh-CN', { hour12: false });
}

function ConversationLogDetail({
  message,
  onLoadArtifact
}: {
  message: AgentFlowDebugMessage;
  onLoadArtifact?: (artifactRef: string) => Promise<unknown>;
}) {
  const firstTraceItem = message.traceSummary[0] ?? null;
  const lastTraceItem = message.traceSummary.at(-1) ?? null;

  return (
    <div className="agent-flow-editor__conversation-log-tab">
      <div className="agent-flow-editor__conversation-log-json-list">
        <NodeRunPayloadSections
          debugPayload={{}}
          includeDebugPayload={false}
          inputPayload={buildDetailInput(message)}
          outputPayload={buildDetailOutput(message)}
          onLoadArtifact={onLoadArtifact}
        />
      </div>
      <section
        aria-label="元数据"
        className="agent-flow-editor__conversation-log-metadata"
      >
        <Typography.Text strong>元数据</Typography.Text>
        <Descriptions
          column={1}
          items={[
            {
              key: 'runId',
              label: '运行 ID',
              children: message.runId ?? '—'
            },
            {
              key: 'status',
              label: '状态',
              children: message.status
            },
            {
              key: 'nodeCount',
              label: '节点数',
              children: `${message.traceSummary.length}`
            },
            {
              key: 'startedAt',
              label: '开始时间',
              children: formatTimestamp(firstTraceItem?.startedAt)
            },
            {
              key: 'finishedAt',
              label: '结束时间',
              children: formatTimestamp(lastTraceItem?.finishedAt)
            }
          ]}
          size="small"
        />
      </section>
    </div>
  );
}

function ConversationTrace({
  message,
  onLoadArtifact
}: {
  message: AgentFlowDebugMessage;
  onLoadArtifact?: (artifactRef: string) => Promise<unknown>;
}) {
  const [selectedNodeKey, setSelectedNodeKey] = useState<string | null>(
    message.traceSummary[0]?.nodeRunId ??
      message.traceSummary[0]?.nodeId ??
      null
  );
  const selectedNode = useMemo(() => {
    if (message.traceSummary.length === 0) {
      return null;
    }

    return (
      message.traceSummary.find(
        (item) => (item.nodeRunId ?? item.nodeId) === selectedNodeKey
      ) ?? message.traceSummary[0]
    );
  }, [message.traceSummary, selectedNodeKey]);

  useEffect(() => {
    setSelectedNodeKey(
      message.traceSummary[0]?.nodeRunId ??
        message.traceSummary[0]?.nodeId ??
        null
    );
  }, [message.id, message.traceSummary]);

  if (message.traceSummary.length === 0 || !selectedNode) {
    return (
      <div className="agent-flow-editor__conversation-log-empty">
        <Empty
          description="暂无追踪记录"
          image={Empty.PRESENTED_IMAGE_SIMPLE}
        />
      </div>
    );
  }

  return (
    <div className="agent-flow-editor__conversation-log-trace">
      <div
        aria-label="追踪节点"
        className="agent-flow-editor__conversation-log-node-list"
      >
        {message.traceSummary.map((item) => {
          const itemKey = item.nodeRunId ?? item.nodeId;
          const selected =
            itemKey === (selectedNode.nodeRunId ?? selectedNode.nodeId);

          return (
            <Button
              key={itemKey}
              aria-pressed={selected}
              className="agent-flow-editor__conversation-log-node"
              type={selected ? 'primary' : 'default'}
              onClick={() => setSelectedNodeKey(itemKey)}
            >
              <span
                aria-label={`${item.nodeType} 节点类型`}
                className="agent-flow-editor__conversation-log-node-icon"
                role="img"
              >
                {getAgentFlowNodeTypeIcon(item.nodeType)}
              </span>
              <span className="agent-flow-editor__conversation-log-node-main">
                <Typography.Text strong>
                  {nodeDisplayName(item)}
                </Typography.Text>
                <Typography.Text type="secondary">
                  {metricText(item)}
                </Typography.Text>
              </span>
              <Tag>{item.status}</Tag>
            </Button>
          );
        })}
      </div>
      <section
        aria-label={`${nodeDisplayName(selectedNode)} 节点详情`}
        className="agent-flow-editor__conversation-log-node-detail"
      >
        <div className="agent-flow-editor__conversation-log-node-detail-header">
          <Space size={8}>
            <span
              aria-label={`${selectedNode.nodeType} 节点类型`}
              className="agent-flow-editor__conversation-log-node-icon"
              role="img"
            >
              {getAgentFlowNodeTypeIcon(selectedNode.nodeType)}
            </span>
            <Typography.Text strong>
              {nodeDisplayName(selectedNode)}
            </Typography.Text>
          </Space>
          <Typography.Text type="secondary">
            {selectedNode.nodeType}
          </Typography.Text>
        </div>
        <div className="agent-flow-editor__conversation-log-json-list">
          <NodeRunPayloadSections
            debugPayload={selectedNode.debugPayload ?? {}}
            inputPayload={selectedNode.inputPayload}
            outputPayload={selectedNode.outputPayload}
            onLoadArtifact={onLoadArtifact}
          />
        </div>
      </section>
    </div>
  );
}

export function ConversationLogPanel({
  message,
  onClose,
  onLoadArtifact
}: {
  message: AgentFlowDebugMessage;
  onClose: () => void;
  onLoadArtifact?: (artifactRef: string) => Promise<unknown>;
}) {
  return (
    <AgentFlowDockPanel
      bodyClassName="agent-flow-editor__conversation-log-body"
      className="agent-flow-editor__conversation-log-panel"
      closeLabel="关闭对话日志"
      title="对话日志"
      onClose={onClose}
    >
      <Tabs
        className="agent-flow-editor__conversation-log-tabs"
        items={[
          {
            key: 'detail',
            label: '详情',
            children: (
              <ConversationLogDetail
                message={message}
                onLoadArtifact={onLoadArtifact}
              />
            )
          },
          {
            key: 'trace',
            label: '追踪',
            children: (
              <ConversationTrace
                message={message}
                onLoadArtifact={onLoadArtifact}
              />
            )
          }
        ]}
      />
    </AgentFlowDockPanel>
  );
}
