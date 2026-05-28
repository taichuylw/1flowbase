import { DownOutlined, RightOutlined, ToolOutlined } from '@ant-design/icons';
import { Tag, Typography } from 'antd';
import { useEffect, useMemo, useRef, useState } from 'react';

import { RuntimeDebugPayloadBlock } from '../../detail/last-run/NodeRunIOCard';
import {
  collectLlmToolCallbacksFromDebugPayloads,
  readLlmToolCallbackDetail,
  type LlmToolCallback
} from './llm-tool-callbacks';
import { i18nText } from '../../../../../shared/i18n/text';

function callbackStatusLabel(status: LlmToolCallback['callbackStatus']) {
  switch (status) {
    case 'returned':
      return i18nText("agentFlow", "auto.key_gnlajeegme");
    case 'cancelled':
      return i18nText("agentFlow", "auto.key_kfppnmjfoo");
    default:
      return i18nText("agentFlow", "auto.key_hadhedbmip");
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
      return i18nText("agentFlow", "auto.key_gmbijkknen");
    case 'failed':
      return i18nText("agentFlow", "auto.key_jhegmpmhnc");
    case 'timed_out':
      return i18nText("agentFlow", "auto.key_pcmdklpbde");
    case 'cancelled':
      return i18nText("agentFlow", "auto.key_fphigmijid");
    default:
      return i18nText("agentFlow", "auto.key_mamolhcoip");
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

function formatTokenDelta(tokenDelta: number) {
  return tokenDelta >= 0 ? `+${tokenDelta}` : `${tokenDelta}`;
}

function formatDuration(durationMs: number) {
  if (durationMs < 1000) {
    return `${durationMs} ms`;
  }

  const seconds = durationMs / 1000;
  const roundedSeconds = Math.round(seconds * 10) / 10;
  return `${Number.isInteger(roundedSeconds) ? roundedSeconds.toFixed(0) : roundedSeconds.toFixed(1)} s`;
}

function toolMetricsSummary(callback: LlmToolCallback) {
  const metrics = [
    typeof callback.token_delta === 'number'
      ? `${formatTokenDelta(callback.token_delta)} tokens`
      : null,
    typeof callback.duration_ms === 'number'
      ? formatDuration(callback.duration_ms)
      : null
  ].filter((metric): metric is string => Boolean(metric));

  return metrics.length > 0 ? metrics.join(' · ') : null;
}

function LlmToolInlineMetrics({ metricText }: { metricText: string | null }) {
  if (metricText === null) {
    return null;
  }

  return (
    <Typography.Text
      className="agent-flow-editor__debug-llm-tool-inline-metrics"
      type="secondary"
    >
      {metricText}
    </Typography.Text>
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
  const metricsText = toolMetricsSummary(callback);

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
          <LlmToolInlineMetrics metricText={metricsText} />
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
          {loading ? <Tag color="processing">{i18nText("agentFlow", "auto.key_mofgpgbhoe")}</Tag> : null}
          {loadFailed ? <Tag color="error">{i18nText("agentFlow", "auto.key_pglhkebofg")}</Tag> : null}
          {!loading && !loadFailed ? (
            <>
              <RuntimeDebugPayloadBlock
                height="11rem"
                payload={callback.requestPayload}
                title={i18nText("agentFlow", "auto.key_dnenbahfoh")}
                onLoadArtifact={onLoadArtifact}
              />
              {callback.parsedResult ? (
                <RuntimeDebugPayloadBlock
                  height="11rem"
                  payload={callback.parsedResult}
                  title={i18nText("agentFlow", "auto.key_jhmokaedkd")}
                  onLoadArtifact={onLoadArtifact}
                />
              ) : null}
              {callback.callbackPayload ? (
                <RuntimeDebugPayloadBlock
                  height="11rem"
                  payload={callback.callbackPayload}
                  title={i18nText("agentFlow", "auto.key_gdkplgflhp")}
                  onLoadArtifact={onLoadArtifact}
                />
              ) : (
                <Typography.Text type="secondary">{i18nText("agentFlow", "auto.key_mbikpabain")}</Typography.Text>
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
          duration_ms: loadedCallback.duration_ms ?? callback.duration_ms,
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
      ? i18nText("agentFlow", "auto.key_bljmhilamp", { value1: effectiveToolCallbacks.length })
      : i18nText("agentFlow", "auto.key_olclgakmgp");

  return (
    <section
      aria-label={i18nText("agentFlow", "auto.llm_tools")}
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
          <Typography.Text strong>{i18nText("agentFlow", "auto.tools")}</Typography.Text>
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
                aria-label={i18nText("agentFlow", "auto.key_onkehibejj")}
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
            <Typography.Text type="secondary">{i18nText("agentFlow", "auto.key_dblmiacohk")}</Typography.Text>
          )}
        </div>
      ) : null}
    </section>
  );
}
