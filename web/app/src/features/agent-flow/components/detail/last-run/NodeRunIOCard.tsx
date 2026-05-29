import { App, Button, Card, Space, Tag, Typography } from 'antd';
import { useEffect, useMemo, useState } from 'react';

import type { NodeLastRun } from '../../../api/runtime';
import { fetchRuntimeDebugArtifact } from '../../../api/runtime';
import { JsonPreviewBlock } from '../../../../../shared/ui/json-preview/JsonPreviewBlock';
import { i18nText } from '../../../../../shared/i18n/text';

interface RuntimeDebugArtifactLocation {
  artifactRef: string;
  path: string[];
}

function findRuntimeDebugArtifactLocation(
  value: unknown,
  path: string[] = []
): RuntimeDebugArtifactLocation | null {
  if (!value || typeof value !== 'object') {
    return null;
  }

  if (Array.isArray(value)) {
    for (const [index, item] of value.entries()) {
      const nestedLocation = findRuntimeDebugArtifactLocation(item, [
        ...path,
        String(index)
      ]);
      if (nestedLocation) {
        return nestedLocation;
      }
    }
    return null;
  }

  const record = value as Record<string, unknown>;
  if (
    record.__runtime_debug_artifact === true &&
    typeof record.artifact_ref === 'string'
  ) {
    return {
      artifactRef: record.artifact_ref,
      path
    };
  }

  for (const [key, nestedValue] of Object.entries(record)) {
    const nestedLocation = findRuntimeDebugArtifactLocation(nestedValue, [
      ...path,
      key
    ]);
    if (nestedLocation) {
      return nestedLocation;
    }
  }

  return null;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return Boolean(value && typeof value === 'object' && !Array.isArray(value));
}

function replacePayloadAtPath(
  currentPayload: unknown,
  path: string[],
  fullPayload: unknown
): unknown {
  if (path.length === 0) {
    return fullPayload;
  }

  const [head, ...tail] = path;

  if (Array.isArray(currentPayload)) {
    const index = Number(head);

    if (
      !Number.isInteger(index) ||
      index < 0 ||
      index >= currentPayload.length
    ) {
      return currentPayload;
    }

    return currentPayload.map((item, itemIndex) =>
      itemIndex === index ? replacePayloadAtPath(item, tail, fullPayload) : item
    );
  }

  if (!isRecord(currentPayload) || !head) {
    return currentPayload;
  }

  return {
    ...currentPayload,
    [head]: replacePayloadAtPath(currentPayload[head], tail, fullPayload)
  };
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

  return debugPayload.console_logs.filter(isRecord).map((entry) => {
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
    <section aria-label={i18nText("agentFlow", "auto.console_log")} className="agent-flow-node-run-console">
      <Typography.Text className="agent-flow-node-run-console__title" strong>
        {i18nText("agentFlow", "auto.console_log")}</Typography.Text>
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
      <RuntimeDebugPayloadBlock
        payload={inputPayload}
        title={i18nText("agentFlow", "auto.input")}
        onLoadArtifact={onLoadArtifact}
      />
      {includeDebugPayload ? (
        <>
          <NodeRunConsoleLogs logs={consoleLogs} />
          <RuntimeDebugPayloadBlock
            payload={processPayload}
            title={i18nText("agentFlow", "auto.data_processing")}
            onLoadArtifact={onLoadArtifact}
          />
        </>
      ) : null}
      <RuntimeDebugPayloadBlock
        payload={outputPayload}
        title={i18nText("agentFlow", "auto.outputs")}
        onLoadArtifact={onLoadArtifact}
      />
    </>
  );
}

export function RuntimeDebugPayloadBlock({
  title,
  payload,
  defaultCollapsed = false,
  height = '220px',
  onLoadArtifact
}: {
  title: string;
  payload: unknown;
  defaultCollapsed?: boolean;
  height?: string;
  onLoadArtifact?: (artifactRef: string) => Promise<unknown>;
}) {
  const { message } = App.useApp();
  const [loadedPayload, setLoadedPayload] = useState<unknown>(null);
  const displayPayload = loadedPayload ?? payload;
  const artifactLocation = useMemo(
    () => findRuntimeDebugArtifactLocation(displayPayload),
    [displayPayload]
  );

  useEffect(() => {
    setLoadedPayload(null);
  }, [payload]);

  const handleLoadFullValue = async () => {
    if (!artifactLocation || !onLoadArtifact) {
      return;
    }

    try {
      const fullPayload = await onLoadArtifact(artifactLocation.artifactRef);
      setLoadedPayload((currentPayload: unknown) =>
        replacePayloadAtPath(
          currentPayload ?? payload,
          artifactLocation.path,
          fullPayload
        )
      );
      message.success(i18nText("agentFlow", "auto.full_value_loaded"));
    } catch {
      message.error(i18nText("agentFlow", "auto.failed_load_full_value"));
    }
  };

  return (
    <JsonPreviewBlock
      actions={
        artifactLocation ? (
          <Space size={6} wrap>
            <Tag color="warning">{i18nText("agentFlow", "auto.truncated")}</Tag>
            <Button
              disabled={!onLoadArtifact}
              onClick={handleLoadFullValue}
              size="small"
            >
              {i18nText("agentFlow", "auto.load_full_value")}</Button>
          </Space>
        ) : null
      }
      defaultCollapsed={defaultCollapsed}
      height={height}
      title={title}
      value={displayPayload}
    />
  );
}

export function NodeRunIOCard({ lastRun }: { lastRun: NodeLastRun }) {
  const applicationId = lastRun.flow_run.application_id;

  return (
    <Card title={i18nText("agentFlow", "auto.node_input_output")}>
      <div className="agent-flow-node-run-json-list">
        <NodeRunPayloadSections
          inputPayload={lastRun.node_run.input_payload}
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
