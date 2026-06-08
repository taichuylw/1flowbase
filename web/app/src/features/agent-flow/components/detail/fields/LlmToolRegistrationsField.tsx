import type {
  FlowAuthoringDocument,
  FlowNodeDocument
} from '@1flowbase/flow-schema';
import { DeleteOutlined, PlusOutlined } from '@ant-design/icons';
import { Button, Input, Select, Space, Switch, Table } from 'antd';

import type { SchemaFieldRendererProps } from '../../../../../shared/schema-ui/registry/create-renderer-registry';
import {
  getLlmVisibleInternalTools,
  getLlmVisibleInternalToolsEnabled,
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

function createToolName(index: number) {
  return `tool_${index + 1}`;
}

function normalizeConnectorId(value: string) {
  return value.replace(/[^A-Za-z0-9_-]/g, '_');
}

function buildNextTool(
  tools: LlmVisibleInternalTool[],
  targetNodeId: string | undefined
): LlmVisibleInternalTool {
  const toolName = createToolName(tools.length);

  return {
    type: 'visible_internal_llm_tool',
    tool_name: toolName,
    connector_id: toolName,
    target_node_id: targetNodeId ?? '',
    input_schema: { type: 'object' }
  };
}

function replaceTool(
  tools: LlmVisibleInternalTool[],
  index: number,
  patch: Partial<LlmVisibleInternalTool>
) {
  return tools.map((tool, toolIndex) =>
    toolIndex === index
      ? {
          ...tool,
          ...patch
        }
      : tool
  );
}

function schemaText(schema: LlmVisibleInternalTool['input_schema']) {
  return JSON.stringify(schema ?? { type: 'object' }, null, 2);
}

function parseInputSchema(value: string) {
  try {
    const parsed = JSON.parse(value) as unknown;

    return isRecord(parsed) ? parsed : undefined;
  } catch {
    return undefined;
  }
}

export function LlmToolRegistrationsField({
  adapter,
  block
}: SchemaFieldRendererProps) {
  const document = getDocument(adapter);
  const currentNode = getCurrentNode(adapter);

  if (!document || !currentNode) {
    return null;
  }

  const currentConfig = getNodeConfig(currentNode);
  const enabled = getLlmVisibleInternalToolsEnabled(currentConfig);
  const tools = getLlmVisibleInternalTools(currentConfig);
  const targetNodes = document.graph.nodes.filter(
    (node) => node.type === 'llm' && node.id !== currentNode.id
  );
  const targetOptions = targetNodes.map((node) => ({
    value: node.id,
    label: node.alias || node.id
  }));

  function updateTools(nextTools: LlmVisibleInternalTool[]) {
    adapter.setValue('config.visible_internal_llm_tools', nextTools);
  }

  return (
    <Space direction="vertical" size={12} style={{ width: '100%' }}>
      <Switch
        aria-label={block.label}
        checked={enabled}
        onChange={(checked) =>
          adapter.setValue('config.visible_internal_llm_tools_enabled', checked)
        }
      />
      {enabled ? (
        <Table<LlmVisibleInternalTool>
          pagination={false}
          rowKey={(tool, index) =>
            tool.connector_id || tool.tool_name || `tool-${index}`
          }
          size="small"
          dataSource={tools}
          locale={{
            emptyText: i18nText('agentFlow', 'auto.no_tool_registrations')
          }}
          columns={[
            {
              title: i18nText('agentFlow', 'auto.tool_name'),
              dataIndex: 'tool_name',
              render: (_value, tool, index) => (
                <Input
                  aria-label={`${block.label}-tool-name-${index + 1}`}
                  value={tool.tool_name}
                  onChange={(event) => {
                    const toolName = event.target.value;
                    updateTools(
                      replaceTool(tools, index, {
                        tool_name: toolName,
                        connector_id:
                          tool.connector_id || normalizeConnectorId(toolName)
                      })
                    );
                  }}
                />
              )
            },
            {
              title: i18nText('agentFlow', 'auto.target_llm'),
              dataIndex: 'target_node_id',
              render: (_value, tool, index) => (
                <Select
                  aria-label={`${block.label}-target-llm-${index + 1}`}
                  options={targetOptions}
                  value={tool.target_node_id || undefined}
                  onChange={(targetNodeId) =>
                    updateTools(
                      replaceTool(tools, index, {
                        target_node_id: targetNodeId
                      })
                    )
                  }
                />
              )
            },
            {
              title: i18nText('agentFlow', 'auto.description'),
              dataIndex: 'description',
              render: (_value, tool, index) => (
                <Input
                  aria-label={`${block.label}-description-${index + 1}`}
                  value={tool.description ?? ''}
                  onChange={(event) =>
                    updateTools(
                      replaceTool(tools, index, {
                        description: event.target.value || undefined
                      })
                    )
                  }
                />
              )
            },
            {
              title: i18nText('agentFlow', 'auto.input_schema'),
              dataIndex: 'input_schema',
              render: (_value, tool, index) => (
                <Input.TextArea
                  aria-label={`${block.label}-input-schema-${index + 1}`}
                  autoSize={{ minRows: 1, maxRows: 4 }}
                  value={schemaText(tool.input_schema)}
                  onChange={(event) => {
                    const inputSchema = parseInputSchema(event.target.value);

                    if (inputSchema) {
                      updateTools(
                        replaceTool(tools, index, { input_schema: inputSchema })
                      );
                    }
                  }}
                />
              )
            },
            {
              title: i18nText('agentFlow', 'auto.connector_id'),
              dataIndex: 'connector_id',
              render: (_value, tool, index) => (
                <Input
                  aria-label={`${block.label}-connector-id-${index + 1}`}
                  value={tool.connector_id ?? tool.tool_name}
                  onChange={(event) =>
                    updateTools(
                      replaceTool(tools, index, {
                        connector_id: normalizeConnectorId(event.target.value)
                      })
                    )
                  }
                />
              )
            },
            {
              title: '',
              key: 'actions',
              render: (_value, _tool, index) => (
                <Button
                  aria-label={i18nText('agentFlow', 'auto.delete_tool')}
                  icon={<DeleteOutlined />}
                  type="text"
                  onClick={() =>
                    updateTools(
                      tools.filter((_, toolIndex) => toolIndex !== index)
                    )
                  }
                />
              )
            }
          ]}
        />
      ) : null}
      {enabled ? (
        <Button
          icon={<PlusOutlined />}
          onClick={() =>
            updateTools([...tools, buildNextTool(tools, targetNodes[0]?.id)])
          }
        >
          {i18nText('agentFlow', 'auto.add_tool_registration')}
        </Button>
      ) : null}
    </Space>
  );
}
