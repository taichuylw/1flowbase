import { DownOutlined, RightOutlined, ToolOutlined } from '@ant-design/icons';
import { Tag, Tooltip, Typography } from 'antd';
import { ReactNode, useEffect, useMemo, useRef, useState } from 'react';

import type { AgentFlowTraceItem } from '../../../api/runtime';
import { RuntimeDebugPayloadBlock } from '../../detail/last-run/NodeRunIOCard';
import { DebugWorkflowNodeItem } from './DebugWorkflowNodeRow';
import {
  collectLlmToolCallbacksFromDebugPayloads,
  readLlmToolCallbackDetail,
  type LlmToolCallback,
  type LlmToolRouteBranchSummary,
  type LlmToolRouteTraceSummary
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
  branch: LlmToolRouteBranchSummary,
  index: number
): AgentFlowTraceItem {
  const fallbackId = `${callback.id}:branch:${index + 1}`;

  return {
    nodeId: branch.nodeId ?? fallbackId,
    nodeRunId: `${fallbackId}:summary`,
    nodeAlias: branch.nodeAlias ?? branch.nodeId ?? `Panel ${index + 1}`,
    nodeType: branch.nodeType ?? 'llm',
    status: branchNodeStatus(branch),
    startedAt: '',
    finishedAt: callback.callbackStatus === 'returned' ? '' : null,
    durationMs: null,
    inputPayload: {},
    outputPayload: {},
    errorPayload: null,
    metricsPayload: {},
    debugPayload: branch.rawPayload
  };
}

function summaryPreviewText(summary: Record<string, unknown> | null) {
  const preview = summary?.preview;

  return typeof preview === 'string' && preview.trim().length > 0
    ? preview
    : null;
}

function LlmToolRouteBranchNode({
  branch,
  callback,
  index,
  onLoadArtifact
}: {
  branch: LlmToolRouteBranchSummary;
  callback: LlmToolCallback;
  index: number;
  onLoadArtifact?: (artifactRef: string) => Promise<unknown>;
}) {
  const [expanded, setExpanded] = useState(false);
  const branchTraceItem = buildRouteBranchTraceItem(callback, branch, index);
  const outputPreview = summaryPreviewText(branch.outputSummary);
  const branchDetailTitle =
    branch.nodeAlias ?? branch.nodeId ?? `Panel ${index + 1}`;

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
          {branch.routeModel || outputPreview ? (
            <div className="agent-flow-editor__debug-llm-route-branch-meta">
              {branch.routeModel ? <Tag>{branch.routeModel}</Tag> : null}
              {outputPreview ? (
                <Typography.Text ellipsis type="secondary">
                  {outputPreview}
                </Typography.Text>
              ) : null}
            </div>
          ) : null}
          <RuntimeDebugPayloadBlock
            height="8rem"
            payload={branch.rawPayload}
            title={branchDetailTitle}
            onLoadArtifact={onLoadArtifact}
          />
        </div>
      </DebugWorkflowNodeItem>
    </div>
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

  if (!callback.routeTrace) {
    return null;
  }

  const routeTraceItem = buildRouteTraceItem(callback);
  const branchSummaries = callback.routeTrace.branchSummaries;
  const routeTraceTitle = routeTraceLabel(callback.routeTrace);

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
          {branchSummaries.length > 0 ? (
            <div className="agent-flow-editor__debug-llm-route-branch-list">
              {branchSummaries.map((branch, index) => (
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
            payload={callback.routeTrace.rawPayload}
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
