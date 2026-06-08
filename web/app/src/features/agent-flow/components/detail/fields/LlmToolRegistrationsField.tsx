import type {
  FlowAuthoringDocument,
  FlowNodeDocument
} from '@1flowbase/flow-schema';
import { DeleteOutlined, EditOutlined, PlusOutlined } from '@ant-design/icons';
import { Button, Input, List, Modal, Select, Space, Switch } from 'antd';
import { useState } from 'react';

import type { SchemaFieldRendererProps } from '../../../../../shared/schema-ui/registry/create-renderer-registry';
import {
  getLlmVisibleInternalTools,
  getLlmVisibleInternalToolsEnabled,
  type LlmVisibleInternalTool
} from '../../../lib/llm-node-config';
import { i18nText } from '../../../../../shared/i18n/text';

const TOOL_FORM_ROW_STYLE = {
  display: 'grid',
  gap: 6,
  color: '#31483a',
  fontSize: 13,
  fontWeight: 600
} as const;

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

interface LlmToolRegistrationDraft {
  tool_name: string;
  target_node_id: string;
  description: string;
  input_schema_text: string;
  connector_id: string;
}

function draftFromTool(tool: LlmVisibleInternalTool): LlmToolRegistrationDraft {
  return {
    tool_name: tool.tool_name,
    target_node_id: tool.target_node_id,
    description: tool.description ?? '',
    input_schema_text: schemaText(tool.input_schema),
    connector_id: tool.connector_id ?? tool.tool_name
  };
}

function toolFromDraft(draft: LlmToolRegistrationDraft) {
  const toolName = draft.tool_name.trim();
  const inputSchema = draft.input_schema_text.trim()
    ? parseInputSchema(draft.input_schema_text)
    : { type: 'object' };

  if (!toolName || !inputSchema) {
    return null;
  }

  return {
    type: 'visible_internal_llm_tool' as const,
    tool_name: toolName,
    connector_id: normalizeConnectorId(draft.connector_id.trim() || toolName),
    target_node_id: draft.target_node_id,
    description: draft.description.trim() || undefined,
    input_schema: inputSchema
  };
}

export function LlmToolRegistrationsField({
  adapter,
  block
}: SchemaFieldRendererProps) {
  const [editingIndex, setEditingIndex] = useState<number | null>(null);
  const [draft, setDraft] = useState<LlmToolRegistrationDraft | null>(null);
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

  function openToolEditor(index: number | null, tool: LlmVisibleInternalTool) {
    setEditingIndex(index);
    setDraft(draftFromTool(tool));
  }

  function closeToolEditor() {
    setEditingIndex(null);
    setDraft(null);
  }

  function updateDraft(patch: Partial<LlmToolRegistrationDraft>) {
    setDraft((currentDraft) =>
      currentDraft
        ? {
            ...currentDraft,
            ...patch
          }
        : currentDraft
    );
  }

  function saveDraft() {
    if (!draft) {
      return;
    }
    const nextTool = toolFromDraft(draft);

    if (!nextTool) {
      return;
    }

    if (editingIndex === null) {
      updateTools([...tools, nextTool]);
    } else {
      updateTools(
        tools.map((tool, toolIndex) =>
          toolIndex === editingIndex ? nextTool : tool
        )
      );
    }
    closeToolEditor();
  }

  const schemaInvalid =
    draft?.input_schema_text.trim() &&
    parseInputSchema(draft.input_schema_text) === undefined;
  const modalTitle = i18nText('agentFlow', 'auto.edit', {
    value1: i18nText('agentFlow', 'auto.tool_registration')
  });

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
        <List
          aria-label={i18nText('agentFlow', 'auto.tool_registration')}
          bordered
          dataSource={tools}
          locale={{
            emptyText: i18nText('agentFlow', 'auto.no_tool_registrations')
          }}
          renderItem={(tool, index) => {
            const toolName = tool.tool_name || createToolName(index);

            return (
              <List.Item
                actions={[
                  <Button
                    aria-label={i18nText('agentFlow', 'auto.edit', {
                      value1: toolName
                    })}
                    icon={<EditOutlined />}
                    key="edit"
                    size="small"
                    type="text"
                    onClick={() => openToolEditor(index, tool)}
                  />,
                  <Button
                    aria-label={i18nText('agentFlow', 'auto.delete_item', {
                      value1: toolName
                    })}
                    danger
                    icon={<DeleteOutlined />}
                    key="delete"
                    size="small"
                    type="text"
                    onClick={() =>
                      updateTools(
                        tools.filter((_, toolIndex) => toolIndex !== index)
                      )
                    }
                  />
                ]}
              >
                {toolName}
              </List.Item>
            );
          }}
          rowKey={(tool) => tool.connector_id || tool.tool_name}
          size="small"
        />
      ) : null}
      {enabled ? (
        <Button
          icon={<PlusOutlined />}
          onClick={() =>
            openToolEditor(null, buildNextTool(tools, targetNodes[0]?.id))
          }
        >
          {i18nText('agentFlow', 'auto.add_tool_registration')}
        </Button>
      ) : null}
      <Modal
        destroyOnHidden
        okButtonProps={{
          disabled: !draft?.tool_name.trim() || Boolean(schemaInvalid)
        }}
        okText={i18nText('agentFlow', 'auto.save_tool')}
        open={draft !== null}
        title={modalTitle}
        onCancel={closeToolEditor}
        onOk={saveDraft}
      >
        {draft ? (
          <form style={{ display: 'grid', gap: 14 }}>
            <label style={TOOL_FORM_ROW_STYLE}>
              <span>{i18nText('agentFlow', 'auto.tool_name')}</span>
              <Input
                aria-label={i18nText('agentFlow', 'auto.tool_name')}
                value={draft.tool_name}
                onChange={(event) =>
                  updateDraft({
                    tool_name: event.target.value,
                    connector_id:
                      draft.connector_id ||
                      normalizeConnectorId(event.target.value)
                  })
                }
              />
            </label>
            <label style={TOOL_FORM_ROW_STYLE}>
              <span>{i18nText('agentFlow', 'auto.target_llm')}</span>
              <Select
                aria-label={i18nText('agentFlow', 'auto.target_llm')}
                options={targetOptions}
                value={draft.target_node_id || undefined}
                onChange={(targetNodeId) =>
                  updateDraft({ target_node_id: targetNodeId })
                }
              />
            </label>
            <label style={TOOL_FORM_ROW_STYLE}>
              <span>{i18nText('agentFlow', 'auto.description')}</span>
              <Input
                aria-label={i18nText('agentFlow', 'auto.description')}
                value={draft.description}
                onChange={(event) =>
                  updateDraft({ description: event.target.value })
                }
              />
            </label>
            <label style={TOOL_FORM_ROW_STYLE}>
              <span>{i18nText('agentFlow', 'auto.input_schema')}</span>
              <Input.TextArea
                aria-label={i18nText('agentFlow', 'auto.input_schema')}
                autoSize={{ minRows: 4, maxRows: 8 }}
                status={schemaInvalid ? 'error' : undefined}
                value={draft.input_schema_text}
                onChange={(event) =>
                  updateDraft({ input_schema_text: event.target.value })
                }
              />
            </label>
            <label style={TOOL_FORM_ROW_STYLE}>
              <span>{i18nText('agentFlow', 'auto.connector_id')}</span>
              <Input
                aria-label={i18nText('agentFlow', 'auto.connector_id')}
                value={draft.connector_id}
                onChange={(event) =>
                  updateDraft({
                    connector_id: normalizeConnectorId(event.target.value)
                  })
                }
              />
            </label>
          </form>
        ) : null}
      </Modal>
    </Space>
  );
}
