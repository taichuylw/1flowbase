import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
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
import { formatDateTime, formatNumber } from '../../../../shared/i18n/format';
import { i18nText } from '../../../../shared/i18n/text';

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

  return formatDateTime(value, { hour12: false });
}

function messageCompatibilityModeLabel(message: AgentFlowDebugMessage) {
  return message.compatibilityModeLabel ?? message.compatibilityMode ?? '—';
}

function formatNullableNumber(value: number | null | undefined) {
  return typeof value === 'number' && Number.isFinite(value)
    ? formatNumber(value)
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
        aria-label={i18nText("agentFlow", "auto.metadata")}
        className="agent-flow-editor__conversation-log-metadata"
      >
        <Typography.Text strong>{i18nText("agentFlow", "auto.metadata")}</Typography.Text>
        <Descriptions
          column={1}
          items={[
            {
              key: 'runId',
              label: i18nText("agentFlow", "auto.run_id"),
              children: message.runId ?? '—'
            },
            {
              key: 'status',
              label: i18nText("agentFlow", "auto.status"),
              children: message.status
            },
            {
              key: 'compatibilityMode',
              label: i18nText("agentFlow", "auto.agreement"),
              children: messageCompatibilityModeLabel(message)
            },
            {
              key: 'totalTokens',
              label: i18nText("agentFlow", "auto.total_tokens"),
              children: formatNullableNumber(message.statistics?.total_tokens)
            },
            {
              key: 'uniqueNodeCount',
              label: i18nText("agentFlow", "auto.real_number_nodes"),
              children: formatNullableNumber(
                message.statistics?.unique_node_count
              )
            },
            {
              key: 'toolCallbackCount',
              label: i18nText("agentFlow", "auto.number_tool_callbacks"),
              children: formatNullableNumber(
                message.statistics?.tool_callback_count
              )
            },
            {
              key: 'startedAt',
              label: i18nText("agentFlow", "auto.start_time"),
              children: formatTimestamp(firstTraceItem?.startedAt)
            },
            {
              key: 'finishedAt',
              label: i18nText("agentFlow", "auto.end_time"),
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
  const traceGroups = useMemo(
    () => groupTraceItemsForDisplay(message.traceSummary),
    [message.traceSummary]
  );

  useEffect(() => {
    setExpandedNodeKey(null);
  }, [message.id]);

  if (message.traceSummary.length === 0) {
    return (
      <div className="agent-flow-editor__conversation-log-empty">
        <Empty
          description={i18nText("agentFlow", "auto.tracking_record_yet")}
          image={Empty.PRESENTED_IMAGE_SIMPLE}
        />
      </div>
    );
  }

  return (
    <div className="agent-flow-editor__conversation-log-trace">
      <div
        aria-label={i18nText("agentFlow", "auto.tracking_nodes")}
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
                aria-label={i18nText("agentFlow", "auto.node_details_alt", { value1: nodeDisplayName(item) })}
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

function useConversationLogArtifactLoader(
  messageId: string,
  onLoadArtifact?: (artifactRef: string) => Promise<unknown>
) {
  const loadArtifactRef = useRef(onLoadArtifact);
  const artifactRequestsRef = useRef<Map<string, Promise<unknown>>>(new Map());
  const hasArtifactLoader = Boolean(onLoadArtifact);

  useEffect(() => {
    loadArtifactRef.current = onLoadArtifact;
  }, [onLoadArtifact]);

  useEffect(() => {
    artifactRequestsRef.current.clear();
  }, [messageId]);

  const loadCachedArtifact = useCallback(
    async (artifactRef: string) => {
      const existingRequest = artifactRequestsRef.current.get(artifactRef);

      if (existingRequest) {
        return existingRequest;
      }

      const loadArtifact = loadArtifactRef.current;
      if (!loadArtifact) {
        throw new Error('missing_conversation_log_artifact_loader');
      }

      const request = loadArtifact(artifactRef).catch((error: unknown) => {
        if (artifactRequestsRef.current.get(artifactRef) === request) {
          artifactRequestsRef.current.delete(artifactRef);
        }
        throw error;
      });

      artifactRequestsRef.current.set(artifactRef, request);
      return request;
    },
    []
  );

  return hasArtifactLoader ? loadCachedArtifact : undefined;
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
  const loadArtifact = useConversationLogArtifactLoader(
    message.id,
    onLoadArtifact
  );

  return (
    <AgentFlowDockPanel
      bodyClassName="agent-flow-editor__conversation-log-body"
      className="agent-flow-editor__conversation-log-panel"
      closeLabel={i18nText("agentFlow", "auto.turn_off_conversation_log")}
      title={i18nText("agentFlow", "auto.conversation_log")}
      onClose={onClose}
    >
      <Tabs
        className="agent-flow-editor__conversation-log-tabs"
        items={[
          {
            key: 'detail',
            label: i18nText("agentFlow", "auto.details"),
            children: (
              <ConversationLogDetail
                message={message}
                onLoadArtifact={loadArtifact}
              />
            )
          },
          {
            key: 'trace',
            label: i18nText("agentFlow", "auto.track"),
            children: (
              <ConversationTrace
                message={message}
                onLoadArtifact={loadArtifact}
              />
            )
          }
        ]}
      />
    </AgentFlowDockPanel>
  );
}
