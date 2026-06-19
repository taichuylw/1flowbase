import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import {
  Alert,
  Button,
  Descriptions,
  Empty,
  Spin,
  Tabs,
  Typography
} from 'antd';

import type {
  AgentFlowDebugMessage,
  AgentFlowTraceItem
} from '../../api/runtime';
import { AgentFlowDockPanel } from '../editor/AgentFlowDockPanel';
import {
  NodeRunPayloadSections,
  type RuntimeDebugArtifactBatchLoader
} from '../detail/last-run/NodeRunIOCard';
import { DebugWorkflowNodeItem } from './conversation/DebugWorkflowNodeRow';
import { DebugWorkflowNodeDetailContent } from './conversation/LlmToolTraceTree';
import {
  groupTraceItemsForDisplay,
  nodeDisplayName
} from './conversation/debug-workflow-trace-utils';
import './conversation-log-panel.css';
import { formatDateTime, formatNumber } from '../../../../shared/i18n/format';
import { i18nText } from '../../../../shared/i18n/text';

const CONVERSATION_LOG_QUERY_STALE_TIME_MS = 60_000;

interface ConversationLogTraceNodeSummary {
  trace_node_id: string;
  stable_locator?: string;
  node_kind: string;
  node_run_id?: string | null;
  node_id?: string | null;
  node_type?: string | null;
  node_mode?: string | null;
  node_alias: string;
  status: string;
  started_at: string;
  finished_at?: string | null;
  duration_ms?: number | null;
  metrics_payload?: Record<string, unknown>;
  has_children: boolean;
  child_count?: number;
  has_content: boolean;
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

interface ConversationLogTraceNodeChildrenPageInfo {
  has_more: boolean;
  next_cursor?: string | null;
  page_size: number;
}

interface ConversationLogTraceNodeChildren {
  projection_status?: ConversationLogTraceProjectionStatus;
  items: ConversationLogTraceNodeSummary[];
  page_info: ConversationLogTraceNodeChildrenPageInfo;
}

interface ConversationLogTraceNodeContent {
  trace_node_id: string;
  node_kind: string;
  projection_status?: ConversationLogTraceProjectionStatus;
  content_kind?: string;
  source_refs?: unknown;
  detail_refs?: unknown;
  payload: Record<string, unknown>;
}

interface ConversationLogTraceNodeDetail {
  trace_node_id: string;
  node_kind: string;
  projection_status?: ConversationLogTraceProjectionStatus;
  detail_ref_id: string;
  detail_kind: string;
  source_refs?: unknown;
  payload: Record<string, unknown>;
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
    traceNodeId: string,
    cursor?: string
  ) => Promise<ConversationLogTraceNodeChildren>;
  loadContent: (
    runId: string,
    traceNodeId: string
  ) => Promise<ConversationLogTraceNodeContent>;
  loadDetail?: (
    runId: string,
    traceNodeId: string,
    detailRefId: string
  ) => Promise<ConversationLogTraceNodeDetail>;
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
  onLoadArtifacts,
  overview
}: {
  message: AgentFlowDebugMessage;
  onLoadArtifact?: (artifactRef: string) => Promise<unknown>;
  onLoadArtifacts?: RuntimeDebugArtifactBatchLoader;
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
          onLoadArtifacts={onLoadArtifacts}
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
  onLoadArtifacts,
  overviewLoader,
  overviewRunId
}: {
  message: AgentFlowDebugMessage;
  onLoadArtifact?: (artifactRef: string) => Promise<unknown>;
  onLoadArtifacts?: RuntimeDebugArtifactBatchLoader;
  overviewLoader: ConversationLogOverviewLoader;
  overviewRunId: string;
}) {
  const overviewQuery = useQuery({
    queryKey: ['conversation-log-run-overview', overviewRunId],
    queryFn: () => overviewLoader.loadOverview(overviewRunId),
    refetchOnWindowFocus: false,
    staleTime: CONVERSATION_LOG_QUERY_STALE_TIME_MS
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
      onLoadArtifacts={onLoadArtifacts}
    />
  );
}

function ConversationLogDetail({
  message,
  onLoadArtifact,
  onLoadArtifacts,
  overviewLoader
}: {
  message: AgentFlowDebugMessage;
  onLoadArtifact?: (artifactRef: string) => Promise<unknown>;
  onLoadArtifacts?: RuntimeDebugArtifactBatchLoader;
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
        onLoadArtifacts={onLoadArtifacts}
      />
    );
  }

  return (
    <ConversationLogDetailContent
      message={message}
      onLoadArtifact={onLoadArtifact}
      onLoadArtifacts={onLoadArtifacts}
    />
  );
}

function ConversationTrace({
  message,
  onLoadArtifact,
  onLoadArtifacts,
  traceLoader
}: {
  message: AgentFlowDebugMessage;
  onLoadArtifact?: (artifactRef: string) => Promise<unknown>;
  onLoadArtifacts?: RuntimeDebugArtifactBatchLoader;
  traceLoader?: ConversationLogTraceLoader;
}) {
  const traceRunId = message.detailRunId ?? message.runId;

  if (traceLoader && traceRunId) {
    return (
      <LazyConversationTrace
        key={`${message.id}:${traceRunId}`}
        onLoadArtifact={onLoadArtifact}
        onLoadArtifacts={onLoadArtifacts}
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
      onLoadArtifacts={onLoadArtifacts}
    />
  );
}

function mapTraceSummaryToTraceItem(
  summary: ConversationLogTraceNodeSummary
): AgentFlowTraceItem {
  const debugPayload =
    summary.node_type === 'tool' && summary.node_mode
      ? { tool_mode: summary.node_mode }
      : {};

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
    debugPayload
  };
}

function isToolModeTraceNode(node: ConversationLogTraceNodeSummary) {
  return node.node_kind === 'fusion' || node.node_kind === 'route';
}

function toolModeFromTraceNodes(nodes: ConversationLogTraceNodeSummary[]) {
  const toolModeNode = nodes.find(isToolModeTraceNode);

  return toolModeNode?.node_kind ?? null;
}

function traceItemWithToolMode(
  item: AgentFlowTraceItem,
  toolMode: string | null
): AgentFlowTraceItem {
  if (item.nodeType !== 'tool' || !toolMode) {
    return item;
  }

  return {
    ...item,
    debugPayload: {
      ...(item.debugPayload ?? {}),
      tool_mode: toolMode
    }
  };
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return Boolean(value && typeof value === 'object' && !Array.isArray(value));
}

function recordArray(value: unknown): Record<string, unknown>[] {
  return Array.isArray(value) ? value.filter(isRecord) : [];
}

function firstStringField(
  record: Record<string, unknown>,
  keys: string[]
): string | null {
  for (const key of keys) {
    const value = record[key];

    if (typeof value === 'string' && value.trim().length > 0) {
      return value;
    }
  }

  return null;
}

function payloadRecordField(
  payload: Record<string, unknown>,
  key: string
): Record<string, unknown> {
  const value = payload[key];

  return isRecord(value) ? value : {};
}

function firstPayloadRecordField(
  payload: Record<string, unknown>,
  keys: string[]
): Record<string, unknown> {
  for (const key of keys) {
    const value = payloadRecordField(payload, key);

    if (Object.keys(value).length > 0) {
      return value;
    }
  }

  return {};
}

function payloadValueHasValue(value: unknown): boolean {
  if (value === null || value === undefined) {
    return false;
  }

  if (Array.isArray(value)) {
    return value.length > 0;
  }

  if (typeof value === 'object') {
    return Object.keys(value).length > 0;
  }

  return true;
}

function pickPayloadFields(
  payload: Record<string, unknown>,
  keys: string[]
): Record<string, unknown> {
  const picked: Record<string, unknown> = {};

  for (const key of keys) {
    const value = payload[key];

    if (payloadValueHasValue(value)) {
      picked[key] = value;
    }
  }

  return picked;
}

function omitPayloadFields(
  payload: Record<string, unknown>,
  keys: string[]
): Record<string, unknown> {
  const omitted = new Set(keys);
  const next: Record<string, unknown> = {};

  for (const [key, value] of Object.entries(payload)) {
    if (!omitted.has(key) && payloadValueHasValue(value)) {
      next[key] = value;
    }
  }

  return next;
}

function mapToolCallbackPayloadToTraceItem(
  fallback: AgentFlowTraceItem,
  payload: Record<string, unknown>
): AgentFlowTraceItem {
  return {
    ...fallback,
    inputPayload: firstPayloadRecordField(payload, [
      'request_payload',
      'tool_call'
    ]),
    outputPayload: firstPayloadRecordField(payload, [
      'parsed_result',
      'tool_result',
      'callback_payload'
    ]),
    debugPayload: pickPayloadFields(payload, [
      'callback_task_id',
      'tool_call_id',
      'callback_status',
      'execution_status',
      'duration_ms',
      'route_trace'
    ])
  };
}

function mapPayloadContentToTraceItem(
  fallback: AgentFlowTraceItem,
  content: ConversationLogTraceNodeContent
): AgentFlowTraceItem {
  const payload = content.payload;

  if (!isRecord(payload)) {
    return fallback;
  }

  if (content.node_kind === 'tool_callback') {
    return mapToolCallbackPayloadToTraceItem(fallback, payload);
  }

  if (content.node_kind === 'fusion' || content.node_kind === 'route') {
    const metricsPayload = payloadRecordField(payload, 'metrics_payload');
    const usage = payloadRecordField(payload, 'usage');
    const effectiveMetrics =
      Object.keys(metricsPayload).length > 0 ? metricsPayload : usage;

    return {
      ...fallback,
      inputPayload: {},
      outputPayload:
        Object.keys(effectiveMetrics).length > 0
          ? { usage: effectiveMetrics }
          : {},
      debugPayload: payload,
      metricsPayload: effectiveMetrics
    };
  }

  if (content.node_kind === 'branch') {
    const inputPayload = payloadRecordField(payload, 'input_payload');
    const outputPayload = payloadRecordField(payload, 'output_payload');
    const debugPayload = payloadRecordField(payload, 'debug_payload');
    const metricsPayload = payloadRecordField(payload, 'metrics_payload');

    return {
      ...fallback,
      inputPayload: Object.keys(inputPayload).length > 0 ? inputPayload : {},
      outputPayload: Object.keys(outputPayload).length > 0 ? outputPayload : {},
      debugPayload: Object.keys(debugPayload).length > 0 ? debugPayload : {},
      metricsPayload:
        Object.keys(metricsPayload).length > 0
          ? metricsPayload
          : fallback.metricsPayload
    };
  }

  const inputPayload = payloadRecordField(payload, 'input_payload');
  const outputPayload = payloadRecordField(payload, 'output_payload');
  const debugPayload = payloadRecordField(payload, 'debug_payload');
  const metricsPayload = payloadRecordField(payload, 'metrics_payload');
  const errorPayload = payloadRecordField(payload, 'error_payload');
  const hasStructuredPayload =
    Object.keys(inputPayload).length > 0 ||
    Object.keys(outputPayload).length > 0 ||
    Object.keys(debugPayload).length > 0 ||
    Object.keys(metricsPayload).length > 0 ||
    Object.keys(errorPayload).length > 0;

  if (!hasStructuredPayload) {
    return {
      ...fallback,
      inputPayload: {},
      outputPayload: {},
      debugPayload: payload
    };
  }

  return {
    ...fallback,
    inputPayload,
    outputPayload,
    errorPayload: Object.keys(errorPayload).length > 0 ? errorPayload : null,
    metricsPayload:
      Object.keys(metricsPayload).length > 0
        ? metricsPayload
        : fallback.metricsPayload,
    debugPayload:
      Object.keys(debugPayload).length > 0
        ? debugPayload
        : omitPayloadFields(payload, [
            'input_payload',
            'output_payload',
            'error_payload',
            'metrics_payload',
            'debug_payload'
          ])
  };
}

function findNodeRunDetailRefId(
  content: ConversationLogTraceNodeContent | undefined
) {
  if (!content || content.node_kind !== 'node_run') {
    return null;
  }

  for (const detailRef of recordArray(content.detail_refs)) {
    if (firstStringField(detailRef, ['detail_kind']) !== 'node_run') {
      continue;
    }

    return firstStringField(detailRef, ['detail_ref_id']);
  }

  return null;
}

function mapNodeRunRecordToTraceItem(
  fallback: AgentFlowTraceItem,
  nodeRun: Record<string, unknown>
): AgentFlowTraceItem {
  const inputPayload = payloadRecordField(nodeRun, 'input_payload');
  const outputPayload = payloadRecordField(nodeRun, 'output_payload');
  const debugPayload = payloadRecordField(nodeRun, 'debug_payload');
  const metricsPayload = payloadRecordField(nodeRun, 'metrics_payload');
  const errorPayload = payloadRecordField(nodeRun, 'error_payload');

  return {
    ...fallback,
    inputPayload,
    outputPayload,
    errorPayload: Object.keys(errorPayload).length > 0 ? errorPayload : null,
    metricsPayload:
      Object.keys(metricsPayload).length > 0
        ? metricsPayload
        : fallback.metricsPayload,
    debugPayload
  };
}

function mapDetailContentToTraceItem(
  fallback: AgentFlowTraceItem,
  detail: ConversationLogTraceNodeDetail | undefined
): AgentFlowTraceItem {
  if (!detail || detail.detail_kind !== 'node_run') {
    return fallback;
  }

  const nodeRun = payloadRecordField(detail.payload, 'node_run');

  if (Object.keys(nodeRun).length === 0) {
    return fallback;
  }

  return mapNodeRunRecordToTraceItem(fallback, nodeRun);
}

function mapTraceContentToTraceItem(
  fallback: AgentFlowTraceItem,
  content: ConversationLogTraceNodeContent | undefined,
  detail?: ConversationLogTraceNodeDetail
): AgentFlowTraceItem {
  const contentItem = content
    ? mapPayloadContentToTraceItem(fallback, content)
    : fallback;

  return mapDetailContentToTraceItem(contentItem, detail);
}

function traceProjectionStatusSucceeded(
  status: ConversationLogTraceProjectionStatus | undefined
) {
  return !status || status.projection_status === 'succeeded';
}

function appendTraceChildrenPage(
  current: ConversationLogTraceNodeSummary[],
  nextPageItems: ConversationLogTraceNodeSummary[]
) {
  if (current.length === 0) {
    return nextPageItems;
  }

  const seenTraceNodeIds = new Set(
    current.map((childNode) => childNode.trace_node_id)
  );
  const next = [...current];

  for (const childNode of nextPageItems) {
    if (!seenTraceNodeIds.has(childNode.trace_node_id)) {
      seenTraceNodeIds.add(childNode.trace_node_id);
      next.push(childNode);
    }
  }

  return next;
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
  onLoadArtifacts,
  runId,
  traceLoader
}: {
  onLoadArtifact?: (artifactRef: string) => Promise<unknown>;
  onLoadArtifacts?: RuntimeDebugArtifactBatchLoader;
  runId: string;
  traceLoader: ConversationLogTraceLoader;
}) {
  const traceTreeQuery = useQuery({
    queryKey: ['conversation-log-trace-tree', runId],
    queryFn: () => traceLoader.loadTree(runId),
    refetchOnWindowFocus: false,
    staleTime: CONVERSATION_LOG_QUERY_STALE_TIME_MS
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
        onLoadArtifacts={onLoadArtifacts}
        runId={runId}
        traceLoader={traceLoader}
      />
    </div>
  );
}

function LazyTraceNodeList({
  nodes,
  onLoadArtifact,
  onLoadArtifacts,
  runId,
  traceLoader
}: {
  nodes: ConversationLogTraceNodeSummary[];
  onLoadArtifact?: (artifactRef: string) => Promise<unknown>;
  onLoadArtifacts?: RuntimeDebugArtifactBatchLoader;
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
          onLoadArtifacts={onLoadArtifacts}
          runId={runId}
          traceLoader={traceLoader}
        />
      ))}
    </div>
  );
}

function FlattenedToolModeTraceNodeChildren({
  nodes,
  onLoadArtifact,
  onLoadArtifacts,
  runId,
  traceLoader
}: {
  nodes: ConversationLogTraceNodeSummary[];
  onLoadArtifact?: (artifactRef: string) => Promise<unknown>;
  onLoadArtifacts?: RuntimeDebugArtifactBatchLoader;
  runId: string;
  traceLoader: ConversationLogTraceLoader;
}) {
  if (nodes.length === 0) {
    return null;
  }

  return (
    <>
      {nodes.map((node) => (
        <FlattenedToolModeTraceNodeChild
          key={node.trace_node_id}
          node={node}
          onLoadArtifact={onLoadArtifact}
          onLoadArtifacts={onLoadArtifacts}
          runId={runId}
          traceLoader={traceLoader}
        />
      ))}
    </>
  );
}

function FlattenedToolModeTraceNodeChild({
  node,
  onLoadArtifact,
  onLoadArtifacts,
  runId,
  traceLoader
}: {
  node: ConversationLogTraceNodeSummary;
  onLoadArtifact?: (artifactRef: string) => Promise<unknown>;
  onLoadArtifacts?: RuntimeDebugArtifactBatchLoader;
  runId: string;
  traceLoader: ConversationLogTraceLoader;
}) {
  const childrenQuery = useQuery({
    enabled: node.has_children,
    queryKey: [
      'conversation-log-trace-node-children',
      runId,
      node.trace_node_id
    ],
    queryFn: () => traceLoader.loadChildren(runId, node.trace_node_id, undefined),
    refetchOnWindowFocus: false,
    staleTime: CONVERSATION_LOG_QUERY_STALE_TIME_MS
  });
  const childProjectionStatus = childrenQuery.data?.projection_status;
  const childNodes = childrenQuery.data?.items ?? [];

  if (!node.has_children) {
    return null;
  }

  return (
    <>
      {childrenQuery.isLoading ? <Spin /> : null}
      {childrenQuery.isError ? (
        <Alert
          message={i18nText('agentFlow', 'auto.loading_failed')}
          showIcon
          type="error"
        />
      ) : null}
      {childProjectionStatus &&
      !traceProjectionStatusSucceeded(childProjectionStatus) ? (
        <TraceProjectionStatusNotice status={childProjectionStatus} />
      ) : null}
      {childNodes.length > 0 ? (
        <LazyTraceNodeList
          nodes={childNodes}
          onLoadArtifact={onLoadArtifact}
          onLoadArtifacts={onLoadArtifacts}
          runId={runId}
          traceLoader={traceLoader}
        />
      ) : null}
    </>
  );
}

function LazyTraceNodeItem({
  node,
  onLoadArtifact,
  onLoadArtifacts,
  runId,
  traceLoader
}: {
  node: ConversationLogTraceNodeSummary;
  onLoadArtifact?: (artifactRef: string) => Promise<unknown>;
  onLoadArtifacts?: RuntimeDebugArtifactBatchLoader;
  runId: string;
  traceLoader: ConversationLogTraceLoader;
}) {
  const [expanded, setExpanded] = useState(false);
  const [childNodes, setChildNodes] = useState<
    ConversationLogTraceNodeSummary[]
  >([]);
  const [childPageInfo, setChildPageInfo] =
    useState<ConversationLogTraceNodeChildrenPageInfo | null>(null);
  const [childProjectionStatus, setChildProjectionStatus] =
    useState<ConversationLogTraceProjectionStatus>();
  const [loadingMoreChildren, setLoadingMoreChildren] = useState(false);
  const [loadMoreChildrenFailed, setLoadMoreChildrenFailed] = useState(false);
  const fallbackItem = useMemo(() => mapTraceSummaryToTraceItem(node), [node]);
  const isToolGroupNode =
    node.node_kind === 'tool_group' || node.node_type === 'tools';
  const contentQuery = useQuery({
    enabled: expanded && node.has_content && !isToolGroupNode,
    queryKey: [
      'conversation-log-trace-node-content',
      runId,
      node.trace_node_id
    ],
    queryFn: () => traceLoader.loadContent(runId, node.trace_node_id),
    refetchOnWindowFocus: false,
    staleTime: CONVERSATION_LOG_QUERY_STALE_TIME_MS
  });
  const nodeRunDetailRefId = useMemo(
    () => findNodeRunDetailRefId(contentQuery.data),
    [contentQuery.data]
  );
  const nodeRunDetailQuery = useQuery({
    enabled:
      expanded &&
      Boolean(nodeRunDetailRefId) &&
      Boolean(traceLoader.loadDetail),
    queryKey: [
      'conversation-log-trace-node-detail',
      runId,
      node.trace_node_id,
      nodeRunDetailRefId
    ],
    queryFn: () => {
      if (!traceLoader.loadDetail || !nodeRunDetailRefId) {
        throw new Error('trace_node_detail_loader_unavailable');
      }

      return traceLoader.loadDetail(
        runId,
        node.trace_node_id,
        nodeRunDetailRefId
      );
    },
    refetchOnWindowFocus: false,
    staleTime: CONVERSATION_LOG_QUERY_STALE_TIME_MS
  });
  const childrenQuery = useQuery({
    enabled: expanded && node.has_children,
    queryKey: [
      'conversation-log-trace-node-children',
      runId,
      node.trace_node_id
    ],
    queryFn: () =>
      traceLoader.loadChildren(runId, node.trace_node_id, undefined),
    refetchOnWindowFocus: false,
    staleTime: CONVERSATION_LOG_QUERY_STALE_TIME_MS
  });
  useEffect(() => {
    setChildNodes([]);
    setChildPageInfo(null);
    setChildProjectionStatus(undefined);
    setLoadingMoreChildren(false);
    setLoadMoreChildrenFailed(false);
  }, [runId, node.trace_node_id]);
  useEffect(() => {
    const childrenPage = childrenQuery.data;
    if (!childrenPage) {
      return;
    }

    setChildNodes(childrenPage.items);
    setChildPageInfo(childrenPage.page_info);
    setChildProjectionStatus(childrenPage.projection_status);
  }, [childrenQuery.data]);
  const loadMoreTraceChildren = useCallback(async () => {
    const cursor = childPageInfo?.next_cursor;
    if (!cursor || loadingMoreChildren) {
      return;
    }

    setLoadingMoreChildren(true);
    setLoadMoreChildrenFailed(false);
    try {
      const nextPage = await traceLoader.loadChildren(
        runId,
        node.trace_node_id,
        cursor
      );
      setChildNodes((current) =>
        appendTraceChildrenPage(current, nextPage.items)
      );
      setChildPageInfo(nextPage.page_info);
      setChildProjectionStatus(nextPage.projection_status);
    } catch {
      setLoadMoreChildrenFailed(true);
    } finally {
      setLoadingMoreChildren(false);
    }
  }, [
    childPageInfo?.next_cursor,
    loadingMoreChildren,
    node.trace_node_id,
    runId,
    traceLoader
  ]);
  const toolModeNodes = useMemo(
    () =>
      node.node_kind === 'tool_callback'
        ? childNodes.filter(isToolModeTraceNode)
        : [],
    [childNodes, node.node_kind]
  );
  const visibleChildNodes = useMemo(
    () =>
      node.node_kind === 'tool_callback'
        ? childNodes.filter((childNode) => !isToolModeTraceNode(childNode))
        : childNodes,
    [childNodes, node.node_kind]
  );
  const item = useMemo(
    () =>
      traceItemWithToolMode(
        mapTraceContentToTraceItem(
          fallbackItem,
          contentQuery.data,
          nodeRunDetailQuery.data
        ),
        toolModeFromTraceNodes(childNodes)
      ),
    [childNodes, contentQuery.data, fallbackItem, nodeRunDetailQuery.data]
  );
  const contentProjectionStatus = contentQuery.data?.projection_status;
  const contentLoading = contentQuery.isLoading || nodeRunDetailQuery.isLoading;
  const loadToolCallbackDetail = traceLoader.loadToolCallbackDetail;
  const childNodesBeforePayload =
    visibleChildNodes.length > 0 || toolModeNodes.length > 0 ? (
      <>
        <FlattenedToolModeTraceNodeChildren
          nodes={toolModeNodes}
          onLoadArtifact={onLoadArtifact}
          onLoadArtifacts={onLoadArtifacts}
          runId={runId}
          traceLoader={traceLoader}
        />
        {visibleChildNodes.length > 0 ? (
          <LazyTraceNodeList
            nodes={visibleChildNodes}
            onLoadArtifact={onLoadArtifact}
            onLoadArtifacts={onLoadArtifacts}
            runId={runId}
            traceLoader={traceLoader}
          />
        ) : null}
      </>
    ) : null;
  const childLoadStatusContent = (
    <>
      {childrenQuery.isLoading ? <Spin /> : null}
      {childProjectionStatus &&
      !traceProjectionStatusSucceeded(childProjectionStatus) ? (
        <TraceProjectionStatusNotice status={childProjectionStatus} />
      ) : null}
      {loadMoreChildrenFailed ? (
        <Alert
          message={i18nText('agentFlow', 'auto.loading_failed')}
          showIcon
          type="error"
        />
      ) : null}
    </>
  );
  const loadMoreChildrenButton = childPageInfo?.has_more ? (
    <Button
      className="agent-flow-editor__conversation-log-load-more"
      loading={loadingMoreChildren}
      size="small"
      type="link"
      onClick={() => {
        void loadMoreTraceChildren();
      }}
    >
      {i18nText('agentFlow', 'auto.load_more_trace_children')}
    </Button>
  ) : null;

  return (
    <DebugWorkflowNodeItem
      expanded={expanded}
      item={item}
      onToggle={() => setExpanded((current) => !current)}
    >
      {isToolGroupNode ? (
        <div className="agent-flow-editor__conversation-log-node-group">
          {childLoadStatusContent}
          {childNodesBeforePayload}
          {loadMoreChildrenButton}
        </div>
      ) : (
        <section
          aria-label={i18nText('agentFlow', 'auto.node_details_alt', {
            value1: nodeDisplayName(fallbackItem)
          })}
          className="agent-flow-editor__conversation-log-node-detail"
        >
          {contentLoading ? (
            <Spin />
          ) : contentProjectionStatus &&
            !traceProjectionStatusSucceeded(contentProjectionStatus) ? (
            <TraceProjectionStatusNotice status={contentProjectionStatus} />
          ) : (
            <div className="agent-flow-editor__conversation-log-json-list">
              <DebugWorkflowNodeDetailContent
                beforePayloadContent={childNodesBeforePayload}
                item={item}
                onLoadArtifact={onLoadArtifact}
                onLoadArtifacts={onLoadArtifacts}
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
          )}
          {childLoadStatusContent}
          {loadMoreChildrenButton}
        </section>
      )}
    </DebugWorkflowNodeItem>
  );
}

function ConversationTraceContent({
  message,
  onLoadArtifact,
  onLoadArtifacts
}: {
  message: AgentFlowDebugMessage;
  onLoadArtifact?: (artifactRef: string) => Promise<unknown>;
  onLoadArtifacts?: RuntimeDebugArtifactBatchLoader;
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
                    onLoadArtifacts={onLoadArtifacts}
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
  onLoadArtifacts,
  overviewLoader,
  traceLoader
}: {
  message: AgentFlowDebugMessage;
  onClose: () => void;
  onLoadArtifact?: (artifactRef: string) => Promise<unknown>;
  onLoadArtifacts?: RuntimeDebugArtifactBatchLoader;
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
                onLoadArtifacts={onLoadArtifacts}
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
                  onLoadArtifacts={onLoadArtifacts}
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
