import type {
  AgentFlowDebugMessage,
  AgentFlowDebugMessageStatus,
  AgentFlowTraceItem,
  FlowDebugRunStreamEvent
} from '../../api/runtime';
import {
  appendReasoningDeltaToAssistantContent,
  appendTextDeltaToAssistantContent,
  closeOpenThinkBlock,
  parseAssistantContent
} from './assistant-content';

function nowIso() {
  return new Date().toISOString();
}

function mapFlowStatus(status: string): AgentFlowDebugMessageStatus {
  switch (status) {
    case 'succeeded':
    case 'completed':
      return 'completed';
    case 'waiting_callback':
      return 'waiting_callback';
    case 'waiting_human':
      return 'waiting_human';
    case 'cancelled':
      return 'cancelled';
    case 'failed':
      return 'failed';
    default:
      return 'running';
  }
}

function durationMs(startedAt: string, finishedAt: string | null) {
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

function upsertTraceItem(
  items: AgentFlowTraceItem[],
  nextItem: AgentFlowTraceItem
) {
  const nextItemKey = getTraceItemKey(nextItem);
  const index = items.findIndex((item) => getTraceItemKey(item) === nextItemKey);

  if (index === -1) {
    return [...items, nextItem];
  }

  return items.map((item, itemIndex) =>
    itemIndex === index ? { ...item, ...nextItem } : item
  );
}

function extractOutputText(output: Record<string, unknown>) {
  for (const key of ['answer', 'text', 'content', 'message']) {
    const value = output[key];

    if (typeof value === 'string' && value.trim().length > 0) {
      return value;
    }
  }

  return '';
}

export function applyDebugStreamEventToTrace(
  items: AgentFlowTraceItem[],
  event: FlowDebugRunStreamEvent
): AgentFlowTraceItem[] {
  if (event.type === 'node_started') {
    const startedAt = event.started_at ?? nowIso();

    return upsertTraceItem(items, {
      nodeRunId: event.node_run_id,
      nodeId: event.node_id,
      nodeAlias: event.title,
      nodeType: event.node_type,
      status: 'running',
      startedAt,
      finishedAt: null,
      durationMs: null,
      inputPayload: event.input_payload ?? {},
      outputPayload: {},
      errorPayload: null,
      metricsPayload: {},
      debugPayload: {}
    });
  }

  if (event.type === 'node_finished') {
    const existing = items.find(
      (item) => getTraceItemKey(item) === getTraceItemKeyFromFinished(event)
    );
    const startedAt = event.started_at ?? existing?.startedAt ?? nowIso();
    const finishedAt = event.finished_at ?? nowIso();

    return upsertTraceItem(items, {
      nodeRunId: event.node_run_id,
      nodeId: event.node_id,
      nodeAlias: existing?.nodeAlias ?? event.node_id,
      nodeType: existing?.nodeType ?? 'node',
      status: event.status,
      startedAt,
      finishedAt,
      durationMs: durationMs(startedAt, finishedAt),
      inputPayload: existing?.inputPayload ?? {},
      outputPayload: event.output_payload ?? {},
      errorPayload: event.error_payload ?? null,
      metricsPayload: event.metrics_payload ?? {},
      debugPayload: event.debug_payload ?? existing?.debugPayload ?? {}
    });
  }

  return items;
}

function getTraceItemKey(item: AgentFlowTraceItem) {
  return item.nodeRunId ?? item.nodeId;
}

function getTraceItemKeyFromFinished(event: {
  node_run_id?: string | null;
  node_id: string;
}) {
  return event.node_run_id ?? event.node_id;
}

export function applyDebugStreamEventToAssistantMessage(
  message: AgentFlowDebugMessage,
  event: FlowDebugRunStreamEvent,
  traceItems: AgentFlowTraceItem[]
): AgentFlowDebugMessage {
  switch (event.type) {
    case 'flow_accepted':
      return {
        ...message,
        runId: event.run_id,
        status: 'running',
        traceSummary: traceItems
      };
    case 'flow_started':
      return {
        ...message,
        runId: event.run_id,
        status: mapFlowStatus(event.status),
        traceSummary: traceItems
      };
    case 'text_delta':
      return {
        ...message,
        content: appendTextDeltaToAssistantContent(message.content, event.text)
      };
    case 'reasoning_delta':
      return {
        ...message,
        content: appendReasoningDeltaToAssistantContent(
          message.content,
          event.text
        )
      };
    case 'node_started':
    case 'node_finished':
      return {
        ...message,
        traceSummary: traceItems
      };
    case 'flow_finished': {
      const closedContent = closeOpenThinkBlock(message.content);
      const outputText = extractOutputText(event.output);
      const nextContent =
        parseAssistantContent(closedContent).answerText || !outputText
          ? closedContent
          : appendTextDeltaToAssistantContent(closedContent, outputText);

      return {
        ...message,
        runId: event.run_id,
        status: mapFlowStatus(event.status),
        content: nextContent,
        rawOutput: event.output,
        traceSummary: traceItems
      };
    }
    case 'flow_failed':
      return {
        ...message,
        runId: event.run_id,
        status: 'failed',
        content: event.error,
        rawOutput: event.error_payload ?? null,
        traceSummary: traceItems
      };
    case 'flow_cancelled':
      return {
        ...message,
        runId: event.run_id,
        status: 'cancelled',
        traceSummary: traceItems
      };
    case 'waiting_human':
    case 'waiting_callback':
      return {
        ...message,
        runId: event.run_id,
        status: mapFlowStatus(event.status),
        traceSummary: traceItems
      };
    case 'replay_expired':
      return {
        ...message,
        status: 'failed',
        content: '调试流已过期，请重新运行。'
      };
    default:
      return message;
  }
}
