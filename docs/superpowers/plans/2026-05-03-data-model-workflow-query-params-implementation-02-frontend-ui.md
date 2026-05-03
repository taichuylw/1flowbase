# Data Model Query Params 02 Frontend UI Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. Update the index plan after each completed task.

**Goal:** Register and render a Data Model query editor for `action=list`.

**Architecture:** The node definition exposes `bindings.query` only for list action. `DataModelQueryField` edits the whole `data_model_query` binding value through the existing schema adapter, using existing selector and Ant Design controls.

**Tech Stack:** React 19, Ant Design 5, Vitest, Testing Library.

---

## Files

- Modify: `web/app/src/features/agent-flow/lib/node-definitions/types.ts`
- Modify: `web/app/src/features/agent-flow/schema/node-schema-fragments.ts`
- Modify: `web/app/src/features/agent-flow/lib/node-definitions/nodes/data-model/index.ts`
- Create: `web/app/src/features/agent-flow/components/bindings/DataModelQueryField.tsx`
- Modify: `web/app/src/features/agent-flow/schema/agent-flow-field-renderers.tsx`
- Test: `web/app/src/features/agent-flow/_tests/node-schema-registry.test.tsx`
- Test: `web/app/src/features/agent-flow/_tests/node-inspector.test.tsx`

### Task 1: Register Data Model Query Schema Field

- [x] **Step 1: Add failing schema tests**

In `web/app/src/features/agent-flow/_tests/node-schema-registry.test.tsx`, update `exposes a real renderer registry for later schema-driven consumers`:

```ts
    expect(agentFlowRendererRegistry.fields.data_model_query).toBeTypeOf('function');
```

Add this test:

```ts
  test('exposes Data Model query params only for list action', () => {
    const schema = resolveAgentFlowNodeSchema('data_model' as never);
    const queryField = findFieldBlock(
      schema.detail.tabs.config.blocks,
      'bindings.query'
    );

    expect(queryField).toEqual(
      expect.objectContaining({
        renderer: 'data_model_query',
        visibleWhen: {
          operator: 'equals',
          path: 'config.action',
          value: 'list'
        }
      })
    );
  });
```

- [x] **Step 2: Confirm failure**

Run:

```bash
pnpm --dir web/app test -- node-schema-registry
```

Expected: FAIL because `data_model_query` is not registered.

- [x] **Step 3: Add editor kind and fragment mapping**

In `web/app/src/features/agent-flow/lib/node-definitions/types.ts`, add:

```ts
  | 'data_model_query'
```

after `data_model`.

In `web/app/src/features/agent-flow/schema/node-schema-fragments.ts`, add:

```ts
  data_model_query: 'data_model_query',
```

after `data_model: 'data_model'`.

- [x] **Step 4: Add node definition field**

In `web/app/src/features/agent-flow/lib/node-definitions/nodes/data-model/index.ts`, insert this field after `config.action`:

```ts
        {
          key: 'bindings.query',
          label: '查询参数',
          editor: 'data_model_query',
          visibleWhen: {
            operator: 'equals',
            path: 'config.action',
            value: 'list'
          }
        },
```

- [x] **Step 5: Add inert renderer registration**

In `web/app/src/features/agent-flow/schema/agent-flow-field-renderers.tsx`, add:

```tsx
function renderDataModelQueryField() {
  return null;
}
```

Add the renderer entry:

```ts
  data_model_query: renderDataModelQueryField,
```

- [x] **Step 6: Verify schema registration**

Run:

```bash
pnpm --dir web/app test -- node-schema-registry
```

Expected: PASS.

### Task 2: Add Query Editor UI

- [x] **Step 1: Add failing UI test**

In `web/app/src/features/agent-flow/_tests/node-inspector.test.tsx`, extend the `orders` fixture with:

```ts
          { code: 'status', title: 'Status', valueType: 'enum', required: false },
          { code: 'customer', title: 'Customer', valueType: 'many_to_one', required: false },
          { code: 'lines', title: 'Lines', valueType: 'one_to_many', required: false }
```

Append this test:

```tsx
  test('edits Data Model list query binding', async () => {
    let latestDocument = createDefaultAgentFlowDocument({ flowId: 'flow-1' });

    renderWithProviders(
      <AgentFlowEditorStoreProvider initialState={createInitialStateWithDataModelNode()}>
        <SelectionSeed nodeId="node-data-model" />
        <DocumentObserver onChange={(document) => { latestDocument = document; }} />
        <NodeConfigTab />
      </AgentFlowEditorStoreProvider>
    );

    expect(await screen.findByText('请先选择 Data Model')).toBeInTheDocument();

    await openSelect('Data Model');
    await selectDataModelOption('orders');
    fireEvent.click(await screen.findByRole('button', { name: '新增过滤条件' }));
    await openSelect('过滤字段 1');
    await selectOption('Status');
    await openSelect('过滤操作符 1');
    await selectOption('eq');
    await openSelect('过滤值来源 1');
    await selectOption('变量');
    await openSelect('过滤变量 1');
    await selectOption('userinput.query');
    fireEvent.click(screen.getByRole('button', { name: '新增排序规则' }));
    await openSelect('排序字段 1');
    await selectOption('Amount');
    await openSelect('排序方向 1');
    await selectOption('desc');
    await openSelect('展开关联');
    await selectOption('Customer');
    fireEvent.change(screen.getByLabelText('页码'), { target: { value: '2' } });
    fireEvent.change(screen.getByLabelText('每页数量'), { target: { value: '50' } });

    await waitFor(() => {
      expect(getDataModelNode(latestDocument).bindings.query).toMatchObject({
        kind: 'data_model_query',
        value: {
          filters: [
            {
              field_code: 'status',
              operator: 'eq',
              value: { kind: 'selector', selector: ['node-start', 'query'] }
            }
          ],
          sorts: [{ field_code: 'amount', direction: 'desc' }],
          expand_relations: ['customer'],
          page: { kind: 'constant', value: 2 },
          page_size: { kind: 'constant', value: 50 }
        }
      });
    });
  });
```

- [x] **Step 2: Confirm failure**

Run:

```bash
pnpm --dir web/app test -- node-inspector
```

Expected: FAIL because the renderer returns `null`.

- [x] **Step 3: Create `DataModelQueryField`**

Create `web/app/src/features/agent-flow/components/bindings/DataModelQueryField.tsx`:

```tsx
import type {
  DataModelQueryBindingValue,
  DataModelQueryFilter,
  DataModelQuerySort,
  DataModelQueryValue
} from '@1flowbase/flow-schema';
import { Button, Flex, Input, InputNumber, Select, Space, Typography } from 'antd';

import type { AgentFlowDataModelFieldOption } from '../../api/data-model-options';
import type { FlowSelectorOption } from '../../lib/selector-options';
import {
  DATA_MODEL_QUERY_OPERATORS,
  normalizeDataModelQueryBindingValue
} from '../../lib/data-model-query-binding';
import { SelectorField } from './SelectorField';

const FILTER_TYPES = new Set(['string', 'enum', 'text', 'datetime', 'number', 'boolean', 'json', 'many_to_one']);
const SORT_TYPES = new Set(['string', 'enum', 'text', 'datetime', 'number', 'boolean']);
const EXPAND_TYPES = new Set(['many_to_one', 'one_to_many']);
const RANGE_TYPES = new Set(['number', 'datetime']);

interface Props {
  ariaLabel: string;
  value: unknown;
  fields: AgentFlowDataModelFieldOption[];
  selectorOptions: FlowSelectorOption[];
  onChange: (value: DataModelQueryBindingValue) => void;
}

function fieldOptions(fields: AgentFlowDataModelFieldOption[], types: Set<string>) {
  return fields
    .filter((field) => types.has(field.valueType))
    .map((field) => ({ value: field.code, label: field.title || field.code }));
}

function operators(fields: AgentFlowDataModelFieldOption[], fieldCode: string) {
  const field = fields.find((entry) => entry.code === fieldCode);
  const values = field && RANGE_TYPES.has(field.valueType)
    ? DATA_MODEL_QUERY_OPERATORS
    : (['eq', 'ne'] as const);

  return values.map((value) => ({ value, label: value }));
}

function nextFilter(fields: AgentFlowDataModelFieldOption[]): DataModelQueryFilter {
  return {
    field_code: fieldOptions(fields, FILTER_TYPES)[0]?.value ?? '',
    operator: 'eq',
    value: { kind: 'constant', value: '' }
  };
}

function nextSort(fields: AgentFlowDataModelFieldOption[]): DataModelQuerySort {
  return {
    field_code: fieldOptions(fields, SORT_TYPES)[0]?.value ?? '',
    direction: 'asc'
  };
}

function QueryValueInput({
  ariaLabel,
  value,
  selectorOptions,
  numeric,
  sourceAriaLabel,
  selectorAriaLabel,
  onChange
}: {
  ariaLabel: string;
  value: DataModelQueryValue;
  selectorOptions: FlowSelectorOption[];
  numeric?: boolean;
  sourceAriaLabel?: string;
  selectorAriaLabel?: string;
  onChange: (value: DataModelQueryValue) => void;
}) {
  return (
    <Space.Compact block>
      <Select
        aria-label={sourceAriaLabel ?? `${ariaLabel}来源`}
        options={[{ value: 'constant', label: '常量' }, { value: 'selector', label: '变量' }]}
        value={value.kind}
        onChange={(kind) =>
          onChange(kind === 'selector'
            ? { kind: 'selector', selector: [] }
            : { kind: 'constant', value: numeric ? 1 : '' })
        }
      />
      {value.kind === 'selector' ? (
        <SelectorField
          ariaLabel={selectorAriaLabel ?? ariaLabel}
          options={selectorOptions}
          value={value.selector}
          onChange={(nextValue) => onChange({ kind: 'selector', selector: nextValue as string[] })}
        />
      ) : numeric ? (
        <InputNumber
          aria-label={ariaLabel}
          min={1}
          value={typeof value.value === 'number' ? value.value : null}
          onChange={(nextValue) => onChange({ kind: 'constant', value: nextValue ?? 1 })}
        />
      ) : (
        <Input
          aria-label={ariaLabel}
          value={typeof value.value === 'string' ? value.value : JSON.stringify(value.value ?? '')}
          onChange={(event) => onChange({ kind: 'constant', value: event.target.value })}
        />
      )}
    </Space.Compact>
  );
}

export function DataModelQueryField({ ariaLabel, value, fields, selectorOptions, onChange }: Props) {
  const query = normalizeDataModelQueryBindingValue(value);
  const filterOptions = fieldOptions(fields, FILTER_TYPES);
  const sortOptions = fieldOptions(fields, SORT_TYPES);
  const expandOptions = fieldOptions(fields, EXPAND_TYPES);
  const update = (patch: Partial<DataModelQueryBindingValue>) => onChange({ ...query, ...patch });

  if (fields.length === 0) {
    return <Typography.Text type="secondary">请先选择 Data Model</Typography.Text>;
  }

  return (
    <Flex aria-label={ariaLabel} vertical gap={12}>
      <Flex vertical gap={8}>
        {query.filters.map((filter, index) => (
          <Space.Compact key={`filter-${index}`} block>
            <Select aria-label={`过滤字段 ${index + 1}`} options={filterOptions} value={filter.field_code || undefined} onChange={(field_code) => update({ filters: query.filters.map((entry, entryIndex) => entryIndex === index ? { ...entry, field_code, operator: 'eq' } : entry) })} />
            <Select aria-label={`过滤操作符 ${index + 1}`} options={operators(fields, filter.field_code)} value={filter.operator} onChange={(operator) => update({ filters: query.filters.map((entry, entryIndex) => entryIndex === index ? { ...entry, operator } : entry) })} />
            <QueryValueInput ariaLabel={`过滤值 ${index + 1}`} sourceAriaLabel={`过滤值来源 ${index + 1}`} selectorAriaLabel={`过滤变量 ${index + 1}`} selectorOptions={selectorOptions} value={filter.value} onChange={(nextValue) => update({ filters: query.filters.map((entry, entryIndex) => entryIndex === index ? { ...entry, value: nextValue } : entry) })} />
            <Button danger type="text" onClick={() => update({ filters: query.filters.filter((_, entryIndex) => entryIndex !== index) })}>删除</Button>
          </Space.Compact>
        ))}
        <Button type="dashed" onClick={() => update({ filters: [...query.filters, nextFilter(fields)] })}>新增过滤条件</Button>
      </Flex>
      <Flex vertical gap={8}>
        {query.sorts.map((sort, index) => (
          <Space.Compact key={`sort-${index}`} block>
            <Select aria-label={`排序字段 ${index + 1}`} options={sortOptions} value={sort.field_code || undefined} onChange={(field_code) => update({ sorts: query.sorts.map((entry, entryIndex) => entryIndex === index ? { ...entry, field_code } : entry) })} />
            <Select aria-label={`排序方向 ${index + 1}`} options={[{ value: 'asc', label: 'asc' }, { value: 'desc', label: 'desc' }]} value={sort.direction} onChange={(direction) => update({ sorts: query.sorts.map((entry, entryIndex) => entryIndex === index ? { ...entry, direction } : entry) })} />
            <Button danger type="text" onClick={() => update({ sorts: query.sorts.filter((_, entryIndex) => entryIndex !== index) })}>删除</Button>
          </Space.Compact>
        ))}
        <Button type="dashed" onClick={() => update({ sorts: [...query.sorts, nextSort(fields)] })}>新增排序规则</Button>
      </Flex>
      <Select aria-label="展开关联" mode="multiple" options={expandOptions} value={query.expand_relations} onChange={(expand_relations) => update({ expand_relations })} />
      <Space.Compact block>
        <QueryValueInput ariaLabel="页码" numeric selectorOptions={selectorOptions} value={query.page} onChange={(page) => update({ page })} />
        <QueryValueInput ariaLabel="每页数量" numeric selectorOptions={selectorOptions} value={query.page_size} onChange={(page_size) => update({ page_size })} />
      </Space.Compact>
    </Flex>
  );
}
```

- [x] **Step 4: Wire real renderer**

In `web/app/src/features/agent-flow/schema/agent-flow-field-renderers.tsx`, import:

```ts
import type { AgentFlowDataModelFieldOption } from '../api/data-model-options';
import { DataModelQueryField } from '../components/bindings/DataModelQueryField';
import { DATA_MODEL_QUERY_DEFAULT_VALUE } from '../lib/data-model-query-binding';
```

Replace `renderDataModelQueryField`:

```tsx
function renderDataModelQueryField({ adapter, block }: SchemaFieldRendererProps) {
  const value = adapter.getValue(block.path);
  const binding = getBindingValue(value, 'data_model_query', DATA_MODEL_QUERY_DEFAULT_VALUE);
  const fields =
    (adapter.getValue('config.data_model_fields') as
      | AgentFlowDataModelFieldOption[]
      | null
      | undefined) ?? [];

  return (
    <DataModelQueryField
      ariaLabel={block.label}
      fields={fields}
      selectorOptions={getSelectorOptions(adapter)}
      value={binding}
      onChange={(nextValue) =>
        adapter.setValue(block.path, {
          kind: 'data_model_query',
          value: nextValue
        })
      }
    />
  );
}
```

- [x] **Step 5: Verify**

Run:

```bash
pnpm --dir web/app test -- node-schema-registry node-inspector
```

Expected: PASS.

- [x] **Step 6: Commit**

Run:

```bash
git add web/app/src/features/agent-flow/lib/node-definitions/types.ts web/app/src/features/agent-flow/schema/node-schema-fragments.ts web/app/src/features/agent-flow/lib/node-definitions/nodes/data-model/index.ts web/app/src/features/agent-flow/components/bindings/DataModelQueryField.tsx web/app/src/features/agent-flow/schema/agent-flow-field-renderers.tsx web/app/src/features/agent-flow/_tests/node-schema-registry.test.tsx web/app/src/features/agent-flow/_tests/node-inspector.test.tsx
git commit -m "feat: add data model query editor"
```
