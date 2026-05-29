import type { FlowNodeDocument } from '@1flowbase/flow-schema';
import { Input, InputNumber, Select, Switch } from 'antd';

import type {
  SchemaFieldRenderer,
  SchemaFieldRendererProps
} from '../../../shared/schema-ui/registry/create-renderer-registry';

import type { AgentFlowDataModelFieldOption } from '../api/data-model-options';
import { ConditionGroupField } from '../components/bindings/ConditionGroupField';
import { DataModelQueryField } from '../components/bindings/DataModelQueryField';
import { NamedBindingsField } from '../components/bindings/NamedBindingsField';
import { SelectorField } from '../components/bindings/SelectorField';
import { StateWriteField } from '../components/bindings/StateWriteField';
import { TemplatedTextField } from '../components/bindings/TemplatedTextField';
import {
  TemplatedNamedBindingsField,
  type TemplatedNamedBindingValue
} from '../components/bindings/TemplatedNamedBindingsField';
import { OutputContractDefinitionField } from '../components/detail/fields/OutputContractDefinitionField';
import { CodeSourceField } from '../components/detail/fields/CodeSourceField';
import { DataModelField } from '../components/detail/fields/DataModelField';
import { LlmModelField } from '../components/detail/fields/LlmModelField';
import { LlmPromptMessagesField } from '../components/detail/fields/LlmPromptMessagesField';
import { LlmResponseFormatField } from '../components/detail/fields/LlmResponseFormatField';
import { StartInputFieldsField } from '../components/detail/fields/StartInputFieldsField';
import { StartModelListField } from '../components/detail/fields/StartModelListField';
import {
  DATA_MODEL_QUERY_DEFAULT_VALUE,
  normalizeDataModelQueryBindingValue
} from '../lib/data-model-query-binding';
import {
  normalizePromptMessagesBinding,
  toPromptMessagesBinding
} from '../lib/llm-prompt-messages';
import { getLlmContextPolicy } from '../lib/llm-node-config';
import type { FlowSelectorOption } from '../lib/selector-options';
import { createTemplateSelectorToken } from '../lib/template-binding';
import { i18nText } from '../../../shared/i18n/text';

function getSelectorOptions(adapter: SchemaFieldRendererProps['adapter']) {
  return (
    (adapter.getDerived('selectorOptions') as
      | FlowSelectorOption[]
      | null
      | undefined) ?? []
  );
}

function hasBindingKind(
  value: unknown,
  kind: string
): value is { kind: string; value: unknown } {
  return (
    typeof value === 'object' &&
    value !== null &&
    'kind' in value &&
    'value' in value &&
    (value as { kind?: unknown }).kind === kind
  );
}

function getBindingValue<T>(value: unknown, kind: string, fallback: T): T {
  return hasBindingKind(value, kind) ? (value.value as T) : fallback;
}

function renderTextField({ adapter, block }: SchemaFieldRendererProps) {
  const value = adapter.getValue(block.path);

  return (
    <Input
      aria-label={block.label}
      value={typeof value === 'string' ? value : ''}
      onChange={(event) => adapter.setValue(block.path, event.target.value)}
    />
  );
}

function renderNumberField({ adapter, block }: SchemaFieldRendererProps) {
  const value = adapter.getValue(block.path);

  return (
    <InputNumber
      aria-label={block.label}
      className="agent-flow-editor__number-field"
      value={typeof value === 'number' && Number.isFinite(value) ? value : null}
      onChange={(nextValue) => adapter.setValue(block.path, nextValue)}
    />
  );
}

function renderStaticSelectField({ adapter, block }: SchemaFieldRendererProps) {
  const value = adapter.getValue(block.path);

  return (
    <Select
      aria-label={block.label}
      options={block.options ?? []}
      value={typeof value === 'string' ? value : undefined}
      onChange={(nextValue) => adapter.setValue(block.path, nextValue)}
    />
  );
}

function renderSelectorField({ adapter, block }: SchemaFieldRendererProps) {
  const value = adapter.getValue(block.path);
  const binding = getBindingValue<string[]>(value, 'selector', []);

  return (
    <SelectorField
      ariaLabel={block.label}
      options={getSelectorOptions(adapter)}
      value={binding}
      onChange={(nextValue) =>
        adapter.setValue(block.path, {
          kind: 'selector',
          value: nextValue as string[]
        })
      }
    />
  );
}

function renderSelectorListField({ adapter, block }: SchemaFieldRendererProps) {
  const value = adapter.getValue(block.path);
  const binding = getBindingValue<string[][]>(value, 'selector_list', []);

  return (
    <SelectorField
      ariaLabel={block.label}
      multiple
      options={getSelectorOptions(adapter)}
      value={binding}
      onChange={(nextValue) =>
        adapter.setValue(block.path, {
          kind: 'selector_list',
          value: nextValue as string[][]
        })
      }
    />
  );
}

function renderTemplatedTextField({
  adapter,
  block
}: SchemaFieldRendererProps) {
  const value = adapter.getValue(block.path);
  const selectorOptions = getSelectorOptions(adapter);
  const isBindingPath = block.path.startsWith('bindings.');
  const stringValue = isBindingPath
    ? hasBindingKind(value, 'templated_text')
      ? getBindingValue<string>(value, 'templated_text', '')
      : hasBindingKind(value, 'selector')
        ? createTemplateSelectorToken(
            getBindingValue<string[]>(value, 'selector', [])
          )
        : ''
    : typeof value === 'string'
      ? value
      : '';

  return (
    <TemplatedTextField
      label={block.label}
      ariaLabel={block.label}
      placeholder={i18nText("agentFlow", "auto.support_text_variable_block_enter_left_curly_bracket_quick_reference")}
      options={selectorOptions}
      value={stringValue}
      onChange={(nextValue) =>
        adapter.setValue(
          block.path,
          isBindingPath
            ? { kind: 'templated_text', value: nextValue }
            : nextValue
        )
      }
    />
  );
}

function renderLlmPromptMessagesField({
  adapter,
  block
}: SchemaFieldRendererProps) {
  const contextPolicy = getLlmContextPolicy({
    context_policy: adapter.getValue('config.context_policy')
  });
  const messages = normalizePromptMessagesBinding(
    adapter.getValue(block.path),
    adapter.getValue('bindings.system_prompt'),
    adapter.getValue('bindings.user_prompt')
  );

  return (
    <LlmPromptMessagesField
      integrationContextEnabled={
        contextPolicy.integration_context === 'enabled'
      }
      options={getSelectorOptions(adapter)}
      value={messages}
      onChange={(nextValue) =>
        adapter.setValue(block.path, toPromptMessagesBinding(nextValue))
      }
    />
  );
}

function renderLlmContextPolicyField({
  adapter,
  block
}: SchemaFieldRendererProps) {
  const contextPolicy = getLlmContextPolicy({
    context_policy: adapter.getValue(block.path)
  });

  return (
    <Switch
      aria-label={block.label}
      checked={contextPolicy.integration_context === 'enabled'}
      onChange={(checked) =>
        adapter.setValue(block.path, {
          integration_context: checked ? 'enabled' : 'disabled'
        })
      }
    />
  );
}

function renderNamedBindingsField({
  adapter,
  block
}: SchemaFieldRendererProps) {
  const value = adapter.getValue(block.path);
  const binding = getBindingValue<Array<{ name: string; selector: string[] }>>(
    value,
    'named_bindings',
    []
  );
  const fields =
    (adapter.getValue('config.data_model_fields') as
      | AgentFlowDataModelFieldOption[]
      | null
      | undefined) ?? [];
  const isDataModelPayload = block.path === 'bindings.payload';
  const nameOptions = isDataModelPayload
    ? fields.filter((field) => field.writable !== false).map((field) => ({
        value: field.code,
        label: field.title || field.code
      }))
    : undefined;

  return (
    <NamedBindingsField
      ariaLabel={block.label}
      options={getSelectorOptions(adapter)}
      value={binding}
      nameOptions={nameOptions}
      namePlaceholder={isDataModelPayload ? i18nText("agentFlow", "auto.field") : undefined}
      selectorLabel={isDataModelPayload ? 'variable' : undefined}
      addButtonLabel={isDataModelPayload ? i18nText("agentFlow", "auto.add_new_field_assignment") : undefined}
      onChange={(nextValue) =>
        adapter.setValue(block.path, {
          kind: 'named_bindings',
          value: nextValue
        })
      }
    />
  );
}

function normalizeTemplatedNamedBindingEntries(value: unknown) {
  return getBindingValue<
    Array<{
      name: string;
      selector?: string[];
      content?: { kind: 'templated_text'; value: string };
    }>
  >(value, 'named_bindings', []).map((entry) => ({
    name: entry.name,
    selector: entry.selector,
    content:
      entry.content?.kind === 'templated_text'
        ? entry.content
        : {
            kind: 'templated_text' as const,
            value: createTemplateSelectorToken(entry.selector ?? [])
          }
  }));
}

function renderTemplatedNamedBindingsField({
  adapter,
  block
}: SchemaFieldRendererProps) {
  const value = adapter.getValue(block.path);
  const binding = normalizeTemplatedNamedBindingEntries(value);

  return (
    <TemplatedNamedBindingsField
      ariaLabel={block.label}
      options={getSelectorOptions(adapter)}
      value={binding}
      onChange={(nextValue: TemplatedNamedBindingValue[]) =>
        adapter.setValue(block.path, {
          kind: 'named_bindings',
          value: nextValue
        })
      }
    />
  );
}

function renderConditionGroupField({
  adapter,
  block
}: SchemaFieldRendererProps) {
  const value = adapter.getValue(block.path);
  const binding = getBindingValue<{
    operator: 'and' | 'or';
    conditions: Array<unknown>;
  }>(value, 'condition_group', { operator: 'and', conditions: [] });

  return (
    <ConditionGroupField
      ariaLabel={block.label}
      options={getSelectorOptions(adapter)}
      value={binding as never}
      onChange={(nextValue) =>
        adapter.setValue(block.path, {
          kind: 'condition_group',
          value: nextValue
        })
      }
    />
  );
}

function renderStateWriteField({ adapter, block }: SchemaFieldRendererProps) {
  const value = adapter.getValue(block.path);
  const binding = getBindingValue<
    Array<{ path: string[]; operator: string; source: string[] | null }>
  >(value, 'state_write', []);

  return (
    <StateWriteField
      ariaLabel={block.label}
      options={getSelectorOptions(adapter)}
      value={binding as never}
      onChange={(nextValue) =>
        adapter.setValue(block.path, {
          kind: 'state_write',
          value: nextValue
        })
      }
    />
  );
}

function renderOutputContractDefinitionField({
  adapter,
  block
}: SchemaFieldRendererProps) {
  const value = adapter.getValue(block.path);
  const outputs = Array.isArray(value)
    ? (value as FlowNodeDocument['outputs'])
    : [];

  return (
    <OutputContractDefinitionField
      value={outputs}
      onChange={(nextValue) => adapter.setValue(block.path, nextValue)}
    />
  );
}

function renderCodeSourceField({ adapter, block }: SchemaFieldRendererProps) {
  return (
    <CodeSourceField
      label={block.label}
      value={adapter.getValue(block.path)}
      onChange={(nextValue) => adapter.setValue(block.path, nextValue)}
    />
  );
}

function renderStartInputFieldsField({
  adapter,
  block
}: SchemaFieldRendererProps) {
  return (
    <StartInputFieldsField
      value={adapter.getValue(block.path)}
      onChange={(nextValue) => adapter.setValue(block.path, nextValue)}
    />
  );
}

function renderStartModelListField({
  adapter,
  block
}: SchemaFieldRendererProps) {
  return (
    <StartModelListField
      value={adapter.getValue(block.path)}
      onChange={(nextValue) => adapter.setValue(block.path, nextValue)}
    />
  );
}

function renderDataModelQueryField({
  adapter,
  block
}: SchemaFieldRendererProps) {
  const value = adapter.getValue(block.path);
  const binding = normalizeDataModelQueryBindingValue(
    getBindingValue(
      value,
      'data_model_query',
      DATA_MODEL_QUERY_DEFAULT_VALUE
    )
  );
  const fields =
    (adapter.getValue('config.data_model_fields') as
      | AgentFlowDataModelFieldOption[]
      | null
      | undefined) ?? [];
  const dataModelCode = adapter.getValue('config.data_model_code');

  return (
    <DataModelQueryField
      ariaLabel={block.label}
      hasDataModelSelected={
        typeof dataModelCode === 'string' && dataModelCode.trim().length > 0
      }
      fields={fields}
      selectorOptions={getSelectorOptions(adapter)}
      value={binding}
      includePagination={
        (adapter.getDerived('node') as FlowNodeDocument | null | undefined)
          ?.type === 'data_model_list'
      }
      onChange={(nextValue) =>
        adapter.setValue(block.path, {
          kind: 'data_model_query',
          value: normalizeDataModelQueryBindingValue(nextValue)
        })
      }
    />
  );
}

export const agentFlowFieldRenderers = {
  text: renderTextField,
  static_select: renderStaticSelectField,
  data_model: DataModelField,
  data_model_query: renderDataModelQueryField,
  code_source: renderCodeSourceField,
  llm_model: LlmModelField,
  llm_context_policy: renderLlmContextPolicyField,
  llm_prompt_messages: renderLlmPromptMessagesField,
  llm_response_format: LlmResponseFormatField,
  number: renderNumberField,
  selector: renderSelectorField,
  selector_list: renderSelectorListField,
  templated_text: renderTemplatedTextField,
  named_bindings: renderNamedBindingsField,
  templated_named_bindings: renderTemplatedNamedBindingsField,
  condition_group: renderConditionGroupField,
  state_write: renderStateWriteField,
  output_contract_definition: renderOutputContractDefinitionField,
  start_input_fields: renderStartInputFieldsField,
  start_model_list: renderStartModelListField,
  header_alias: ({ adapter, block }) => {
    const value = adapter.getValue(block.path);

    return (
      <Input
        aria-label={block.label}
        className="agent-flow-editor__inspector-title-input"
        value={typeof value === 'string' ? value : ''}
        onChange={(event) => adapter.setValue(block.path, event.target.value)}
      />
    );
  },
  header_description: ({ adapter, block }) => {
    const value = adapter.getValue(block.path);

    return (
      <Input.TextArea
        aria-label={block.label}
        autoSize={{ minRows: 1, maxRows: 3 }}
        className="agent-flow-editor__inspector-description-input"
        placeholder={i18nText("agentFlow", "auto.add_description")}
        value={typeof value === 'string' ? value : ''}
        onChange={(event) => adapter.setValue(block.path, event.target.value)}
      />
    );
  }
} satisfies Record<string, SchemaFieldRenderer>;
