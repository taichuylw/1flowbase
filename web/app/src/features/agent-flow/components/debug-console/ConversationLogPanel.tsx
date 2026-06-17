import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { Descriptions, Empty, Spin, Tabs, Typography } from 'antd';

import type {
  AgentFlowDebugMessage,
  AgentFlowTraceItem
} from '../../api/runtime';
import { AgentFlowDockPanel } from '../editor/AgentFlowDockPanel';
import { NodeRunPayloadSections } from '../detail/last-run/NodeRunIOCard';
import { DebugWorkflowNodeItem } from './conversation/DebugWorkflowNodeRow';
import { DebugWorkflowNodeDetailContent } from './conversation/LlmToolTraceTree';
import {
  groupTraceItemsForDisplay,
  nodeDisplayName
} from './conversation/debug-workflow-trace-utils';
import './conversation-log-panel.css';
import { formatDateTime, formatNumber } from '../../../../shared/i18n/format';
import { i18nText } from '../../../../shared/i18n/text';

interface ConversationLogTraceNodeSummary {
  trace_node_id: string;
  node_kind: string;
  node_run_id?: string | null;
  node_id?: string | null;
  node_type?: string | null;
  node_alias: string;
  status: string;
  started_at: string;
  finished_at?: string | null;
  duration_ms?: number | null;
  metrics_payload?: Record<string, unknown>;
  has_children: boolean;
  has_content: boolean;
}

interface ConversationLogNodeRunContent {
  id: string;
  node_id: string;
  node_type: string;
  node_alias: string;
  status: string;
  input_payload: Record<string, unknown>;
  output_payload: Record<string, unknown>;
  error_payload: Record<string, unknown> | null;
  metrics_payload: Record<string, unknown>;
  debug_payload?: Record<string, unknown>;
  started_at: string;
  finished_at: string | null;
}

interface ConversationLogTraceTree {
  nodes: ConversationLogTraceNodeSummary[];
}

interface ConversationLogTraceNodeChildren {
  items: ConversationLogTraceNodeSummary[];
}

interface ConversationLogTraceNodeContent {
  trace_node_id: string;
  node_kind: string;
  node_run?: ConversationLogNodeRunContent | null;
}

interface ConversationLogTraceNodeGroup {
  key: string;
  item: AgentFlowTraceItem;
  nodes: ConversationLogTraceNodeSummary[];
}

export interface ConversationLogTraceLoader {
  loadTree: (runId: string) => Promise<ConversationLogTraceTree>;
  loadChildren: (
    runId: string,
    traceNodeId: string
  ) => Promise<ConversationLogTraceNodeChildren>;
  loadContent: (
    runId: string,
    traceNodeId: string
  ) => Promise<ConversationLogTraceNodeContent>;
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
        aria-label={i18nText('agentFlow', 'auto.metadata')}
        className="agent-flow-editor__conversation-log-metadata"
      >
        <Typography.Text strong>
          {i18nText('agentFlow', 'auto.metadata')}
        </Typography.Text>
        <Descriptions
          column={1}
          items={[
            {
              key: 'runId',
              label: i18nText('agentFlow', 'auto.run_id'),
              children: message.runId ?? '—'
            },
            {
              key: 'status',
              label: i18nText('agentFlow', 'auto.status'),
              children: message.status
            },
            {
              key: 'compatibilityMode',
              label: i18nText('agentFlow', 'auto.agreement'),
              children: messageCompatibilityModeLabel(message)
            },
            {
              key: 'totalTokens',
              label: i18nText('agentFlow', 'auto.total_tokens'),
              children: formatNullableNumber(message.statistics?.total_tokens)
            },
            {
              key: 'uniqueNodeCount',
              label: i18nText('agentFlow', 'auto.real_number_nodes'),
              children: formatNullableNumber(
                message.statistics?.unique_node_count
              )
            },
            {
              key: 'toolCallbackCount',
              label: i18nText('agentFlow', 'auto.number_tool_callbacks'),
              children: formatNullableNumber(
                message.statistics?.tool_callback_count
              )
            },
            {
              key: 'startedAt',
              label: i18nText('agentFlow', 'auto.start_time'),
              children: formatTimestamp(firstTraceItem?.startedAt)
            },
            {
              key: 'finishedAt',
              label: i18nText('agentFlow', 'auto.end_time'),
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
  onLoadArtifact,
  traceLoader
}: {
  message: AgentFlowDebugMessage;
  onLoadArtifact?: (artifactRef: string) => Promise<unknown>;
  traceLoader?: ConversationLogTraceLoader;
}) {
  const traceRunId = message.detailRunId ?? message.runId;

  if (traceLoader && traceRunId) {
    return (
      <LazyConversationTrace
        key={`${message.id}:${traceRunId}`}
        onLoadArtifact={onLoadArtifact}
        runId={traceRunId}
        traceLoader={traceLoader}
      />
    );
  }

  return (
    <ConversationTraceContent
      key={message.id}
      message={message}
      onLoadArtifact={onLoadArtifact}
    />
  );
}

function mapTraceSummaryToTraceItem(
  summary: ConversationLogTraceNodeSummary
): AgentFlowTraceItem {
  return {
    nodeId: summary.node_id ?? summary.trace_node_id,
    nodeRunId: summary.node_run_id ?? summary.trace_node_id,
    nodeAlias: summary.node_alias,
    nodeType: summary.node_type ?? summary.node_kind,
    status: summary.status,
    startedAt: summary.started_at,
    finishedAt: summary.finished_at ?? null,
    durationMs: summary.duration_ms ?? null,
    inputPayload: {},
    outputPayload: {},
    errorPayload: null,
    metricsPayload: summary.metrics_payload ?? {},
    debugPayload: {}
  };
}

function mapTraceContentToTraceItem(
  fallback: AgentFlowTraceItem,
  content: ConversationLogTraceNodeContent | undefined
): AgentFlowTraceItem {
  const nodeRun = content?.node_run;

  if (!nodeRun) {
    return fallback;
  }

  return {
    nodeId: nodeRun.node_id,
    nodeRunId: nodeRun.id,
    nodeAlias: nodeRun.node_alias,
    nodeType: nodeRun.node_type,
    status: nodeRun.status,
    startedAt: nodeRun.started_at,
    finishedAt: nodeRun.finished_at,
    durationMs: traceItemDurationMs(nodeRun.started_at, nodeRun.finished_at),
    inputPayload: nodeRun.input_payload,
    outputPayload: nodeRun.output_payload,
    errorPayload: nodeRun.error_payload,
    metricsPayload: nodeRun.metrics_payload,
    debugPayload: nodeRun.debug_payload ?? {}
  };
}

function traceItemDurationMs(startedAt: string, finishedAt: string | null) {
  if (!finishedAt) {
    return null;
  }

  return Math.max(
    new Date(finishedAt).getTime() - new Date(startedAt).getTime(),
    0
  );
}

function groupTraceNodeSummariesForDisplay(
  nodes: ConversationLogTraceNodeSummary[]
): ConversationLogTraceNodeGroup[] {
  const entries = nodes.map((node) => ({
    node,
    item: mapTraceSummaryToTraceItem(node)
  }));
  const nodeByItemKey = new Map<string, ConversationLogTraceNodeSummary>();

  for (const entry of entries) {
    nodeByItemKey.set(entry.item.nodeRunId ?? entry.item.nodeId, entry.node);
  }

  return groupTraceItemsForDisplay(entries.map((entry) => entry.item)).map(
    (group) => ({
      key: group.key,
      item: group.item,
      nodes: group.items
        .map((item) => nodeByItemKey.get(item.nodeRunId ?? item.nodeId))
        .filter((node): node is ConversationLogTraceNodeSummary => Boolean(node))
    })
  );
}

function mergeLoadedTraceItemsForDisplay(
  fallback: AgentFlowTraceItem,
  items: AgentFlowTraceItem[]
) {
  if (items.length === 0) {
    return fallback;
  }

  return groupTraceItemsForDisplay(items).at(0)?.item ?? fallback;
}

function LazyConversationTrace({
  onLoadArtifact,
  runId,
  traceLoader
}: {
  onLoadArtifact?: (artifactRef: string) => Promise<unknown>;
  runId: string;
  traceLoader: ConversationLogTraceLoader;
}) {
  const traceTreeQuery = useQuery({
    queryKey: ['conversation-log-trace-tree', runId],
    queryFn: () => traceLoader.loadTree(runId)
  });

  if (traceTreeQuery.isLoading) {
    return (
      <div className="agent-flow-editor__conversation-log-empty">
        <Spin />
      </div>
    );
  }

  const nodes = traceTreeQuery.data?.nodes ?? [];

  if (nodes.length === 0) {
    return (
      <div className="agent-flow-editor__conversation-log-empty">
        <Empty
          description={i18nText('agentFlow', 'auto.tracking_record_yet')}
          image={Empty.PRESENTED_IMAGE_SIMPLE}
        />
      </div>
    );
  }

  return (
    <div className="agent-flow-editor__conversation-log-trace">
      <LazyTraceNodeList
        nodes={nodes}
        onLoadArtifact={onLoadArtifact}
        runId={runId}
        traceLoader={traceLoader}
      />
    </div>
  );
}

function LazyTraceNodeList({
  nodes,
  onLoadArtifact,
  runId,
  traceLoader
}: {
  nodes: ConversationLogTraceNodeSummary[];
  onLoadArtifact?: (artifactRef: string) => Promise<unknown>;
  runId: string;
  traceLoader: ConversationLogTraceLoader;
}) {
  const groups = useMemo(() => groupTraceNodeSummariesForDisplay(nodes), [nodes]);

  return (
    <div
      aria-label={i18nText('agentFlow', 'auto.tracking_nodes')}
      className="agent-flow-editor__conversation-log-node-list"
    >
      {groups.map((group) => (
        <LazyTraceNodeGroupItem
          key={group.key}
          group={group}
          onLoadArtifact={onLoadArtifact}
          runId={runId}
          traceLoader={traceLoader}
        />
      ))}
    </div>
  );
}

function LazyTraceNodeGroupItem({
  group,
  onLoadArtifact,
  runId,
  traceLoader
}: {
  group: ConversationLogTraceNodeGroup;
  onLoadArtifact?: (artifactRef: string) => Promise<unknown>;
  runId: string;
  traceLoader: ConversationLogTraceLoader;
}) {
  const [expanded, setExpanded] = useState(false);
  const traceNodeIds = useMemo(
    () => group.nodes.map((node) => node.trace_node_id),
    [group.nodes]
  );
  const traceNodeIdsKey = traceNodeIds.join('|');
  const contentQuery = useQuery({
    enabled: expanded && group.nodes.some((node) => node.has_content),
    queryKey: [
      'conversation-log-trace-node-group-content',
      runId,
      traceNodeIdsKey
    ],
    queryFn: async () =>
      Promise.all(
        group.nodes
          .filter((node) => node.has_content)
          .map(async (node) => ({
            node,
            content: await traceLoader.loadContent(runId, node.trace_node_id)
          }))
      )
  });
  const childrenQuery = useQuery({
    enabled: expanded && group.nodes.some((node) => node.has_children),
    queryKey: [
      'conversation-log-trace-node-group-children',
      runId,
      traceNodeIdsKey
    ],
    queryFn: async () => {
      const responses = await Promise.all(
        group.nodes
          .filter((node) => node.has_children)
          .map((node) => traceLoader.loadChildren(runId, node.trace_node_id))
      );

      return responses.flatMap((response) => response.items);
    }
  });
  const loadedItems = useMemo(
    () =>
      contentQuery.data?.map(({ node, content }) =>
        mapTraceContentToTraceItem(mapTraceSummaryToTraceItem(node), content)
      ) ?? [],
    [contentQuery.data]
  );
  const item = useMemo(
    () => mergeLoadedTraceItemsForDisplay(group.item, loadedItems),
    [group.item, loadedItems]
  );
  const childNodes = childrenQuery.data ?? [];

  return (
    <DebugWorkflowNodeItem
      expanded={expanded}
      item={item}
      onToggle={() => setExpanded((current) => !current)}
    >
      <section
        aria-label={i18nText('agentFlow', 'auto.node_details_alt', {
          value1: nodeDisplayName(group.item)
        })}
        className="agent-flow-editor__conversation-log-node-detail"
      >
        {contentQuery.isLoading ? (
          <Spin />
        ) : (
          <div className="agent-flow-editor__conversation-log-json-list">
            <DebugWorkflowNodeDetailContent
              item={item}
              onLoadArtifact={onLoadArtifact}
            />
          </div>
        )}
        {childNodes.length > 0 ? (
          <LazyTraceNodeList
            nodes={childNodes}
            onLoadArtifact={onLoadArtifact}
            runId={runId}
            traceLoader={traceLoader}
          />
        ) : null}
      </section>
    </DebugWorkflowNodeItem>
  );
}

function ConversationTraceContent({
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

  if (message.traceSummary.length === 0) {
    return (
      <div className="agent-flow-editor__conversation-log-empty">
        <Empty
          description={i18nText('agentFlow', 'auto.tracking_record_yet')}
          image={Empty.PRESENTED_IMAGE_SIMPLE}
        />
      </div>
    );
  }

  return (
    <div className="agent-flow-editor__conversation-log-trace">
      <div
        aria-label={i18nText('agentFlow', 'auto.tracking_nodes')}
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
                aria-label={i18nText('agentFlow', 'auto.node_details_alt', {
                  value1: nodeDisplayName(item)
                })}
                className="agent-flow-editor__conversation-log-node-detail"
              >
                <div className="agent-flow-editor__conversation-log-json-list">
                  <DebugWorkflowNodeDetailContent
                    item={item}
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
  const artifactRequestsRef = useRef<Map<string, Promise<unknown>> | null>(
    null
  );
  const hasArtifactLoader = Boolean(onLoadArtifact);

  useEffect(() => {
    loadArtifactRef.current = onLoadArtifact;
  }, [onLoadArtifact]);

  useEffect(() => {
    artifactRequestsRef.current?.clear();
  }, [messageId]);

  const loadCachedArtifact = useCallback(async (artifactRef: string) => {
    if (artifactRequestsRef.current === null) {
      artifactRequestsRef.current = new Map();
    }

    const artifactRequests = artifactRequestsRef.current;
    const existingRequest = artifactRequests.get(artifactRef);

    if (existingRequest) {
      return existingRequest;
    }

    const loadArtifact = loadArtifactRef.current;
    if (!loadArtifact) {
      throw new Error('missing_conversation_log_artifact_loader');
    }

    const request = loadArtifact(artifactRef).catch((error: unknown) => {
      if (artifactRequests.get(artifactRef) === request) {
        artifactRequests.delete(artifactRef);
      }
      throw error;
    });

    artifactRequests.set(artifactRef, request);
    return request;
  }, []);

  return hasArtifactLoader ? loadCachedArtifact : undefined;
}

export function ConversationLogPanel({
  message,
  onClose,
  onLoadArtifact,
  traceLoader
}: {
  message: AgentFlowDebugMessage;
  onClose: () => void;
  onLoadArtifact?: (artifactRef: string) => Promise<unknown>;
  traceLoader?: ConversationLogTraceLoader;
}) {
  const loadArtifact = useConversationLogArtifactLoader(
    message.id,
    onLoadArtifact
  );
  const [activeTabKey, setActiveTabKey] = useState('detail');

  return (
    <AgentFlowDockPanel
      bodyClassName="agent-flow-editor__conversation-log-body"
      className="agent-flow-editor__conversation-log-panel"
      closeLabel={i18nText('agentFlow', 'auto.turn_off_conversation_log')}
      title={i18nText('agentFlow', 'auto.conversation_log')}
      onClose={onClose}
    >
      <Tabs
        activeKey={activeTabKey}
        className="agent-flow-editor__conversation-log-tabs"
        items={[
          {
            key: 'detail',
            label: i18nText('agentFlow', 'auto.details'),
            children: (
              <ConversationLogDetail
                message={message}
                onLoadArtifact={loadArtifact}
              />
            )
          },
          {
            key: 'trace',
            label: i18nText('agentFlow', 'auto.track'),
            children:
              activeTabKey === 'trace' ? (
                <ConversationTrace
                  message={message}
                  onLoadArtifact={loadArtifact}
                  traceLoader={traceLoader}
                />
              ) : null
          }
        ]}
        onChange={setActiveTabKey}
      />
    </AgentFlowDockPanel>
  );
}
