import { useState } from 'react';
import { DownOutlined, RightOutlined } from '@ant-design/icons';
import { Tag, Typography } from 'antd';

import type { AgentFlowTraceItem } from '../../../api/runtime';
import { NodeRunPayloadSections } from '../../detail/last-run/NodeRunIOCard';
import { DebugWorkflowNodeItem, StatusIcon } from './DebugWorkflowNodeRow';
import { getTraceItemKey } from './debug-workflow-trace-utils';

function workflowStatus(items: AgentFlowTraceItem[]) {
  if (items.some((item) => item.status === 'failed')) {
    return 'failed';
  }

  if (items.some((item) => item.status === 'waiting_human')) {
    return 'waiting_human';
  }

  if (items.some((item) => item.status === 'waiting_callback')) {
    return 'waiting_callback';
  }

  if (items.some((item) => item.status === 'running')) {
    return 'running';
  }

  if (items.every((item) => item.status === 'succeeded')) {
    return 'succeeded';
  }

  return 'running';
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return Boolean(value && typeof value === 'object' && !Array.isArray(value));
}

function firstRecordField(
  record: Record<string, unknown>,
  keys: string[]
): Record<string, unknown> {
  for (const key of keys) {
    const value = record[key];

    if (isRecord(value)) {
      return value;
    }
  }

  return {};
}

function firstStringField(
  record: Record<string, unknown>,
  keys: string[]
): string | null {
  for (const key of keys) {
    const value = record[key];

    if (typeof value === 'string' && value.trim().length > 0) {
      return value;
    }
  }

  return null;
}

function hasKeys(record: Record<string, unknown>) {
  return Object.keys(record).length > 0;
}

function readLlmRounds(debugPayload: unknown): Record<string, unknown>[] {
  if (!isRecord(debugPayload) || !Array.isArray(debugPayload.llm_rounds)) {
    return [];
  }

  return debugPayload.llm_rounds.filter(isRecord);
}

function debugPayloadWithoutLlmRounds(debugPayload: unknown) {
  if (!isRecord(debugPayload) || !Array.isArray(debugPayload.llm_rounds)) {
    return debugPayload;
  }

  return Object.fromEntries(
    Object.entries(debugPayload).filter(([key]) => key !== 'llm_rounds')
  );
}

function roundHasToolResults(round: Record<string, unknown>) {
  return Array.isArray(round.tool_results) && round.tool_results.length > 0;
}

function roundHasAssistantToolCalls(round: Record<string, unknown>) {
  const assistant = firstRecordField(round, ['assistant', 'assistant_message']);

  return Array.isArray(assistant.tool_calls);
}

function toolCallbackNumber(
  rounds: Record<string, unknown>[],
  currentIndex: number
) {
  return rounds
    .slice(0, currentIndex + 1)
    .filter(roundHasToolResults).length;
}

function roundTitle(
  rounds: Record<string, unknown>[],
  round: Record<string, unknown>,
  index: number
) {
  const kind = firstStringField(round, ['kind', 'phase', 'type']);
  const finishReason = firstStringField(round, ['finish_reason']);
  const number = index + 1;

  if (kind === 'final_answer' || kind === 'final') {
    return 'Final Answer';
  }

  if (!roundHasAssistantToolCalls(round) && finishReason === 'stop') {
    return 'Final Answer';
  }

  if (
    kind === 'tool_callback' ||
    kind === 'tool_result' ||
    roundHasToolResults(round)
  ) {
    return `Tool Callback #${toolCallbackNumber(rounds, index)}`;
  }

  return `Round #${number}`;
}

function roundProcessPayload(round: Record<string, unknown>) {
  const payload = {
    ...firstRecordField(round, ['debug_payload', 'debug', 'process_payload'])
  };

  for (const key of [
    'tool_calls',
    'tool_results',
    'provider_events',
    'usage',
    'finish_reason'
  ]) {
    if (round[key] !== undefined && payload[key] === undefined) {
      payload[key] = round[key];
    }
  }

  return payload;
}

function roundOutputPayload(round: Record<string, unknown>) {
  const output = firstRecordField(round, [
    'output_payload',
    'output',
    'response',
    'assistant',
    'assistant_message'
  ]);

  if (hasKeys(output)) {
    return output;
  }

  const text = firstStringField(round, ['final_answer', 'answer', 'text']);
  return text ? { text } : {};
}

export function LlmRoundTimeline({
  debugPayload,
  onLoadArtifact
}: {
  debugPayload: unknown;
  onLoadArtifact?: (artifactRef: string) => Promise<unknown>;
}) {
  const rounds = readLlmRounds(debugPayload);

  if (rounds.length === 0) {
    return null;
  }

  return (
    <section
      aria-label="LLM 回合"
      className="agent-flow-editor__debug-llm-rounds"
    >
      {rounds.map((round, index) => {
        const inputPayload = firstRecordField(round, [
          'input_payload',
          'input',
          'request_payload',
          'request'
        ]);
        const processPayload = roundProcessPayload(round);
        const outputPayload = roundOutputPayload(round);
        const status = firstStringField(round, ['status', 'finish_reason']);

        const title = roundTitle(rounds, round, index);

        return (
          <article
            key={`${title}-${index}`}
            className="agent-flow-editor__debug-llm-round"
          >
            <div className="agent-flow-editor__debug-llm-round-header">
              <Typography.Text strong>{title}</Typography.Text>
              {status ? <Tag>{status}</Tag> : null}
            </div>
            <NodeRunPayloadSections
              includeDebugPayload={hasKeys(processPayload)}
              inputPayload={inputPayload}
              debugPayload={processPayload}
              outputPayload={outputPayload}
              onLoadArtifact={onLoadArtifact}
            />
          </article>
        );
      })}
    </section>
  );
}

export function DebugWorkflowProcess({
  items,
  onLoadArtifact
}: {
  items: AgentFlowTraceItem[];
  onLoadArtifact?: (artifactRef: string) => Promise<unknown>;
}) {
  const [expanded, setExpanded] = useState(true);
  const [expandedNodeKeys, setExpandedNodeKeys] = useState<Set<string>>(
    () => new Set()
  );

  if (items.length === 0) {
    return null;
  }

  const status = workflowStatus(items);

  return (
    <div
      aria-label="工作流"
      className="agent-flow-editor__debug-workflow-process"
      role="group"
    >
      <button
        aria-expanded={expanded}
        className="agent-flow-editor__debug-workflow-header"
        onClick={() => setExpanded((current) => !current)}
        type="button"
      >
        <span className="agent-flow-editor__debug-workflow-title">
          <StatusIcon status={status} />
          <Typography.Text>工作流</Typography.Text>
        </span>
        {expanded ? (
          <DownOutlined className="agent-flow-editor__debug-workflow-collapse" />
        ) : (
          <RightOutlined className="agent-flow-editor__debug-workflow-collapse" />
        )}
      </button>
      {expanded ? (
        <div className="agent-flow-editor__debug-workflow-collapse-list">
          {items.map((item) => {
            const itemKey = getTraceItemKey(item);
            const nodeExpanded = expandedNodeKeys.has(itemKey);

            return (
              <DebugWorkflowNodeItem
                key={itemKey}
                expanded={nodeExpanded}
                item={item}
                onToggle={() => {
                  setExpandedNodeKeys((current) => {
                    const next = new Set(current);

                    if (next.has(itemKey)) {
                      next.delete(itemKey);
                    } else {
                      next.add(itemKey);
                    }

                    return next;
                  });
                }}
              >
                <div className="agent-flow-editor__debug-workflow-node-detail">
                  <LlmRoundTimeline
                    debugPayload={item.debugPayload}
                    onLoadArtifact={onLoadArtifact}
                  />
                  <NodeRunPayloadSections
                    inputPayload={item.inputPayload}
                    debugPayload={debugPayloadWithoutLlmRounds(
                      item.debugPayload
                    )}
                    outputPayload={item.outputPayload}
                    onLoadArtifact={onLoadArtifact}
                  />
                </div>
              </DebugWorkflowNodeItem>
            );
          })}
        </div>
      ) : null}
    </div>
  );
}
