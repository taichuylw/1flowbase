import type { FlowNodeDocument } from '@1flowbase/flow-schema';
import { Card, Typography } from 'antd';
import type { SchemaAdapter } from '../../../../../shared/schema-ui/registry/create-renderer-registry';
import {
  getNodeDefinitionMeta,
  nodeDefinitions
} from '../../../lib/node-definitions';
import { useAgentFlowEditorStore } from '../../../store/editor/provider';
import {
  selectSelectedNodeId,
  selectWorkingDocument
} from '../../../store/editor/selectors';
import { i18nText } from '../../../../../shared/i18n/text';

export function NodeSummaryCard({
  adapter
}: {
  adapter?: SchemaAdapter;
} = {}) {
  const document = useAgentFlowEditorStore(selectWorkingDocument);
  const selectedNodeId = useAgentFlowEditorStore(selectSelectedNodeId);
  const selectedNode: FlowNodeDocument | null =
    (adapter?.getDerived('node') as FlowNodeDocument | null | undefined) ??
    (selectedNodeId
      ? document.graph.nodes.find((node) => node.id === selectedNodeId) ?? null
      : null);
  const definition =
    selectedNode
      ? nodeDefinitions[selectedNode.type] ??
        null
      : null;
  const definitionMeta =
    selectedNode
      ? ((adapter?.getDerived('definitionMeta') as ReturnType<typeof getNodeDefinitionMeta> | null | undefined) ??
        getNodeDefinitionMeta(selectedNode.type))
      : null;

  if (!selectedNode || !definition || !definitionMeta) {
    return null;
  }

  return (
    <Card
      extra={
        definitionMeta.helpHref ? (
          <Typography.Link href={definitionMeta.helpHref} target="_blank">
            {i18nText("agentFlow", "auto.k_39c1887749")}</Typography.Link>
        ) : null
      }
      title={i18nText("agentFlow", "auto.k_88a5351e42")}
    >
      <Typography.Paragraph>
        {definition.summary ?? definitionMeta.summary}
      </Typography.Paragraph>
    </Card>
  );
}
