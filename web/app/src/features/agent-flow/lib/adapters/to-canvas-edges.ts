import type { FlowAuthoringDocument } from '@1flowbase/flow-schema';

import type { AgentFlowCanvasEdge } from '../../components/canvas/node-types';
import type { NodePickerOption } from '../plugin-node-definitions';
import {
  createLlmToolSourceHandleId,
  getLlmVisibleInternalTools,
  getLlmVisibleInternalToolsEnabled
} from '../llm-node-config';

function toVisibleInternalLlmToolEdges(
  document: FlowAuthoringDocument,
  activeContainerId: string | null,
  visibleNodeIds: Set<string>
): AgentFlowCanvasEdge[] {
  const visibleLlmNodeIds = new Set(
    document.graph.nodes
      .filter(
        (node) =>
          node.containerId === activeContainerId &&
          node.type === 'llm' &&
          visibleNodeIds.has(node.id)
      )
      .map((node) => node.id)
  );

  return document.graph.nodes
    .filter(
      (node) =>
        node.containerId === activeContainerId &&
        node.type === 'llm' &&
        getLlmVisibleInternalToolsEnabled(node.config)
    )
    .flatMap((sourceNode) =>
      getLlmVisibleInternalTools(sourceNode.config).flatMap((tool) => {
        const connectorId = tool.connector_id || tool.tool_name;

        if (
          !tool.target_node_id ||
          !visibleLlmNodeIds.has(tool.target_node_id)
        ) {
          return [];
        }

        return [
          {
            id: `llm-tool-${sourceNode.id}-${connectorId}-${tool.target_node_id}`,
            type: 'agentFlowEdge' as const,
            source: sourceNode.id,
            target: tool.target_node_id,
            sourceHandle: createLlmToolSourceHandleId(connectorId),
            targetHandle: null,
            animated: false,
            selectable: false,
            style: {
              stroke: '#2f9e44',
              strokeDasharray: '5 5',
              strokeWidth: 2
            },
            data: {}
          }
        ];
      })
    );
}

export function toCanvasEdges(
  document: FlowAuthoringDocument,
  activeContainerId: string | null,
  selectedEdgeId: string | null,
  actions: {
    nodePickerOptions: NodePickerOption[];
    onInsertNode: (edgeId: string, option: NodePickerOption) => void;
  }
): AgentFlowCanvasEdge[] {
  const visibleNodeIds = new Set(
    document.graph.nodes
      .filter((node) => node.containerId === activeContainerId)
      .map((node) => node.id)
  );

  const topologyEdges = document.graph.edges
    .filter(
      (edge) =>
        edge.containerId === activeContainerId &&
        visibleNodeIds.has(edge.source) &&
        visibleNodeIds.has(edge.target)
    )
    .map((edge) => ({
      id: edge.id,
      type: 'agentFlowEdge' as const,
      selected: edge.id === selectedEdgeId,
      source: edge.source,
      target: edge.target,
      sourceHandle: edge.sourceHandle,
      targetHandle: edge.targetHandle,
      animated: false,
      style: { stroke: '#b2c8b9', strokeWidth: 2 },
      data: {
        nodePickerOptions: actions.nodePickerOptions,
        onInsertNode: actions.onInsertNode
      }
    }));

  return [
    ...topologyEdges,
    ...toVisibleInternalLlmToolEdges(
      document,
      activeContainerId,
      visibleNodeIds
    )
  ];
}
