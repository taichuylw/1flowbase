import { useEffect, useState } from 'react';
import { Descriptions, Empty, Tabs, Typography } from 'antd';

import type { AgentFlowDebugMessage } from '../../api/runtime';
import { AgentFlowDockPanel } from '../editor/AgentFlowDockPanel';
import { NodeRunPayloadSections } from '../detail/last-run/NodeRunIOCard';
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
        aria-label={i18nText("agentFlow", "auto.key_nljodhfffg")}
        className="agent-flow-editor__conversation-log-metadata"
      >
        <Typography.Text strong>{i18nText("agentFlow", "auto.key_nljodhfffg")}</Typography.Text>
        <Descriptions
          column={1}
          items={[
            {
              key: 'runId',
              label: i18nText("agentFlow", "auto.key_mbbijacdpl"),
              children: message.runId ?? '—'
            },
            {
              key: 'status',
              label: i18nText("agentFlow", "auto.key_gcojfbkgjc"),
              children: message.status
            },
            {
              key: 'compatibilityMode',
              label: i18nText("agentFlow", "auto.key_lamedbghfl"),
              children: messageCompatibilityModeLabel(message)
            },
            {
              key: 'totalTokens',
              label: i18nText("agentFlow", "auto.key_bfbnomhojn"),
              children: formatNullableNumber(message.statistics?.total_tokens)
            },
            {
              key: 'uniqueNodeCount',
              label: i18nText("agentFlow", "auto.key_hpbmcmmpab"),
              children: formatNullableNumber(
                message.statistics?.unique_node_count
              )
            },
            {
              key: 'toolCallbackCount',
              label: i18nText("agentFlow", "auto.key_lpffmmgkgj"),
              children: formatNullableNumber(
                message.statistics?.tool_callback_count
              )
            },
            {
              key: 'startedAt',
              label: i18nText("agentFlow", "auto.key_oiigikpgol"),
              children: formatTimestamp(firstTraceItem?.startedAt)
            },
            {
              key: 'finishedAt',
              label: i18nText("agentFlow", "auto.key_kalljpejkl"),
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
          description={i18nText("agentFlow", "auto.key_fhlmabmhfl")}
          image={Empty.PRESENTED_IMAGE_SIMPLE}
        />
      </div>
    );
  }

  const traceGroups = groupTraceItemsForDisplay(message.traceSummary);

  return (
    <div className="agent-flow-editor__conversation-log-trace">
      <div
        aria-label={i18nText("agentFlow", "auto.key_pggfflkfcn")}
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
                aria-label={i18nText("agentFlow", "auto.key_gaadjpbcol", { value1: nodeDisplayName(item) })}
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
      closeLabel={i18nText("agentFlow", "auto.key_ncjbnpojeo")}
      title={i18nText("agentFlow", "auto.key_fhgmcinggi")}
      onClose={onClose}
    >
      <Tabs
        className="agent-flow-editor__conversation-log-tabs"
        items={[
          {
            key: 'detail',
            label: i18nText("agentFlow", "auto.key_epffoobogi"),
            children: (
              <ConversationLogDetail
                message={message}
                onLoadArtifact={onLoadArtifact}
              />
            )
          },
          {
            key: 'trace',
            label: i18nText("agentFlow", "auto.key_ijghndmgnc"),
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
