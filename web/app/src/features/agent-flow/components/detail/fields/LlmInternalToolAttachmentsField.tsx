import type {
  FlowAuthoringDocument,
  FlowNodeDocument
} from '@1flowbase/flow-schema';
import { Empty, Select } from 'antd';

import type { SchemaFieldRendererProps } from '../../../../../shared/schema-ui/registry/create-renderer-registry';
import {
  getLlmExecutionRole,
  getLlmVisibleInternalTools,
  type LlmVisibleInternalTool
} from '../../../lib/llm-node-config';
import { i18nText } from '../../../../../shared/i18n/text';

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function getNodeConfig(node: FlowNodeDocument) {
  return isRecord(node.config) ? node.config : {};
}

function getDocument(adapter: SchemaFieldRendererProps['adapter']) {
  return adapter.getDerived('document') as FlowAuthoringDocument | null;
}

function getCurrentNode(adapter: SchemaFieldRendererProps['adapter']) {
  return adapter.getDerived('node') as FlowNodeDocument | null;
}

function visibleInternalToolName(targetNodeId: string) {
  return `visible_${targetNodeId.replace(/[^A-Za-z0-9_-]/g, '_')}`;
}

function buildVisibleInternalTool(
  targetNode: FlowNodeDocument
): LlmVisibleInternalTool {
  return {
    type: 'visible_internal_llm_tool',
    tool_name: visibleInternalToolName(targetNode.id),
    target_node_id: targetNode.id,
    description: targetNode.alias,
    input_schema: { type: 'object' }
  };
}

export function LlmInternalToolAttachmentsField({
  adapter,
  block
}: SchemaFieldRendererProps) {
  const document = getDocument(adapter);
  const currentNode = getCurrentNode(adapter);

  if (!document || !currentNode) {
    return null;
  }

  const currentTools = getLlmVisibleInternalTools(getNodeConfig(currentNode));
  const selectedTargetIds = currentTools.map((tool) => tool.target_node_id);
  const targetNodes = document.graph.nodes.filter(
    (node) =>
      node.type === 'llm' &&
      node.id !== currentNode.id &&
      getLlmExecutionRole(getNodeConfig(node)) === 'visible_internal_llm_tool'
  );
  const options = targetNodes.map((node) => ({
    value: node.id,
    label: node.alias || node.id
  }));

  if (options.length === 0) {
    return (
      <Empty
        image={Empty.PRESENTED_IMAGE_SIMPLE}
        description={i18nText('agentFlow', 'auto.no_internal_llm_tool_targets')}
      />
    );
  }

  return (
    <Select
      aria-label={block.label}
      mode="multiple"
      allowClear
      options={options}
      value={selectedTargetIds.filter((targetNodeId) =>
        targetNodes.some((node) => node.id === targetNodeId)
      )}
      onChange={(nextTargetIds) => {
        const nextTools = nextTargetIds.flatMap((targetNodeId) => {
          const targetNode = targetNodes.find(
            (node) => node.id === targetNodeId
          );

          return targetNode ? [buildVisibleInternalTool(targetNode)] : [];
        });

        adapter.setValue('config.visible_internal_llm_tools', nextTools);
      }}
    />
  );
}
