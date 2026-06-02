import type { FlowAuthoringDocument } from '@1flowbase/flow-schema';

import { getNodeVariableOutputs } from './variables/start-node-variables';
import {
  agentFlowSystemVariables,
  systemVariableNodeId
} from './variables/system-variables';
import {
  environmentVariableNodeId,
  formatEnvironmentVariableTitle,
  type AgentFlowEnvironmentVariable
} from './variables/application-environment-variables';
import { outputHasLlmContextSchema } from './output-contract/schema';
import { formatNodeVariableLabel } from './variables/variable-labels';
import { i18nText } from '../../../shared/i18n/text';

export interface FlowSelectorOption {
  nodeId: string;
  nodeLabel: string;
  outputKey: string;
  outputLabel: string;
  valueType: string;
  jsonSchema?: Record<string, unknown>;
  value: string[];
  displayLabel: string;
}

interface CascaderSelectorOption {
  label: string;
  value: string;
  children?: CascaderSelectorOption[];
}

function outputSelectorValue(nodeId: string, output: { key: string; selector?: string[] }) {
  return [
    nodeId,
    ...(output.selector && output.selector.length > 0
      ? output.selector
      : [output.key])
  ];
}

function collectUpstreamNodeIds(
  document: FlowAuthoringDocument,
  nodeId: string
): Set<string> {
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

export function listVisibleSelectorOptions(
  document: FlowAuthoringDocument,
  nodeId: string,
  environmentVariables: AgentFlowEnvironmentVariable[] = []
): FlowSelectorOption[] {
  const visibleNodeIds = collectUpstreamNodeIds(document, nodeId);
  const systemOptions = agentFlowSystemVariables.map((variable) => ({
    nodeId: systemVariableNodeId,
    nodeLabel: i18nText("agentFlow", "auto.system_variables"),
    outputKey: variable.key,
    outputLabel: variable.title,
    valueType: variable.valueType,
    jsonSchema: variable.jsonSchema,
    value: [systemVariableNodeId, variable.key],
    displayLabel: variable.title
  }));
  const environmentOptions = environmentVariables.map((variable) => ({
    nodeId: environmentVariableNodeId,
    nodeLabel: i18nText("agentFlow", "auto.environment_variables"),
    outputKey: variable.name,
    outputLabel: formatEnvironmentVariableTitle(variable.name),
    valueType: variable.value_type,
    value: [environmentVariableNodeId, variable.name],
    displayLabel: formatEnvironmentVariableTitle(variable.name)
  }));

  const nodeOptions = document.graph.nodes
    .filter((node) => visibleNodeIds.has(node.id))
    .flatMap((node) =>
      getNodeVariableOutputs(node).map((output) => ({
        nodeId: node.id,
        nodeLabel: node.alias,
        outputKey: output.key,
        outputLabel: output.key,
        valueType: output.valueType,
        jsonSchema: output.jsonSchema,
        value: outputSelectorValue(node.id, output),
        displayLabel: formatNodeVariableLabel(node.alias, output.key)
      }))
    );

  return [...systemOptions, ...environmentOptions, ...nodeOptions];
}

export function listLlmContextSelectorOptions(
  document: FlowAuthoringDocument,
  nodeId: string,
  environmentVariables: AgentFlowEnvironmentVariable[] = []
) {
  return listVisibleSelectorOptions(document, nodeId, environmentVariables).filter(
    (option) => outputHasLlmContextSchema(option)
  );
}

export function toCascaderSelectorOptions(options: FlowSelectorOption[]) {
  const groups = new Map<
    string,
    {
      label: string;
      value: string;
      children: CascaderSelectorOption[];
    }
  >();

  for (const option of options) {
    if (!groups.has(option.nodeId)) {
      groups.set(option.nodeId, {
        label: option.nodeLabel,
        value: option.nodeId,
        children: []
      });
    }

    const group = groups.get(option.nodeId);
    if (!group) {
      continue;
    }

    appendCascaderSelectorPath(
      group.children,
      option.value.slice(1),
      option.outputLabel
    );
  }

  return [...groups.values()];
}

function appendCascaderSelectorPath(
  children: CascaderSelectorOption[],
  path: string[],
  outputLabel: string
) {
  const [segment, ...rest] = path;

  if (!segment) {
    return;
  }

  const label = rest.length === 0 ? outputLabel : segment;
  let child = children.find((candidate) => candidate.value === segment);

  if (!child) {
    child = { label, value: segment };
    children.push(child);
  } else if (rest.length === 0) {
    child.label = label;
  }

  if (rest.length > 0) {
    child.children ??= [];
    appendCascaderSelectorPath(child.children, rest, outputLabel);
  }
}

export function isSelectorVisible(
  document: FlowAuthoringDocument,
  nodeId: string,
  selector: string[],
  environmentVariables: AgentFlowEnvironmentVariable[] = []
): boolean {
  if (selector.length < 2) {
    return false;
  }

  return listVisibleSelectorOptions(
    document,
    nodeId,
    environmentVariables
  ).some(
    (option) =>
      option.value.length === selector.length &&
      option.value.every((segment, index) => segment === selector[index])
  );
}

export function encodeSelectorValue(value: string[]): string {
  return JSON.stringify(value);
}

export function decodeSelectorValue(value: string): string[] {
  try {
    const parsed = JSON.parse(value);

    return Array.isArray(parsed)
      ? parsed.filter((segment) => typeof segment === 'string')
      : [];
  } catch {
    return [];
  }
}
