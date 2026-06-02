import { DeleteOutlined, PlusOutlined } from '@ant-design/icons';
import type { editor } from 'monaco-editor';
import { Button, Checkbox, Empty, Input, Select, Tabs, Typography } from 'antd';
import { Suspense, lazy, useMemo, useRef, useState } from 'react';

import type { FlowNodeDocument } from '@1flowbase/flow-schema';
import {
  outputTypeSupportsJsonSchema,
  parseJsonSchemaInput
} from '../../../lib/output-contract/schema';
import { isOutputVariableKeyAllowed } from '../../../lib/output-contract/variable-key';
import { FloatingSettingsPanel } from '../FloatingSettingsPanel';
import { i18nText } from '../../../../../shared/i18n/text';

const MonacoEditor = lazy(() => import('@monaco-editor/react'));

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

type SchemaEditorTab = 'fields' | 'json';
type SchemaFieldType = 'string' | 'number' | 'boolean' | 'object' | 'array';
type SchemaRootType = 'object' | 'array';

const schemaFieldTypeOptions = [
  { value: 'string', label: 'String' },
  { value: 'number', label: 'Number' },
  { value: 'boolean', label: 'Boolean' },
  { value: 'object', label: 'Object' },
  { value: 'array', label: 'Array' }
] satisfies Array<{ value: SchemaFieldType; label: string }>;

interface SchemaFieldRow {
  key: string;
  type: SchemaFieldType;
  required: boolean;
}

const JSON_SCHEMA_EDITOR_OPTIONS = {
  automaticLayout: true,
  minimap: { enabled: false },
  fontSize: 13,
  lineHeight: 20,
  lineNumbersMinChars: 3,
  scrollBeyondLastLine: false,
  tabSize: 2,
  wordWrap: 'on',
  padding: {
    top: 12,
    bottom: 12
  },
  scrollbar: {
    verticalScrollbarSize: 8,
    horizontalScrollbarSize: 8
  }
} satisfies editor.IStandaloneEditorConstructionOptions;

function createNextOutput(
  index: number,
  selectorForKey?: (key: string) => string[] | undefined
): FlowNodeDocument['outputs'][number] {
  const key = `output_${index + 1}`;

  return {
    key,
    title: key,
    valueType: 'string',
    selector: selectorForKey?.(key)
  };
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function stringifySchema(schema: Record<string, unknown>) {
  return JSON.stringify(schema, null, 2);
}

function fallbackSchemaRootType(
  output: FlowNodeDocument['outputs'][number]
): SchemaRootType {
  return output.valueType === 'array' ? 'array' : 'object';
}

function schemaRootType(
  schema: Record<string, unknown>,
  fallback: SchemaRootType
): SchemaRootType {
  if (schema.type === 'array' || schema.type === 'object') {
    return schema.type;
  }

  return fallback;
}

function defaultSchemaForOutput(
  output: FlowNodeDocument['outputs'][number]
): Record<string, unknown> {
  if (isRecord(output.jsonSchema)) {
    return output.jsonSchema;
  }

  if (output.valueType === 'array') {
    return {
      type: 'array',
      items: {
        type: 'object',
        required: [],
        properties: {}
      }
    };
  }

  return {
    type: 'object',
    required: [],
    properties: {}
  };
}

function schemaFieldType(value: unknown): SchemaFieldType {
  return schemaFieldTypeOptions.some((option) => option.value === value)
    ? (value as SchemaFieldType)
    : 'string';
}

function propertySchemaForType(type: SchemaFieldType): Record<string, unknown> {
  if (type === 'object') {
    return { type: 'object', properties: {} };
  }

  if (type === 'array') {
    return { type: 'array', items: {} };
  }

  return { type };
}

function schemaRowsFromSchema(schema: unknown): SchemaFieldRow[] {
  if (!isRecord(schema)) {
    return [];
  }

  const root =
    schema.type === 'array' && isRecord(schema.items) ? schema.items : schema;
  if (!isRecord(root)) {
    return [];
  }

  const properties = isRecord(root.properties) ? root.properties : {};
  const required = Array.isArray(root.required)
    ? root.required.filter((entry): entry is string => typeof entry === 'string')
    : [];

  return Object.entries(properties).map(([key, property]) => ({
    key,
    type: schemaFieldType(isRecord(property) ? property.type : undefined),
    required: required.includes(key)
  }));
}

function schemaFromRows(
  rootType: SchemaRootType,
  rows: SchemaFieldRow[]
): Record<string, unknown> {
  const visibleRows = rows
    .map((row) => ({
      ...row,
      key: row.key.trim()
    }))
    .filter((row) => row.key.length > 0);
  const properties = Object.fromEntries(
    visibleRows.map((row) => [row.key, propertySchemaForType(row.type)])
  );
  const required = visibleRows
    .filter((row) => row.required)
    .map((row) => row.key);
  const objectSchema = {
    type: 'object',
    required,
    properties
  };

  if (rootType === 'array') {
    return {
      type: 'array',
      items: objectSchema
    };
  }

  return objectSchema;
}

function JsonSchemaEditorFallback() {
  return (
    <div className="agent-flow-json-schema-settings__editor-loading">
      {i18nText("agentFlow", "auto.loading_json_schema_editor")}
    </div>
  );
}

function JsonSchemaCodeEditor({
  value,
  onChange
}: {
  value: string;
  onChange: (value: string) => void;
}) {
  const options = useMemo(
    () => ({
      ...JSON_SCHEMA_EDITOR_OPTIONS,
      ariaLabel: i18nText("agentFlow", "auto.json_schema_content")
    }),
    []
  );

  return (
    <div className="agent-flow-json-schema-settings__editor">
      <Suspense fallback={<JsonSchemaEditorFallback />}>
        <MonacoEditor
          defaultLanguage="json"
          height="100%"
          language="json"
          options={options}
          theme="vs"
          value={value}
          onChange={(nextValue) => onChange(nextValue ?? '')}
        />
      </Suspense>
    </div>
  );
}

export function OutputContractDefinitionField({
  value,
  onChange,
  syncTitleWithKey = false,
  selectorForKey
}: {
  value: FlowNodeDocument['outputs'];
  onChange: (value: FlowNodeDocument['outputs']) => void;
  syncTitleWithKey?: boolean;
  selectorForKey?: (key: string) => string[] | undefined;
}) {
  const [editingIndex, setEditingIndex] = useState<number | null>(null);
  const [schemaText, setSchemaText] = useState('');
  const [schemaRows, setSchemaRows] = useState<SchemaFieldRow[]>([]);
  const [schemaRoot, setSchemaRoot] = useState<SchemaRootType>('object');
  const [schemaTab, setSchemaTab] = useState<SchemaEditorTab>('fields');
  const [schemaError, setSchemaError] = useState<string | null>(null);
  const schemaTriggerRef = useRef<HTMLElement | null>(null);
  const editingOutput =
    editingIndex === null ? null : value[editingIndex] ?? null;

  function openSchemaEditor(index: number, trigger: HTMLElement | null) {
    const output = value[index];
    const schema = defaultSchemaForOutput(output);

    schemaTriggerRef.current = trigger;
    setEditingIndex(index);
    setSchemaText(stringifySchema(schema));
    setSchemaRows(schemaRowsFromSchema(schema));
    setSchemaRoot(schemaRootType(schema, fallbackSchemaRootType(output)));
    setSchemaTab('fields');
    setSchemaError(null);
  }

  function closeSchemaEditor() {
    setEditingIndex(null);
    setSchemaText('');
    setSchemaRows([]);
    setSchemaRoot('object');
    setSchemaTab('fields');
    setSchemaError(null);
  }

  function emitChange(nextValue: FlowNodeDocument['outputs']) {
    onChange(
      nextValue.map((output) => ({
        ...output,
        title: syncTitleWithKey ? output.key : output.title,
        selector: selectorForKey ? selectorForKey(output.key) : output.selector
      }))
    );
  }

  function updateSchemaRow(index: number, patch: Partial<SchemaFieldRow>) {
    setSchemaRows((current) =>
      current.map((row, rowIndex) =>
        rowIndex === index ? { ...row, ...patch } : row
      )
    );
  }

  function addSchemaRow() {
    setSchemaRows((current) => [
      ...current,
      { key: `field_${current.length + 1}`, type: 'string', required: true }
    ]);
  }

  function removeSchemaRow(index: number) {
    setSchemaRows((current) =>
      current.filter((_, rowIndex) => rowIndex !== index)
    );
  }

  function switchSchemaTab(nextTab: string) {
    if (nextTab === 'json') {
      const schema = schemaFromRows(schemaRoot, schemaRows);

      setSchemaText(stringifySchema(schema));
      setSchemaTab('json');
      setSchemaError(null);
      return;
    }

    const parsed = parseJsonSchemaInput(schemaText);
    if (!parsed.ok) {
      setSchemaError(parsed.message);
      return;
    }

    setSchemaRows(schemaRowsFromSchema(parsed.schema));
    setSchemaRoot(schemaRootType(parsed.schema, schemaRoot));
    setSchemaTab('fields');
    setSchemaError(null);
  }

  function saveSchema() {
    if (editingIndex === null || !editingOutput) {
      return;
    }

    const parsed =
      schemaTab === 'fields'
        ? {
            ok: true as const,
            schema: schemaFromRows(schemaRoot, schemaRows)
          }
        : parseJsonSchemaInput(schemaText);

    if (!parsed.ok) {
      setSchemaError(parsed.message);
      return;
    }

    const nextType = parsed.schema.type;
    emitChange(
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

  const schemaPanelFooter = (
    <div className="agent-flow-json-schema-settings__footer">
      <Button
        aria-label={i18nText("agentFlow", "auto.cancel")}
        onClick={closeSchemaEditor}
      >
        {i18nText("agentFlow", "auto.cancel")}
      </Button>
      <Button
        aria-label={i18nText("agentFlow", "auto.save")}
        type="primary"
        onClick={saveSchema}
      >
        {i18nText("agentFlow", "auto.save")}
      </Button>
    </div>
  );

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
          onClick={() =>
            emitChange([...value, createNextOutput(value.length, selectorForKey)])
          }
        />
      </div>
      {value.length > 0 ? (
        <div className="agent-flow-output-contract-editor__list">
          {value.map((output, index) => {
            const outputKeyIsValid =
              output.key.length === 0 ||
              isOutputVariableKeyAllowed(output.key);

            return (
              <div
                key={`${output.key}-${index}`}
                className={`agent-flow-output-contract-editor__row${
                  syncTitleWithKey
                    ? ' agent-flow-output-contract-editor__row--synced-title'
                    : ''
                }`}
              >
                <label className="agent-flow-output-contract-editor__cell">
                  <span>{i18nText("agentFlow", "auto.variable_name")}</span>
                  <Input
                    aria-label={i18nText("agentFlow", "auto.output_variable_name", { value1: index + 1 })}
                    status={outputKeyIsValid ? undefined : 'error'}
                    value={output.key}
                    onChange={(event) =>
                      emitChange(
                        value.map((candidate, candidateIndex) =>
                          candidateIndex === index
                            ? {
                                ...candidate,
                                key: event.target.value,
                                title: syncTitleWithKey
                                  ? event.target.value
                                  : candidate.title
                              }
                            : candidate
                        )
                      )
                    }
                  />
                  {!outputKeyIsValid ? (
                    <Typography.Text type="danger">
                      {i18nText("agentFlow", "auto.output_variable_name_format_hint")}
                    </Typography.Text>
                  ) : null}
                </label>
                {!syncTitleWithKey ? (
                  <label className="agent-flow-output-contract-editor__cell">
                    <span>{i18nText("agentFlow", "auto.display_name")}</span>
                    <Input
                      aria-label={i18nText("agentFlow", "auto.output_display_name", { value1: index + 1 })}
                      value={output.title}
                      onChange={(event) =>
                        emitChange(
                          value.map((candidate, candidateIndex) =>
                            candidateIndex === index
                              ? { ...candidate, title: event.target.value }
                              : candidate
                          )
                        )
                      }
                    />
                  </label>
                ) : null}
                <label className="agent-flow-output-contract-editor__cell">
                  <span>{i18nText("agentFlow", "auto.type")}</span>
                  <Select
                    aria-label={i18nText("agentFlow", "auto.output_type", { value1: index + 1 })}
                    options={valueTypeOptions}
                    value={output.valueType}
                    onChange={(valueType) =>
                      emitChange(
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
                    onClick={(event) =>
                      openSchemaEditor(index, event.currentTarget)
                    }
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
                    emitChange(
                      value.filter((_, outputIndex) => outputIndex !== index)
                    )
                  }
                />
              </div>
            );
          })}
        </div>
      ) : (
        <Empty
          image={Empty.PRESENTED_IMAGE_SIMPLE}
          description={i18nText("agentFlow", "auto.output_variables_yet")}
        />
      )}
      <FloatingSettingsPanel
        className="agent-flow-json-schema-settings__panel"
        closeLabel={i18nText("agentFlow", "auto.close_json_schema_settings")}
        defaultWidth={620}
        initialHeight={560}
        minHeight={420}
        minWidth={520}
        open={editingOutput !== null}
        title="JSON Schema"
        triggerRef={schemaTriggerRef}
        footer={schemaPanelFooter}
        dragHandleTestId="agent-flow-json-schema-settings-drag-handle"
        leftResizeHandleTestId="agent-flow-json-schema-settings-resize-left"
        rightResizeHandleTestId="agent-flow-json-schema-settings-resize-right"
        bottomResizeHandleTestId="agent-flow-json-schema-settings-resize-bottom"
        onClose={closeSchemaEditor}
      >
        <div className="agent-flow-json-schema-settings">
          <Tabs
            activeKey={schemaTab}
            onChange={switchSchemaTab}
            items={[
              {
                key: 'fields',
                label: i18nText("agentFlow", "auto.schema_fields"),
                children: (
                  <div className="agent-flow-json-schema-settings__fields">
                    <div className="agent-flow-json-schema-settings__field-head">
                      <span>{i18nText("agentFlow", "auto.field_name")}</span>
                      <span>{i18nText("agentFlow", "auto.type")}</span>
                      <span>{i18nText("agentFlow", "auto.required")}</span>
                      <span>{i18nText("agentFlow", "auto.operation")}</span>
                    </div>
                    <div className="agent-flow-json-schema-settings__field-rows">
                      {schemaRows.map((row, index) => (
                        <div
                          className="agent-flow-json-schema-settings__field-row"
                          key={`${row.key}-${index}`}
                        >
                          <Input
                            aria-label={i18nText(
                              "agentFlow",
                              "auto.schema_field_name",
                              { value1: index + 1 }
                            )}
                            value={row.key}
                            onChange={(event) =>
                              updateSchemaRow(index, {
                                key: event.target.value
                              })
                            }
                          />
                          <Select
                            aria-label={i18nText(
                              "agentFlow",
                              "auto.schema_field_type",
                              { value1: index + 1 }
                            )}
                            options={schemaFieldTypeOptions}
                            value={row.type}
                            onChange={(type) =>
                              updateSchemaRow(index, { type })
                            }
                          />
                          <Checkbox
                            aria-label={i18nText(
                              "agentFlow",
                              "auto.schema_field_required",
                              { value1: index + 1 }
                            )}
                            checked={row.required}
                            onChange={(event) =>
                              updateSchemaRow(index, {
                                required: event.target.checked
                              })
                            }
                          />
                          <Button
                            aria-label={i18nText(
                              "agentFlow",
                              "auto.delete_schema_field",
                              { value1: row.key || index + 1 }
                            )}
                            danger
                            icon={<DeleteOutlined />}
                            size="small"
                            type="text"
                            onClick={() => removeSchemaRow(index)}
                          />
                        </div>
                      ))}
                    </div>
                    <Button
                      aria-label={i18nText("agentFlow", "auto.add_schema_field")}
                      className="agent-flow-json-schema-settings__add-field"
                      icon={<PlusOutlined />}
                      type="dashed"
                      onClick={addSchemaRow}
                    >
                      {i18nText("agentFlow", "auto.add_schema_field")}
                    </Button>
                  </div>
                )
              },
              {
                key: 'json',
                label: i18nText("agentFlow", "auto.json_parse"),
                children: (
                  <JsonSchemaCodeEditor
                    value={schemaText}
                    onChange={(nextValue) => {
                      setSchemaText(nextValue);
                      setSchemaError(null);
                    }}
                  />
                )
              }
            ]}
          />
          <Typography.Text type={schemaError ? 'danger' : 'secondary'}>
            {schemaError ?? i18nText("agentFlow", "auto.json_schema_parse_hint")}
          </Typography.Text>
        </div>
      </FloatingSettingsPanel>
    </div>
  );
}
