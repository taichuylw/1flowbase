import { DeleteOutlined } from '@ant-design/icons';
import {
  Button,
  Input,
  InputNumber,
  Segmented,
  Select,
  Switch
} from 'antd';
import type {
  NamedBindingEntry,
  NamedBindingExpression
} from '@1flowbase/flow-schema';

import type { FlowSelectorOption } from '../../lib/selector-options';
import {
  getNamedBindingExpression,
  isNamedBindingNameAllowed
} from '../../lib/named-binding-expressions';
import { parseTemplateSelectorTokens } from '../../lib/template-binding';
import { SelectorField } from './SelectorField';
import { TemplatedTextField } from './TemplatedTextField';
import { i18nText } from '../../../../shared/i18n/text';

export type TemplatedNamedBindingValue = NamedBindingEntry;

type ParameterValueType =
  | 'string'
  | 'number'
  | 'boolean'
  | 'object'
  | 'array'
  | 'json'
  | 'unknown';

interface TemplatedNamedBindingsFieldProps {
  ariaLabel: string;
  value: TemplatedNamedBindingValue[];
  options: FlowSelectorOption[];
  onChange: (value: TemplatedNamedBindingValue[]) => void;
}

const parameterValueTypeOptions: Array<{
  label: string;
  value: ParameterValueType;
}> = [
  { label: i18nText("agentFlow", "auto.string"), value: 'string' },
  { label: i18nText("agentFlow", "auto.number"), value: 'number' },
  { label: i18nText("agentFlow", "auto.boolean"), value: 'boolean' },
  { label: i18nText("agentFlow", "auto.object"), value: 'object' },
  { label: i18nText("agentFlow", "auto.array"), value: 'array' },
  { label: 'JSON', value: 'json' },
  { label: i18nText("agentFlow", "auto.unknown"), value: 'unknown' }
];

function isParameterNameValid(name: string) {
  return name.length === 0 || isNamedBindingNameAllowed(name);
}

function normalizeParameterValueType(value: string | undefined) {
  if (!value) {
    return undefined;
  }

  if (value.startsWith('array')) {
    return 'array' as const;
  }

  if (
    value === 'string' ||
    value === 'number' ||
    value === 'boolean' ||
    value === 'object' ||
    value === 'array' ||
    value === 'json' ||
    value === 'unknown'
  ) {
    return value;
  }

  return 'unknown' as const;
}

function findSelectorOption(options: FlowSelectorOption[], selector: string[]) {
  return options.find(
    (option) =>
      option.value.length === selector.length &&
      option.value.every((segment, index) => selector[index] === segment)
  );
}

function inferValueTypeFromExpression(
  expression: NamedBindingExpression | null,
  options: FlowSelectorOption[]
) {
  if (!expression) {
    return undefined;
  }

  if (expression.kind === 'selector') {
    return normalizeParameterValueType(
      findSelectorOption(options, expression.selector)?.valueType
    );
  }

  if (expression.kind === 'templated_text') {
    return 'string' as const;
  }

  const constantValue = expression.value;

  if (typeof constantValue === 'string') {
    return 'string' as const;
  }

  if (typeof constantValue === 'number') {
    return 'number' as const;
  }

  if (typeof constantValue === 'boolean') {
    return 'boolean' as const;
  }

  if (Array.isArray(constantValue)) {
    return 'array' as const;
  }

  if (typeof constantValue === 'object' && constantValue !== null) {
    return 'object' as const;
  }

  return undefined;
}

function defaultConstantValue(valueType: ParameterValueType | undefined) {
  switch (valueType) {
    case 'number':
      return 0;
    case 'boolean':
      return false;
    case 'array':
      return [];
    case 'object':
    case 'json':
      return {};
    case 'string':
    case 'unknown':
    default:
      return '';
  }
}

function createConstantExpression(
  valueType: ParameterValueType | undefined
): NamedBindingExpression {
  return {
    kind: 'constant',
    value: defaultConstantValue(valueType)
  };
}

function expressionToText(
  expression: NamedBindingExpression | null,
  valueType: ParameterValueType | undefined
) {
  if (expression?.kind === 'templated_text') {
    return expression.value;
  }

  if (expression?.kind !== 'constant') {
    return '';
  }

  if (typeof expression.value === 'string') {
    return expression.value;
  }

  if (valueType === 'object' || valueType === 'array' || valueType === 'json') {
    return JSON.stringify(expression.value, null, 2);
  }

  return String(expression.value ?? '');
}

function parseJsonConstant(
  nextValue: string,
  valueType: ParameterValueType | undefined
) {
  try {
    const parsed = JSON.parse(nextValue);

    if (valueType === 'array' && !Array.isArray(parsed)) {
      return nextValue;
    }

    if (
      valueType === 'object' &&
      (typeof parsed !== 'object' || parsed === null || Array.isArray(parsed))
    ) {
      return nextValue;
    }

    return parsed;
  } catch {
    return nextValue;
  }
}

function isJsonConstantInvalid(
  expression: NamedBindingExpression | null,
  valueType: ParameterValueType | undefined
) {
  if (
    expression?.kind !== 'constant' ||
    (valueType !== 'object' && valueType !== 'array' && valueType !== 'json') ||
    typeof expression.value !== 'string'
  ) {
    return false;
  }

  return parseJsonConstant(expression.value, valueType) === expression.value;
}

function optionMatchesValueType(
  option: FlowSelectorOption,
  valueType: ParameterValueType | undefined
) {
  if (!valueType) {
    return true;
  }

  const optionValueType = normalizeParameterValueType(option.valueType);

  if (!optionValueType) {
    return false;
  }

  if (optionValueType === 'json') {
    return (
      valueType === 'json' ||
      valueType === 'object' ||
      valueType === 'array'
    );
  }

  return optionValueType === valueType;
}

export function TemplatedNamedBindingsField({
  ariaLabel,
  value,
  options,
  onChange
}: TemplatedNamedBindingsFieldProps) {
  function updateEntry(index: number, nextEntry: TemplatedNamedBindingValue) {
    onChange(
      value.map((entry, entryIndex) =>
        entryIndex === index ? nextEntry : entry
      )
    );
  }

  function renderLiteralEditor(
    entry: TemplatedNamedBindingValue,
    index: number,
    expression: NamedBindingExpression | null,
    valueType: ParameterValueType | undefined,
    entryLabel: string
  ) {
    if (valueType === 'number') {
      return (
        <InputNumber
          aria-label={`${ariaLabel}-${index}-value`}
          value={
            expression?.kind === 'constant' &&
            typeof expression.value === 'number'
              ? expression.value
              : null
          }
          onChange={(nextValue) =>
            updateEntry(index, {
              ...entry,
              valueType,
              value: {
                kind: 'constant',
                value: typeof nextValue === 'number' ? nextValue : null
              }
            })
          }
        />
      );
    }

    if (valueType === 'boolean') {
      return (
        <Switch
          aria-label={`${ariaLabel}-${index}-value`}
          checked={
            expression?.kind === 'constant' &&
            typeof expression.value === 'boolean'
              ? expression.value
              : false
          }
          onChange={(checked) =>
            updateEntry(index, {
              ...entry,
              valueType,
              value: { kind: 'constant', value: checked }
            })
          }
        />
      );
    }

    if (
      valueType === 'object' ||
      valueType === 'array' ||
      valueType === 'json'
    ) {
      return (
        <Input.TextArea
          aria-label={`${ariaLabel}-${index}-value`}
          autoSize={{ minRows: 2, maxRows: 6 }}
          status={
            isJsonConstantInvalid(expression, valueType) ? 'error' : undefined
          }
          value={expressionToText(expression, valueType)}
          onChange={(event) =>
            updateEntry(index, {
              ...entry,
              valueType,
              value: {
                kind: 'constant',
                value: parseJsonConstant(event.target.value, valueType)
              }
            })
          }
        />
      );
    }

    return (
      <TemplatedTextField
        ariaLabel={`${ariaLabel}-${index}-value`}
        displayMode="input"
        label={entryLabel}
        options={options}
        placeholder={i18nText(
          "agentFlow",
          "auto.enter_text_enter_reference_variable"
        )}
        value={expressionToText(expression, valueType)}
        onChange={(nextValue) =>
          updateEntry(index, {
            ...entry,
            valueType: valueType ?? 'string',
            value:
              parseTemplateSelectorTokens(nextValue).length > 0
                ? { kind: 'templated_text', value: nextValue }
                : { kind: 'constant', value: nextValue }
          })
        }
      />
    );
  }

  return (
    <div className="agent-flow-templated-binding-list">
      {value.map((entry, index) => {
        const entryLabel =
          entry.name ||
          i18nText("agentFlow", "auto.variable", { value1: index + 1 });
        const expression = getNamedBindingExpression(entry);
        const valueType =
          normalizeParameterValueType(entry.valueType) ??
          inferValueTypeFromExpression(expression, options);
        const valueMode =
          expression?.kind === 'selector' ? 'selector' : 'constant';
        const selectorOptions = options.filter((option) =>
          optionMatchesValueType(option, valueType)
        );

        return (
          <div
            key={`${entry.name}-${index}`}
            className="agent-flow-templated-binding-row"
          >
            <div className="agent-flow-templated-binding-row__name">
              <Input
                aria-label={`${ariaLabel}-${index}-name`}
                placeholder={i18nText("agentFlow", "auto.variable_name")}
                status={
                  isParameterNameValid(entry.name) ? undefined : 'error'
                }
                value={entry.name}
                onChange={(event) =>
                  updateEntry(index, { ...entry, name: event.target.value })
                }
              />
            </div>
            <div className="agent-flow-templated-binding-row__type">
              <Select
                allowClear
                aria-label={`${ariaLabel}-${index}-type`}
                options={parameterValueTypeOptions}
                placeholder={i18nText("agentFlow", "auto.please_select_type")}
                value={valueType}
                onChange={(nextValue) => {
                  const nextValueType = normalizeParameterValueType(nextValue);
                  const matchedOption =
                    expression?.kind === 'selector'
                      ? findSelectorOption(options, expression.selector)
                      : undefined;

                  updateEntry(index, {
                    ...entry,
                    valueType: nextValueType,
                    value:
                      expression?.kind === 'selector' &&
                      matchedOption &&
                      optionMatchesValueType(matchedOption, nextValueType)
                        ? expression
                        : createConstantExpression(nextValueType)
                  });
                }}
              />
            </div>
            <div className="agent-flow-templated-binding-row__value">
              <div className="agent-flow-templated-binding-row__value-mode">
                <Segmented
                  aria-label={`${ariaLabel}-${index}-value-mode`}
                  options={[
                    {
                      label: i18nText("agentFlow", "auto.value"),
                      value: 'constant'
                    },
                    {
                      label: i18nText("agentFlow", "auto.variable_alt"),
                      value: 'selector'
                    }
                  ]}
                  size="small"
                  value={valueMode}
                  onChange={(nextMode) =>
                    updateEntry(index, {
                      ...entry,
                      value:
                        nextMode === 'selector'
                          ? { kind: 'selector', selector: [] }
                          : createConstantExpression(valueType)
                    })
                  }
                />
              </div>
              {valueMode === 'selector' ? (
                <SelectorField
                  ariaLabel={`${ariaLabel}-${index}-value`}
                  options={selectorOptions}
                  value={
                    expression?.kind === 'selector' ? expression.selector : []
                  }
                  onChange={(nextSelector) => {
                    const selector = Array.isArray(nextSelector[0])
                      ? []
                      : (nextSelector as string[]);
                    const matchedOption = findSelectorOption(
                      options,
                      selector
                    );

                    updateEntry(index, {
                      ...entry,
                      valueType:
                        valueType ??
                        normalizeParameterValueType(matchedOption?.valueType),
                      value: { kind: 'selector', selector }
                    });
                  }}
                />
              ) : (
                renderLiteralEditor(
                  entry,
                  index,
                  expression,
                  valueType,
                  entryLabel
                )
              )}
            </div>
            <Button
              aria-label={i18nText("agentFlow", "auto.delete_variable", {
                value1: entry.name || index + 1
              })}
              className="agent-flow-templated-binding-row__delete"
              danger
              icon={<DeleteOutlined />}
              size="small"
              type="text"
              onClick={() =>
                onChange(value.filter((_, itemIndex) => itemIndex !== index))
              }
            />
          </div>
        );
      })}
      <Button
        type="dashed"
        onClick={() =>
          onChange([
            ...value,
            {
              name: '',
              valueType: 'string',
              value: { kind: 'constant', value: '' }
            }
          ])
        }
      >
        {i18nText("agentFlow", "auto.add_new_variable")}
      </Button>
    </div>
  );
}
