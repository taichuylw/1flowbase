import { App, Button, Card, Space, Tag, Typography } from 'antd';
import { useEffect, useMemo, useState } from 'react';

import type { NodeLastRun } from '../../../api/runtime';
import { fetchRuntimeDebugArtifact } from '../../../api/runtime';
import { JsonPreviewBlock } from '../../../../../shared/ui/json-preview/JsonPreviewBlock';

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
  if (
    record.kind === 'start_input_summary' &&
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

function isRecord(value: unknown): value is Record<string, unknown> {
  return Boolean(value && typeof value === 'object' && !Array.isArray(value));
}

type ConsoleLogLevel = 'info' | 'warn' | 'error';

interface ConsoleLogEntryView {
  level: ConsoleLogLevel;
  message: string;
  args: unknown[];
}

function normalizeConsoleLogLevel(value: unknown): ConsoleLogLevel {
  if (value === 'warn' || value === 'error') {
    return value;
  }

  return 'info';
}

function formatConsoleLogMessage(value: unknown) {
  if (typeof value === 'string') {
    return value;
  }

  if (Array.isArray(value)) {
    return value
      .map((item) => {
        if (typeof item === 'string') {
          return item;
        }

        const serialized = JSON.stringify(item);
        return serialized ?? String(item);
      })
      .join(' ');
  }

  return '';
}

function readConsoleLogs(debugPayload: unknown): ConsoleLogEntryView[] {
  if (!isRecord(debugPayload) || !Array.isArray(debugPayload.console_logs)) {
    return [];
  }

  return debugPayload.console_logs
    .filter(isRecord)
    .map((entry) => {
      const args = Array.isArray(entry.args) ? entry.args : [];

      return {
        level: normalizeConsoleLogLevel(entry.level),
        message: formatConsoleLogMessage(entry.message || args),
        args
      };
    });
}

function pickProcessPayload(debugPayload: unknown) {
  if (!isRecord(debugPayload)) {
    return {};
  }

  const consoleLogs = readConsoleLogs(debugPayload);

  if (consoleLogs.length === 0) {
    return debugPayload;
  }

  return {
    ...debugPayload,
    console_logs: consoleLogs
  };
}

function getConsoleLogTagColor(level: ConsoleLogLevel) {
  if (level === 'error') {
    return 'error';
  }

  if (level === 'warn') {
    return 'warning';
  }

  return 'processing';
}

function NodeRunConsoleLogs({ logs }: { logs: ConsoleLogEntryView[] }) {
  if (logs.length === 0) {
    return null;
  }

  return (
    <section aria-label="控制台日志" className="agent-flow-node-run-console">
      <Typography.Text
        className="agent-flow-node-run-console__title"
        strong
      >
        控制台日志
      </Typography.Text>
      <div className="agent-flow-node-run-console__list">
        {logs.map((log, index) => (
          <div
            key={`${log.level}-${index}`}
            className="agent-flow-node-run-console__row"
          >
            <Tag
              className="agent-flow-node-run-console__level"
              color={getConsoleLogTagColor(log.level)}
            >
              {log.level.toUpperCase()}
            </Tag>
            <Typography.Text className="agent-flow-node-run-console__message">
              {log.message}
            </Typography.Text>
          </div>
        ))}
      </div>
    </section>
  );
}

export function NodeRunPayloadSections({
  inputPayload,
  debugPayload,
  outputPayload,
  includeDebugPayload = true,
  onLoadArtifact
}: {
  inputPayload: unknown;
  debugPayload: unknown;
  outputPayload: unknown;
  includeDebugPayload?: boolean;
  onLoadArtifact?: (artifactRef: string) => Promise<unknown>;
}) {
  const processPayload = pickProcessPayload(debugPayload);
  const consoleLogs = readConsoleLogs(debugPayload);

  return (
    <>
      <NodeRunJsonBlock
        payload={inputPayload}
        title="输入"
        onLoadArtifact={onLoadArtifact}
      />
      {includeDebugPayload ? (
        <>
          <NodeRunConsoleLogs logs={consoleLogs} />
          <NodeRunJsonBlock
            payload={processPayload}
            title="数据处理"
            onLoadArtifact={onLoadArtifact}
          />
        </>
      ) : null}
      <NodeRunJsonBlock
        payload={outputPayload}
        title="输出"
        onLoadArtifact={onLoadArtifact}
      />
    </>
  );
}

function NodeRunJsonBlock({
  title,
  payload,
  onLoadArtifact
}: {
  title: string;
  payload: unknown;
  onLoadArtifact?: (artifactRef: string) => Promise<unknown>;
}) {
  const { message } = App.useApp();
  const [loadedPayload, setLoadedPayload] = useState<unknown>(null);
  const artifactRef = useMemo(
    () => findRuntimeDebugArtifactRef(payload),
    [payload]
  );
  const displayPayload = loadedPayload ?? payload;

  useEffect(() => {
    setLoadedPayload(null);
  }, [payload]);

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
    <JsonPreviewBlock
      actions={
        artifactRef ? (
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
        ) : null
      }
      height="220px"
      title={title}
      value={displayPayload}
    />
  );
}

export function NodeRunIOCard({ lastRun }: { lastRun: NodeLastRun }) {
  const applicationId = lastRun.flow_run.application_id;

  return (
    <Card title="节点输入输出">
      <div className="agent-flow-node-run-json-list">
        <NodeRunPayloadSections
          inputPayload={
            lastRun.node_run.input_payload_view ?? lastRun.node_run.input_payload
          }
          debugPayload={lastRun.node_run.debug_payload}
          outputPayload={lastRun.node_run.output_payload}
          onLoadArtifact={(artifactRef) =>
            fetchRuntimeDebugArtifact(applicationId, artifactRef)
          }
        />
      </div>
    </Card>
  );
}
