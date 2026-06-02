import { DeleteOutlined, FileTextOutlined, PlusOutlined } from '@ant-design/icons';
import type { editor } from 'monaco-editor';
import {
  Button,
  Checkbox,
  Empty,
  Input,
  Select,
  Tabs,
  Tooltip,
  Typography
} from 'antd';
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
  children?: SchemaFieldRow[];
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

function schemaRowsFromObjectSchema(schema: unknown): SchemaFieldRow[] {
  if (!isRecord(schema)) {
    return [];
  }

  const properties = isRecord(schema.properties) ? schema.properties : {};
  const required = Array.isArray(schema.required)
    ? schema.required.filter((entry): entry is string => typeof entry === 'string')
    : [];

  return Object.entries(properties).map(([key, property]) =>
    schemaRowFromProperty(key, property, required.includes(key))
  );
}

function schemaRowFromProperty(
  key: string,
  property: unknown,
  required: boolean
): SchemaFieldRow {
  const type = schemaFieldType(isRecord(property) ? property.type : undefined);

  if (type === 'object') {
    return {
      key,
      type,
      required,
      children: schemaRowsFromObjectSchema(property)
    };
  }

  if (type === 'array') {
    const items = isRecord(property) ? property.items : undefined;

    return {
      key,
      type,
      required,
      children: schemaRowsFromObjectSchema(items)
    };
  }

  return { key, type, required };
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

  return schemaRowsFromObjectSchema(root);
}

function propertySchemaFromRow(row: SchemaFieldRow): Record<string, unknown> {
  if (row.type === 'object') {
    return objectSchemaFromRows(row.children ?? []);
  }

  if (row.type === 'array') {
    return {
      type: 'array',
      items: objectSchemaFromRows(row.children ?? [])
    };
  }

  return propertySchemaForType(row.type);
}

function objectSchemaFromRows(rows: SchemaFieldRow[]) {
  const visibleRows = rows
    .map((row) => ({
      ...row,
      key: row.key.trim()
    }))
    .filter((row) => row.key.length > 0);
  const properties = Object.fromEntries(
    visibleRows.map((row) => [row.key, propertySchemaFromRow(row)])
  );
  const required = visibleRows
    .filter((row) => row.required)
    .map((row) => row.key);

  return {
    type: 'object',
    required,
    properties
  };
}

function schemaFromRows(
  rootType: SchemaRootType,
  rows: SchemaFieldRow[]
): Record<string, unknown> {
  const objectSchema = objectSchemaFromRows(rows);

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

  function updateSchemaRowsAtPath(
    rows: SchemaFieldRow[],
    path: number[],
    updater: (row: SchemaFieldRow) => SchemaFieldRow
  ): SchemaFieldRow[] {
    const [head, ...tail] = path;

    if (head === undefined) {
      return rows;
    }

    return rows.map((row, rowIndex) => {
      if (rowIndex !== head) {
        return row;
      }

      if (tail.length === 0) {
        return updater(row);
      }

      return {
        ...row,
        children: updateSchemaRowsAtPath(row.children ?? [], tail, updater)
      };
    });
  }

  function updateSchemaRow(path: number[], patch: Partial<SchemaFieldRow>) {
    setSchemaRows((current) =>
      updateSchemaRowsAtPath(current, path, (row) => {
        const nextType = patch.type ?? row.type;

        return {
          ...row,
          ...patch,
          children:
            nextType === 'object' || nextType === 'array'
              ? patch.children ?? row.children ?? []
              : undefined
        };
      })
    );
  }

  function createSchemaRow(index: number): SchemaFieldRow {
    return {
      key: `field_${index + 1}`,
      type: 'string',
      required: true
    };
  }

  function addSchemaRow(parentPath?: number[]) {
    if (!parentPath) {
      setSchemaRows((current) => [...current, createSchemaRow(current.length)]);
      return;
    }

    setSchemaRows((current) =>
      updateSchemaRowsAtPath(current, parentPath, (row) => {
        const children = row.children ?? [];

        return {
          ...row,
          children: [...children, createSchemaRow(children.length)]
        };
      })
    );
  }

  function removeSchemaRow(path: number[]) {
    const removeAtPath = (
      rows: SchemaFieldRow[],
      targetPath: number[]
    ): SchemaFieldRow[] => {
      const [head, ...tail] = targetPath;

      if (head === undefined) {
        return rows;
      }

      if (tail.length === 0) {
        return rows.filter((_, rowIndex) => rowIndex !== head);
      }

      return rows.map((row, rowIndex) =>
        rowIndex === head
          ? { ...row, children: removeAtPath(row.children ?? [], tail) }
          : row
      );
    };

    setSchemaRows((current) => removeAtPath(current, path));
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

  function renderSchemaRows(rows: SchemaFieldRow[], parentPath: number[] = []) {
    return rows.map((row, index) => {
      const path = [...parentPath, index];
      const pathLabel = path.map((item) => item + 1).join('.');
      const hasChildren = row.type === 'object' || row.type === 'array';

      return (
        <div
          className="agent-flow-json-schema-settings__field-node"
          key={`${pathLabel}-${row.key}`}
        >
          <div
            className="agent-flow-json-schema-settings__field-row"
            style={{ paddingLeft: parentPath.length * 18 }}
          >
            <Input
              aria-label={i18nText(
                "agentFlow",
                "auto.schema_field_name",
                { value1: pathLabel }
              )}
              value={row.key}
              onChange={(event) =>
                updateSchemaRow(path, {
                  key: event.target.value
                })
              }
            />
            <Select
              aria-label={i18nText(
                "agentFlow",
                "auto.schema_field_type",
                { value1: pathLabel }
              )}
              options={schemaFieldTypeOptions}
              value={row.type}
              onChange={(type) => updateSchemaRow(path, { type })}
            />
            <Checkbox
              aria-label={i18nText(
                "agentFlow",
                "auto.schema_field_required",
                { value1: pathLabel }
              )}
              checked={row.required}
              onChange={(event) =>
                updateSchemaRow(path, {
                  required: event.target.checked
                })
              }
            />
            <Button
              aria-label={i18nText(
                "agentFlow",
                "auto.delete_schema_field",
                { value1: row.key || pathLabel }
              )}
              danger
              icon={<DeleteOutlined />}
              size="small"
              type="text"
              onClick={() => removeSchemaRow(path)}
            />
          </div>
          {hasChildren ? (
            <div
              className="agent-flow-json-schema-settings__field-children"
              style={{ paddingLeft: (parentPath.length + 1) * 18 }}
            >
              {row.children && row.children.length > 0
                ? renderSchemaRows(row.children, path)
                : null}
              <Button
                aria-label={i18nText(
                  "agentFlow",
                  "auto.add_child_schema_field",
                  { value1: row.key || pathLabel }
                )}
                className="agent-flow-json-schema-settings__add-child-field"
                icon={<PlusOutlined />}
                size="small"
                type="dashed"
                onClick={() => addSchemaRow(path)}
              >
                {i18nText("agentFlow", "auto.add_child_schema_field", {
                  value1: row.key || pathLabel
                })}
              </Button>
            </div>
          ) : null}
        </div>
      );
    });
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
                  <Tooltip title="编辑 JSON Schema">
                    <Button
                      aria-label="编辑 JSON Schema"
                      className={
                        output.jsonSchema
                          ? 'agent-flow-output-contract-editor__schema agent-flow-output-contract-editor__schema--active'
                          : 'agent-flow-output-contract-editor__schema'
                      }
                      icon={<FileTextOutlined />}
                      size="small"
                      type="text"
                      onClick={(event) =>
                        openSchemaEditor(index, event.currentTarget)
                      }
                    />
                  </Tooltip>
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
                      {renderSchemaRows(schemaRows)}
                    </div>
                    <Button
                      aria-label={i18nText("agentFlow", "auto.add_schema_field")}
                      className="agent-flow-json-schema-settings__add-field"
                      icon={<PlusOutlined />}
                      type="dashed"
                      onClick={() => addSchemaRow()}
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
