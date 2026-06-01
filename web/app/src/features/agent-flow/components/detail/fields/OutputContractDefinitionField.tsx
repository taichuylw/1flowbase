import { DeleteOutlined, PlusOutlined } from '@ant-design/icons';
import { Button, Empty, Input, Modal, Select, Space, Typography } from 'antd';
import { useState } from 'react';

import type { FlowNodeDocument } from '@1flowbase/flow-schema';
import {
  outputTypeSupportsJsonSchema,
  parseJsonSchemaInput
} from '../../../lib/output-contract/schema';
import { i18nText } from '../../../../../shared/i18n/text';

const valueTypeOptions = [
  { value: 'string', label: 'String' },
  { value: 'number', label: 'Number' },
  { value: 'boolean', label: 'Boolean' },
  { value: 'object', label: 'Object' },
  { value: 'array', label: 'Array' },
  { value: 'json', label: 'JSON' },
  { value: 'unknown', label: 'Unknown' }
] satisfies Array<{
  value: FlowNodeDocument['outputs'][number]['valueType'];
  label: string;
}>;

function createNextOutput(index: number): FlowNodeDocument['outputs'][number] {
  const key = `output_${index + 1}`;

  return {
    key,
    title: key,
    valueType: 'string'
  };
}

export function OutputContractDefinitionField({
  value,
  onChange
}: {
  value: FlowNodeDocument['outputs'];
  onChange: (value: FlowNodeDocument['outputs']) => void;
}) {
  const [editingIndex, setEditingIndex] = useState<number | null>(null);
  const [schemaText, setSchemaText] = useState('');
  const [schemaError, setSchemaError] = useState<string | null>(null);
  const editingOutput =
    editingIndex === null ? null : value[editingIndex] ?? null;

  function openSchemaEditor(index: number) {
    const output = value[index];
    setEditingIndex(index);
    setSchemaText(JSON.stringify(output?.jsonSchema ?? {}, null, 2));
    setSchemaError(null);
  }

  function closeSchemaEditor() {
    setEditingIndex(null);
    setSchemaText('');
    setSchemaError(null);
  }

  function saveSchema() {
    if (editingIndex === null) {
      return;
    }

    const parsed = parseJsonSchemaInput(schemaText);
    if (!parsed.ok) {
      setSchemaError(parsed.message);
      return;
    }

    const nextType = parsed.schema.type;
    onChange(
      value.map((candidate, candidateIndex) => {
        if (candidateIndex !== editingIndex) {
          return candidate;
        }

        return {
          ...candidate,
          valueType:
            nextType === 'object' || nextType === 'array'
              ? nextType
              : candidate.valueType,
          jsonSchema: parsed.schema
        };
      })
    );
    closeSchemaEditor();
  }

  return (
    <div className="agent-flow-output-contract-editor">
      <div className="agent-flow-output-contract-editor__header">
        <Typography.Text className="agent-flow-node-detail__section-subtitle">
          {i18nText("agentFlow", "auto.variables_produced_nodes_referenced_downstream_nodes")}</Typography.Text>
        <Button
          aria-label={i18nText("agentFlow", "auto.add_new_output_variable")}
          icon={<PlusOutlined />}
          size="small"
          type="text"
          onClick={() => onChange([...value, createNextOutput(value.length)])}
        />
      </div>
      {value.length > 0 ? (
        <div className="agent-flow-output-contract-editor__list">
          {value.map((output, index) => (
            <div
              key={`${output.key}-${index}`}
              className="agent-flow-output-contract-editor__row"
            >
              <label className="agent-flow-output-contract-editor__cell">
                <span>{i18nText("agentFlow", "auto.variable_name")}</span>
                <Input
                  aria-label={i18nText("agentFlow", "auto.output_variable_name", { value1: index + 1 })}
                  value={output.key}
                  onChange={(event) =>
                    onChange(
                      value.map((candidate, candidateIndex) =>
                        candidateIndex === index
                          ? { ...candidate, key: event.target.value }
                          : candidate
                      )
                    )
                  }
                />
              </label>
              <label className="agent-flow-output-contract-editor__cell">
                <span>{i18nText("agentFlow", "auto.display_name")}</span>
                <Input
                  aria-label={i18nText("agentFlow", "auto.output_display_name", { value1: index + 1 })}
                  value={output.title}
                  onChange={(event) =>
                    onChange(
                      value.map((candidate, candidateIndex) =>
                        candidateIndex === index
                          ? { ...candidate, title: event.target.value }
                          : candidate
                      )
                    )
                  }
                />
              </label>
              <label className="agent-flow-output-contract-editor__cell">
                <span>{i18nText("agentFlow", "auto.type")}</span>
                <Select
                  aria-label={i18nText("agentFlow", "auto.output_type", { value1: index + 1 })}
                  options={valueTypeOptions}
                  value={output.valueType}
                  onChange={(valueType) =>
                    onChange(
                      value.map((candidate, candidateIndex) =>
                        candidateIndex === index
                          ? {
                              ...candidate,
                              valueType,
                              jsonSchema: outputTypeSupportsJsonSchema(valueType)
                                ? candidate.jsonSchema
                                : undefined
                            }
                          : candidate
                      )
                    )
                  }
                />
              </label>
              {outputTypeSupportsJsonSchema(output.valueType) ? (
                <Button
                  aria-label="编辑 JSON Schema"
                  className="agent-flow-output-contract-editor__schema"
                  size="small"
                  type={output.jsonSchema ? 'primary' : 'default'}
                  onClick={() => openSchemaEditor(index)}
                >
                  Schema
                </Button>
              ) : null}
              <Button
                aria-label={i18nText("agentFlow", "auto.delete_output_variable", { value1: output.key || index + 1 })}
                className="agent-flow-output-contract-editor__delete"
                danger
                icon={<DeleteOutlined />}
                size="small"
                type="text"
                onClick={() =>
                  onChange(
                    value.filter((_, outputIndex) => outputIndex !== index)
                  )
                }
              />
            </div>
          ))}
        </div>
      ) : (
        <Empty
          image={Empty.PRESENTED_IMAGE_SIMPLE}
          description={i18nText("agentFlow", "auto.output_variables_yet")}
        />
      )}
      <Modal
        destroyOnHidden
        okText="保存"
        open={editingOutput !== null}
        title="JSON Schema"
        onCancel={closeSchemaEditor}
        onOk={saveSchema}
      >
        <Space direction="vertical" size={8} style={{ width: '100%' }}>
          <Input.TextArea
            aria-label="JSON Schema"
            autoSize={{ minRows: 10, maxRows: 18 }}
            value={schemaText}
            onChange={(event) => {
              setSchemaText(event.target.value);
              setSchemaError(null);
            }}
          />
          <Typography.Text type={schemaError ? 'danger' : 'secondary'}>
            {schemaError ?? '支持识别 JSON、JSON Schema 等描述协议'}
          </Typography.Text>
        </Space>
      </Modal>
    </div>
  );
}
