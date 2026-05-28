import { useEffect, useState } from 'react';
import { Descriptions, Empty, Tabs, Typography } from 'antd';

import type { AgentFlowDebugMessage } from '../../api/runtime';
import { AgentFlowDockPanel } from '../editor/AgentFlowDockPanel';
import { NodeRunPayloadSections } from '../detail/last-run/NodeRunIOCard';
import { AnswerSnapshotTrace } from './conversation/AnswerSnapshotTrace';
import { DebugWorkflowNodeItem } from './conversation/DebugWorkflowNodeRow';
import { LlmToolTraceTree } from './conversation/LlmToolTraceTree';
import {
  groupTraceItemsForDisplay,
  nodeDisplayName
} from './conversation/debug-workflow-trace-utils';
import { stripLlmRoundsFromDebugPayload } from './conversation/llm-tool-callbacks';
import './conversation-log-panel.css';

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

function messageCompatibilityModeLabel(message: AgentFlowDebugMessage) {
  return message.compatibilityModeLabel ?? message.compatibilityMode ?? '—';
}

function formatNullableNumber(value: number | null | undefined) {
  return typeof value === 'number' && Number.isFinite(value)
    ? value.toLocaleString('zh-CN')
    : '-';
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
              key: 'compatibilityMode',
              label: '协议',
              children: messageCompatibilityModeLabel(message)
            },
            {
              key: 'totalTokens',
              label: '总 tokens',
              children: formatNullableNumber(message.statistics?.total_tokens)
            },
            {
              key: 'uniqueNodeCount',
              label: '真实节点数',
              children: formatNullableNumber(
                message.statistics?.unique_node_count
              )
            },
            {
              key: 'toolCallbackCount',
              label: '工具回调次数',
              children: formatNullableNumber(
                message.statistics?.tool_callback_count
              )
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
  const [expandedNodeKey, setExpandedNodeKey] = useState<string | null>(null);

  useEffect(() => {
    setExpandedNodeKey(null);
  }, [message.id]);

  if (message.traceSummary.length === 0) {
    return (
      <div className="agent-flow-editor__conversation-log-empty">
        <Empty
          description="暂无追踪记录"
          image={Empty.PRESENTED_IMAGE_SIMPLE}
        />
      </div>
    );
  }

  const traceGroups = groupTraceItemsForDisplay(message.traceSummary);

  return (
    <div className="agent-flow-editor__conversation-log-trace">
      <div
        aria-label="追踪节点"
        className="agent-flow-editor__conversation-log-node-list"
      >
        {traceGroups.map((group) => {
          const item = group.item;
          const nodeExpanded = group.key === expandedNodeKey;

          return (
            <DebugWorkflowNodeItem
              key={group.key}
              expanded={nodeExpanded}
              item={item}
              onToggle={() =>
                setExpandedNodeKey((current) =>
                  current === group.key ? null : group.key
                )
              }
            >
              <section
                aria-label={`${nodeDisplayName(item)} 节点详情`}
                className="agent-flow-editor__conversation-log-node-detail"
              >
                <div className="agent-flow-editor__conversation-log-json-list">
                  <LlmToolTraceTree
                    debugPayload={item.debugPayload}
                    debugPayloads={group.items.map(
                      (traceItem) => traceItem.debugPayload
                    )}
                    onLoadArtifact={onLoadArtifact}
                  />
                  {item.answerSnapshot ? (
                    <AnswerSnapshotTrace
                      snapshot={item.answerSnapshot}
                      onLoadArtifact={onLoadArtifact}
                    />
                  ) : null}
                  <NodeRunPayloadSections
                    debugPayload={stripLlmRoundsFromDebugPayload(
                      item.debugPayload ?? {}
                    )}
                    inputPayload={item.inputPayload}
                    outputPayload={item.outputPayload}
                    onLoadArtifact={onLoadArtifact}
                  />
                </div>
              </section>
            </DebugWorkflowNodeItem>
          );
        })}
      </div>
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
