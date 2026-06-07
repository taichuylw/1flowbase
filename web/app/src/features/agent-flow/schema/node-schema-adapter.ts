import type {
  FlowAuthoringDocument,
  FlowBinding,
  FlowNodeDocument
} from '@1flowbase/flow-schema';

import {
  replaceNodeOutputs,
  updateNodeField
} from '../lib/document/transforms/node';
import { getDirectDownstreamNodes } from '../lib/document/relations';
import { listVisibleSelectorOptions } from '../lib/selector-options';
import { getNodeDefinitionMeta } from '../lib/node-definitions';
import { getBuiltinNodeRuntimeContract } from '../lib/node-definitions/contracts';
import { parseHttpRequestUrlParts } from '../lib/http-request/url';
import type { AgentFlowEnvironmentVariable } from '../lib/variables/application-environment-variables';
import type { AgentFlowIssue } from '../lib/validate-document';

import type { SchemaAdapter } from '../../../shared/schema-ui/registry/create-renderer-registry';

function getNode(document: FlowAuthoringDocument, nodeId: string) {
  const node = document.graph.nodes.find(
    (candidate) => candidate.id === nodeId
  );

  if (!node) {
    throw new Error(`Missing agent-flow node: ${nodeId}`);
  }

  return node;
}

function omitKey<T extends Record<string, unknown>>(value: T, key: string) {
  if (!(key in value)) {
    return value;
  }

  const nextValue = { ...value };

  delete nextValue[key];
  return nextValue;
}

function createRootValues(node: FlowNodeDocument) {
  return {
    ...node,
    config: {
      ...node.config,
      output_contract: getDisplayOutputs(node)
    }
  };
}

function toHttpRequestParamsBinding(
  params: Array<{ name: string; value: string }>
): FlowBinding {
  return {
    kind: 'named_bindings',
    value: params.map((param) => ({
      name: param.name,
      value: { kind: 'templated_text', value: param.value }
    }))
  };
}

function updateHttpRequestUrlField(
  document: FlowAuthoringDocument,
  nodeId: string,
  value: string
) {
  const parsed = parseHttpRequestUrlParts(value);
  const nextDocument = updateNodeField(document, {
    nodeId,
    fieldKey: 'config.url',
    value: parsed.url
  });

  if (parsed.params.length === 0) {
    return nextDocument;
  }

  return updateNodeField(nextDocument, {
    nodeId,
    fieldKey: 'bindings.params',
    value: toHttpRequestParamsBinding(parsed.params)
  });
}

function getDisplayOutputs(node: FlowNodeDocument) {
  if (node.type !== 'http_request') {
    return node.outputs;
  }

  const contractOutputs =
    getBuiltinNodeRuntimeContract(node.type)?.defaults.outputs ?? [];
  const outputsByKey = new Map(node.outputs.map((output) => [output.key, output]));

  return [
    ...node.outputs,
    ...contractOutputs.filter((output) => !outputsByKey.has(output.key))
  ];
}

function groupFieldIssues(
  issues: AgentFlowIssue[],
  nodeId: string
): Record<string, AgentFlowIssue[]> {
  const grouped: Record<string, AgentFlowIssue[]> = {};

  for (const issue of issues) {
    if (issue.nodeId !== nodeId || !issue.fieldKey) {
      continue;
    }

    grouped[issue.fieldKey] = [...(grouped[issue.fieldKey] ?? []), issue];
  }

  return grouped;
}

export function createAgentFlowNodeSchemaAdapter({
  document,
  nodeId,
  setWorkingDocument,
  dispatch,
  environmentVariables = [],
  issues = []
}: {
  document: FlowAuthoringDocument;
  nodeId: string;
  environmentVariables?: AgentFlowEnvironmentVariable[];
  issues?: AgentFlowIssue[];
  setWorkingDocument: (
    update:
      | FlowAuthoringDocument
      | ((document: FlowAuthoringDocument) => FlowAuthoringDocument)
  ) => void;
  dispatch: (actionKey: string, payload?: unknown) => void;
}): SchemaAdapter {
  const node = getNode(document, nodeId);

  return {
    getValue(path: string) {
      if (path === 'alias') {
        return node.alias;
      }

      if (path === 'description') {
        return node.description ?? '';
      }

      if (path === 'config.output_contract' || path === 'outputs') {
        return getDisplayOutputs(node);
      }

      if (path.startsWith('outputs.')) {
        return node.outputs.find(
          (output) => output.key === path.slice('outputs.'.length)
        )?.title;
      }

      if (path.startsWith('config.')) {
        return node.config[path.slice('config.'.length)];
      }

      if (path.startsWith('bindings.')) {
        return node.bindings[path.slice('bindings.'.length)];
      }

      return undefined;
    },
    setValue(path: string, value: unknown) {
      if (path === 'config.output_contract' && Array.isArray(value)) {
        setWorkingDocument((currentDocument) => {
          const nextDocument = replaceNodeOutputs(
            currentDocument,
            nodeId,
            value
          );

          return {
            ...nextDocument,
            graph: {
              ...nextDocument.graph,
              nodes: nextDocument.graph.nodes.map((candidate) =>
                candidate.id === nodeId
                  ? {
                      ...candidate,
                      config: omitKey(candidate.config, 'output_contract')
                    }
                  : candidate
              )
            }
          };
        });

        return;
      }

      if (
        node.type === 'http_request' &&
        path === 'config.url' &&
        typeof value === 'string'
      ) {
        setWorkingDocument((currentDocument) =>
          updateHttpRequestUrlField(currentDocument, nodeId, value)
        );

        return;
      }

      setWorkingDocument((currentDocument) =>
        updateNodeField(currentDocument, {
          nodeId,
          fieldKey: path,
          value: value as never
        })
      );
    },
    getDerived(key: string) {
      if (key === 'rootValues') {
        return createRootValues(node);
      }

      if (key === 'fieldIssues') {
        return groupFieldIssues(issues, nodeId);
      }

      if (key === 'node' || key === 'selectedNode') {
        return node;
      }

      if (key === 'definitionMeta') {
        return getNodeDefinitionMeta(node.type);
      }

      if (key === 'selectorOptions') {
        return listVisibleSelectorOptions(
          document,
          nodeId,
          environmentVariables
        );
      }

      if (key === 'environmentVariables') {
        return environmentVariables;
      }

      if (key === 'downstreamNodes') {
        return getDirectDownstreamNodes(document, nodeId);
      }

      if (key === 'outputContract') {
        return node.outputs;
      }

      return null;
    },
    dispatch
  };
}
