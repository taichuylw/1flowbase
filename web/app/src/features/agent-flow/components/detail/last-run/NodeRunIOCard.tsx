import {
  CheckOutlined,
  CopyOutlined,
  DownOutlined,
  FullscreenOutlined
} from '@ant-design/icons';
import { App, Button, Card, Modal, Space, Tag, Tooltip } from 'antd';
import { Suspense, lazy, useEffect, useMemo, useState } from 'react';

import type { NodeLastRun } from '../../../api/runtime';
import { fetchRuntimeDebugArtifact } from '../../../api/runtime';
import { useClipboardCopy } from '../../../../../shared/ui/clipboard/use-clipboard-copy';

const MonacoEditor = lazy(() => import('@monaco-editor/react'));

function formatJson(payload: unknown) {
  return JSON.stringify(payload, null, 2);
}

function findRuntimeDebugArtifactRef(value: unknown): string | null {
  if (!value || typeof value !== 'object') {
    return null;
  }

  if (Array.isArray(value)) {
    for (const item of value) {
      const nestedRef = findRuntimeDebugArtifactRef(item);
      if (nestedRef) {
        return nestedRef;
      }
    }
    return null;
  }

  const record = value as Record<string, unknown>;
  if (
    record.__runtime_debug_artifact === true &&
    typeof record.artifact_ref === 'string'
  ) {
    return record.artifact_ref;
  }

  for (const nestedValue of Object.values(record)) {
    const nestedRef = findRuntimeDebugArtifactRef(nestedValue);
    if (nestedRef) {
      return nestedRef;
    }
  }

  return null;
}

function getDurationMs(startedAt: string, finishedAt: string | null) {
  if (!finishedAt) {
    return null;
  }

  const started = Date.parse(startedAt);
  const finished = Date.parse(finishedAt);

  if (Number.isNaN(started) || Number.isNaN(finished)) {
    return null;
  }

  return Math.max(0, finished - started);
}

function getMetricsPayload(lastRun: NodeLastRun) {
  const metrics = lastRun.node_run.metrics_payload ?? {};
  const errorPayload =
    lastRun.node_run.error_payload &&
    typeof lastRun.node_run.error_payload === 'object'
      ? lastRun.node_run.error_payload
      : {};

  return {
    usage:
      (metrics.usage as unknown) ??
      (typeof metrics.total_tokens === 'number' || typeof metrics.total_tokens === 'string'
        ? { total_tokens: metrics.total_tokens }
        : null),
    duration_ms: getDurationMs(
      lastRun.node_run.started_at,
      lastRun.node_run.finished_at
    ),
    route:
      (metrics.route as unknown) ?? {
        provider_instance_id:
          (metrics.provider_instance_id as unknown) ??
          (errorPayload as Record<string, unknown>).provider_instance_id,
        provider_code:
          (metrics.provider_code as unknown) ??
          (errorPayload as Record<string, unknown>).provider_code,
        protocol:
          (metrics.protocol as unknown) ??
          (errorPayload as Record<string, unknown>).protocol
      },
    attempt:
      (metrics.attempt as unknown) ??
      (metrics.attempt_id as unknown) ??
      (metrics.attempt_index as unknown) ??
      null,
    finish_reason:
      (metrics.finish_reason as unknown) ??
      (errorPayload as Record<string, unknown>).finish_reason ??
      null,
    metrics_payload: metrics
  };
}

const EDITOR_OPTIONS = {
  readOnly: true,
  domReadOnly: true,
  minimap: { enabled: false },
  scrollBeyondLastLine: false,
  wordWrap: 'on' as const,
  lineNumbersMinChars: 2,
  fontSize: 12,
  lineHeight: 18,
  folding: true,
  renderLineHighlight: 'none' as const,
  overviewRulerBorder: false,
  automaticLayout: true,
  padding: {
    top: 8,
    bottom: 8
  },
  scrollbar: {
    verticalScrollbarSize: 8,
    horizontalScrollbarSize: 8
  }
};

function JsonEditorFallback() {
  return (
    <div className="agent-flow-node-run-json-viewer__loading">
      正在加载 JSON 查看器
    </div>
  );
}

function JsonEditor({ height, value }: { height: string; value: string }) {
  return (
    <Suspense fallback={<JsonEditorFallback />}>
      <MonacoEditor
        defaultLanguage="json"
        height={height}
        options={EDITOR_OPTIONS}
        theme="vs"
        value={value}
      />
    </Suspense>
  );
}

export function NodeRunJsonBlock({
  title,
  payload,
  onLoadArtifact
}: {
  title: string;
  payload: unknown;
  onLoadArtifact?: (artifactRef: string) => Promise<unknown>;
}) {
  const { message } = App.useApp();
  const [collapsed, setCollapsed] = useState(false);
  const [expanded, setExpanded] = useState(false);
  const [loadedPayload, setLoadedPayload] = useState<unknown>(null);
  const { copied, copy } = useClipboardCopy();
  const artifactRef = useMemo(() => findRuntimeDebugArtifactRef(payload), [payload]);
  const displayPayload = loadedPayload ?? payload;
  const value = useMemo(() => formatJson(displayPayload), [displayPayload]);

  useEffect(() => {
    setLoadedPayload(null);
  }, [payload]);

  const handleCopy = async () => {
    try {
      await copy(value);
      message.success('已复制');
    } catch {
      message.error('复制失败');
    }
  };

  const handleLoadFullValue = async () => {
    if (!artifactRef || !onLoadArtifact) {
      return;
    }

    try {
      setLoadedPayload(await onLoadArtifact(artifactRef));
      message.success('已加载完整值');
    } catch {
      message.error('加载完整值失败');
    }
  };

  return (
    <section className="agent-flow-node-run-json-viewer">
      <pre
        aria-label={`${title} JSON`}
        className="agent-flow-node-run-json__a11y"
      >
        {value}
      </pre>
      <div className="agent-flow-node-run-json-viewer__header">
        <button
          aria-label={title}
          aria-expanded={!collapsed}
          className="agent-flow-node-run-json-viewer__toggle"
          onClick={() => setCollapsed((current) => !current)}
          type="button"
        >
          <DownOutlined className="agent-flow-node-run-json-viewer__toggle-icon" />
          <span className="agent-flow-node-run-json-viewer__title">
            {title}
          </span>
        </button>
        <div className="agent-flow-node-run-json-viewer__actions">
          {artifactRef ? (
            <Space size={6} wrap>
              <Tag color="warning">已截断</Tag>
              <Button
                disabled={!onLoadArtifact}
                onClick={handleLoadFullValue}
                size="small"
              >
                加载完整值
              </Button>
            </Space>
          ) : null}
          <Tooltip title="复制 JSON">
            <Button
              aria-label={`复制${title} JSON`}
              icon={copied ? <CheckOutlined /> : <CopyOutlined />}
              onClick={handleCopy}
              size="small"
              type="text"
            />
          </Tooltip>
          <Tooltip title="放大查看">
            <Button
              aria-label={`放大查看${title} JSON`}
              disabled={collapsed}
              icon={<FullscreenOutlined />}
              onClick={() => setExpanded(true)}
              size="small"
              type="text"
            />
          </Tooltip>
        </div>
      </div>
      {!collapsed ? (
        <div className="agent-flow-node-run-json-viewer__editor">
          <JsonEditor height="220px" value={value} />
        </div>
      ) : null}
      <Modal
        centered
        className="agent-flow-node-run-json-modal"
        footer={null}
        onCancel={() => setExpanded(false)}
        open={expanded}
        title={`${title} JSON`}
        width="min(960px, calc(100vw - 48px))"
      >
        <div className="agent-flow-node-run-json-modal__editor">
          <JsonEditor height="70vh" value={value} />
        </div>
      </Modal>
    </section>
  );
}

export function NodeRunIOCard({ lastRun }: { lastRun: NodeLastRun }) {
  const applicationId = lastRun.flow_run.application_id;

  return (
    <Card title="节点输入输出">
      <div className="agent-flow-node-run-json-list">
        <NodeRunJsonBlock
          payload={lastRun.node_run.input_payload}
          title="输入"
          onLoadArtifact={(artifactRef) =>
            fetchRuntimeDebugArtifact(applicationId, artifactRef)
          }
        />
        <NodeRunJsonBlock
          payload={lastRun.node_run.output_payload}
          title="输出"
          onLoadArtifact={(artifactRef) =>
            fetchRuntimeDebugArtifact(applicationId, artifactRef)
          }
        />
        <NodeRunJsonBlock
          payload={getMetricsPayload(lastRun)}
          title="指标"
          onLoadArtifact={(artifactRef) =>
            fetchRuntimeDebugArtifact(applicationId, artifactRef)
          }
        />
        <NodeRunJsonBlock
          payload={lastRun.node_run.error_payload}
          title="错误"
          onLoadArtifact={(artifactRef) =>
            fetchRuntimeDebugArtifact(applicationId, artifactRef)
          }
        />
        <NodeRunJsonBlock
          payload={lastRun.node_run.debug_payload ?? {}}
          title="Debug"
          onLoadArtifact={(artifactRef) =>
            fetchRuntimeDebugArtifact(applicationId, artifactRef)
          }
        />
      </div>
    </Card>
  );
}
