import { Alert, Button, Descriptions, Input, Space, Typography } from 'antd';
import { useEffect, useMemo, useState } from 'react';

import type { NormalizedFrontstageBlockCatalogEntry } from '../lib/block-catalog';
import type { FrontstageBlockInstance } from '../lib/page-document';
import {
  createRestrictedBlockRunPlan,
  type RestrictedBlockLoaderLimits
} from '../lib/restricted-block-loader';

export interface JsBlockTrialPanelProps {
  block: FrontstageBlockInstance | null | undefined;
  catalogEntry: NormalizedFrontstageBlockCatalogEntry | null | undefined;
  code: string;
  contextSnapshot: Record<string, unknown>;
  limits?: RestrictedBlockLoaderLimits;
  onCodeChange?: (code: string) => void;
  onContextSnapshotChange?: (contextSnapshot: Record<string, unknown>) => void;
  onLimitsChange?: (limits: RestrictedBlockLoaderLimits) => void;
}

type JsonDraftKind = 'context' | 'limits';

interface JsonDraftError {
  kind: JsonDraftKind;
  message: string;
}

function stringifyDraft(value: unknown): string {
  return JSON.stringify(value ?? {}, null, 2);
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function parseJsonObject(
  value: string,
  label: string
):
  | { ok: true; value: Record<string, unknown> }
  | { ok: false; message: string } {
  try {
    const parsed = JSON.parse(value) as unknown;
    if (!isRecord(parsed)) {
      return { ok: false, message: `${label} 必须是 JSON object。` };
    }
    return { ok: true, value: parsed };
  } catch {
    return { ok: false, message: `${label} 不是有效 JSON。` };
  }
}

function formatList(values: readonly unknown[] | undefined): string {
  if (!values || values.length === 0) {
    return '无';
  }
  return values.map(String).join(', ');
}

function formatKeys(value: Record<string, unknown>): string {
  return formatList(Object.keys(value));
}

function formatNumber(value: number | undefined): string {
  return typeof value === 'number' ? String(value) : '未设置';
}

function toRuntimeLimits(
  value: Record<string, unknown>
): RestrictedBlockLoaderLimits {
  return value as RestrictedBlockLoaderLimits;
}

export function JsBlockTrialPanel({
  block,
  catalogEntry,
  code,
  contextSnapshot,
  limits,
  onCodeChange,
  onContextSnapshotChange,
  onLimitsChange
}: JsBlockTrialPanelProps) {
  const [contextDraft, setContextDraft] = useState(() =>
    stringifyDraft(contextSnapshot)
  );
  const [limitsDraft, setLimitsDraft] = useState(() => stringifyDraft(limits));
  const [draftError, setDraftError] = useState<JsonDraftError | null>(null);

  useEffect(() => {
    setContextDraft(stringifyDraft(contextSnapshot));
  }, [contextSnapshot]);

  useEffect(() => {
    setLimitsDraft(stringifyDraft(limits));
  }, [limits]);

  const runPlan = useMemo(() => {
    if (!block || !catalogEntry) {
      return null;
    }

    return createRestrictedBlockRunPlan({
      block,
      catalogEntry,
      code,
      contextSnapshot,
      limits
    });
  }, [block, catalogEntry, code, contextSnapshot, limits]);

  function applyContextDraft() {
    const parsed = parseJsonObject(contextDraft, 'Context snapshot');
    if (!parsed.ok) {
      setDraftError({ kind: 'context', message: parsed.message });
      return;
    }

    setDraftError(null);
    onContextSnapshotChange?.(parsed.value);
  }

  function applyLimitsDraft() {
    const parsed = parseJsonObject(limitsDraft, 'Runtime limits');
    if (!parsed.ok) {
      setDraftError({ kind: 'limits', message: parsed.message });
      return;
    }

    setDraftError(null);
    onLimitsChange?.(toRuntimeLimits(parsed.value));
  }

  if (!block) {
    return (
      <Alert
        type="info"
        showIcon
        message="请选择一个区块"
        description="JS Block 试运行计划需要当前选中的区块。"
      />
    );
  }

  if (!catalogEntry) {
    return (
      <Alert
        type="warning"
        showIcon
        message="缺少区块目录条目"
        description="当前区块无法匹配可用 catalog entry。"
      />
    );
  }

  return (
    <Space direction="vertical" size="small" style={{ width: '100%' }}>
      <Typography.Title level={5} style={{ margin: 0 }}>
        JS Block 试运行
      </Typography.Title>

      <Space direction="vertical" size="small" style={{ width: '100%' }}>
        <Typography.Text strong>JS 代码</Typography.Text>
        <Input.TextArea
          aria-label="JS 代码"
          value={code}
          rows={5}
          readOnly={!onCodeChange}
          onChange={(event) => onCodeChange?.(event.target.value)}
        />
      </Space>

      <Space direction="vertical" size="small" style={{ width: '100%' }}>
        <Typography.Text strong>Context snapshot</Typography.Text>
        <Input.TextArea
          aria-label="Context snapshot"
          value={contextDraft}
          rows={4}
          onChange={(event) => setContextDraft(event.target.value)}
        />
        <Button
          size="small"
          disabled={!onContextSnapshotChange}
          onClick={applyContextDraft}
        >
          更新 context
        </Button>
      </Space>

      <Space direction="vertical" size="small" style={{ width: '100%' }}>
        <Typography.Text strong>Runtime limits</Typography.Text>
        <Input.TextArea
          aria-label="Runtime limits"
          value={limitsDraft}
          rows={4}
          onChange={(event) => setLimitsDraft(event.target.value)}
        />
        <Button
          size="small"
          disabled={!onLimitsChange}
          onClick={applyLimitsDraft}
        >
          更新 limits
        </Button>
      </Space>

      {draftError ? (
        <Alert
          type="error"
          showIcon
          message={
            draftError.kind === 'context'
              ? 'Context 更新失败'
              : 'Limits 更新失败'
          }
          description={draftError.message}
        />
      ) : null}

      {runPlan?.ok ? (
        <Space direction="vertical" size="small" style={{ width: '100%' }}>
          <Alert type="success" showIcon message="Run plan 已生成" />
          <Descriptions
            bordered
            size="small"
            column={1}
            title="Request 摘要"
            items={[
              {
                key: 'requestId',
                label: 'Request ID',
                children: runPlan.request.requestId
              },
              {
                key: 'blockId',
                label: 'Block ID',
                children: runPlan.request.blockId
              },
              {
                key: 'sourceLength',
                label: 'Source length',
                children: `${runPlan.request.source.length} chars`
              },
              {
                key: 'timeout',
                label: 'Timeout',
                children: `${runPlan.request.limits.timeoutMs}ms`
              },
              {
                key: 'props',
                label: 'Props keys',
                children: formatKeys(runPlan.request.props)
              },
              {
                key: 'context',
                label: 'Context keys',
                children: formatKeys(runPlan.request.contextSnapshot)
              }
            ]}
          />
          <Descriptions
            bordered
            size="small"
            column={1}
            title="Schema validation options"
            data-testid="js-block-trial-schema-options"
            items={[
              {
                key: 'maxDepth',
                label: 'Max depth',
                children: formatNumber(runPlan.schemaValidationOptions.maxDepth)
              },
              {
                key: 'maxNodes',
                label: 'Max nodes',
                children: formatNumber(runPlan.schemaValidationOptions.maxNodes)
              },
              {
                key: 'data',
                label: 'Data permissions',
                children: formatList(
                  runPlan.schemaValidationOptions.allowedDataPermissions
                )
              },
              {
                key: 'actions',
                label: 'Actions',
                children: formatList(
                  runPlan.schemaValidationOptions.allowedActions
                )
              },
              {
                key: 'events',
                label: 'Events',
                children: formatList(runPlan.schemaValidationOptions.allowedEvents)
              }
            ]}
          />
          <Descriptions
            bordered
            size="small"
            column={1}
            title="Mediator policy"
            data-testid="js-block-trial-mediator-policy"
            items={[
              {
                key: 'actions',
                label: 'Actions',
                children: formatList(runPlan.mediatorPolicy.allowedActions)
              },
              {
                key: 'events',
                label: 'Events',
                children: formatList(runPlan.mediatorPolicy.allowedEvents)
              },
              {
                key: 'models',
                label: 'Data models',
                children: formatList(runPlan.mediatorPolicy.allowedDataModels)
              },
              {
                key: 'operations',
                label: 'Data operations',
                children: formatList(runPlan.mediatorPolicy.allowedDataOperations)
              },
              {
                key: 'maxEventChainDepth',
                label: 'Max event chain depth',
                children: formatNumber(runPlan.mediatorPolicy.maxEventChainDepth)
              }
            ]}
          />
        </Space>
      ) : null}

      {runPlan && !runPlan.ok ? (
        <Space direction="vertical" size="small" style={{ width: '100%' }}>
          <Alert
            type="error"
            showIcon
            message="Run plan 被拒绝"
            description={runPlan.message}
          />
          <Descriptions
            bordered
            size="small"
            column={1}
            title="Rejection"
            items={[
              { key: 'code', label: 'Code', children: runPlan.code },
              { key: 'path', label: 'Path', children: runPlan.path },
              { key: 'blockId', label: 'Block ID', children: runPlan.blockId },
              {
                key: 'catalogId',
                label: 'Catalog ID',
                children: runPlan.catalogId
              }
            ]}
          />
        </Space>
      ) : null}
    </Space>
  );
}
