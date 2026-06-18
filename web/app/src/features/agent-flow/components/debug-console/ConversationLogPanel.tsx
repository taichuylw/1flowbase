import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { Alert, Descriptions, Empty, Spin, Tabs, Typography } from 'antd';

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
  stable_locator?: string;
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

interface ConversationLogTraceProjectionStatus {
  projection_status:
    | 'pending'
    | 'running'
    | 'succeeded'
    | 'failed'
    | 'stale'
    | 'partial';
  projection_version: number;
  source_watermark: string;
  attempt_count: number;
  last_attempt_at?: string | null;
  last_success_at?: string | null;
  last_error_code?: string | null;
  last_error_stage?: string | null;
  last_error_source_kind?: string | null;
  last_error_source_locator?: string | null;
  last_error_ref?: string | null;
  retriable: boolean;
}

interface ConversationLogTraceTree {
  projection_status?: ConversationLogTraceProjectionStatus;
  nodes: ConversationLogTraceNodeSummary[];
}

interface ConversationLogTraceNodeChildren {
  projection_status?: ConversationLogTraceProjectionStatus;
  items: ConversationLogTraceNodeSummary[];
}

interface ConversationLogTraceNodeContent {
  trace_node_id: string;
  node_kind: string;
  projection_status?: ConversationLogTraceProjectionStatus;
  node_run?: ConversationLogNodeRunContent | null;
}

interface ConversationLogRunOverview {
  run: {
    id: string;
    status?: string;
    compatibility_mode?: string | null;
    started_at?: string | null;
    finished_at?: string | null;
  };
  statistics?: {
    total_tokens?: number | null;
    unique_node_count?: number | null;
    tool_callback_count?: number | null;
  };
  flow_run: {
    id: string;
    status: string;
    input_payload: Record<string, unknown>;
    output_payload: Record<string, unknown>;
    error_payload?: Record<string, unknown> | null;
    started_at: string;
    finished_at?: string | null;
  };
  answer_snapshot?: {
    text: string;
    output_payload: Record<string, unknown>;
  } | null;
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
  loadToolCallbackDetail?: (
    runId: string,
    traceNodeId: string,
    toolCallId: string
  ) => Promise<unknown>;
}

export interface ConversationLogOverviewLoader {
  loadOverview: (runId: string) => Promise<ConversationLogRunOverview>;
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

function overviewCompatibilityModeLabel(
  message: AgentFlowDebugMessage,
  overview: ConversationLogRunOverview | undefined
) {
  return (
    overview?.run.compatibility_mode ?? messageCompatibilityModeLabel(message)
  );
}

function formatNullableNumber(value: number | null | undefined) {
  return typeof value === 'number' && Number.isFinite(value)
    ? formatNumber(value)
    : '-';
}

function overviewDetailInput(
  message: AgentFlowDebugMessage,
  overview: ConversationLogRunOverview | undefined
) {
  return overview?.flow_run.input_payload ?? buildDetailInput(message);
}

function overviewDetailOutput(
  message: AgentFlowDebugMessage,
  overview: ConversationLogRunOverview | undefined
) {
  if (!overview) {
    return buildDetailOutput(message);
  }

  if (Object.keys(overview.flow_run.output_payload).length > 0) {
    return overview.flow_run.output_payload;
  }

  const answerPayload = overview.answer_snapshot?.output_payload;
  if (answerPayload && Object.keys(answerPayload).length > 0) {
    return answerPayload;
  }

  return {
    answer: overview.answer_snapshot?.text ?? message.content
  };
}

function ConversationLogDetailContent({
  message,
  onLoadArtifact,
  overview
}: {
  message: AgentFlowDebugMessage;
  onLoadArtifact?: (artifactRef: string) => Promise<unknown>;
  overview?: ConversationLogRunOverview;
}) {
  const firstTraceItem = message.traceSummary[0] ?? null;
  const lastTraceItem = message.traceSummary.at(-1) ?? null;
  const startedAt =
    overview?.flow_run.started_at ??
    overview?.run.started_at ??
    firstTraceItem?.startedAt;
  const finishedAt =
    overview?.flow_run.finished_at ??
    overview?.run.finished_at ??
    lastTraceItem?.finishedAt;

  return (
    <div className="agent-flow-editor__conversation-log-tab">
      <div className="agent-flow-editor__conversation-log-json-list">
        <NodeRunPayloadSections
          debugPayload={{}}
          includeDebugPayload={false}
          inputPayload={overviewDetailInput(message, overview)}
          outputPayload={overviewDetailOutput(message, overview)}
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
              children: overview?.flow_run.id ?? message.runId ?? '—'
            },
            {
              key: 'status',
              label: i18nText('agentFlow', 'auto.status'),
              children: overview?.flow_run.status ?? message.status
            },
            {
              key: 'compatibilityMode',
              label: i18nText('agentFlow', 'auto.agreement'),
              children: overviewCompatibilityModeLabel(message, overview)
            },
            {
              key: 'totalTokens',
              label: i18nText('agentFlow', 'auto.total_tokens'),
              children: formatNullableNumber(
                overview?.statistics?.total_tokens ??
                  message.statistics?.total_tokens
              )
            },
            {
              key: 'uniqueNodeCount',
              label: i18nText('agentFlow', 'auto.real_number_nodes'),
              children: formatNullableNumber(
                overview?.statistics?.unique_node_count ??
                  message.statistics?.unique_node_count
              )
            },
            {
              key: 'toolCallbackCount',
              label: i18nText('agentFlow', 'auto.number_tool_callbacks'),
              children: formatNullableNumber(
                overview?.statistics?.tool_callback_count ??
                  message.statistics?.tool_callback_count
              )
            },
            {
              key: 'startedAt',
              label: i18nText('agentFlow', 'auto.start_time'),
              children: formatTimestamp(startedAt)
            },
            {
              key: 'finishedAt',
              label: i18nText('agentFlow', 'auto.end_time'),
              children: formatTimestamp(finishedAt)
            }
          ]}
          size="small"
        />
      </section>
    </div>
  );
}

function ConversationLogLazyDetail({
  message,
  onLoadArtifact,
  overviewLoader,
  overviewRunId
}: {
  message: AgentFlowDebugMessage;
  onLoadArtifact?: (artifactRef: string) => Promise<unknown>;
  overviewLoader: ConversationLogOverviewLoader;
  overviewRunId: string;
}) {
  const overviewQuery = useQuery({
    queryKey: ['conversation-log-run-overview', overviewRunId],
    queryFn: () => overviewLoader.loadOverview(overviewRunId)
  });

  if (overviewQuery.isLoading) {
    return (
      <div className="agent-flow-editor__conversation-log-empty">
        <Spin />
      </div>
    );
  }

  return (
    <ConversationLogDetailContent
      message={message}
      overview={overviewQuery.data}
      onLoadArtifact={onLoadArtifact}
    />
  );
}

function ConversationLogDetail({
  message,
  onLoadArtifact,
  overviewLoader
}: {
  message: AgentFlowDebugMessage;
  onLoadArtifact?: (artifactRef: string) => Promise<unknown>;
  overviewLoader?: ConversationLogOverviewLoader;
}) {
  const overviewRunId = message.detailRunId ?? message.runId;

  if (overviewLoader && overviewRunId) {
    return (
      <ConversationLogLazyDetail
        message={message}
        overviewLoader={overviewLoader}
        overviewRunId={overviewRunId}
        onLoadArtifact={onLoadArtifact}
      />
    );
  }

  return (
    <ConversationLogDetailContent
      message={message}
      onLoadArtifact={onLoadArtifact}
    />
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

function traceProjectionStatusSucceeded(
  status: ConversationLogTraceProjectionStatus | undefined
) {
  return !status || status.projection_status === 'succeeded';
}

function traceProjectionStatusMessage(
  status: ConversationLogTraceProjectionStatus
) {
  switch (status.projection_status) {
    case 'pending':
      return i18nText('agentFlow', 'auto.trace_projection_pending');
    case 'running':
      return i18nText('agentFlow', 'auto.trace_projection_running');
    case 'failed':
      return i18nText('agentFlow', 'auto.trace_projection_failed');
    case 'stale':
      return i18nText('agentFlow', 'auto.trace_projection_stale');
    case 'partial':
      return i18nText('agentFlow', 'auto.trace_projection_partial');
    case 'succeeded':
      return i18nText('agentFlow', 'auto.trace_projection_succeeded');
  }
}

function TraceProjectionStatusNotice({
  status
}: {
  status: ConversationLogTraceProjectionStatus;
}) {
  return (
    <Alert
      className="agent-flow-editor__conversation-log-projection-status"
      message={traceProjectionStatusMessage(status)}
      showIcon
      type={status.projection_status === 'failed' ? 'error' : 'info'}
    />
  );
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

  const projectionStatus = traceTreeQuery.data?.projection_status;
  const nodes = traceTreeQuery.data?.nodes ?? [];

  if (!traceProjectionStatusSucceeded(projectionStatus) && projectionStatus) {
    return (
      <div className="agent-flow-editor__conversation-log-trace">
        <TraceProjectionStatusNotice status={projectionStatus} />
      </div>
    );
  }

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
  return (
    <div
      aria-label={i18nText('agentFlow', 'auto.tracking_nodes')}
      className="agent-flow-editor__conversation-log-node-list"
    >
      {nodes.map((node) => (
        <LazyTraceNodeItem
          key={node.trace_node_id}
          node={node}
          onLoadArtifact={onLoadArtifact}
          runId={runId}
          traceLoader={traceLoader}
        />
      ))}
    </div>
  );
}

function LazyTraceNodeItem({
  node,
  onLoadArtifact,
  runId,
  traceLoader
}: {
  node: ConversationLogTraceNodeSummary;
  onLoadArtifact?: (artifactRef: string) => Promise<unknown>;
  runId: string;
  traceLoader: ConversationLogTraceLoader;
}) {
  const [expanded, setExpanded] = useState(false);
  const fallbackItem = useMemo(() => mapTraceSummaryToTraceItem(node), [node]);
  const contentQuery = useQuery({
    enabled: expanded && node.has_content,
    queryKey: [
      'conversation-log-trace-node-content',
      runId,
      node.trace_node_id
    ],
    queryFn: () => traceLoader.loadContent(runId, node.trace_node_id)
  });
  const childrenQuery = useQuery({
    enabled: expanded && node.has_children,
    queryKey: [
      'conversation-log-trace-node-children',
      runId,
      node.trace_node_id
    ],
    queryFn: () => traceLoader.loadChildren(runId, node.trace_node_id)
  });
  const item = useMemo(
    () => mapTraceContentToTraceItem(fallbackItem, contentQuery.data),
    [contentQuery.data, fallbackItem]
  );
  const childNodes = childrenQuery.data?.items ?? [];
  const childProjectionStatus = childrenQuery.data?.projection_status;
  const contentProjectionStatus = contentQuery.data?.projection_status;
  const loadToolCallbackDetail = traceLoader.loadToolCallbackDetail;

  return (
    <DebugWorkflowNodeItem
      expanded={expanded}
      item={item}
      onToggle={() => setExpanded((current) => !current)}
    >
      <section
        aria-label={i18nText('agentFlow', 'auto.node_details_alt', {
          value1: nodeDisplayName(fallbackItem)
        })}
        className="agent-flow-editor__conversation-log-node-detail"
      >
        {node.has_content ? (
          contentQuery.isLoading ? (
            <Spin />
          ) : contentProjectionStatus &&
            !traceProjectionStatusSucceeded(contentProjectionStatus) ? (
            <TraceProjectionStatusNotice status={contentProjectionStatus} />
          ) : (
            <div className="agent-flow-editor__conversation-log-json-list">
              <DebugWorkflowNodeDetailContent
                item={item}
                onLoadArtifact={onLoadArtifact}
                onLoadToolCallbackDetail={
                  loadToolCallbackDetail
                    ? (toolCallId) =>
                        loadToolCallbackDetail(
                          runId,
                          node.trace_node_id,
                          toolCallId
                        )
                    : undefined
                }
              />
            </div>
          )
        ) : null}
        {childrenQuery.isLoading ? <Spin /> : null}
        {childProjectionStatus &&
        !traceProjectionStatusSucceeded(childProjectionStatus) ? (
          <TraceProjectionStatusNotice status={childProjectionStatus} />
        ) : null}
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
  overviewLoader,
  traceLoader
}: {
  message: AgentFlowDebugMessage;
  onClose: () => void;
  onLoadArtifact?: (artifactRef: string) => Promise<unknown>;
  overviewLoader?: ConversationLogOverviewLoader;
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
                overviewLoader={overviewLoader}
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
