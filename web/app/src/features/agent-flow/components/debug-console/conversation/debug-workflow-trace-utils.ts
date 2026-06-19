import type { AgentFlowTraceItem } from '../../../api/runtime';
import { i18nText } from '../../../../../shared/i18n/text';

export interface AgentFlowTraceDisplayGroup {
  key: string;
  item: AgentFlowTraceItem;
  items: AgentFlowTraceItem[];
}

export function getTraceItemKey(item: AgentFlowTraceItem) {
  return item.nodeRunId ?? item.nodeId;
}

export function nodeDisplayName(item: AgentFlowTraceItem) {
  if (item.nodeType === 'start') {
    return i18nText('agentFlow', 'auto.user_input');
  }

  if (item.nodeType === 'answer') {
    return i18nText('agentFlow', 'auto.reply_directly');
  }

  if (item.nodeType === 'fusion') {
    return i18nText('agentFlow', 'auto.tool_mode_fusion');
  }

  if (item.nodeType === 'route') {
    return i18nText('agentFlow', 'auto.tool_mode_agent');
  }

  return item.nodeAlias;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return Boolean(value && typeof value === 'object' && !Array.isArray(value));
}

function payloadHasKeys(payload: Record<string, unknown>) {
  return Object.keys(payload).length > 0;
}

function groupStatus(items: AgentFlowTraceItem[]) {
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

  return items.at(-1)?.status ?? 'running';
}

function mergedDuration(items: AgentFlowTraceItem[]) {
  const durations = items
    .map((item) => item.durationMs)
    .filter((duration): duration is number => typeof duration === 'number');

  if (durations.length === 0) {
    return null;
  }

  return durations.reduce((total, duration) => total + duration, 0);
}

function firstPayload(
  items: AgentFlowTraceItem[],
  selector: (item: AgentFlowTraceItem) => Record<string, unknown>
) {
  const item = items.find((entry) => payloadHasKeys(selector(entry)));
  return item ? selector(item) : {};
}

function lastPayload(
  items: AgentFlowTraceItem[],
  selector: (item: AgentFlowTraceItem) => Record<string, unknown>
) {
  const item = [...items]
    .reverse()
    .find((entry) => payloadHasKeys(selector(entry)));
  return item ? selector(item) : {};
}

function mergeDebugPayloads(items: AgentFlowTraceItem[]) {
  const merged: Record<string, unknown> = {};
  const llmRounds: unknown[] = [];
  const visibleInternalRouteTraces: unknown[] = [];
  const visibleInternalRouteEvents: unknown[] = [];

  for (const item of items) {
    const debugPayload = item.debugPayload;

    if (!isRecord(debugPayload)) {
      continue;
    }

    for (const [key, value] of Object.entries(debugPayload)) {
      if (key === 'llm_rounds') {
        if (Array.isArray(value)) {
          llmRounds.push(...value);
        } else if (merged.llm_rounds === undefined) {
          merged.llm_rounds = value;
        }
        continue;
      }
      if (key === 'visible_internal_llm_tool_trace') {
        if (Array.isArray(value)) {
          visibleInternalRouteTraces.push(...value);
        } else if (merged.visible_internal_llm_tool_trace === undefined) {
          merged.visible_internal_llm_tool_trace = value;
        }
        continue;
      }
      if (key === 'visible_internal_llm_tool_events') {
        if (Array.isArray(value)) {
          visibleInternalRouteEvents.push(...value);
        } else if (merged.visible_internal_llm_tool_events === undefined) {
          merged.visible_internal_llm_tool_events = value;
        }
        continue;
      }

      if (merged[key] === undefined) {
        merged[key] = value;
      }
    }
  }

  if (llmRounds.length > 0) {
    merged.llm_rounds = llmRounds;
  }
  if (visibleInternalRouteTraces.length > 0) {
    merged.visible_internal_llm_tool_trace = visibleInternalRouteTraces;
  }
  if (visibleInternalRouteEvents.length > 0) {
    merged.visible_internal_llm_tool_events = visibleInternalRouteEvents;
  }

  return merged;
}

function lastAnswerSnapshot(items: AgentFlowTraceItem[]) {
  return [...items].reverse().find((item) => item.answerSnapshot)
    ?.answerSnapshot;
}

function mergeTraceGroupItems(items: AgentFlowTraceItem[]): AgentFlowTraceItem {
  const firstItem = items[0];
  const lastItem = items.at(-1) ?? firstItem;

  return {
    ...firstItem,
    nodeRunId: firstItem.nodeRunId ?? firstItem.nodeId,
    status: groupStatus(items),
    startedAt: firstItem.startedAt,
    finishedAt: items.some((item) => item.finishedAt === null)
      ? null
      : lastItem.finishedAt,
    durationMs: mergedDuration(items),
    inputPayload: firstPayload(items, (item) => item.inputPayload),
    outputPayload: lastPayload(items, (item) => item.outputPayload),
    errorPayload:
      [...items].reverse().find((item) => item.errorPayload)?.errorPayload ??
      null,
    metricsPayload: lastPayload(items, (item) => item.metricsPayload),
    debugPayload: mergeDebugPayloads(items),
    answerSnapshot: lastAnswerSnapshot(items)
  };
}

export function groupTraceItemsForDisplay(
  items: AgentFlowTraceItem[]
): AgentFlowTraceDisplayGroup[] {
  const groups: AgentFlowTraceDisplayGroup[] = [];
  const llmGroupIndexByNodeId = new Map<string, number>();

  for (const item of items) {
    if (item.nodeType !== 'llm') {
      groups.push({
        key: getTraceItemKey(item),
        item,
        items: [item]
      });
      continue;
    }

    const groupIndex = llmGroupIndexByNodeId.get(item.nodeId);

    if (groupIndex === undefined) {
      llmGroupIndexByNodeId.set(item.nodeId, groups.length);
      groups.push({
        key: `llm:${item.nodeId}`,
        item,
        items: [item]
      });
      continue;
    }

    const group = groups[groupIndex];
    const groupItems = [...group.items, item];

    groups[groupIndex] = {
      ...group,
      item: mergeTraceGroupItems(groupItems),
      items: groupItems
    };
  }

  return groups;
}
