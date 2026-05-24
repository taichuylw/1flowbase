import { DownOutlined, RightOutlined, ToolOutlined } from '@ant-design/icons';
import { Tag, Typography } from 'antd';
import { useEffect, useMemo, useRef, useState } from 'react';

import { RuntimeDebugPayloadBlock } from '../../detail/last-run/NodeRunIOCard';
import {
  collectLlmToolCallbacksFromDebugPayloads,
  readLlmToolCallbackDetail,
  type LlmToolCallback
} from './llm-tool-callbacks';

function callbackStatusLabel(status: LlmToolCallback['callbackStatus']) {
  switch (status) {
    case 'returned':
      return '已返回';
    case 'cancelled':
      return '已取消';
    default:
      return '等待回调';
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
      return '执行成功';
    case 'failed':
      return '执行失败';
    case 'timed_out':
      return '执行超时';
    case 'cancelled':
      return '执行取消';
    default:
      return '执行未知';
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

function toolTokenSummary(callback: LlmToolCallback) {
  return callback.token_delta;
}

function formatTokenDelta(tokenDelta: number) {
  return tokenDelta >= 0 ? `+${tokenDelta}` : `${tokenDelta}`;
}

function LlmToolInlineTokenSummary({
  totalTokens
}: {
  totalTokens: number | null;
}) {
  if (totalTokens === null) {
    return null;
  }

  return (
    <span className="agent-flow-editor__debug-llm-tool-inline-tokens">
      <Tag>{formatTokenDelta(totalTokens)} tokens</Tag>
    </span>
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
  const totalTokens = toolTokenSummary(callback);

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
          <LlmToolInlineTokenSummary totalTokens={totalTokens} />
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
          {loading ? <Tag color="processing">加载中</Tag> : null}
          {loadFailed ? <Tag color="error">加载失败</Tag> : null}
          {!loading && !loadFailed ? (
            <>
              <RuntimeDebugPayloadBlock
                height="11rem"
                payload={callback.requestPayload}
                title="工具调用"
                onLoadArtifact={onLoadArtifact}
              />
              {callback.parsedResult ? (
                <RuntimeDebugPayloadBlock
                  height="11rem"
                  payload={callback.parsedResult}
                  title="解析结果"
                  onLoadArtifact={onLoadArtifact}
                />
              ) : null}
              {callback.callbackPayload ? (
                <RuntimeDebugPayloadBlock
                  height="11rem"
                  payload={callback.callbackPayload}
                  title="完整回调"
                  onLoadArtifact={onLoadArtifact}
                />
              ) : (
                <Typography.Text type="secondary">等待回调返回</Typography.Text>
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
          token_delta: loadedCallback.token_delta ?? callback.token_delta,
          detailArtifactRef:
            callback.detailArtifactRef ?? loadedCallback.detailArtifactRef
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
      ? `${effectiveToolCallbacks.length} 次工具回调`
      : '需加载';

  return (
    <section
      aria-label="LLM Tools"
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
          <Typography.Text strong>Tools</Typography.Text>
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
                aria-label="工具回调列表"
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
            <Typography.Text type="secondary">工具回调已截断</Typography.Text>
          )}
        </div>
      ) : null}
    </section>
  );
}
