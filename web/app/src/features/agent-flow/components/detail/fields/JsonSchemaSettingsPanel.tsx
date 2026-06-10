import { DeleteOutlined, PlusOutlined } from '@ant-design/icons';
import type { editor } from 'monaco-editor';
import { Button, Checkbox, Input, Select, Tabs, Typography } from 'antd';
import {
  Suspense,
  forwardRef,
  lazy,
  useEffect,
  useImperativeHandle,
  useMemo,
  useRef,
  useState,
  type ReactNode,
  type RefObject
} from 'react';

import { parseJsonSchemaInput } from '../../../lib/output-contract/schema';
import { FloatingSettingsPanel } from '../FloatingSettingsPanel';
import { i18nText } from '../../../../../shared/i18n/text';
import {
  jsonSchemaRootType,
  type JsonSchemaRootType
} from './json-schema-utils';

const MonacoEditor = lazy(() => import('@monaco-editor/react'));

type SchemaEditorTab = 'fields' | 'json';
type SchemaFieldType = 'string' | 'number' | 'boolean' | 'object' | 'array';

const schemaFieldTypeOptions = [
  { value: 'string', label: 'String' },
  { value: 'number', label: 'Number' },
  { value: 'boolean', label: 'Boolean' },
  { value: 'object', label: 'Object' },
  { value: 'array', label: 'Array' }
] satisfies Array<{ value: SchemaFieldType; label: string }>;

interface SchemaFieldRow {
  id: string;
  key: string;
  type: SchemaFieldType;
  description: string;
  required: boolean;
  children?: SchemaFieldRow[];
  baseSchema?: Record<string, unknown>;
}

let schemaFieldRowIdSeed = 0;

function createSchemaFieldRowId() {
  schemaFieldRowIdSeed += 1;
  return `schema-field-row-${schemaFieldRowIdSeed}`;
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

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function stringifySchema(schema: Record<string, unknown>) {
  return JSON.stringify(schema, null, 2);
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
    ? schema.required.filter(
        (entry): entry is string => typeof entry === 'string'
      )
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
  const baseSchema = isRecord(property) ? property : undefined;
  const type = schemaFieldType(baseSchema?.type);
  const description =
    baseSchema && typeof baseSchema.description === 'string'
      ? baseSchema.description
      : '';

  if (type === 'object') {
    return {
      id: createSchemaFieldRowId(),
      key,
      type,
      description,
      required,
      baseSchema,
      children: schemaRowsFromObjectSchema(property)
    };
  }

  if (type === 'array') {
    const items = baseSchema?.items;

    return {
      id: createSchemaFieldRowId(),
      key,
      type,
      description,
      required,
      baseSchema,
      children: schemaRowsFromObjectSchema(items)
    };
  }

  return {
    id: createSchemaFieldRowId(),
    key,
    type,
    description,
    required,
    baseSchema
  };
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

function withDescription(schema: Record<string, unknown>, description: string) {
  const trimmed = description.trim();
  const nextSchema = { ...schema };

  if (trimmed) {
    nextSchema.description = trimmed;
  } else {
    delete nextSchema.description;
  }

  return nextSchema;
}

function compatibleBaseSchema(row: SchemaFieldRow) {
  return row.baseSchema?.type === row.type ? row.baseSchema : undefined;
}

function propertySchemaFromRow(row: SchemaFieldRow): Record<string, unknown> {
  const baseSchema = compatibleBaseSchema(row);

  if (row.type === 'object') {
    return withDescription(
      objectSchemaFromRows(row.children ?? [], baseSchema),
      row.description
    );
  }

  if (row.type === 'array') {
    const baseItemsSchema = isRecord(baseSchema?.items)
      ? baseSchema.items
      : undefined;

    return withDescription(
      {
        ...(baseSchema ?? {}),
        type: 'array',
        items: objectSchemaFromRows(row.children ?? [], baseItemsSchema)
      },
      row.description
    );
  }

  return withDescription(
    {
      ...(baseSchema ?? {}),
      ...propertySchemaForType(row.type)
    },
    row.description
  );
}

function objectSchemaFromRows(
  rows: SchemaFieldRow[],
  baseSchema?: Record<string, unknown>
) {
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
    ...(baseSchema ?? {}),
    type: 'object',
    required,
    properties
  };
}

function schemaFromRows(
  rootType: JsonSchemaRootType,
  rows: SchemaFieldRow[],
  baseSchema?: Record<string, unknown>
): Record<string, unknown> {
  const objectSchema =
    rootType === 'array'
      ? objectSchemaFromRows(
          rows,
          isRecord(baseSchema?.items) ? baseSchema.items : undefined
        )
      : objectSchemaFromRows(rows, baseSchema);

  if (rootType === 'array') {
    return {
      ...(baseSchema ?? {}),
      type: 'array',
      items: objectSchema
    };
  }

  return objectSchema;
}

function JsonSchemaEditorFallback() {
  return (
    <div className="agent-flow-json-schema-settings__editor-loading">
      {i18nText('agentFlow', 'auto.loading_json_schema_editor')}
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
      ariaLabel: i18nText('agentFlow', 'auto.json_schema_content')
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

function readonlySchemaValue(value: unknown) {
  if (value === undefined) {
    return '';
  }

  if (typeof value === 'string') {
    return JSON.stringify(value);
  }

  if (
    typeof value === 'number' ||
    typeof value === 'boolean' ||
    value === null
  ) {
    return String(value);
  }

  return JSON.stringify(value);
}

function readonlySchemaValueType(value: unknown) {
  if (Array.isArray(value)) {
    return 'Array';
  }

  if (value === null) {
    return 'Null';
  }

  if (typeof value === 'object') {
    return 'Object';
  }

  if (typeof value === 'string') {
    return 'String';
  }

  if (typeof value === 'number') {
    return 'Number';
  }

  if (typeof value === 'boolean') {
    return 'Boolean';
  }

  return 'Unknown';
}

interface JsonSchemaSettingsPanelProps {
  open: boolean;
  schema: Record<string, unknown>;
  fallbackRootType?: JsonSchemaRootType;
  className?: string;
  title?: string;
  triggerRef: RefObject<HTMLElement | null>;
  onClose: () => void;
  onSave: (schema: Record<string, unknown>) => void;
}

type JsonSchemaEditorResult =
  | { ok: true; schema: Record<string, unknown> }
  | { ok: false; message: string };

export interface JsonSchemaEditorContentHandle {
  getSchema: () => JsonSchemaEditorResult;
}

interface JsonSchemaEditorContentProps {
  schema: Record<string, unknown>;
  fallbackRootType?: JsonSchemaRootType;
  active?: boolean;
  className?: string;
  live?: boolean;
  resetKey?: string | number | null;
  onChange?: (schema: Record<string, unknown>) => void;
  onValidityChange?: (valid: boolean) => void;
}

export const JsonSchemaEditorContent = forwardRef<
  JsonSchemaEditorContentHandle,
  JsonSchemaEditorContentProps
>(function JsonSchemaEditorContent(
  {
    active = true,
    schema,
    fallbackRootType = 'object',
    className,
    live = false,
    resetKey,
    onChange,
    onValidityChange
  }: JsonSchemaEditorContentProps,
  ref
) {
  const [schemaText, setSchemaText] = useState('');
  const [schemaRows, setSchemaRows] = useState<SchemaFieldRow[]>([]);
  const [schemaBase, setSchemaBase] = useState<Record<string, unknown>>(schema);
  const [schemaRoot, setSchemaRoot] =
    useState<JsonSchemaRootType>(fallbackRootType);
  const [schemaTab, setSchemaTab] = useState<SchemaEditorTab>('fields');
  const [schemaError, setSchemaError] = useState<string | null>(null);
  const lastResetKeyRef = useRef<string | number | null | undefined>(undefined);

  useEffect(() => {
    if (!active) {
      return;
    }

    if (resetKey !== undefined && lastResetKeyRef.current === resetKey) {
      return;
    }

    lastResetKeyRef.current = resetKey;
    setSchemaBase(schema);
    setSchemaText(stringifySchema(schema));
    setSchemaRows(schemaRowsFromSchema(schema));
    setSchemaRoot(jsonSchemaRootType(schema, fallbackRootType));
    setSchemaTab('fields');
    setSchemaError(null);
    onValidityChange?.(true);
  }, [active, fallbackRootType, onValidityChange, resetKey, schema]);

  function notifyValidSchema(nextSchema: Record<string, unknown>) {
    if (!live) {
      return;
    }

    setSchemaError(null);
    onValidityChange?.(true);
    onChange?.(nextSchema);
  }

  function notifyInvalidSchema(message: string) {
    if (!live) {
      return;
    }

    setSchemaError(message);
    onValidityChange?.(false);
  }

  function readCurrentSchema(): JsonSchemaEditorResult {
    if (schemaTab === 'fields') {
      return {
        ok: true,
        schema: schemaFromRows(schemaRoot, schemaRows, schemaBase)
      };
    }

    return parseJsonSchemaInput(schemaText);
  }

  function getSchema() {
    const parsed = readCurrentSchema();

    if (!parsed.ok) {
      setSchemaError(parsed.message);
      onValidityChange?.(false);
      return parsed;
    }

    setSchemaError(null);
    onValidityChange?.(true);
    return parsed;
  }

  useImperativeHandle(ref, () => ({ getSchema }));

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
    const nextRows = updateSchemaRowsAtPath(schemaRows, path, (row) => {
      const nextType = patch.type ?? row.type;
      const typeChanged = patch.type !== undefined && patch.type !== row.type;

      return {
        ...row,
        ...patch,
        baseSchema: typeChanged
          ? undefined
          : (patch.baseSchema ?? row.baseSchema),
        children:
          nextType === 'object' || nextType === 'array'
            ? (patch.children ?? row.children ?? [])
            : undefined
      };
    });

    setSchemaRows(nextRows);
    notifyValidSchema(schemaFromRows(schemaRoot, nextRows, schemaBase));
  }

  function createSchemaRow(index: number): SchemaFieldRow {
    return {
      id: createSchemaFieldRowId(),
      key: `field_${index + 1}`,
      type: 'string',
      description: '',
      required: true
    };
  }

  function addSchemaRow(parentPath?: number[]) {
    if (!parentPath) {
      const nextRows = [...schemaRows, createSchemaRow(schemaRows.length)];

      setSchemaRows(nextRows);
      notifyValidSchema(schemaFromRows(schemaRoot, nextRows, schemaBase));
      return;
    }

    const nextRows = updateSchemaRowsAtPath(schemaRows, parentPath, (row) => {
      const children = row.children ?? [];

      return {
        ...row,
        children: [...children, createSchemaRow(children.length)]
      };
    });

    setSchemaRows(nextRows);
    notifyValidSchema(schemaFromRows(schemaRoot, nextRows, schemaBase));
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

    const nextRows = removeAtPath(schemaRows, path);

    setSchemaRows(nextRows);
    notifyValidSchema(schemaFromRows(schemaRoot, nextRows, schemaBase));
  }

  function switchSchemaTab(nextTab: string) {
    if (nextTab === 'json') {
      const nextSchema = schemaFromRows(schemaRoot, schemaRows, schemaBase);

      setSchemaBase(nextSchema);
      setSchemaText(stringifySchema(nextSchema));
      setSchemaTab('json');
      setSchemaError(null);
      notifyValidSchema(nextSchema);
      return;
    }

    const parsed = parseJsonSchemaInput(schemaText);
    if (!parsed.ok) {
      setSchemaError(parsed.message);
      onValidityChange?.(false);
      return;
    }

    setSchemaRows(schemaRowsFromSchema(parsed.schema));
    setSchemaBase(parsed.schema);
    setSchemaRoot(jsonSchemaRootType(parsed.schema, schemaRoot));
    setSchemaTab('fields');
    setSchemaError(null);
    notifyValidSchema(parsed.schema);
  }

  function updateSchemaText(nextValue: string) {
    setSchemaText(nextValue);

    if (!live) {
      setSchemaError(null);
      return;
    }

    const parsed = parseJsonSchemaInput(nextValue);

    if (!parsed.ok) {
      notifyInvalidSchema(parsed.message);
      return;
    }

    setSchemaBase(parsed.schema);
    notifyValidSchema(parsed.schema);
  }

  function renderReadonlySchemaRow(
    key: string,
    value: unknown,
    indent = 0,
    operation?: ReactNode
  ) {
    return (
      <div
        className="agent-flow-json-schema-settings__field-row agent-flow-json-schema-settings__field-row--readonly"
        style={{ paddingLeft: indent }}
      >
        <Input
          aria-label={i18nText('agentFlow', 'auto.schema_field_name', {
            value1: key
          })}
          disabled
          value={key}
        />
        <Input
          aria-label={i18nText('agentFlow', 'auto.schema_field_value', {
            value1: key
          })}
          disabled
          value={readonlySchemaValue(value)}
        />
        <Input disabled value={readonlySchemaValueType(value)} />
        <Checkbox disabled checked={false} />
        {operation ?? (
          <Typography.Text type="secondary">
            {i18nText('agentFlow', 'auto.read_only')}
          </Typography.Text>
        )}
      </div>
    );
  }

  function renderReadonlySchemaNode(key: string, value: unknown, indent = 0) {
    return (
      <div
        className="agent-flow-json-schema-settings__field-node"
        key={`readonly-${indent}-${key}`}
      >
        {renderReadonlySchemaRow(key, value, indent)}
      </div>
    );
  }

  function renderReadonlySchemaNodes(
    schema: Record<string, unknown>,
    excludedKeys: string[],
    indent = 0
  ) {
    const excluded = new Set(excludedKeys);

    return Object.entries(schema)
      .filter(([key]) => !excluded.has(key))
      .map(([key, value]) => renderReadonlySchemaNode(key, value, indent));
  }

  function renderPropertiesSchemaNode(
    objectSchema: Record<string, unknown>,
    indent = 0
  ) {
    const properties = isRecord(objectSchema.properties)
      ? objectSchema.properties
      : {};

    return (
      <div
        className="agent-flow-json-schema-settings__field-node"
        key={`properties-${indent}`}
      >
        {renderReadonlySchemaRow(
          'properties',
          properties,
          indent,
          <Button
            aria-label={i18nText('agentFlow', 'auto.add_schema_field')}
            icon={<PlusOutlined />}
            size="small"
            type="text"
            onClick={() => addSchemaRow()}
          />
        )}
        <div
          className="agent-flow-json-schema-settings__field-children"
          style={{ paddingLeft: indent + 18 }}
        >
          {renderSchemaRows(schemaRows)}
        </div>
      </div>
    );
  }

  function renderSchemaStructureRows() {
    const currentSchema = schemaFromRows(schemaRoot, schemaRows, schemaBase);

    if (schemaRoot === 'array') {
      const itemSchema = isRecord(currentSchema.items)
        ? currentSchema.items
        : objectSchemaFromRows(schemaRows);

      return (
        <>
          {renderReadonlySchemaNode('type', currentSchema.type ?? 'array')}
          <div
            className="agent-flow-json-schema-settings__field-node"
            key="items"
          >
            {renderReadonlySchemaRow('items', itemSchema)}
            <div
              className="agent-flow-json-schema-settings__field-children"
              style={{ paddingLeft: 18 }}
            >
              {renderReadonlySchemaNode('type', itemSchema.type ?? 'object')}
              {renderReadonlySchemaNode(
                'required',
                Array.isArray(itemSchema.required) ? itemSchema.required : []
              )}
              {renderPropertiesSchemaNode(itemSchema)}
              {renderReadonlySchemaNodes(itemSchema, [
                'type',
                'required',
                'properties'
              ])}
            </div>
          </div>
          {renderReadonlySchemaNodes(currentSchema, ['type', 'items'])}
        </>
      );
    }

    return (
      <>
        {renderReadonlySchemaNode('type', currentSchema.type ?? 'object')}
        {renderReadonlySchemaNode(
          'required',
          Array.isArray(currentSchema.required) ? currentSchema.required : []
        )}
        {renderPropertiesSchemaNode(currentSchema)}
        {renderReadonlySchemaNodes(currentSchema, [
          'type',
          'required',
          'properties'
        ])}
      </>
    );
  }

  function renderSchemaRows(rows: SchemaFieldRow[], parentPath: number[] = []) {
    return rows.map((row, index) => {
      const path = [...parentPath, index];
      const pathLabel = path.map((item) => item + 1).join('.');
      const hasChildren = row.type === 'object' || row.type === 'array';

      return (
        <div
          className="agent-flow-json-schema-settings__field-node"
          key={row.id}
        >
          <div
            className="agent-flow-json-schema-settings__field-row"
            style={{ paddingLeft: parentPath.length * 18 }}
          >
            <Input
              aria-label={i18nText('agentFlow', 'auto.schema_field_name', {
                value1: pathLabel
              })}
              value={row.key}
              onChange={(event) =>
                updateSchemaRow(path, {
                  key: event.target.value
                })
              }
            />
            <Input
              aria-label={i18nText(
                'agentFlow',
                'auto.schema_field_description',
                {
                  value1: pathLabel
                }
              )}
              value={row.description}
              onChange={(event) =>
                updateSchemaRow(path, {
                  description: event.target.value
                })
              }
            />
            <Select
              aria-label={i18nText('agentFlow', 'auto.schema_field_type', {
                value1: pathLabel
              })}
              options={schemaFieldTypeOptions}
              value={row.type}
              onChange={(type) => updateSchemaRow(path, { type })}
            />
            <Checkbox
              aria-label={i18nText('agentFlow', 'auto.schema_field_required', {
                value1: pathLabel
              })}
              checked={row.required}
              onChange={(event) =>
                updateSchemaRow(path, {
                  required: event.target.checked
                })
              }
            />
            <Button
              aria-label={i18nText('agentFlow', 'auto.delete_schema_field', {
                value1: row.key || pathLabel
              })}
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
                  'agentFlow',
                  'auto.add_child_schema_field',
                  { value1: row.key || pathLabel }
                )}
                className="agent-flow-json-schema-settings__add-child-field"
                icon={<PlusOutlined />}
                size="small"
                type="dashed"
                onClick={() => addSchemaRow(path)}
              />
            </div>
          ) : null}
        </div>
      );
    });
  }

  return (
    <div
      className={['agent-flow-json-schema-settings', className]
        .filter(Boolean)
        .join(' ')}
    >
      <Tabs
        activeKey={schemaTab}
        onChange={switchSchemaTab}
        items={[
          {
            key: 'fields',
            label: i18nText('agentFlow', 'auto.schema_fields'),
            children: (
              <div className="agent-flow-json-schema-settings__fields">
                <div className="agent-flow-json-schema-settings__field-head">
                  <span>{i18nText('agentFlow', 'auto.field_name')}</span>
                  <span>
                    {i18nText('agentFlow', 'auto.schema_value_or_description')}
                  </span>
                  <span>{i18nText('agentFlow', 'auto.type')}</span>
                  <span>{i18nText('agentFlow', 'auto.required')}</span>
                  <span>{i18nText('agentFlow', 'auto.operation')}</span>
                </div>
                <div className="agent-flow-json-schema-settings__field-rows">
                  {renderSchemaStructureRows()}
                </div>
              </div>
            )
          },
          {
            key: 'json',
            label: i18nText('agentFlow', 'auto.json_parse'),
            children: (
              <JsonSchemaCodeEditor
                value={schemaText}
                onChange={updateSchemaText}
              />
            )
          }
        ]}
      />
      <Typography.Text type={schemaError ? 'danger' : 'secondary'}>
        {schemaError ?? i18nText('agentFlow', 'auto.json_schema_parse_hint')}
      </Typography.Text>
    </div>
  );
});

JsonSchemaEditorContent.displayName = 'JsonSchemaEditorContent';

export function JsonSchemaInlineEditor({
  schema,
  fallbackRootType = 'object',
  resetKey,
  onChange,
  onValidityChange
}: {
  schema: Record<string, unknown>;
  fallbackRootType?: JsonSchemaRootType;
  resetKey?: string | number | null;
  onChange: (schema: Record<string, unknown>) => void;
  onValidityChange?: (valid: boolean) => void;
}) {
  return (
    <JsonSchemaEditorContent
      active
      className="agent-flow-json-schema-settings--inline"
      fallbackRootType={fallbackRootType}
      live
      resetKey={resetKey}
      schema={schema}
      onChange={onChange}
      onValidityChange={onValidityChange}
    />
  );
}

export function JsonSchemaSettingsPanel({
  open,
  schema,
  fallbackRootType = 'object',
  className,
  title = 'JSON Schema',
  triggerRef,
  onClose,
  onSave
}: JsonSchemaSettingsPanelProps) {
  const editorRef = useRef<JsonSchemaEditorContentHandle>(null);

  function saveSchema() {
    const parsed = editorRef.current?.getSchema();

    if (!parsed?.ok) {
      return;
    }

    onSave(parsed.schema);
  }

  const schemaPanelFooter = (
    <div className="agent-flow-json-schema-settings__footer">
      <Button
        aria-label={i18nText('agentFlow', 'auto.cancel')}
        onClick={onClose}
      >
        {i18nText('agentFlow', 'auto.cancel')}
      </Button>
      <Button
        aria-label={i18nText('agentFlow', 'auto.save')}
        type="primary"
        onClick={saveSchema}
      >
        {i18nText('agentFlow', 'auto.save')}
      </Button>
    </div>
  );

  return (
    <FloatingSettingsPanel
      className={['agent-flow-json-schema-settings__panel', className]
        .filter(Boolean)
        .join(' ')}
      closeLabel={i18nText('agentFlow', 'auto.close_json_schema_settings')}
      defaultWidth={720}
      initialHeight={560}
      minHeight={420}
      minWidth={620}
      open={open}
      title={title}
      triggerRef={triggerRef}
      footer={schemaPanelFooter}
      dragHandleTestId="agent-flow-json-schema-settings-drag-handle"
      leftResizeHandleTestId="agent-flow-json-schema-settings-resize-left"
      rightResizeHandleTestId="agent-flow-json-schema-settings-resize-right"
      bottomResizeHandleTestId="agent-flow-json-schema-settings-resize-bottom"
      onClose={onClose}
    >
      <JsonSchemaEditorContent
        ref={editorRef}
        active={open}
        fallbackRootType={fallbackRootType}
        schema={schema}
      />
    </FloatingSettingsPanel>
  );
}
