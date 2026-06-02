import type { FlowAuthoringDocument } from '@1flowbase/flow-schema';

import {
  fetchRuntimeDebugArtifact,
  type AgentFlowVariableGroup,
  type FlowDebugRunDetail,
  type NodeDebugPreviewVariableCache
} from '../../api/runtime';
import {
  type NodePreviewDisplayVariableCache,
  type NodeVariableDisplayMeta
} from '../../lib/debug-console/variable-groups';
import { getNodeVariableOutputs } from '../../lib/variables/start-node-variables';

function isRecord(value: unknown): value is Record<string, unknown> {
  return Boolean(value && typeof value === 'object' && !Array.isArray(value));
}

function isRuntimeDebugArtifactPreview(value: unknown): value is {
  __runtime_debug_artifact: true;
  artifact_ref: string;
} {
  return (
    isRecord(value) &&
    value.__runtime_debug_artifact === true &&
    typeof value.artifact_ref === 'string'
  );
}

async function hydrateRuntimeDebugArtifacts(
  value: unknown,
  loadArtifact: (artifactRef: string) => Promise<unknown>
): Promise<unknown> {
  if (isRuntimeDebugArtifactPreview(value)) {
    try {
      return await loadArtifact(value.artifact_ref);
    } catch {
      return value;
    }
  }

  if (Array.isArray(value)) {
    return Promise.all(
      value.map((entry) => hydrateRuntimeDebugArtifacts(entry, loadArtifact))
    );
  }

  if (!isRecord(value)) {
    return value;
  }

  const entries = await Promise.all(
    Object.entries(value).map(async ([key, entryValue]) => [
      key,
      await hydrateRuntimeDebugArtifacts(entryValue, loadArtifact)
    ])
  );

  return Object.fromEntries(entries);
}

export async function hydrateRunDetailArtifacts(
  applicationId: string,
  detail: FlowDebugRunDetail
): Promise<FlowDebugRunDetail> {
  const artifactRequests = new Map<string, Promise<unknown>>();
  const loadArtifact = (artifactRef: string) => {
    const existingRequest = artifactRequests.get(artifactRef);

    if (existingRequest) {
      return existingRequest;
    }

    const request = fetchRuntimeDebugArtifact(applicationId, artifactRef);
    artifactRequests.set(artifactRef, request);
    return request;
  };

  const [flowInputPayload, flowOutputPayload, nodeRuns] = await Promise.all([
    hydrateRuntimeDebugArtifacts(detail.flow_run.input_payload, loadArtifact),
    hydrateRuntimeDebugArtifacts(detail.flow_run.output_payload, loadArtifact),
    Promise.all(
      detail.node_runs.map(async (nodeRun) => ({
        ...nodeRun,
        input_payload: await hydrateRuntimeDebugArtifacts(
          nodeRun.input_payload,
          loadArtifact
        ),
        output_payload: await hydrateRuntimeDebugArtifacts(
          nodeRun.output_payload,
          loadArtifact
        )
      }))
    )
  ]);

  return {
    ...detail,
    flow_run: {
      ...detail.flow_run,
      input_payload: isRecord(flowInputPayload) ? flowInputPayload : {},
      output_payload: isRecord(flowOutputPayload) ? flowOutputPayload : {}
    },
    node_runs: nodeRuns.map((nodeRun) => ({
      ...nodeRun,
      input_payload: isRecord(nodeRun.input_payload)
        ? nodeRun.input_payload
        : {},
      output_payload: isRecord(nodeRun.output_payload)
        ? nodeRun.output_payload
        : {}
    }))
  };
}

function mergeVariablePayload(
  currentCache: NodeDebugPreviewVariableCache,
  nodeId: string,
  payload: Record<string, unknown>
) {
  return {
    ...currentCache,
    [nodeId]: {
      ...(currentCache[nodeId] ?? {}),
      ...payload
    }
  };
}

export function mergeVariableCache(
  currentCache: NodeDebugPreviewVariableCache,
  nextCache: NodeDebugPreviewVariableCache
) {
  let mergedCache = currentCache;

  for (const [nodeId, payload] of Object.entries(nextCache)) {
    mergedCache = mergeVariablePayload(mergedCache, nodeId, payload);
  }

  return mergedCache;
}

export function removeVariableCacheKeys(
  currentCache: NodeDebugPreviewVariableCache,
  consumedCache: NodeDebugPreviewVariableCache
) {
  let changed = false;
  const nextCache: NodeDebugPreviewVariableCache = {};

  for (const [nodeId, payload] of Object.entries(currentCache)) {
    const consumedPayload = consumedCache[nodeId];
    const nextPayload: Record<string, unknown> = {};

    for (const [key, value] of Object.entries(payload)) {
      if (
        consumedPayload &&
        Object.prototype.hasOwnProperty.call(consumedPayload, key)
      ) {
        changed = true;
        continue;
      }
      nextPayload[key] = value;
    }

    if (Object.keys(nextPayload).length > 0) {
      nextCache[nodeId] = nextPayload;
    }
  }

  return changed ? nextCache : currentCache;
}

export function parseVariableCacheItemKey(
  key: string
): { nodeId: string; variableKey: string } | null {
  const separatorIndex = key.indexOf('.');

  if (separatorIndex <= 0 || separatorIndex === key.length - 1) {
    return null;
  }

  return {
    nodeId: key.slice(0, separatorIndex),
    variableKey: key.slice(separatorIndex + 1)
  };
}

function readVariableCacheValue(
  cache: NodeDebugPreviewVariableCache,
  key: string
) {
  const parsed = parseVariableCacheItemKey(key);
  if (!parsed) {
    return { found: false as const };
  }

  const payload = cache[parsed.nodeId];
  if (
    !payload ||
    !Object.prototype.hasOwnProperty.call(payload, parsed.variableKey)
  ) {
    return { found: false as const };
  }

  return { found: true as const, value: payload[parsed.variableKey] };
}

export function applyVariableOverridesToGroups(
  groups: AgentFlowVariableGroup[],
  variableOverrides: NodeDebugPreviewVariableCache
): AgentFlowVariableGroup[] {
  if (Object.keys(variableOverrides).length === 0) {
    return groups;
  }

  return groups.map((group) => ({
    ...group,
    items: group.items.map((item) => {
      const override = readVariableCacheValue(variableOverrides, item.key);
      return override.found ? { ...item, value: override.value } : item;
    })
  }));
}

export function removeCachedVariableItemsFromGroups(
  groups: AgentFlowVariableGroup[],
  variableCache: NodeDebugPreviewVariableCache
): AgentFlowVariableGroup[] {
  return groups.flatMap((group) => {
    const items = group.items.filter(
      (item) => !readVariableCacheValue(variableCache, item.key).found
    );

    return items.length > 0 ? [{ ...group, items }] : [];
  });
}

export function mergeVariableGroupsByTitle(
  groups: AgentFlowVariableGroup[]
): AgentFlowVariableGroup[] {
  const groupsByTitle = new Map<string, AgentFlowVariableGroup>();

  for (const group of groups) {
    const existing = groupsByTitle.get(group.title);
    if (existing) {
      existing.items.push(...group.items);
      continue;
    }

    groupsByTitle.set(group.title, { ...group, items: [...group.items] });
  }

  return Array.from(groupsByTitle.values());
}

function readOutputSelectorValue(
  payload: Record<string, unknown>,
  selector: string[]
): { found: true; value: unknown } | { found: false } {
  let current: unknown = payload;

  for (const segment of selector) {
    if (
      !isRecord(current) ||
      !Object.prototype.hasOwnProperty.call(current, segment)
    ) {
      return { found: false };
    }

    current = current[segment];
  }

  return { found: true, value: current };
}

function projectNodeVariablePayload(
  document: FlowAuthoringDocument,
  nodeId: string,
  payload: Record<string, unknown>
) {
  const node = document.graph.nodes.find((entry) => entry.id === nodeId);

  if (!node) {
    return {};
  }

  return getNodeVariableOutputs(node).reduce<Record<string, unknown>>(
    (projected, output) => {
      if (Object.prototype.hasOwnProperty.call(payload, output.key)) {
        projected[output.key] = payload[output.key];
        return projected;
      }

      const selector = output.selector?.length ? output.selector : undefined;
      if (!selector) {
        return projected;
      }

      const selected = readOutputSelectorValue(payload, selector);
      if (selected.found) {
        projected[output.key] = selected.value;
      }

      return projected;
    },
    {}
  );
}

export function projectVariableCache(
  document: FlowAuthoringDocument,
  variableCache: NodeDebugPreviewVariableCache
): NodeDebugPreviewVariableCache {
  let cache: NodeDebugPreviewVariableCache = {};

  for (const [nodeId, payload] of Object.entries(variableCache)) {
    if (isRecord(payload)) {
      const projectedPayload = projectNodeVariablePayload(
        document,
        nodeId,
        payload
      );

      if (Object.keys(projectedPayload).length > 0) {
        cache = mergeVariablePayload(cache, nodeId, projectedPayload);
      }
    }
  }

  return cache;
}

export function buildInputVariableCacheFromRunDetail(
  detail: FlowDebugRunDetail
): NodeDebugPreviewVariableCache {
  let cache: NodeDebugPreviewVariableCache = {};

  if (isRecord(detail.flow_run.input_payload)) {
    for (const [nodeId, payload] of Object.entries(
      detail.flow_run.input_payload
    )) {
      if (isRecord(payload)) {
        cache = mergeVariablePayload(cache, nodeId, payload);
      }
    }
  }

  for (const nodeRun of detail.node_runs) {
    if (isRecord(nodeRun.input_payload)) {
      cache = mergeVariablePayload(
        cache,
        nodeRun.node_id,
        nodeRun.input_payload
      );
    }
  }

  return cache;
}

export function buildDisplayVariableCache(
  outputCache: NodeDebugPreviewVariableCache
): NodePreviewDisplayVariableCache {
  const displayCache: NodePreviewDisplayVariableCache = {};

  for (const [nodeId, payload] of Object.entries(outputCache)) {
    displayCache[nodeId] ??= {};
    displayCache[nodeId].output = payload;
  }

  return displayCache;
}

export function buildNodeVariableDisplayMetadata(
  document: FlowAuthoringDocument
): Record<string, NodeVariableDisplayMeta> {
  return Object.fromEntries(
    document.graph.nodes.map((node) => [
      node.id,
      {
        label: node.alias,
        nodeType: node.type,
        outputs: getNodeVariableOutputs(node)
      }
    ])
  );
}

export function createDebugSessionState(
  applicationId: string,
  draftId: string,
  persistedDebugSessionId?: string
) {
  const scope = `${applicationId}:${draftId}`;

  if (
    typeof persistedDebugSessionId === 'string' &&
    persistedDebugSessionId.startsWith(`${scope}:`)
  ) {
    return {
      scope,
      id: persistedDebugSessionId
    };
  }

  const random =
    typeof globalThis.crypto?.randomUUID === 'function'
      ? globalThis.crypto.randomUUID()
      : `${Date.now()}-${Math.random().toString(36).slice(2)}`;

  return {
    scope,
    id: `${scope}:${random}`
  };
}
