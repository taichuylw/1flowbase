import type {
  FlowAuthoringDocument,
  FlowBinding,
  FlowNodeDocument,
  FlowNodeOutputDocument
} from '@1flowbase/flow-schema';
import {
  cancelConsoleFlowRun,
  deleteConsoleDebugVariableCacheEntries,
  getConsoleApplicationRunDetail,
  getConsoleApplicationRunNodeLastRun,
  getConsoleDebugVariableSnapshot,
  getConsoleRuntimeDebugArtifact,
  startConsoleFlowDebugRun,
  startConsoleFlowDebugRunStream,
  getConsoleNodeLastRun,
  startConsoleNodeDebugPreview,
  upsertConsoleDebugVariableCacheEntry,
  type ConsoleAnswerSnapshot,
  type ConsoleApplicationRunDetail,
  type ConsoleDebugVariableSnapshot,
  type ConsoleFlowDebugStreamEvent,
  type ConsoleFlowDebugStreamCursor,
  type ConsoleFlowDebugStreamHandlers,
  type ConsoleNodeLastRun,
  type ConsoleRuntimeDebugArtifactPreview
} from '@1flowbase/api-client';

import { getApplicationsApiBaseUrl } from '../../applications/api/applications';
import {
  extractDataModelQuerySelectors,
  getActiveNodeBindings
} from '../lib/data-model-query-binding';
import {
  getNodeVariableOutputs,
  getStartInputFields
} from '../lib/variables/start-node-variables';
import {
  agentFlowSystemVariables,
  systemVariableNodeId
} from '../lib/variables/system-variables';
import type { AgentFlowEnvironmentVariable } from '../lib/variables/application-environment-variables';
import { extractNamedBindingSelectors } from '../lib/named-binding-expressions';
import {
  collectConditionSelectors,
  collectIfElseBranchSelectors
} from '../lib/if-else-branches';
import {
  TEMPLATE_SELECTOR_REGEX,
  parseTemplateSelectorTokens
} from '../lib/template-binding';
import { i18nText } from '../../../shared/i18n/text';

export type NodeLastRun = ConsoleNodeLastRun;
export type FlowDebugRunDetail = ConsoleApplicationRunDetail;
export type RuntimeDebugArtifactPreview = ConsoleRuntimeDebugArtifactPreview;
export type DebugVariableSnapshot = ConsoleDebugVariableSnapshot & {
  variable_cache: NodeDebugPreviewVariableCache;
};
export type FlowDebugRunStreamEvent = ConsoleFlowDebugStreamEvent;
export type FlowDebugRunStreamHandlers = ConsoleFlowDebugStreamHandlers;
export type AgentFlowDebugMessageStatus =
  | 'running'
  | 'completed'
  | 'waiting_callback'
  | 'waiting_human'
  | 'cancelled'
  | 'failed';

export interface AgentFlowAnswerSnapshot {
  kind: ConsoleAnswerSnapshot['kind'];
  text: string;
  outputPayload: Record<string, unknown>;
  complete: boolean;
  materializedFrom: string;
  answerNodeId: string;
  answerNodeRunId: string;
  waitingNodeId?: string | null;
  waitingNodeRunId?: string | null;
}

export interface AgentFlowTraceItem {
  nodeId: string;
  nodeRunId?: string;
  nodeAlias: string;
  nodeType: string;
  status: string;
  startedAt: string;
  finishedAt: string | null;
  durationMs: number | null;
  inputPayload: Record<string, unknown>;
  outputPayload: Record<string, unknown>;
  errorPayload: Record<string, unknown> | null;
  metricsPayload: Record<string, unknown>;
  debugPayload?: Record<string, unknown>;
  answerSnapshot?: AgentFlowAnswerSnapshot;
}

export interface AgentFlowRunStatistics {
  total_tokens: number | null;
  unique_node_count: number;
  tool_callback_count: number;
}

export interface AgentFlowVariableItem {
  key: string;
  label: string;
  value: unknown;
  isReadOnly?: boolean;
  isTruncated?: boolean;
  artifactRef?: string;
  helperText?: string;
}

export interface AgentFlowVariableGroup {
  title: string;
  items: AgentFlowVariableItem[];
}

export interface AgentFlowDebugMessage {
  id: string;
  role: 'system' | 'user' | 'assistant';
  content: string;
  status: AgentFlowDebugMessageStatus;
  runId: string | null;
  detailRunId?: string | null;
  canOpenDetail?: boolean;
  compatibilityMode?: string | null;
  compatibilityModeLabel?: string | null;
  rawOutput: Record<string, unknown> | null;
  statistics?: AgentFlowRunStatistics;
  traceSummary: AgentFlowTraceItem[];
}

export interface AgentFlowRunContextField {
  nodeId: string;
  nodeLabel: string;
  key: string;
  title: string;
  valueType: FlowNodeDocument['outputs'][number]['valueType'];
  value: unknown;
}

export interface AgentFlowRunContext {
  environmentLabel: 'draft';
  remembered: boolean;
  fields: AgentFlowRunContextField[];
}

export interface NodeDebugPreviewVariableField {
  nodeId: string;
  nodeLabel: string;
  key: string;
  title: string;
  valueType: FlowNodeDocument['outputs'][number]['valueType'];
  value: unknown;
  inputPath?: string[];
}

export type NodeDebugPreviewVariableCache = Record<
  string,
  Record<string, unknown>
>;

export interface NodeDebugPreviewPlan {
  input_payload: Record<string, Record<string, unknown>>;
  missing_fields: NodeDebugPreviewVariableField[];
}

export interface NodeDebugVariableConfirmationPlan {
  input_payload: Record<string, Record<string, unknown>>;
  fields: NodeDebugPreviewVariableField[];
}

interface EnvironmentVariableUpdateOperation {
  path: string[];
  operator: 'set' | 'append' | 'clear' | 'increment';
  source?: string[] | null;
  value?: StateWriteValueExpression | null;
}

type StateWriteValueExpression =
  | { kind: 'constant'; value: unknown }
  | { kind: 'selector'; selector: string[] }
  | { kind: 'templated_text'; value: string };

export const nodeLastRunQueryKey = (applicationId: string, nodeId: string) =>
  [
    'applications',
    applicationId,
    'runtime',
    'nodes',
    nodeId,
    'last-run'
  ] as const;

export const applicationRunNodeLastRunQueryKey = (
  applicationId: string,
  runId: string,
  nodeId: string
) =>
  [
    'applications',
    applicationId,
    'runtime',
    'runs',
    runId,
    'nodes',
    nodeId,
    'last-run'
  ] as const;

export function fetchNodeLastRun(applicationId: string, nodeId: string) {
  return getConsoleNodeLastRun(
    applicationId,
    nodeId,
    getApplicationsApiBaseUrl()
  );
}

export function fetchDebugVariableSnapshot(applicationId: string) {
  return getConsoleDebugVariableSnapshot(
    applicationId,
    getApplicationsApiBaseUrl()
  );
}

export function upsertDebugVariableCacheEntry(
  applicationId: string,
  input: {
    node_id: string;
    variable_key: string;
    value: unknown;
  },
  csrfToken: string
) {
  return upsertConsoleDebugVariableCacheEntry(
    applicationId,
    input,
    csrfToken,
    getApplicationsApiBaseUrl()
  );
}

export function deleteDebugVariableCacheEntries(
  applicationId: string,
  input: {
    keys?: Array<{
      node_id: string;
      variable_key: string;
    }>;
  },
  csrfToken: string
) {
  return deleteConsoleDebugVariableCacheEntries(
    applicationId,
    input,
    csrfToken,
    getApplicationsApiBaseUrl()
  );
}

export function nodeLastRunToFlowDebugRunDetail(
  lastRun: ConsoleNodeLastRun
): FlowDebugRunDetail {
  return {
    flow_run: lastRun.flow_run,
    node_runs: [lastRun.node_run],
    checkpoints: lastRun.checkpoints,
    callback_tasks: [],
    events: lastRun.events
  };
}

export function startNodeDebugPreview(
  applicationId: string,
  nodeId: string,
  input: {
    input_payload: Record<string, Record<string, unknown>>;
    document?: FlowAuthoringDocument;
    debug_session_id?: string;
  },
  csrfToken: string
) {
  return startConsoleNodeDebugPreview(
    applicationId,
    nodeId,
    input,
    csrfToken,
    getApplicationsApiBaseUrl()
  );
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return Boolean(value && typeof value === 'object' && !Array.isArray(value));
}

export function extractNodePreviewVariableOutput(
  lastRun: ConsoleNodeLastRun
): Record<string, unknown> {
  const outputPayload = lastRun.node_run.output_payload;

  if (!isRecord(outputPayload)) {
    return {};
  }

  return outputPayload;
}

export function buildFlowDebugRunInput(
  document: FlowAuthoringDocument,
  inputValues?: Record<string, unknown>,
  environmentVariables: AgentFlowEnvironmentVariable[] = []
) {
  const startNode = document.graph.nodes.find((node) => node.type === 'start');
  const startPayload: Record<string, unknown> = {};
  const inputPayload: Record<string, Record<string, unknown>> = {
    [startNode?.id ?? 'node-start']: startPayload
  };

  for (const output of startNode ? getNodeVariableOutputs(startNode) : []) {
    startPayload[output.key] =
      inputValues &&
      Object.prototype.hasOwnProperty.call(inputValues, output.key)
        ? inputValues[output.key]
        : buildPreviewValue(startNode, output.key);
  }

  if (environmentVariables.length > 0) {
    inputPayload.env = environmentVariables.reduce<Record<string, unknown>>(
      (payload, variable) => {
        payload[variable.name] = variable.value;
        return payload;
      },
      {}
    );
  }

  return {
    input_payload: inputPayload
  };
}

export function startFlowDebugRun(
  applicationId: string,
  input: {
    input_payload: Record<string, Record<string, unknown>>;
    document?: FlowAuthoringDocument;
    debug_session_id?: string;
  },
  csrfToken: string
) {
  return startConsoleFlowDebugRun(
    applicationId,
    input,
    csrfToken,
    getApplicationsApiBaseUrl()
  );
}

export function startFlowDebugRunStream(
  applicationId: string,
  input: {
    input_payload: Record<string, Record<string, unknown>>;
    document?: FlowAuthoringDocument;
    debug_session_id?: string;
  },
  csrfToken: string,
  handlers: FlowDebugRunStreamHandlers,
  cursor?: ConsoleFlowDebugStreamCursor
) {
  return startConsoleFlowDebugRunStream(
    applicationId,
    input,
    csrfToken,
    handlers,
    {
      cursor,
      baseUrl: getApplicationsApiBaseUrl()
    }
  );
}

export function fetchApplicationRunDetail(
  applicationId: string,
  runId: string
) {
  return getConsoleApplicationRunDetail(
    applicationId,
    runId,
    getApplicationsApiBaseUrl()
  );
}

export function fetchApplicationRunNodeLastRun(
  applicationId: string,
  runId: string,
  nodeId: string
) {
  return getConsoleApplicationRunNodeLastRun(
    applicationId,
    runId,
    nodeId,
    getApplicationsApiBaseUrl()
  );
}

export function fetchRuntimeDebugArtifact(
  applicationId: string,
  artifactId: string
) {
  return getConsoleRuntimeDebugArtifact(
    applicationId,
    artifactId,
    getApplicationsApiBaseUrl()
  );
}

export function cancelFlowDebugRun(
  applicationId: string,
  runId: string,
  csrfToken: string
) {
  return cancelConsoleFlowRun(
    applicationId,
    runId,
    csrfToken,
    getApplicationsApiBaseUrl()
  );
}

function normalizeSelectorPath(value: string[] | null | undefined) {
  if (!value || value.length < 2) {
    return null;
  }

  return value;
}

function extractTemplateSelectors(template: string) {
  return parseTemplateSelectorTokens(template);
}

function normalizeConfigSelectorPath(value: unknown) {
  if (!Array.isArray(value)) {
    return null;
  }

  const selector = value.filter(
    (segment): segment is string => typeof segment === 'string'
  );

  return selector.length >= 2 ? selector : null;
}

function configKeyCarriesSelector(key: string | undefined) {
  return (
    key === 'selector' ||
    Boolean(key?.endsWith('_selector')) ||
    Boolean(key?.endsWith('Selector'))
  );
}

function extractConfigReferenceSelectors(
  value: unknown,
  key?: string
): string[][] {
  const selectors: string[][] = [];

  if (typeof value === 'string') {
    selectors.push(...extractTemplateSelectors(value));
    return selectors;
  }

  if (configKeyCarriesSelector(key)) {
    const selector = normalizeConfigSelectorPath(value);

    if (selector) {
      selectors.push(selector);
    }
  }

  if (Array.isArray(value)) {
    return [
      ...selectors,
      ...value.flatMap((entry) => extractConfigReferenceSelectors(entry))
    ];
  }

  if (!isRecord(value)) {
    return selectors;
  }

  if (value.kind === 'selector') {
    const selector = normalizeConfigSelectorPath(value.selector);

    if (selector) {
      selectors.push(selector);
    }
  }

  return [
    ...selectors,
    ...Object.entries(value).flatMap(([entryKey, entryValue]) =>
      extractConfigReferenceSelectors(entryValue, entryKey)
    )
  ];
}

function normalizeStateWriteValueExpression(
  value: unknown
): StateWriteValueExpression | null {
  if (!isRecord(value)) {
    return null;
  }

  if (value.kind === 'constant') {
    return {
      kind: 'constant',
      value: value.value
    };
  }

  if (value.kind === 'selector') {
    const selector = normalizeConfigSelectorPath(value.selector);

    return selector ? { kind: 'selector', selector } : null;
  }

  if (value.kind === 'templated_text' && typeof value.value === 'string') {
    return {
      kind: 'templated_text',
      value: value.value
    };
  }

  return null;
}

function extractStateWriteValueSelectors(value: unknown): string[][] {
  const expression = normalizeStateWriteValueExpression(value);

  if (!expression) {
    return [];
  }

  if (expression.kind === 'selector') {
    return [expression.selector];
  }

  if (expression.kind === 'templated_text') {
    return extractTemplateSelectors(expression.value);
  }

  return [];
}

function extractSelectors(binding: FlowBinding): string[][] {
  switch (binding.kind) {
    case 'selector': {
      const selector = normalizeSelectorPath(binding.value);

      return selector ? [selector] : [];
    }
    case 'selector_list':
      return binding.value
        .map((value) => normalizeSelectorPath(value))
        .filter((value): value is string[] => value !== null);
    case 'prompt_messages':
      return binding.value.flatMap((message) =>
        extractTemplateSelectors(message.content.value)
      );
    case 'named_bindings':
      return extractNamedBindingSelectors(binding.value);
    case 'condition_group':
      return collectConditionSelectors(binding.value)
        .map((condition) => normalizeSelectorPath(condition))
        .filter((value): value is string[] => value !== null);
    case 'if_else_branches':
      return collectIfElseBranchSelectors(binding.value.branches)
        .map((condition) => normalizeSelectorPath(condition))
        .filter((value): value is string[] => value !== null);
    case 'state_write':
      return binding.value.flatMap((entry) => {
        const source = normalizeSelectorPath(entry.source);

        return [
          ...(source ? [source] : []),
          ...extractStateWriteValueSelectors(entry.value)
        ];
      });
    case 'data_model_query':
      return extractDataModelQuerySelectors(binding.value)
        .map((value) => normalizeSelectorPath(value))
        .filter((value): value is string[] => value !== null);
    case 'templated_text':
      return extractTemplateSelectors(binding.value);
  }
}

function collectNodeReferenceSelectors(node: FlowNodeDocument) {
  return [
    ...getActiveNodeBindings(node).flatMap(([, binding]) =>
      extractSelectors(binding)
    ),
    ...extractConfigReferenceSelectors(node.config)
  ];
}

function collectUpstreamNodeIds(
  document: FlowAuthoringDocument,
  nodeId: string
) {
  const visited = new Set<string>();
  const queue = [nodeId];

  while (queue.length > 0) {
    const currentNodeId = queue.shift();

    if (!currentNodeId) {
      continue;
    }

    for (const edge of document.graph.edges) {
      if (edge.target !== currentNodeId || visited.has(edge.source)) {
        continue;
      }

      visited.add(edge.source);
      queue.push(edge.source);
    }
  }

  return visited;
}

function getEnvironmentVariableUpdateOperations(
  node: FlowNodeDocument
): EnvironmentVariableUpdateOperation[] {
  const binding = node.bindings.operations;

  if (node.type !== 'variable_assigner' || binding?.kind !== 'state_write') {
    return [];
  }

  return binding.value.filter(
    (operation): operation is EnvironmentVariableUpdateOperation =>
      operation.operator === 'set' &&
      operation.path.length === 2 &&
      operation.path[0] === 'env' &&
      operation.path[1].trim().length > 0 &&
      normalizeStateWriteValueExpression(operation.value) !== null
  );
}

function hasPreviewVariableValue(value: unknown) {
  return value !== undefined && value !== null && value !== '';
}

function findNodeOutput(node: FlowNodeDocument | undefined, outputKey: string) {
  return node
    ? getNodeVariableOutputs(node).find((output) => output.key === outputKey)
    : undefined;
}

function findExternalVariableOutput(nodeId: string, outputKey: string) {
  if (nodeId === systemVariableNodeId) {
    return agentFlowSystemVariables.find((output) => output.key === outputKey);
  }

  return undefined;
}

function selectorsMatch(left: string[] | undefined, right: string[]) {
  return (
    Array.isArray(left) &&
    left.length === right.length &&
    left.every((segment, index) => segment === right[index])
  );
}

function findNodeOutputBySelector(
  node: FlowNodeDocument | undefined,
  selector: string[]
) {
  const outputKey = selectorOutputKey(selector);

  if (!node) {
    return findExternalVariableOutput(selector[0] ?? '', outputKey);
  }

  const selectorTail = selector.slice(1);
  const outputs = getNodeVariableOutputs(node);

  return (
    outputs.find((output) => selectorsMatch(output.selector, selectorTail)) ??
    outputs.find((output) => output.key === outputKey) ??
    (selectorTail.length === 1
      ? outputs.find((output) => output.key === selectorTail[0])
      : undefined)
  );
}

function selectorOutputKey(selector: string[]) {
  return selector[selector.length - 1] ?? '';
}

function writeNestedPreviewValue(
  target: Record<string, unknown>,
  path: string[],
  value: unknown
) {
  let current = target;

  for (const [index, segment] of path.entries()) {
    if (index === path.length - 1) {
      current[segment] = value;
      return;
    }

    const next = current[segment];

    if (!isRecord(next)) {
      current[segment] = {};
    }

    current = current[segment] as Record<string, unknown>;
  }
}

function writePreviewInputValue({
  inputPayload,
  sourceNodeId,
  sourceNode,
  sourceOutput,
  outputKey,
  value
}: {
  inputPayload: Record<string, Record<string, unknown>>;
  sourceNodeId: string;
  sourceNode: FlowNodeDocument | undefined;
  sourceOutput: FlowNodeOutputDocument | undefined;
  outputKey: string;
  value: unknown;
}) {
  const inputNodeId = sourceNode?.id ?? sourceNodeId;

  if (!inputNodeId) {
    return;
  }

  inputPayload[inputNodeId] ??= {};

  if (sourceNode?.type === 'code' && sourceOutput?.selector?.length) {
    writeNestedPreviewValue(
      inputPayload[inputNodeId],
      sourceOutput.selector,
      value
    );
    inputPayload[inputNodeId].error ??= null;
    return;
  }

  inputPayload[inputNodeId][outputKey] = value;
}

function readCachedOutputValue(
  payload: Record<string, unknown> | undefined,
  output: FlowNodeOutputDocument | undefined,
  outputKey: string
): { found: true; value: unknown } | { found: false } {
  if (!payload) {
    return { found: false };
  }

  if (Object.prototype.hasOwnProperty.call(payload, outputKey)) {
    return { found: true, value: payload[outputKey] };
  }

  if (!output?.selector?.length) {
    return { found: false };
  }

  let current: unknown = payload;

  for (const segment of output.selector) {
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

function readPreviewInputValue(
  inputPayload: Record<string, Record<string, unknown>>,
  selector: string[]
): { found: true; value: unknown } | { found: false } {
  const [nodeId, ...path] = selector;

  if (!nodeId || path.length === 0) {
    return { found: false };
  }

  let current: unknown = inputPayload[nodeId];

  for (const segment of path) {
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

function buildNodePreviewVariableField({
  document,
  nodeId,
  outputKey,
  sourceNode,
  sourceOutput,
  value
}: {
  document: FlowAuthoringDocument;
  nodeId: string;
  outputKey: string;
  sourceNode?: FlowNodeDocument;
  sourceOutput?: FlowNodeOutputDocument;
  value: unknown;
}): NodeDebugPreviewVariableField {
  const node =
    sourceNode ?? document.graph.nodes.find((entry) => entry.id === nodeId);
  const output =
    sourceOutput ??
    findNodeOutput(node, outputKey) ??
    findExternalVariableOutput(nodeId, outputKey);
  const inputPath =
    node?.type === 'code' && sourceOutput?.selector?.length
      ? [...sourceOutput.selector]
      : undefined;

  return {
    nodeId,
    nodeLabel: node?.alias ?? nodeId,
    key: outputKey,
    title: output?.title ?? `${node?.alias ?? nodeId}.${outputKey}`,
    valueType: output?.valueType ?? inferPreviewValueType(value),
    value,
    inputPath
  };
}

function inferPreviewValueType(
  value: unknown
): FlowNodeDocument['outputs'][number]['valueType'] {
  if (Array.isArray(value)) {
    return 'array';
  }

  switch (typeof value) {
    case 'boolean':
      return 'boolean';
    case 'number':
      return 'number';
    case 'string':
      return 'string';
    case 'object':
      return value === null ? 'unknown' : 'json';
    default:
      return 'unknown';
  }
}

function buildMissingPreviewField(
  document: FlowAuthoringDocument,
  nodeId: string,
  outputKey: string
): NodeDebugPreviewVariableField {
  const node = document.graph.nodes.find((entry) => entry.id === nodeId);

  return buildNodePreviewVariableField({
    document,
    nodeId,
    outputKey,
    sourceNode: node,
    sourceOutput: findNodeOutput(node, outputKey),
    value: buildPreviewValue(node, outputKey)
  });
}

function hasMissingPreviewField(
  fields: NodeDebugPreviewVariableField[],
  nodeId: string,
  outputKey: string
) {
  return fields.some((field) => field.nodeId === nodeId && field.key === outputKey);
}

function resolvePreviewSelectorValue({
  document,
  variableCache,
  inputPayload,
  missingFields,
  selector
}: {
  document: FlowAuthoringDocument;
  variableCache: NodeDebugPreviewVariableCache;
  inputPayload: Record<string, Record<string, unknown>>;
  missingFields: NodeDebugPreviewVariableField[];
  selector: string[];
}): { found: true; value: unknown } | { found: false } {
  const sourceNodeId = selector[0] ?? '';
  const outputKey = selectorOutputKey(selector);
  const sourceNode = document.graph.nodes.find(
    (entry) => entry.id === sourceNodeId
  );
  const sourceOutput = findNodeOutputBySelector(sourceNode, selector);
  const previewInputValue = readPreviewInputValue(inputPayload, selector);
  const cachedOutput =
    previewInputValue.found
      ? previewInputValue
      : readCachedOutputValue(
          variableCache[sourceNodeId],
          sourceOutput,
          outputKey
        );
  const value =
    cachedOutput.found && hasPreviewVariableValue(cachedOutput.value)
      ? cachedOutput.value
      : canUseStartPreviewDefault(sourceNode, outputKey)
        ? buildPreviewValue(sourceNode, outputKey)
        : undefined;

  if (value !== undefined) {
    return { found: true, value };
  }

  if (!hasMissingPreviewField(missingFields, sourceNodeId, outputKey)) {
    missingFields.push(
      buildMissingPreviewField(document, sourceNodeId, outputKey)
    );
  }

  return { found: false };
}

function stringifyTemplateReplacement(value: unknown) {
  if (typeof value === 'string') {
    return value;
  }

  if (value === null) {
    return 'null';
  }

  if (typeof value === 'object') {
    return JSON.stringify(value);
  }

  return String(value);
}

function renderPreviewTemplateValue({
  template,
  document,
  variableCache,
  inputPayload,
  missingFields
}: {
  template: string;
  document: FlowAuthoringDocument;
  variableCache: NodeDebugPreviewVariableCache;
  inputPayload: Record<string, Record<string, unknown>>;
  missingFields: NodeDebugPreviewVariableField[];
}): { found: true; value: string } | { found: false } {
  let hasMissingSelector = false;
  TEMPLATE_SELECTOR_REGEX.lastIndex = 0;
  const rendered = template.replace(
    TEMPLATE_SELECTOR_REGEX,
    (_match, selectorPath: string) => {
      const selector = selectorPath.split('.');
      const resolved = resolvePreviewSelectorValue({
        document,
        variableCache,
        inputPayload,
        missingFields,
        selector
      });

      if (!resolved.found) {
        hasMissingSelector = true;
        return _match;
      }

      return stringifyTemplateReplacement(resolved.value);
    }
  );
  TEMPLATE_SELECTOR_REGEX.lastIndex = 0;

  return hasMissingSelector ? { found: false } : { found: true, value: rendered };
}

function resolveEnvironmentVariableUpdateValue({
  expression,
  document,
  variableCache,
  inputPayload,
  missingFields
}: {
  expression: StateWriteValueExpression;
  document: FlowAuthoringDocument;
  variableCache: NodeDebugPreviewVariableCache;
  inputPayload: Record<string, Record<string, unknown>>;
  missingFields: NodeDebugPreviewVariableField[];
}): { found: true; value: unknown } | { found: false } {
  if (expression.kind === 'constant') {
    return { found: true, value: expression.value };
  }

  if (expression.kind === 'selector') {
    return resolvePreviewSelectorValue({
      document,
      variableCache,
      inputPayload,
      missingFields,
      selector: expression.selector
    });
  }

  return renderPreviewTemplateValue({
    template: expression.value,
    document,
    variableCache,
    inputPayload,
    missingFields
  });
}

function applyEnvironmentVariableUpdatesToPreviewPlan({
  document,
  nodeId,
  variableCache,
  inputPayload,
  missingFields
}: {
  document: FlowAuthoringDocument;
  nodeId: string;
  variableCache: NodeDebugPreviewVariableCache;
  inputPayload: Record<string, Record<string, unknown>>;
  missingFields: NodeDebugPreviewVariableField[];
}) {
  const upstreamNodeIds = collectUpstreamNodeIds(document, nodeId);

  for (const node of document.graph.nodes) {
    if (!upstreamNodeIds.has(node.id)) {
      continue;
    }

    for (const operation of getEnvironmentVariableUpdateOperations(node)) {
      const expression = normalizeStateWriteValueExpression(operation.value);

      if (!expression) {
        continue;
      }

      const resolved = resolveEnvironmentVariableUpdateValue({
        expression,
        document,
        variableCache,
        inputPayload,
        missingFields
      });

      if (!resolved.found) {
        continue;
      }

      inputPayload.env ??= {};
      inputPayload.env[operation.path[1]] = resolved.value;
    }
  }
}

function isRequiredStartPreviewKey(node: FlowNodeDocument, outputKey: string) {
  if (outputKey === 'query') {
    return true;
  }

  return getStartInputFields(node).some(
    (field) => field.key === outputKey && field.required
  );
}

function canUseStartPreviewDefault(
  node: FlowNodeDocument | undefined,
  outputKey: string
) {
  return (
    node?.type === 'start' &&
    !isRequiredStartPreviewKey(node, outputKey)
  );
}

function buildStringPreviewValue(
  node: FlowNodeDocument | undefined,
  outputKey: string
) {
  if (
    node?.type === 'start' &&
    (outputKey === 'query' ||
      outputKey === 'system' ||
      outputKey === 'model' ||
      outputKey === 'reasoning_effort')
  ) {
    return '';
  }

  if (outputKey === 'text' || outputKey === 'answer') {
    return i18nText('agentFlow', 'auto.debug_preview_output');
  }

  return i18nText('agentFlow', 'auto.debug_preview_value', {
    value1: node?.alias ?? i18nText('agentFlow', 'auto.fallback_node_label'),
    value2: outputKey
  });
}

function buildPreviewValue(
  node: FlowNodeDocument | undefined,
  outputKey: string
) {
  const startInputField =
    node?.type === 'start'
      ? getStartInputFields(node).find((field) => field.key === outputKey)
      : undefined;

  if (startInputField?.defaultValue !== undefined) {
    return startInputField.defaultValue;
  }

  const output = node
    ? getNodeVariableOutputs(node).find((entry) => entry.key === outputKey)
    : undefined;

  if (output?.valueType.startsWith('array')) {
    return [];
  }

  switch (output?.valueType) {
    case 'number':
      return 1;
    case 'boolean':
      return true;
    case 'json':
    case 'unknown':
      return {};
    case 'string':
    default:
      return buildStringPreviewValue(node, outputKey);
  }
}

export function buildNodeDebugPreviewInput(
  document: FlowAuthoringDocument,
  nodeId: string
) {
  const node = document.graph.nodes.find((entry) => entry.id === nodeId);
  const inputPayload: Record<string, Record<string, unknown>> = {};

  if (!node) {
    return { input_payload: inputPayload };
  }

  const selectors = collectNodeReferenceSelectors(node);

  for (const selector of selectors) {
    const sourceNodeId = selector[0] ?? '';
    const outputKey = selectorOutputKey(selector);
    const sourceNode = document.graph.nodes.find(
      (entry) => entry.id === sourceNodeId
    );
    const sourceOutput = findNodeOutputBySelector(sourceNode, selector);

    writePreviewInputValue({
      inputPayload,
      sourceNodeId,
      sourceNode,
      sourceOutput,
      outputKey,
      value: buildPreviewValue(sourceNode, outputKey)
    });
  }

  return { input_payload: inputPayload };
}

export function buildNodeDebugPreviewPlan(
  document: FlowAuthoringDocument,
  nodeId: string,
  variableCache: NodeDebugPreviewVariableCache = {}
): NodeDebugPreviewPlan {
  const node = document.graph.nodes.find((entry) => entry.id === nodeId);
  const inputPayload: Record<string, Record<string, unknown>> = {};
  const missingFields: NodeDebugPreviewVariableField[] = [];

  if (!node) {
    return { input_payload: inputPayload, missing_fields: missingFields };
  }

  if (node.type === 'start') {
    const visitedStartKeys = new Set<string>();

    for (const output of getNodeVariableOutputs(node)) {
      if (visitedStartKeys.has(output.key)) {
        continue;
      }

      visitedStartKeys.add(output.key);

      const cachedOutput = readCachedOutputValue(
        variableCache[node.id],
        output,
        output.key
      );

      if (cachedOutput.found && hasPreviewVariableValue(cachedOutput.value)) {
        inputPayload[node.id] ??= {};
        inputPayload[node.id][output.key] = cachedOutput.value;
        continue;
      }

      inputPayload[node.id] ??= {};
      inputPayload[node.id][output.key] = buildPreviewValue(node, output.key);

      if (isRequiredStartPreviewKey(node, output.key)) {
        missingFields.push(
          buildMissingPreviewField(document, node.id, output.key)
        );
      }
    }

    return { input_payload: inputPayload, missing_fields: missingFields };
  }

  const selectors = collectNodeReferenceSelectors(node);
  const visited = new Set<string>();

  for (const selector of selectors) {
    const sourceNodeId = selector[0] ?? '';
    const outputKey = selectorOutputKey(selector);
    const cacheKey = selector.join('.');
    const sourceNode = document.graph.nodes.find(
      (entry) => entry.id === sourceNodeId
    );
    const sourceOutput = findNodeOutputBySelector(sourceNode, selector);
    const cachedOutput = readCachedOutputValue(
      variableCache[sourceNodeId],
      sourceOutput,
      outputKey
    );

    if (visited.has(cacheKey)) {
      continue;
    }

    visited.add(cacheKey);

    if (cachedOutput.found && hasPreviewVariableValue(cachedOutput.value)) {
      writePreviewInputValue({
        inputPayload,
        sourceNodeId,
        sourceNode,
        sourceOutput,
        outputKey,
        value: cachedOutput.value
      });
      continue;
    }

    if (canUseStartPreviewDefault(sourceNode, outputKey)) {
      writePreviewInputValue({
        inputPayload,
        sourceNodeId,
        sourceNode,
        sourceOutput,
        outputKey,
        value: buildPreviewValue(sourceNode, outputKey)
      });
      continue;
    }

    missingFields.push(
      buildMissingPreviewField(document, sourceNodeId, outputKey)
    );
  }

  applyEnvironmentVariableUpdatesToPreviewPlan({
    document,
    nodeId,
    variableCache,
    inputPayload,
    missingFields
  });

  return { input_payload: inputPayload, missing_fields: missingFields };
}

export function buildNodeDebugVariableConfirmationPlan(
  document: FlowAuthoringDocument,
  nodeId: string,
  variableCache: NodeDebugPreviewVariableCache = {}
): NodeDebugVariableConfirmationPlan {
  const node = document.graph.nodes.find((entry) => entry.id === nodeId);
  const inputPayload: Record<string, Record<string, unknown>> = {};
  const fields: NodeDebugPreviewVariableField[] = [];

  if (!node) {
    return { input_payload: inputPayload, fields };
  }

  if (node.type === 'start') {
    const visitedStartKeys = new Set<string>();

    for (const output of getNodeVariableOutputs(node)) {
      if (visitedStartKeys.has(output.key)) {
        continue;
      }

      visitedStartKeys.add(output.key);

      const cachedOutput = readCachedOutputValue(
        variableCache[node.id],
        output,
        output.key
      );
      const value = cachedOutput.found
        ? cachedOutput.value
        : buildPreviewValue(node, output.key);

      writePreviewInputValue({
        inputPayload,
        sourceNodeId: node.id,
        sourceNode: node,
        sourceOutput: output,
        outputKey: output.key,
        value
      });
      fields.push(
        buildNodePreviewVariableField({
          document,
          nodeId: node.id,
          outputKey: output.key,
          sourceNode: node,
          sourceOutput: output,
          value
        })
      );
    }

    return { input_payload: inputPayload, fields };
  }

  const selectors = collectNodeReferenceSelectors(node);
  const visited = new Set<string>();

  for (const selector of selectors) {
    const sourceNodeId = selector[0] ?? '';
    const outputKey = selectorOutputKey(selector);
    const cacheKey = selector.join('.');
    const sourceNode = document.graph.nodes.find(
      (entry) => entry.id === sourceNodeId
    );
    const sourceOutput = findNodeOutputBySelector(sourceNode, selector);

    if (visited.has(cacheKey)) {
      continue;
    }

    visited.add(cacheKey);

    const cachedOutput = readCachedOutputValue(
      variableCache[sourceNodeId],
      sourceOutput,
      outputKey
    );
    const value = cachedOutput.found
      ? cachedOutput.value
      : buildPreviewValue(sourceNode, outputKey);

    writePreviewInputValue({
      inputPayload,
      sourceNodeId,
      sourceNode,
      sourceOutput,
      outputKey,
      value
    });
    fields.push(
      buildNodePreviewVariableField({
        document,
        nodeId: sourceNodeId,
        outputKey,
        sourceNode,
        sourceOutput,
        value
      })
    );
  }

  return { input_payload: inputPayload, fields };
}
