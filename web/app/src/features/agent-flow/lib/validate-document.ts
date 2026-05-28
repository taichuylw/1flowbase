import {
  getLlmNodeOutputs,
  validatePublicOutputKey,
  type FlowAuthoringDocument
} from '@1flowbase/flow-schema';
import type { FlowBinding, FlowNodeDocument } from '@1flowbase/flow-schema';

import { evaluateSchemaRule } from '../../../shared/schema-ui/runtime/rule-evaluator';
import type { AgentFlowModelProviderOptions } from '../api/model-provider-options';
import {
  extractDataModelQuerySelectors,
  getActiveNodeBindings
} from './data-model-query-binding';
import { getLlmModelProvider } from './llm-node-config';
import { getBuiltinNodeRuntimeContract } from './node-definitions/contracts';
import type {
  InspectorSectionKey,
  NodeDefinitionField
} from './node-definitions';
import { findInspectorSectionKey, nodeDefinitions } from './node-definitions';
import { hasPluginContributionRef } from './plugin-node-definitions';
import { isSelectorVisible } from './selector-options';
import { parseTemplateSelectorTokens } from './template-binding';
import {
  environmentVariableNodeId,
  type AgentFlowEnvironmentVariable
} from './application-environment-variables';
import { systemVariableNodeId } from './system-variables';
import { i18nText } from '../../../shared/i18n/text';

export interface AgentFlowIssue {
  id: string;
  scope: 'field' | 'node' | 'global';
  level: 'error' | 'warning';
  nodeId: string | null;
  sectionKey: InspectorSectionKey | null;
  fieldKey?: string | null;
  title: string;
  message: string;
}

function isMissingRequiredField(
  node: FlowNodeDocument,
  fieldKey: string
): boolean {
  if (fieldKey === 'alias') {
    return node.alias.trim().length === 0;
  }

  if (fieldKey.startsWith('config.')) {
    if (fieldKey === 'config.model_provider') {
      const modelProvider = getLlmModelProvider(node.config);
      return (
        modelProvider.provider_code.trim().length === 0 ||
        modelProvider.model_id.trim().length === 0
      );
    }

    const configValue = node.config[fieldKey.slice('config.'.length)];

    if (typeof configValue === 'string') {
      return configValue.trim().length === 0;
    }

    return configValue === undefined || configValue === null;
  }

  if (fieldKey.startsWith('outputs.')) {
    const outputKey = fieldKey.slice('outputs.'.length);
    const output = node.outputs.find((item) => item.key === outputKey);

    return !output || output.title.trim().length === 0;
  }

  if (!fieldKey.startsWith('bindings.')) {
    return false;
  }

  const binding = node.bindings[fieldKey.slice('bindings.'.length)];

  if (!binding) {
    return true;
  }

  switch (binding.kind) {
    case 'templated_text':
      return binding.value.trim().length === 0;
    case 'selector':
      return binding.value.length === 0;
    case 'selector_list':
      return binding.value.length === 0;
    case 'data_model_query':
      return false;
    case 'prompt_messages':
      return binding.value.length === 0;
    case 'named_bindings':
      return binding.value.length === 0;
    case 'condition_group':
      return binding.value.conditions.length === 0;
    case 'state_write':
      return binding.value.length === 0;
  }
}

function createNodeRuleValues(node: FlowNodeDocument): Record<string, unknown> {
  return {
    ...node,
    config: {
      ...node.config,
      output_contract: node.outputs
    }
  };
}

function isFieldVisibleForNode(
  node: FlowNodeDocument,
  field: NodeDefinitionField
) {
  return evaluateSchemaRule(field.visibleWhen, {
    values: createNodeRuleValues(node),
    capabilities: []
  });
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function getAllowedPluginNodeOutputKeys(
  node: FlowNodeDocument
): Set<string> | null {
  const rawSnapshot = node.output_schema_snapshot;

  if (!isRecord(rawSnapshot) || !Array.isArray(rawSnapshot.outputs)) {
    return null;
  }

  const allowed = rawSnapshot.outputs
    .map((entry) => {
      if (!isRecord(entry)) {
        return null;
      }

      if (typeof entry.key !== 'string') {
        return null;
      }

      const outputKey = entry.key.trim();

      return outputKey.length > 0 ? outputKey : null;
    })
    .filter((key): key is string => key !== null);

  return new Set(allowed);
}

function collectBindingSelectors(binding: FlowBinding): string[][] {
  switch (binding.kind) {
    case 'templated_text':
      return parseTemplateSelectorTokens(binding.value);
    case 'selector':
      return [binding.value];
    case 'selector_list':
      return binding.value;
    case 'prompt_messages':
      return binding.value.flatMap((message) =>
        parseTemplateSelectorTokens(message.content.value)
      );
    case 'named_bindings':
      return binding.value.flatMap((entry) =>
        entry.content?.kind === 'templated_text'
          ? parseTemplateSelectorTokens(entry.content.value)
          : entry.selector
            ? [entry.selector]
            : []
      );
    case 'condition_group':
      return binding.value.conditions.flatMap((condition) => {
        const selectors = [condition.left];

        if (Array.isArray(condition.right)) {
          selectors.push(condition.right);
        }

        return selectors;
      });
    case 'state_write':
      return binding.value.flatMap((entry) =>
        entry.source ? [entry.source] : []
      );
    case 'data_model_query':
      return extractDataModelQuerySelectors(binding.value);
  }
}

function pushFieldIssue(
  issues: AgentFlowIssue[],
  node: FlowNodeDocument,
  fieldKey: string,
  title: string,
  message: string,
  sectionKey?: InspectorSectionKey | null
) {
  issues.push({
    id: `${node.id}-${fieldKey}-${issues.length}`,
    scope: 'field',
    level: 'error',
    nodeId: node.id,
    sectionKey: sectionKey ?? findInspectorSectionKey(node.type, fieldKey),
    fieldKey,
    title,
    message
  });
}

function isRuntimeSelectorSource(source: string) {
  return (
    source === systemVariableNodeId || source === environmentVariableNodeId
  );
}

function getAllowedPublicOutputKeysForNode(
  node: FlowNodeDocument
): Set<string> | null {
  if (node.type === 'plugin_node' && hasPluginContributionRef(node)) {
    return getAllowedPluginNodeOutputKeys(node);
  }

  if (node.type === 'llm') {
    return new Set(getLlmNodeOutputs(node.config).map((output) => output.key));
  }

  if (node.type === 'code') {
    return new Set(node.outputs.map((output) => output.key));
  }

  const contract = getBuiltinNodeRuntimeContract(node.type);

  if (!contract) {
    return null;
  }

  return new Set(contract.defaults.outputs.map((output) => output.key));
}

interface AnswerPresentationReference {
  nodeId: string;
  outputKey: string;
}

function collectAnswerPresentationReferences(
  node: FlowNodeDocument
): AnswerPresentationReference[] {
  const binding = node.bindings.answer_template;

  if (!binding) {
    return [];
  }

  const selectors =
    binding.kind === 'selector'
      ? [binding.value]
      : binding.kind === 'templated_text'
        ? parseTemplateSelectorTokens(binding.value)
        : [];

  return selectors
    .filter((selector) => selector.length >= 2)
    .map((selector) => ({
      nodeId: selector[0],
      outputKey: selector[1]
    }));
}

function buildNodeDependencyMap(
  document: FlowAuthoringDocument,
  nodeIds: Set<string>
): Map<string, Set<string>> {
  const dependencies = new Map<string, Set<string>>();

  for (const node of document.graph.nodes) {
    dependencies.set(node.id, new Set());
  }

  for (const edge of document.graph.edges) {
    if (!nodeIds.has(edge.source) || !nodeIds.has(edge.target)) {
      continue;
    }
    dependencies.get(edge.target)?.add(edge.source);
  }

  for (const node of document.graph.nodes) {
    const nodeDependencies = dependencies.get(node.id);
    if (!nodeDependencies) {
      continue;
    }

    for (const [, binding] of getActiveNodeBindings(node)) {
      for (const selector of collectBindingSelectors(binding)) {
        const sourceNodeId = selector[0] ?? '';
        if (
          selector.length >= 2 &&
          !isRuntimeSelectorSource(sourceNodeId) &&
          nodeIds.has(sourceNodeId) &&
          sourceNodeId !== node.id
        ) {
          nodeDependencies.add(sourceNodeId);
        }
      }
    }
  }

  return dependencies;
}

function nodeDependsOn(
  dependencies: Map<string, Set<string>>,
  nodeId: string,
  dependencyNodeId: string
): boolean {
  const stack = [nodeId];
  const visited = new Set<string>();

  while (stack.length > 0) {
    const current = stack.pop();
    if (!current || visited.has(current)) {
      continue;
    }
    visited.add(current);

    for (const dependency of dependencies.get(current) ?? []) {
      if (dependency === dependencyNodeId) {
        return true;
      }
      stack.push(dependency);
    }
  }

  return false;
}

function formatAnswerPresentationReference(
  reference: AnswerPresentationReference,
  nodeById: Map<string, FlowNodeDocument>
): string {
  const node = nodeById.get(reference.nodeId);
  const nodeLabel = node?.alias.trim() || reference.nodeId;

  return `${nodeLabel}.${reference.outputKey}`;
}

function validateAnswerPresentationReferences(
  issues: AgentFlowIssue[],
  answerNode: FlowNodeDocument,
  nodeById: Map<string, FlowNodeDocument>,
  dependencies: Map<string, Set<string>>
) {
  const references = collectAnswerPresentationReferences(answerNode);
  const seen = new Set<string>();

  for (const reference of references) {
    const key = `${reference.nodeId}.${reference.outputKey}`;
    if (!seen.has(key)) {
      seen.add(key);
      continue;
    }

    pushFieldIssue(
      issues,
      answerNode,
      'bindings.answer_template',
      i18nText("agentFlow", "auto.k_8ddb3ef04a"),
      i18nText("agentFlow", "auto.k_233c242d44", { value1: formatAnswerPresentationReference(
        reference,
        nodeById
      ) })
    );
  }

  for (let index = 0; index < references.length; index += 1) {
    const current = references[index];
    for (const later of references.slice(index + 1)) {
      if (!nodeDependsOn(dependencies, current.nodeId, later.nodeId)) {
        continue;
      }

      pushFieldIssue(
        issues,
        answerNode,
        'bindings.answer_template',
        i18nText("agentFlow", "auto.k_d088ac228b"),
        i18nText("agentFlow", "auto.k_7ec89eb5f1", { value1: formatAnswerPresentationReference(
          current,
          nodeById
        ), value2: formatAnswerPresentationReference(
          later,
          nodeById
        ) })
      );
      break;
    }
  }
}

export function validateDocument(
  document: FlowAuthoringDocument,
  providerOptions?: AgentFlowModelProviderOptions | null,
  environmentVariables: AgentFlowEnvironmentVariable[] = []
): AgentFlowIssue[] {
  const issues: AgentFlowIssue[] = [];
  const nodeIds = new Set(document.graph.nodes.map((node) => node.id));
  const nodeById = new Map(document.graph.nodes.map((node) => [node.id, node]));
  const dependencies = buildNodeDependencyMap(document, nodeIds);
  const startNodes = document.graph.nodes.filter(
    (node) => node.type === 'start'
  );
  const answerNodes = document.graph.nodes.filter(
    (node) => node.type === 'answer'
  );
  const providerMap = new Map(
    (providerOptions?.providers ?? []).map((provider) => [
      provider.provider_code,
      provider
    ])
  );

  if (startNodes.length !== 1) {
    issues.push({
      id: 'global-start-count',
      scope: 'global',
      level: 'error',
      nodeId: null,
      sectionKey: null,
      fieldKey: null,
      title: i18nText("agentFlow", "auto.k_ec4fe5d27f"),
      message: i18nText("agentFlow", "auto.k_df4dcc7220")
    });
  }

  if (answerNodes.length === 0) {
    issues.push({
      id: 'global-answer-missing',
      scope: 'global',
      level: 'error',
      nodeId: null,
      sectionKey: null,
      fieldKey: null,
      title: i18nText("agentFlow", "auto.k_7e0914363c"),
      message: i18nText("agentFlow", "auto.k_aa95340236")
    });
  }

  for (const edge of document.graph.edges) {
    if (nodeIds.has(edge.source) && nodeIds.has(edge.target)) {
      continue;
    }

    if (nodeIds.has(edge.source)) {
      issues.push({
        id: `${edge.id}-dangling`,
        scope: 'node',
        level: 'warning',
        nodeId: edge.source,
        sectionKey: 'basics',
        fieldKey: null,
        title: i18nText("agentFlow", "auto.k_2e9a93d149"),
        message: i18nText("agentFlow", "auto.k_b343ad9eb0")
      });
    }
  }

  for (const node of document.graph.nodes) {
    const definition = nodeDefinitions[node.type];

    if (definition) {
      for (const section of definition.sections) {
        for (const field of section.fields) {
          if (
            field.required &&
            isFieldVisibleForNode(node, field) &&
            isMissingRequiredField(node, field.key)
          ) {
            if (node.type === 'llm' && field.key === 'config.model_provider') {
              const modelProvider = getLlmModelProvider(node.config);
              const providerMissing =
                modelProvider.provider_code.trim().length === 0;

              pushFieldIssue(
                issues,
                node,
                field.key,
                providerMissing ? i18nText("agentFlow", "auto.k_024ea49d91") : i18nText("agentFlow", "auto.k_41120ce991"),
                providerMissing ? i18nText("agentFlow", "auto.k_bf767813d4") : i18nText("agentFlow", "auto.k_faa88f53cb")
              );
              continue;
            }

            pushFieldIssue(
              issues,
              node,
              field.key,
              i18nText("agentFlow", "auto.k_eac7be0d60", { value1: field.label }),
              i18nText("agentFlow", "auto.k_7379c1eb7d", { value1: field.label })
            );
          }
        }
      }
    }

    if (node.type === 'llm') {
      const modelProvider = getLlmModelProvider(node.config);
      const providerCode = modelProvider.provider_code.trim();
      const model = modelProvider.model_id.trim();

      if (providerOptions && providerCode.length > 0) {
        const provider = providerMap.get(providerCode);

        if (!provider) {
          pushFieldIssue(
            issues,
            node,
            'config.model_provider',
            i18nText("agentFlow", "auto.k_8b8ced23f7"),
            i18nText("agentFlow", "auto.k_c2d9373321"),
            'inputs'
          );
        } else if (model.length > 0) {
          const matchingModelGroups = provider.model_groups.filter((group) =>
            group.models.some((entry) => entry.model_id === model)
          );

          if (matchingModelGroups.length === 0) {
            pushFieldIssue(
              issues,
              node,
              'config.model_provider',
              i18nText("agentFlow", "auto.k_a24c3d32e6"),
              i18nText("agentFlow", "auto.k_d497ba2a81")
            );
          } else if (matchingModelGroups.length > 1) {
            pushFieldIssue(
              issues,
              node,
              'config.model_provider',
              i18nText("agentFlow", "auto.k_be4f860b2b"),
              i18nText("agentFlow", "auto.k_65fb2ddddd")
            );
          }
        }
      }
    }

    if (node.type === 'plugin_node' && !hasPluginContributionRef(node)) {
      issues.push({
        id: `${node.id}-plugin-ref-missing`,
        scope: 'node',
        level: 'error',
        nodeId: node.id,
        sectionKey: 'basics',
        fieldKey: null,
        title: i18nText("agentFlow", "auto.k_76b2f7c78b"),
        message:
          i18nText("agentFlow", "auto.k_1323179e36")
      });
    }

    if (node.type === 'answer') {
      validateAnswerPresentationReferences(
        issues,
        node,
        nodeById,
        dependencies
      );
    }

    for (const [bindingKey, bindingValue] of getActiveNodeBindings(node)) {
      const selectors = collectBindingSelectors(bindingValue);

      for (const selector of selectors) {
        const sourceNodeId = selector[0] ?? '';

        if (
          selector.length === 0 ||
          isSelectorVisible(document, node.id, selector, environmentVariables)
        ) {
          continue;
        }

        if (
          selector.length >= 2 &&
          !isRuntimeSelectorSource(sourceNodeId) &&
          !nodeIds.has(sourceNodeId)
        ) {
          pushFieldIssue(
            issues,
            node,
            `bindings.${bindingKey}`,
            i18nText("agentFlow", "auto.k_3c5c696e82"),
            i18nText("agentFlow", "auto.k_8f4b29ccbe", { value1: sourceNodeId })
          );
          continue;
        }

        pushFieldIssue(
          issues,
          node,
          `bindings.${bindingKey}`,
          i18nText("agentFlow", "auto.k_e96bd1da50"),
          i18nText("agentFlow", "auto.k_b772b3b2b2")
        );
      }
    }

    const seenOutputKeys = new Set<string>();
    const allowedPublicOutputKeys = getAllowedPublicOutputKeysForNode(node);

    if (node.type === 'code') {
      const language =
        typeof node.config.language === 'string'
          ? node.config.language.trim()
          : '';

      if (language.length > 0 && language !== 'javascript') {
        pushFieldIssue(
          issues,
          node,
          'config.language',
          i18nText("agentFlow", "auto.k_d0ce6fae48"),
          i18nText("agentFlow", "auto.k_da8dd565b0")
        );
      }
    }

    if (node.type === 'code' && node.outputs.length === 0) {
      pushFieldIssue(
        issues,
        node,
        'config.output_contract',
        i18nText("agentFlow", "auto.k_1ac0f51ab0"),
        i18nText("agentFlow", "auto.k_946646d9b6")
      );
    }

    for (const output of node.outputs) {
      const outputKey = output.key.trim();

      if (outputKey.length === 0) {
        pushFieldIssue(
          issues,
          node,
          'config.output_contract',
          i18nText("agentFlow", "auto.k_437801c02f"),
          i18nText("agentFlow", "auto.k_05d855be28")
        );
        continue;
      }

      if (seenOutputKeys.has(outputKey)) {
        pushFieldIssue(
          issues,
          node,
          'config.output_contract',
          i18nText("agentFlow", "auto.k_afbae2cee4"),
          i18nText("agentFlow", "auto.k_912653eed6")
        );
        continue;
      }

      seenOutputKeys.add(outputKey);

      const publicOutputKeyValidation = validatePublicOutputKey(outputKey);

      if (!publicOutputKeyValidation.ok) {
        pushFieldIssue(
          issues,
          node,
          'config.output_contract',
          i18nText("agentFlow", "auto.k_d0ba848f7b"),
          i18nText("agentFlow", "auto.k_129a5a768b")
        );
        continue;
      }

      if (allowedPublicOutputKeys && !allowedPublicOutputKeys.has(outputKey)) {
        pushFieldIssue(
          issues,
          node,
          'config.output_contract',
          i18nText("agentFlow", "auto.k_0873e82e58"),
          i18nText("agentFlow", "auto.k_74bdd135cd")
        );
      }
    }

    if (
      node.type !== 'start' &&
      !document.graph.edges.some(
        (edge) => edge.target === node.id && nodeIds.has(edge.source)
      )
    ) {
      issues.push({
        id: `${node.id}-orphan`,
        scope: 'node',
        level: 'warning',
        nodeId: node.id,
        sectionKey: 'basics',
        fieldKey: null,
        title: i18nText("agentFlow", "auto.k_dcd585ab01", { value1: node.alias }),
        message: i18nText("agentFlow", "auto.k_f98a7352a1")
      });
    }
  }

  return issues;
}
