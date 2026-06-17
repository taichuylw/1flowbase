import { DownOutlined, RightOutlined, ToolOutlined } from '@ant-design/icons';
import { Tag, Tooltip, Typography } from 'antd';
import { ReactNode, useEffect, useMemo, useRef, useState } from 'react';

import type { AgentFlowTraceItem } from '../../../api/runtime';
import {
  NodeRunPayloadSections,
  RuntimeDebugPayloadBlock
} from '../../detail/last-run/NodeRunIOCard';
import { AnswerSnapshotTrace } from './AnswerSnapshotTrace';
import { DebugWorkflowNodeItem } from './DebugWorkflowNodeRow';
import {
  collectLlmToolCallbacksFromDebugPayloads,
  readLlmToolRouteTraceDetail,
  readLlmToolCallbackDetail,
  type LlmToolCallback,
  type LlmToolRouteBranchTrace,
  type LlmToolRouteBranchSummary,
  type LlmToolRouteTraceSummary,
  stripLlmRoundsFromDebugPayload
} from './llm-tool-callbacks';
import { i18nText } from '../../../../../shared/i18n/text';
import { formatTokens, formatDurationScaled } from './metrics-formatter';

function callbackStatusLabel(status: LlmToolCallback['callbackStatus']) {
  switch (status) {
    case 'returned':
      return i18nText('agentFlow', 'auto.returned');
    case 'cancelled':
      return i18nText('agentFlow', 'auto.canceled');
    default:
      return i18nText('agentFlow', 'auto.wait_for_callback');
  }
}

function callbackStatusColor(status: LlmToolCallback['callbackStatus']) {
  switch (status) {
    case 'returned':
      return 'success';
    case 'cancelled':
      return 'default';
    default:
      return 'warning';
  }
}

function executionStatusLabel(status: LlmToolCallback['executionStatus']) {
  switch (status) {
    case 'succeeded':
      return i18nText('agentFlow', 'auto.executed_successfully');
    case 'failed':
      return i18nText('agentFlow', 'auto.execution_failed');
    case 'timed_out':
      return i18nText('agentFlow', 'auto.execution_timeout');
    case 'cancelled':
      return i18nText('agentFlow', 'auto.execution_cancel');
    default:
      return i18nText('agentFlow', 'auto.execution_unknown');
  }
}

function executionStatusColor(status: LlmToolCallback['executionStatus']) {
  switch (status) {
    case 'succeeded':
      return 'success';
    case 'failed':
    case 'timed_out':
      return 'error';
    case 'cancelled':
      return 'default';
    default:
      return 'default';
  }
}

function callUsageTotalTokens(callback: LlmToolCallback): number | null {
  const totalTokens = callback.call_usage?.total_tokens;

  return typeof totalTokens === 'number' && Number.isFinite(totalTokens)
    ? totalTokens
    : null;
}

function LlmToolInlineMetrics({ callback }: { callback: LlmToolCallback }) {
  const elements: ReactNode[] = [];
  const totalTokens = callUsageTotalTokens(callback);

  if (typeof totalTokens === 'number') {
    const formattedTokens = `${formatTokens(totalTokens)} tokens`;
    elements.push(
      <Tooltip title={`${totalTokens.toLocaleString()} tokens`} key="tokens">
        <span>{formattedTokens}</span>
      </Tooltip>
    );
  }

  if (typeof callback.duration_ms === 'number') {
    const formattedDuration = formatDurationScaled(callback.duration_ms);
    elements.push(
      <Tooltip
        title={`${callback.duration_ms.toLocaleString()} ms`}
        key="duration"
      >
        <span>{formattedDuration}</span>
      </Tooltip>
    );
  }

  if (elements.length === 0) {
    return null;
  }

  const joined: ReactNode[] = [];
  elements.forEach((el, index) => {
    joined.push(el);
    if (index < elements.length - 1) {
      joined.push(<span key={`dot-${index}`}> · </span>);
    }
  });

  return (
    <Typography.Text
      className="agent-flow-editor__debug-llm-tool-inline-metrics"
      type="secondary"
    >
      {joined}
    </Typography.Text>
  );
}

function routeNodeStatus(callback: LlmToolCallback) {
  const traceStatus = callback.routeTrace?.status;

  if (traceStatus) {
    switch (traceStatus) {
      case 'succeeded':
      case 'returned_to_main':
      case 'route_completed':
        return 'succeeded';
      case 'failed':
        return 'failed';
      case 'cancelled':
      case 'canceled':
        return 'cancelled';
      case 'waiting_callback':
        return 'waiting_callback';
      default:
        return traceStatus;
    }
  }

  switch (callback.executionStatus) {
    case 'succeeded':
      return 'succeeded';
    case 'failed':
    case 'timed_out':
      return 'failed';
    case 'cancelled':
      return 'cancelled';
    default:
      return callback.callbackStatus === 'returned'
        ? 'succeeded'
        : 'waiting_callback';
  }
}

function routeNodeAlias(routeTrace: LlmToolRouteTraceSummary) {
  return routeTrace.routeNodeAlias ?? 'LLM';
}

function routeTraceLabel(routeTrace: LlmToolRouteTraceSummary) {
  return routeTrace.routeKind === 'fusion'
    ? 'fusion'
    : i18nText('agentFlow', 'auto.route_trace');
}

function routeNodeOutputPayload(callback: LlmToolCallback) {
  if (callback.call_usage) {
    return {
      usage: callback.call_usage
    };
  }

  return {};
}

function buildRouteTraceItem(callback: LlmToolCallback): AgentFlowTraceItem {
  const routeTrace = callback.routeTrace;

  return {
    nodeId:
      routeTrace?.routeNodeId ??
      routeTrace?.targetNodeId ??
      `${callback.id}:route`,
    nodeRunId: routeTrace?.detailArtifactRef ?? `${callback.id}:route`,
    nodeAlias: routeTrace ? routeNodeAlias(routeTrace) : 'LLM',
    nodeType: 'llm',
    status: routeNodeStatus(callback),
    startedAt: '',
    finishedAt: callback.callbackStatus === 'returned' ? '' : null,
    durationMs: callback.duration_ms,
    inputPayload: {},
    outputPayload: routeNodeOutputPayload(callback),
    errorPayload:
      callback.executionStatus === 'failed'
        ? (callback.parsedResult ?? {})
        : null,
    metricsPayload: {},
    debugPayload: routeTrace?.rawPayload ?? {}
  };
}

function branchNodeStatus(branch: LlmToolRouteBranchSummary) {
  switch (branch.status) {
    case 'succeeded':
    case 'returned_to_main':
    case 'route_completed':
      return 'succeeded';
    case 'failed':
      return 'failed';
    case 'cancelled':
    case 'canceled':
      return 'cancelled';
    case 'waiting_callback':
      return 'waiting_callback';
    default:
      return branch.status ?? 'succeeded';
  }
}

function buildRouteBranchTraceItem(
  callback: LlmToolCallback,
  branch: LlmToolRouteBranchSummary | LlmToolRouteBranchTrace,
  index: number
): AgentFlowTraceItem {
  const fallbackId = `${callback.id}:branch:${index + 1}`;
  const loadedBranchTrace = isRouteBranchTrace(branch);
  const inputPayload = loadedBranchTrace ? branch.inputPayload : {};
  const outputPayload = loadedBranchTrace ? branch.outputPayload : {};
  const metricsPayload = loadedBranchTrace ? branch.metricsPayload : {};
  const debugPayload = loadedBranchTrace ? branch.debugPayload : {};

  return {
    nodeId: branch.nodeId ?? fallbackId,
    nodeRunId: `${fallbackId}:summary`,
    nodeAlias: branch.nodeAlias ?? branch.nodeId ?? `Panel ${index + 1}`,
    nodeType: branch.nodeType ?? 'llm',
    status: branchNodeStatus(branch),
    startedAt: '',
    finishedAt: callback.callbackStatus === 'returned' ? '' : null,
    durationMs: null,
    inputPayload,
    outputPayload,
    errorPayload: null,
    metricsPayload,
    debugPayload
  };
}

function isRouteBranchTrace(
  branch: LlmToolRouteBranchSummary | LlmToolRouteBranchTrace
): branch is LlmToolRouteBranchTrace {
  return Object.prototype.hasOwnProperty.call(branch, 'inputPayload');
}

function runtimeDetailPayloadHasValue(value: unknown): boolean {
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

export function DebugWorkflowNodeDetailContent({
  item,
  onLoadArtifact
}: {
  item: AgentFlowTraceItem;
  onLoadArtifact?: (artifactRef: string) => Promise<unknown>;
}) {
  const debugPayload = stripLlmRoundsFromDebugPayload(item.debugPayload ?? {});
  const hasNodeDetail =
    runtimeDetailPayloadHasValue(item.inputPayload) ||
    runtimeDetailPayloadHasValue(debugPayload) ||
    runtimeDetailPayloadHasValue(item.outputPayload) ||
    Boolean(item.answerSnapshot) ||
    collectLlmToolCallbacksFromDebugPayloads([item.debugPayload]).length > 0;

  if (!hasNodeDetail) {
    return (
      <Typography.Text
        className="agent-flow-editor__debug-workflow-node-missing-detail"
        type="secondary"
      >
        {i18nText('agentFlow', 'auto.node_detail_summary_only')}
      </Typography.Text>
    );
  }

  return (
    <>
      <LlmToolTraceTree
        debugPayload={item.debugPayload}
        onLoadArtifact={onLoadArtifact}
      />
      {item.answerSnapshot ? (
        <AnswerSnapshotTrace
          snapshot={item.answerSnapshot}
          onLoadArtifact={onLoadArtifact}
        />
      ) : null}
      <NodeRunPayloadSections
        debugPayload={debugPayload}
        inputPayload={item.inputPayload}
        outputPayload={item.outputPayload}
        onLoadArtifact={onLoadArtifact}
      />
    </>
  );
}

function LlmToolRouteBranchNode({
  branch,
  callback,
  index,
  onLoadArtifact
}: {
  branch: LlmToolRouteBranchSummary | LlmToolRouteBranchTrace;
  callback: LlmToolCallback;
  index: number;
  onLoadArtifact?: (artifactRef: string) => Promise<unknown>;
}) {
  const [expanded, setExpanded] = useState(false);
  const branchTraceItem = buildRouteBranchTraceItem(callback, branch, index);

  return (
    <div
      className="agent-flow-editor__debug-llm-route-branch-node"
      data-testid="debug-llm-route-branch-node"
    >
      <DebugWorkflowNodeItem
        expanded={expanded}
        item={branchTraceItem}
        onToggle={() => setExpanded((current) => !current)}
      >
        <div className="agent-flow-editor__debug-workflow-node-detail agent-flow-editor__debug-llm-route-branch-detail">
          <DebugWorkflowNodeDetailContent
            item={branchTraceItem}
            onLoadArtifact={onLoadArtifact}
          />
        </div>
      </DebugWorkflowNodeItem>
    </div>
  );
}

function routeTraceStatusText(callback: LlmToolCallback) {
  return executionStatusLabel(
    routeNodeStatus(callback) === 'failed' ? 'failed' : 'succeeded'
  );
}

function LlmToolRouteNode({
  callback,
  onLoadArtifact
}: {
  callback: LlmToolCallback;
  onLoadArtifact?: (artifactRef: string) => Promise<unknown>;
}) {
  const [expanded, setExpanded] = useState(true);
  const [loadedRouteTrace, setLoadedRouteTrace] =
    useState<LlmToolRouteTraceSummary | null>(null);
  const [loadingRouteTrace, setLoadingRouteTrace] = useState(false);
  const [routeTraceLoadFailed, setRouteTraceLoadFailed] = useState(false);
  const sourceRouteTrace = callback.routeTrace;
  const routeKind = sourceRouteTrace?.routeKind ?? null;
  const detailArtifactRef = sourceRouteTrace?.detailArtifactRef ?? null;

  useEffect(() => {
    setLoadedRouteTrace(null);
    setLoadingRouteTrace(false);
    setRouteTraceLoadFailed(false);
  }, [callback.id, detailArtifactRef, routeKind]);

  useEffect(() => {
    if (
      !expanded ||
      routeKind !== 'fusion' ||
      !detailArtifactRef ||
      !onLoadArtifact ||
      loadedRouteTrace ||
      routeTraceLoadFailed
    ) {
      return;
    }

    let active = true;
    setLoadingRouteTrace(true);
    setRouteTraceLoadFailed(false);
    void onLoadArtifact(detailArtifactRef)
      .then((payload) => {
        if (!active) {
          return;
        }
        const detail = readLlmToolRouteTraceDetail(payload);
        if (!detail) {
          throw new Error('invalid_route_trace_detail');
        }
        setLoadedRouteTrace(detail);
      })
      .catch(() => {
        if (active) {
          setRouteTraceLoadFailed(true);
        }
      })
      .finally(() => {
        if (active) {
          setLoadingRouteTrace(false);
        }
      });

    return () => {
      active = false;
    };
  }, [
    detailArtifactRef,
    expanded,
    loadedRouteTrace,
    onLoadArtifact,
    routeKind,
    routeTraceLoadFailed
  ]);

  if (!sourceRouteTrace) {
    return null;
  }

  const routeTrace = loadedRouteTrace ?? sourceRouteTrace;
  const branchTraces = routeTrace.branchTraces;
  const branchNodes =
    branchTraces.length > 0 ? branchTraces : routeTrace.branchSummaries;
  const routeTraceTitle = routeTraceLabel(routeTrace);

  if (routeTrace.routeKind === 'fusion') {
    return (
      <div
        className="agent-flow-editor__debug-llm-route-node"
        data-testid="debug-llm-route-node"
      >
        <button
          aria-expanded={expanded}
          className="agent-flow-editor__debug-llm-route-group-trigger"
          onClick={() => setExpanded((current) => !current)}
          type="button"
        >
          <span className="agent-flow-editor__debug-llm-route-group-main">
            <Typography.Text strong>{routeTraceTitle}</Typography.Text>
            <Typography.Text type="secondary">
              {routeTraceStatusText(callback)}
            </Typography.Text>
          </span>
          <Tag className="agent-flow-editor__debug-llm-tool-route-tag">
            {routeTraceTitle}
          </Tag>
          {expanded ? (
            <DownOutlined className="agent-flow-editor__debug-workflow-collapse" />
          ) : (
            <RightOutlined className="agent-flow-editor__debug-workflow-collapse" />
          )}
        </button>
        {expanded ? (
          <div className="agent-flow-editor__debug-llm-route-node-detail">
            {loadingRouteTrace ? (
              <Tag color="processing">
                {i18nText('agentFlow', 'auto.loading')}
              </Tag>
            ) : null}
            {routeTraceLoadFailed ? (
              <Tag color="error">
                {i18nText('agentFlow', 'auto.loading_failed')}
              </Tag>
            ) : null}
            {branchNodes.length > 0 ? (
              <div className="agent-flow-editor__debug-llm-route-branch-list">
                {branchNodes.map((branch, index) => (
                  <LlmToolRouteBranchNode
                    key={`${branch.nodeId ?? callback.id}-${index}`}
                    branch={branch}
                    callback={callback}
                    index={index}
                    onLoadArtifact={onLoadArtifact}
                  />
                ))}
              </div>
            ) : null}
          </div>
        ) : null}
      </div>
    );
  }

  const routeTraceItem = buildRouteTraceItem(callback);

  return (
    <div
      className="agent-flow-editor__debug-llm-route-node"
      data-testid="debug-llm-route-node"
    >
      <DebugWorkflowNodeItem
        expanded={expanded}
        item={routeTraceItem}
        onToggle={() => setExpanded((current) => !current)}
      >
        <div className="agent-flow-editor__debug-workflow-node-detail agent-flow-editor__debug-llm-route-node-detail">
          {branchNodes.length > 0 ? (
            <div className="agent-flow-editor__debug-llm-route-branch-list">
              {branchNodes.map((branch, index) => (
                <LlmToolRouteBranchNode
                  key={`${branch.nodeId ?? callback.id}-${index}`}
                  branch={branch}
                  callback={callback}
                  index={index}
                  onLoadArtifact={onLoadArtifact}
                />
              ))}
            </div>
          ) : null}
          <RuntimeDebugPayloadBlock
            height="11rem"
            payload={sourceRouteTrace.rawPayload}
            title={routeTraceTitle}
            onLoadArtifact={onLoadArtifact}
          />
        </div>
      </DebugWorkflowNodeItem>
    </div>
  );
}

function LlmToolCallbackItem({
  callback,
  expanded,
  loadFailed,
  loading,
  onLoadArtifact,
  onToggle
}: {
  callback: LlmToolCallback;
  expanded: boolean;
  loadFailed: boolean;
  loading: boolean;
  onLoadArtifact?: (artifactRef: string) => Promise<unknown>;
  onToggle: () => void;
}) {
  return (
    <article
      className="agent-flow-editor__debug-llm-tool-item"
      data-expanded={expanded ? 'true' : 'false'}
    >
      <button
        aria-expanded={expanded}
        className="agent-flow-editor__debug-llm-tool-trigger"
        onClick={onToggle}
        type="button"
      >
        <span className="agent-flow-editor__debug-llm-tool-main">
          <Typography.Text strong>{callback.name}</Typography.Text>
          {callback.routeTrace ? (
            <Tag className="agent-flow-editor__debug-llm-tool-route-tag">
              {routeTraceLabel(callback.routeTrace)}
            </Tag>
          ) : (
            <LlmToolInlineMetrics callback={callback} />
          )}
        </span>
        <Tag color={callbackStatusColor(callback.callbackStatus)}>
          {callbackStatusLabel(callback.callbackStatus)}
        </Tag>
        {callback.executionStatus === 'unknown' ? null : (
          <Tag color={executionStatusColor(callback.executionStatus)}>
            {executionStatusLabel(callback.executionStatus)}
          </Tag>
        )}
        {expanded ? (
          <DownOutlined className="agent-flow-editor__debug-workflow-collapse" />
        ) : (
          <RightOutlined className="agent-flow-editor__debug-workflow-collapse" />
        )}
      </button>
      {expanded ? (
        <div className="agent-flow-editor__debug-llm-tool-detail">
          {loading ? (
            <Tag color="processing">
              {i18nText('agentFlow', 'auto.loading')}
            </Tag>
          ) : null}
          {loadFailed ? (
            <Tag color="error">
              {i18nText('agentFlow', 'auto.loading_failed')}
            </Tag>
          ) : null}
          {!loading && !loadFailed ? (
            <>
              <LlmToolRouteNode
                callback={callback}
                onLoadArtifact={onLoadArtifact}
              />
              <RuntimeDebugPayloadBlock
                height="11rem"
                payload={callback.requestPayload}
                title={i18nText('agentFlow', 'auto.tool_call')}
                onLoadArtifact={onLoadArtifact}
              />
              {callback.parsedResult ? (
                <RuntimeDebugPayloadBlock
                  height="11rem"
                  payload={callback.parsedResult}
                  title={i18nText('agentFlow', 'auto.parse_results')}
                  onLoadArtifact={onLoadArtifact}
                />
              ) : null}
              {callback.callbackPayload ? (
                <RuntimeDebugPayloadBlock
                  height="11rem"
                  payload={callback.callbackPayload}
                  title={i18nText('agentFlow', 'auto.full_callback')}
                  onLoadArtifact={onLoadArtifact}
                />
              ) : (
                <Typography.Text type="secondary">
                  {i18nText('agentFlow', 'auto.wait_callback_return')}
                </Typography.Text>
              )}
            </>
          ) : null}
        </div>
      ) : null}
    </article>
  );
}

export function LlmToolTraceTree({
  debugPayload,
  debugPayloads,
  onLoadArtifact
}: {
  debugPayload: unknown;
  debugPayloads?: unknown[];
  onLoadArtifact?: (artifactRef: string) => Promise<unknown>;
}) {
  const [toolsExpanded, setToolsExpanded] = useState(false);
  const [expandedToolKey, setExpandedToolKey] = useState<string | null>(null);
  const [loadedToolCallbacks, setLoadedToolCallbacks] = useState<
    Record<string, Omit<LlmToolCallback, 'key'>>
  >({});
  const [loadingToolKey, setLoadingToolKey] = useState<string | null>(null);
  const [failedToolKeys, setFailedToolKeys] = useState<Set<string>>(
    () => new Set()
  );
  const mountedRef = useRef(true);
  const debugPayloadList = useMemo(
    () => debugPayloads ?? [debugPayload],
    [debugPayload, debugPayloads]
  );
  const toolCallbacks = useMemo(
    () => collectLlmToolCallbacksFromDebugPayloads(debugPayloadList),
    [debugPayloadList]
  );
  const effectiveToolCallbacks = useMemo(
    () =>
      toolCallbacks.map((callback) => {
        const loadedCallback = loadedToolCallbacks[callback.id];

        if (!loadedCallback) {
          return callback;
        }

        return {
          ...callback,
          ...loadedCallback,
          key: callback.key,
          call_usage: loadedCallback.call_usage ?? callback.call_usage,
          result_context_usage:
            loadedCallback.result_context_usage ??
            callback.result_context_usage,
          duration_ms: loadedCallback.duration_ms ?? callback.duration_ms,
          detailArtifactRef:
            callback.detailArtifactRef ?? loadedCallback.detailArtifactRef,
          routeTrace: loadedCallback.routeTrace ?? callback.routeTrace
        };
      }),
    [loadedToolCallbacks, toolCallbacks]
  );
  useEffect(() => {
    mountedRef.current = true;

    return () => {
      mountedRef.current = false;
    };
  }, []);

  useEffect(() => {
    setToolsExpanded(false);
    setExpandedToolKey(null);
    setLoadedToolCallbacks({});
    setLoadingToolKey(null);
    setFailedToolKeys(new Set());
  }, [debugPayload]);

  const loadToolCallbackDetail = (callback: LlmToolCallback) => {
    if (!callback.detailArtifactRef || !onLoadArtifact) {
      return;
    }
    if (loadedToolCallbacks[callback.id] || loadingToolKey === callback.key) {
      return;
    }

    setLoadingToolKey(callback.key);
    setFailedToolKeys((current) => {
      const next = new Set(current);
      next.delete(callback.key);
      return next;
    });

    void onLoadArtifact(callback.detailArtifactRef)
      .then((payload) => {
        if (!mountedRef.current) {
          return;
        }
        const loadedCallback = readLlmToolCallbackDetail(payload);

        if (!loadedCallback) {
          throw new Error('invalid_tool_callback_detail');
        }

        setLoadedToolCallbacks((current) => ({
          ...current,
          [callback.id]: loadedCallback
        }));
      })
      .catch(() => {
        if (!mountedRef.current) {
          return;
        }

        setFailedToolKeys((current) => new Set(current).add(callback.key));
      })
      .finally(() => {
        if (!mountedRef.current) {
          return;
        }

        setLoadingToolKey((current) =>
          current === callback.key ? null : current
        );
      });
  };

  if (effectiveToolCallbacks.length === 0) {
    return null;
  }

  const handleToggleTools = () => {
    setToolsExpanded((current) => !current);
  };

  const summaryText =
    effectiveToolCallbacks.length > 0
      ? i18nText('agentFlow', 'auto.tool_callbacks', {
          value1: effectiveToolCallbacks.length
        })
      : i18nText('agentFlow', 'auto.need_to_load');

  return (
    <section
      aria-label={i18nText('agentFlow', 'auto.llm_tools')}
      className="agent-flow-editor__debug-llm-tools"
    >
      <button
        aria-expanded={toolsExpanded}
        className="agent-flow-editor__debug-llm-tools-trigger"
        onClick={handleToggleTools}
        type="button"
      >
        <span className="agent-flow-editor__debug-llm-tools-title">
          <ToolOutlined className="agent-flow-editor__debug-llm-tools-icon" />
          <Typography.Text strong>
            {i18nText('agentFlow', 'auto.tools')}
          </Typography.Text>
          <Typography.Text type="secondary">{summaryText}</Typography.Text>
        </span>
        {toolsExpanded ? (
          <DownOutlined className="agent-flow-editor__debug-workflow-collapse" />
        ) : (
          <RightOutlined className="agent-flow-editor__debug-workflow-collapse" />
        )}
      </button>
      {toolsExpanded ? (
        <div className="agent-flow-editor__debug-llm-tools-body">
          {effectiveToolCallbacks.length > 0 ? (
            <>
              <div
                aria-label={i18nText('agentFlow', 'auto.tool_callback_list')}
                className="agent-flow-editor__debug-llm-tool-list"
              >
                {effectiveToolCallbacks.map((callback) => {
                  const expanded = expandedToolKey === callback.key;

                  return (
                    <LlmToolCallbackItem
                      key={callback.key}
                      callback={callback}
                      expanded={expanded}
                      loadFailed={failedToolKeys.has(callback.key)}
                      loading={loadingToolKey === callback.key}
                      onLoadArtifact={onLoadArtifact}
                      onToggle={() => {
                        const nextExpanded = !expanded;
                        setExpandedToolKey(nextExpanded ? callback.key : null);

                        if (nextExpanded) {
                          loadToolCallbackDetail(callback);
                        }
                      }}
                    />
                  );
                })}
              </div>
            </>
          ) : (
            <Typography.Text type="secondary">
              {i18nText('agentFlow', 'auto.tool_callback_truncated')}
            </Typography.Text>
          )}
        </div>
      ) : null}
    </section>
  );
}
