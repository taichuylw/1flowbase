import type { FlowNodeDocument } from '@1flowbase/flow-schema';
import { DeleteOutlined, EditOutlined, PlusOutlined } from '@ant-design/icons';
import { Button, Input, List, Switch, Typography } from 'antd';
import { useRef, useState } from 'react';

import type { SchemaFieldRendererProps } from '../../../../../shared/schema-ui/registry/create-renderer-registry';
import {
  DEFAULT_LLM_INTERNAL_LLM_NODE_POLICY,
  getLlmVisibleInternalTools,
  getLlmVisibleInternalToolsEnabled,
  isLlmToolIdentifier,
  type LlmInternalLlmNodePolicy,
  type LlmVisibleInternalTool
} from '../../../lib/llm-node-config';
import { i18nText } from '../../../../../shared/i18n/text';
import { FloatingSettingsPanel } from '../FloatingSettingsPanel';
import { JsonSchemaInlineEditor } from './json-schema/JsonSchemaSettingsPanel';
import { createDefaultJsonSchema } from './json-schema/json-schema-utils';

const TOOL_FORM_ROW_STYLE = {
  display: 'grid',
  gap: 6,
  color: '#31483a',
  fontSize: 13,
  fontWeight: 600
} as const;

const TOOL_FORM_ERROR_STYLE = {
  fontSize: 12,
  fontWeight: 400
} as const;

const TOOL_FORM_SWITCH_ROW_STYLE = {
  ...TOOL_FORM_ROW_STYLE,
  alignItems: 'center',
  gridTemplateColumns: 'minmax(0, 1fr) auto'
} as const;

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function getNodeConfig(node: FlowNodeDocument) {
  return isRecord(node.config) ? node.config : {};
}

function getCurrentNode(adapter: SchemaFieldRendererProps['adapter']) {
  return adapter.getDerived('node') as FlowNodeDocument | null;
}

function createToolName(index: number) {
  return `tool_${index + 1}`;
}

function createNextToolName(tools: LlmVisibleInternalTool[]) {
  const usedIdentifiers = new Set(
    tools.flatMap((tool) => [tool.tool_name, tool.connector_id ?? ''])
  );
  let index = 0;

  while (usedIdentifiers.has(createToolName(index))) {
    index += 1;
  }

  return createToolName(index);
}

function buildNextTool(
  tools: LlmVisibleInternalTool[]
): LlmVisibleInternalTool {
  const toolName = createNextToolName(tools);

  return {
    type: 'visible_internal_llm_tool',
    tool_name: toolName,
    connector_id: toolName,
    target_node_id: '',
    internal_llm_node_policy: DEFAULT_LLM_INTERNAL_LLM_NODE_POLICY,
    input_schema: { type: 'object' }
  };
}

interface LlmToolRegistrationDraft {
  tool_name: string;
  description: string;
  input_schema: Record<string, unknown>;
  connector_id: string;
  internal_llm_node_policy: LlmInternalLlmNodePolicy;
}

function draftFromTool(tool: LlmVisibleInternalTool): LlmToolRegistrationDraft {
  return {
    tool_name: tool.tool_name,
    description: tool.description ?? '',
    input_schema: isRecord(tool.input_schema)
      ? tool.input_schema
      : createDefaultJsonSchema(),
    connector_id: tool.connector_id ?? tool.tool_name,
    internal_llm_node_policy:
      tool.internal_llm_node_policy ?? DEFAULT_LLM_INTERNAL_LLM_NODE_POLICY
  };
}

function toolFromDraft(draft: LlmToolRegistrationDraft, targetNodeId: string) {
  const toolName = draft.tool_name.trim();
  const connectorId = draft.connector_id.trim();

  if (!isLlmToolIdentifier(toolName) || !isLlmToolIdentifier(connectorId)) {
    return null;
  }

  return {
    type: 'visible_internal_llm_tool' as const,
    tool_name: toolName,
    connector_id: connectorId,
    target_node_id: targetNodeId,
    description: draft.description.trim() || undefined,
    internal_llm_node_policy: draft.internal_llm_node_policy,
    input_schema: draft.input_schema
  };
}

function identifierError(
  value: string,
  existingIdentifiers: Set<string>
): string | null {
  const trimmedValue = value.trim();

  if (!isLlmToolIdentifier(trimmedValue)) {
    return i18nText('agentFlow', 'auto.tool_identifier_rule');
  }

  if (existingIdentifiers.has(trimmedValue)) {
    return i18nText('agentFlow', 'auto.tool_identifier_duplicate');
  }

  return null;
}

export function LlmToolRegistrationsField({
  adapter,
  block
}: SchemaFieldRendererProps) {
  const [editingIndex, setEditingIndex] = useState<number | null>(null);
  const [draft, setDraft] = useState<LlmToolRegistrationDraft | null>(null);
  const [schemaEditorValid, setSchemaEditorValid] = useState(true);
  const toolEditorTriggerRef = useRef<HTMLElement | null>(null);
  const currentNode = getCurrentNode(adapter);

  if (!currentNode) {
    return null;
  }

  const currentConfig = getNodeConfig(currentNode);
  const enabled = getLlmVisibleInternalToolsEnabled(currentConfig);
  const tools = getLlmVisibleInternalTools(currentConfig);
  const existingToolNames = new Set(
    tools.flatMap((tool, index) =>
      index === editingIndex ? [] : [tool.tool_name]
    )
  );
  const existingConnectorIds = new Set(
    tools.flatMap((tool, index) =>
      index === editingIndex ? [] : [tool.connector_id || tool.tool_name]
    )
  );
  const toolNameError = draft
    ? identifierError(draft.tool_name, existingToolNames)
    : null;
  const connectorIdError = draft
    ? identifierError(draft.connector_id, existingConnectorIds)
    : null;
  const toolEditorValid =
    draft !== null && !toolNameError && !connectorIdError && schemaEditorValid;

  function updateTools(nextTools: LlmVisibleInternalTool[]) {
    adapter.setValue('config.visible_internal_llm_tools', nextTools);
  }

  function openToolEditor(
    index: number | null,
    tool: LlmVisibleInternalTool,
    trigger: HTMLElement | null
  ) {
    toolEditorTriggerRef.current = trigger;
    setEditingIndex(index);
    setDraft(draftFromTool(tool));
    setSchemaEditorValid(true);
  }

  function closeToolEditor() {
    setEditingIndex(null);
    setDraft(null);
    setSchemaEditorValid(true);
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
    const currentTargetNodeId =
      editingIndex === null ? '' : (tools[editingIndex]?.target_node_id ?? '');
    const nextTool = toolFromDraft(draft, currentTargetNodeId);

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

  const modalTitle = i18nText('agentFlow', 'auto.edit', {
    value1: i18nText('agentFlow', 'auto.tool_registration')
  });
  const addToolLabel = i18nText('agentFlow', 'auto.add_tool_registration');
  const toolEditorFooter = (
    <div className="agent-flow-llm-tool-registration-panel__footer">
      <Button onClick={closeToolEditor}>
        {i18nText('agentFlow', 'auto.cancel')}
      </Button>
      <Button disabled={!toolEditorValid} type="primary" onClick={saveDraft}>
        {i18nText('agentFlow', 'auto.save_tool')}
      </Button>
    </div>
  );

  return (
    <div className="agent-flow-llm-tool-registrations">
      <div
        className="agent-flow-llm-tool-registrations__toolbar"
        data-testid="agent-flow-llm-tool-registrations-toolbar"
      >
        <Typography.Text
          strong
          className="agent-flow-llm-tool-registrations__label"
        >
          {block.label}
        </Typography.Text>
        <Button
          aria-label={addToolLabel}
          className="agent-flow-llm-tool-registrations__add"
          disabled={!enabled}
          icon={
            <PlusOutlined data-testid="agent-flow-llm-tool-registration-add-icon" />
          }
          shape="circle"
          size="small"
          type="text"
          onClick={(event) =>
            openToolEditor(null, buildNextTool(tools), event.currentTarget)
          }
        />
        <Switch
          aria-label={block.label}
          checked={enabled}
          className="agent-flow-llm-tool-registrations__switch"
          onChange={(checked) =>
            adapter.setValue(
              'config.visible_internal_llm_tools_enabled',
              checked
            )
          }
        />
      </div>
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
                    onClick={(event) =>
                      openToolEditor(index, tool, event.currentTarget)
                    }
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
      <FloatingSettingsPanel
        className="agent-flow-llm-tool-registration-panel"
        closeLabel={i18nText('agentFlow', 'auto.close', {
          value1: i18nText('agentFlow', 'auto.tool_registration')
        })}
        defaultWidth={720}
        footer={toolEditorFooter}
        initialHeight={520}
        minHeight={360}
        minWidth={560}
        open={draft !== null}
        title={modalTitle}
        triggerRef={toolEditorTriggerRef}
        onClose={closeToolEditor}
      >
        {draft ? (
          <form className="agent-flow-llm-tool-registration-form">
            <label style={TOOL_FORM_ROW_STYLE}>
              <span>{i18nText('agentFlow', 'auto.tool_name')}</span>
              <Input
                aria-label={i18nText('agentFlow', 'auto.tool_name')}
                status={toolNameError ? 'error' : undefined}
                value={draft.tool_name}
                onChange={(event) =>
                  updateDraft({
                    tool_name: event.target.value,
                    connector_id: draft.connector_id || event.target.value
                  })
                }
              />
              {toolNameError ? (
                <Typography.Text type="danger" style={TOOL_FORM_ERROR_STYLE}>
                  {toolNameError}
                </Typography.Text>
              ) : null}
            </label>
            <label style={TOOL_FORM_ROW_STYLE}>
              <span>{i18nText('agentFlow', 'auto.tool_identifier')}</span>
              <Input
                aria-label={i18nText('agentFlow', 'auto.tool_identifier')}
                status={connectorIdError ? 'error' : undefined}
                value={draft.connector_id}
                onChange={(event) =>
                  updateDraft({ connector_id: event.target.value })
                }
              />
              {connectorIdError ? (
                <Typography.Text type="danger" style={TOOL_FORM_ERROR_STYLE}>
                  {connectorIdError}
                </Typography.Text>
              ) : null}
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
            <div style={TOOL_FORM_SWITCH_ROW_STYLE}>
              <span>
                {i18nText('agentFlow', 'auto.internal_llm_node_policy')}
              </span>
              <Switch
                aria-label={i18nText(
                  'agentFlow',
                  'auto.internal_llm_node_policy'
                )}
                checked={draft.internal_llm_node_policy === 'allowed'}
                onChange={(checked) =>
                  updateDraft({
                    internal_llm_node_policy: checked ? 'allowed' : 'forbidden'
                  })
                }
              />
            </div>
            <div style={TOOL_FORM_ROW_STYLE}>
              <span>{i18nText('agentFlow', 'auto.input_parameters')}</span>
              <div className="agent-flow-llm-tool-registration-schema">
                <JsonSchemaInlineEditor
                  resetKey={editingIndex ?? 'new'}
                  schema={draft.input_schema}
                  onChange={(schema) => updateDraft({ input_schema: schema })}
                  onValidityChange={setSchemaEditorValid}
                />
              </div>
            </div>
          </form>
        ) : null}
      </FloatingSettingsPanel>
    </div>
  );
}
