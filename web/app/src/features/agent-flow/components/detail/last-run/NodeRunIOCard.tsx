import { LineHeightOutlined } from '@ant-design/icons';
import { App, Button, Card, Space, Tag, Tooltip } from 'antd';
import { useEffect, useMemo, useState } from 'react';

import type { NodeLastRun } from '../../../api/runtime';
import { fetchRuntimeDebugArtifacts } from '../../../api/runtime';
import { JsonPreviewBlock } from '../../../../../shared/ui/json-preview/JsonPreviewBlock';
import { i18nText } from '../../../../../shared/i18n/text';

interface RuntimeDebugArtifactLocation {
  artifactRef: string;
  path: string[];
}

export interface RuntimeDebugArtifactLoadResult {
  artifacts: Array<{
    artifact_ref: string;
    value: unknown;
  }>;
}

export type RuntimeDebugArtifactBatchLoader = (
  artifactRefs: string[]
) => Promise<RuntimeDebugArtifactLoadResult>;

function collectRuntimeDebugArtifactLocations(
  value: unknown,
  path: string[] = []
): RuntimeDebugArtifactLocation[] {
  if (!value || typeof value !== 'object') {
    return [];
  }

  if (Array.isArray(value)) {
    return value.flatMap((item, index) =>
      collectRuntimeDebugArtifactLocations(item, [...path, String(index)])
    );
  }

  const record = value as Record<string, unknown>;
  if (
    record.__runtime_debug_artifact === true &&
    typeof record.artifact_ref === 'string'
  ) {
    return [
      {
        artifactRef: record.artifact_ref,
        path
      }
    ];
  }

  return Object.entries(record).flatMap(([key, nestedValue]) =>
    collectRuntimeDebugArtifactLocations(nestedValue, [...path, key])
  );
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

function uniqueArtifactRefs(locations: RuntimeDebugArtifactLocation[]) {
  const refs: string[] = [];
  const seen = new Set<string>();

  for (const location of locations) {
    if (seen.has(location.artifactRef)) {
      continue;
    }

    seen.add(location.artifactRef);
    refs.push(location.artifactRef);
  }

  return refs;
}

function artifactValueMapFromResult(result: RuntimeDebugArtifactLoadResult) {
  return new Map(
    result.artifacts.map((artifact) => [artifact.artifact_ref, artifact.value])
  );
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

function runtimePayloadHasValue(value: unknown): boolean {
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

export function NodeRunPayloadSections({
  inputPayload,
  debugPayload,
  outputPayload,
  includeDebugPayload = true,
  hideEmptyPayloads = false,
  onLoadArtifact,
  onLoadArtifacts
}: {
  inputPayload: unknown;
  debugPayload: unknown;
  outputPayload: unknown;
  includeDebugPayload?: boolean;
  hideEmptyPayloads?: boolean;
  onLoadArtifact?: (artifactRef: string) => Promise<unknown>;
  onLoadArtifacts?: RuntimeDebugArtifactBatchLoader;
}) {
  const processPayload = pickProcessPayload(debugPayload);
  const showInputPayload =
    !hideEmptyPayloads || runtimePayloadHasValue(inputPayload);
  const showDebugPayload =
    includeDebugPayload &&
    (!hideEmptyPayloads || runtimePayloadHasValue(processPayload));
  const showOutputPayload =
    !hideEmptyPayloads || runtimePayloadHasValue(outputPayload);

  return (
    <>
      {showInputPayload ? (
        <RuntimeDebugPayloadBlock
          payload={inputPayload}
          title={i18nText("agentFlow", "auto.input")}
          onLoadArtifact={onLoadArtifact}
          onLoadArtifacts={onLoadArtifacts}
        />
      ) : null}
      {showDebugPayload ? (
        <RuntimeDebugPayloadBlock
          payload={processPayload}
          title={i18nText("agentFlow", "auto.data_processing")}
          onLoadArtifact={onLoadArtifact}
          onLoadArtifacts={onLoadArtifacts}
        />
      ) : null}
      {showOutputPayload ? (
        <RuntimeDebugPayloadBlock
          payload={outputPayload}
          title={i18nText("agentFlow", "auto.outputs")}
          onLoadArtifact={onLoadArtifact}
          onLoadArtifacts={onLoadArtifacts}
        />
      ) : null}
    </>
  );
}

export function RuntimeDebugPayloadBlock({
  title,
  payload,
  defaultCollapsed = false,
  height = '220px',
  onLoadArtifact,
  onLoadArtifacts
}: {
  title: string;
  payload: unknown;
  defaultCollapsed?: boolean;
  height?: string;
  onLoadArtifact?: (artifactRef: string) => Promise<unknown>;
  onLoadArtifacts?: RuntimeDebugArtifactBatchLoader;
}) {
  const { message } = App.useApp();
  const [loadedPayload, setLoadedPayload] = useState<unknown>(null);
  const [loadingArtifacts, setLoadingArtifacts] = useState(false);
  const displayPayload = loadedPayload ?? payload;
  const artifactLocations = useMemo(
    () => collectRuntimeDebugArtifactLocations(displayPayload),
    [displayPayload]
  );
  const artifactRefs = useMemo(
    () => uniqueArtifactRefs(artifactLocations),
    [artifactLocations]
  );
  const canLoadArtifacts =
    artifactRefs.length > 0 && Boolean(onLoadArtifacts || onLoadArtifact);

  useEffect(() => {
    setLoadedPayload(null);
  }, [payload]);

  const handleLoadFullValue = async () => {
    if (artifactLocations.length === 0 || !canLoadArtifacts) {
      return;
    }

    setLoadingArtifacts(true);
    try {
      const artifactValues = onLoadArtifacts
        ? artifactValueMapFromResult(await onLoadArtifacts(artifactRefs))
        : new Map(
            await Promise.all(
              artifactRefs.map(async (artifactRef) => [
                artifactRef,
                await onLoadArtifact!(artifactRef)
              ] as const)
            )
          );

      for (const artifactRef of artifactRefs) {
        if (!artifactValues.has(artifactRef)) {
          throw new Error(`Missing runtime debug artifact ${artifactRef}`);
        }
      }

      setLoadedPayload((currentPayload: unknown) =>
        artifactLocations.reduce(
          (nextPayload, location) =>
            replacePayloadAtPath(
              nextPayload,
              location.path,
              artifactValues.get(location.artifactRef)
            ),
          currentPayload ?? payload
        )
      );
      message.success(i18nText("agentFlow", "auto.full_value_loaded"));
    } catch {
      message.error(i18nText("agentFlow", "auto.failed_load_full_value"));
    } finally {
      setLoadingArtifacts(false);
    }
  };

  return (
    <JsonPreviewBlock
      actions={
        artifactLocations.length > 0 ? (
          <Space size={6} wrap>
            <Tag color="warning">{i18nText("agentFlow", "auto.truncated")}</Tag>
            <Tooltip title={i18nText("agentFlow", "auto.load_full_value")}>
              <Button
                aria-label={i18nText("agentFlow", "auto.load_full_value")}
                disabled={!canLoadArtifacts}
                icon={<LineHeightOutlined />}
                loading={loadingArtifacts}
                onClick={handleLoadFullValue}
                size="small"
                type="text"
              />
            </Tooltip>
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
          onLoadArtifacts={(artifactRefs) =>
            fetchRuntimeDebugArtifacts(applicationId, artifactRefs)
          }
        />
      </div>
    </Card>
  );
}
