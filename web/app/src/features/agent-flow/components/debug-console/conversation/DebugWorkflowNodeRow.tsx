import {
  CheckCircleFilled,
  DownOutlined,
  LoadingOutlined,
  RightOutlined,
  WarningFilled
} from '@ant-design/icons';
import { Tag, Typography } from 'antd';
import type { ReactNode } from 'react';

import type { AgentFlowTraceItem } from '../../../api/runtime';
import { getAgentFlowNodeTypeIcon } from '../../../lib/node-type-icons';
import { nodeDisplayName } from './debug-workflow-trace-utils';
import { collectLlmToolCallbacks } from './llm-tool-callbacks';
import './debug-message.css';
import { i18nText } from '../../../../../shared/i18n/text';

function statusTone(status: string) {
  switch (status) {
    case 'succeeded':
      return 'success';
    case 'failed':
      return 'error';
    case 'waiting_human':
    case 'waiting_callback':
      return 'warning';
    default:
      return 'running';
  }
}

function readOutputTotalTokens(outputPayload: unknown) {
  if (
    !outputPayload ||
    typeof outputPayload !== 'object' ||
    Array.isArray(outputPayload)
  ) {
    return null;
  }

  const usage = (outputPayload as Record<string, unknown>).usage;

  if (!usage || typeof usage !== 'object' || Array.isArray(usage)) {
    return null;
  }

  const totalTokens = (usage as Record<string, unknown>).total_tokens;
  return typeof totalTokens === 'number' ? totalTokens : null;
}

function formatDuration(durationMs: number) {
  if (durationMs < 1000) {
    return `${durationMs} ms`;
  }

  const seconds = durationMs / 1000;
  const roundedSeconds = Math.round(seconds * 10) / 10;
  return `${Number.isInteger(roundedSeconds) ? roundedSeconds.toFixed(0) : roundedSeconds.toFixed(1)} s`;
}

function metricText(item: AgentFlowTraceItem) {
  const tokens = readOutputTotalTokens(item.outputPayload);
  const duration =
    item.durationMs == null ? null : formatDuration(item.durationMs);
  const toolCount =
    item.nodeType === 'llm'
      ? collectLlmToolCallbacks(item.debugPayload).length
      : 0;
  const metrics = [
    typeof tokens === 'number' ? `${tokens} tokens` : null,
    duration,
    toolCount > 0 ? i18nText("agentFlow", "auto.tools_alt", { value1: toolCount }) : null
  ].filter((metric): metric is string => Boolean(metric));

  if (metrics.length > 0) {
    return metrics.join(' · ');
  }

  return i18nText("agentFlow", "auto.in_progress");
}

export function StatusIcon({ status }: { status: string }) {
  const tone = statusTone(status);

  if (tone === 'running') {
    return (
      <LoadingOutlined
        aria-label={i18nText("agentFlow", "auto.status_alt", { value1: status })}
        className="agent-flow-editor__debug-workflow-status-icon"
        spin
      />
    );
  }

  if (tone === 'error' || tone === 'warning') {
    return (
      <WarningFilled
        aria-label={i18nText("agentFlow", "auto.status_alt", { value1: status })}
        className={`agent-flow-editor__debug-workflow-status-icon agent-flow-editor__debug-workflow-status-icon--${tone}`}
      />
    );
  }

  return (
    <CheckCircleFilled
      aria-label={i18nText("agentFlow", "auto.status_alt", { value1: status })}
      className={`agent-flow-editor__debug-workflow-status-icon agent-flow-editor__debug-workflow-status-icon--${tone}`}
    />
  );
}

export function NodeTypeIcon({ nodeType }: { nodeType: string }) {
  return (
    <span
      aria-label={i18nText("agentFlow", "auto.node_type_alt", { value1: nodeType })}
      className="agent-flow-editor__debug-workflow-node-icon"
      role="img"
    >
      {getAgentFlowNodeTypeIcon(nodeType)}
    </span>
  );
}

export function DebugWorkflowNodeRow({ item }: { item: AgentFlowTraceItem }) {
  return (
    <span
      className="agent-flow-editor__debug-workflow-row"
      data-testid="debug-workflow-node-row"
    >
      <NodeTypeIcon nodeType={item.nodeType} />
      <span className="agent-flow-editor__debug-workflow-node-main">
        <Typography.Text strong>{nodeDisplayName(item)}</Typography.Text>
        <Typography.Text
          className="agent-flow-editor__debug-workflow-metric"
          type="secondary"
        >
          {metricText(item)}
        </Typography.Text>
      </span>
      <Tag className="agent-flow-editor__debug-workflow-node-type">
        {item.nodeType}
      </Tag>
      <StatusIcon status={item.status} />
    </span>
  );
}

export function DebugWorkflowNodeItem({
  item,
  expanded,
  selected = false,
  children,
  onToggle
}: {
  item: AgentFlowTraceItem;
  expanded: boolean;
  selected?: boolean;
  children: ReactNode;
  onToggle: () => void;
}) {
  return (
    <div
      className="agent-flow-editor__debug-workflow-node-item"
      data-expanded={expanded ? 'true' : 'false'}
      data-selected={selected ? 'true' : 'false'}
      data-testid="debug-workflow-node-item"
    >
      <button
        aria-expanded={expanded}
        className="agent-flow-editor__debug-workflow-node-trigger"
        onClick={onToggle}
        type="button"
      >
        <DebugWorkflowNodeRow item={item} />
        {expanded ? (
          <DownOutlined className="agent-flow-editor__debug-workflow-collapse" />
        ) : (
          <RightOutlined className="agent-flow-editor__debug-workflow-collapse" />
        )}
      </button>
      {expanded ? children : null}
    </div>
  );
}
