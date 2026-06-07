import { Button, Input, InputNumber, Select, Switch } from 'antd';

import type { FlowSelectorOption } from '../../lib/selector-options';
import type { AgentFlowEnvironmentVariable } from '../../lib/variables/application-environment-variables';
import { createTemplateSelectorToken } from '../../lib/template-binding';
import { TemplatedTextField } from './TemplatedTextField';
import { i18nText } from '../../../../shared/i18n/text';

export type EnvironmentVariableUpdateExpression =
  | { kind: 'selector'; selector: string[] }
  | { kind: 'constant'; value: unknown }
  | { kind: 'templated_text'; value: string };

export interface EnvironmentVariableUpdateValue {
  path: string[];
  operator: 'set' | 'append' | 'clear' | 'increment';
  source?: string[] | null;
  value?: EnvironmentVariableUpdateExpression | null;
}

interface EnvironmentVariableUpdateFieldProps {
  ariaLabel: string;
  value: EnvironmentVariableUpdateValue[];
  environmentVariables: AgentFlowEnvironmentVariable[];
  selectorOptions: FlowSelectorOption[];
  onChange: (value: EnvironmentVariableUpdateValue[]) => void;
}

function getTargetName(entry: EnvironmentVariableUpdateValue) {
  return entry.path[0] === 'env' ? entry.path[1] ?? '' : '';
}

function findEnvironmentVariable(
  environmentVariables: AgentFlowEnvironmentVariable[],
  entry: EnvironmentVariableUpdateValue
) {
  const targetName = getTargetName(entry);

  return environmentVariables.find((variable) => variable.name === targetName);
}

function isNumberValueType(valueType: string) {
  return valueType === 'number';
}

function isBooleanValueType(valueType: string) {
  return valueType === 'boolean';
}

function isStringValueType(valueType: string) {
  return valueType === 'string';
}

function createDefaultValueExpression(
  variable: AgentFlowEnvironmentVariable | undefined
): EnvironmentVariableUpdateExpression | null {
  if (!variable) {
    return null;
  }

  if (isStringValueType(variable.value_type)) {
    return { kind: 'templated_text', value: '' };
  }

  if (isNumberValueType(variable.value_type)) {
    return { kind: 'constant', value: 0 };
  }

  if (isBooleanValueType(variable.value_type)) {
    return { kind: 'constant', value: false };
  }

  return null;
}

function getTemplatedTextValue(entry: EnvironmentVariableUpdateValue) {
  if (entry.value?.kind === 'templated_text') {
    return entry.value.value;
  }

  if (entry.value?.kind === 'constant') {
    return entry.value.value === null || entry.value.value === undefined
      ? ''
      : String(entry.value.value);
  }

  if (entry.value?.kind === 'selector') {
    return createTemplateSelectorToken(entry.value.selector);
  }

  return entry.source ? createTemplateSelectorToken(entry.source) : '';
}

function getNumberValue(entry: EnvironmentVariableUpdateValue) {
  return entry.value?.kind === 'constant' &&
    typeof entry.value.value === 'number'
    ? entry.value.value
    : null;
}

function getBooleanValue(entry: EnvironmentVariableUpdateValue) {
  return entry.value?.kind === 'constant' &&
    typeof entry.value.value === 'boolean'
    ? entry.value.value
    : false;
}

export function EnvironmentVariableUpdateField({
  ariaLabel,
  value,
  environmentVariables,
  selectorOptions,
  onChange
}: EnvironmentVariableUpdateFieldProps) {
  const targetOptions = environmentVariables.map((variable) => ({
    label: `env.${variable.name}`,
    value: variable.name
  }));

  function updateEntry(
    index: number,
    updater: (
      entry: EnvironmentVariableUpdateValue
    ) => EnvironmentVariableUpdateValue
  ) {
    onChange(
      value.map((item, itemIndex) =>
        itemIndex === index ? updater(item) : item
      )
    );
  }

  function renderValueEditor(
    entry: EnvironmentVariableUpdateValue,
    index: number
  ) {
    const variable = findEnvironmentVariable(environmentVariables, entry);

    if (!variable) {
      return (
        <Input
          aria-label={`${ariaLabel}-${index}-value`}
          disabled
          value=""
          placeholder={i18nText("agentFlow", "auto.select_environment_variable")}
        />
      );
    }

    if (isStringValueType(variable.value_type)) {
      return (
        <TemplatedTextField
          ariaLabel={`${ariaLabel}-${index}-value`}
          displayMode="input"
          label={`env.${variable.name}`}
          options={selectorOptions}
          placeholder={i18nText(
            "agentFlow",
            "auto.support_text_variable_block_enter_left_curly_bracket_quick_reference"
          )}
          value={getTemplatedTextValue(entry)}
          onChange={(nextValue) =>
            updateEntry(index, (item) => ({
              ...item,
              operator: 'set',
              source: null,
              value: { kind: 'templated_text', value: nextValue }
            }))
          }
        />
      );
    }

    if (isNumberValueType(variable.value_type)) {
      return (
        <InputNumber
          aria-label={`${ariaLabel}-${index}-value`}
          value={getNumberValue(entry)}
          onChange={(nextValue) =>
            updateEntry(index, (item) => ({
              ...item,
              operator: 'set',
              source: null,
              value: { kind: 'constant', value: nextValue ?? 0 }
            }))
          }
        />
      );
    }

    if (isBooleanValueType(variable.value_type)) {
      return (
        <Switch
          aria-label={`${ariaLabel}-${index}-value`}
          checked={getBooleanValue(entry)}
          onChange={(checked) =>
            updateEntry(index, (item) => ({
              ...item,
              operator: 'set',
              source: null,
              value: { kind: 'constant', value: checked }
            }))
          }
        />
      );
    }

    return (
      <Input
        aria-label={`${ariaLabel}-${index}-value`}
        disabled
        value=""
        placeholder={i18nText(
          "agentFlow",
          "auto.environment_variable_update_type_not_available"
        )}
      />
    );
  }

  return (
    <div className="agent-flow-binding-list">
      {value.map((entry, index) => (
        <div
          key={`${getTargetName(entry)}-${index}`}
          className="agent-flow-environment-variable-update-row"
        >
          <Select
            aria-label={`${ariaLabel}-${index}-target`}
            options={targetOptions}
            placeholder={i18nText("agentFlow", "auto.select_environment_variable")}
            value={getTargetName(entry) || undefined}
            onChange={(targetName) =>
              updateEntry(index, (item) => {
                const variable = environmentVariables.find(
                  (candidate) => candidate.name === targetName
                );

                return {
                  ...item,
                  path: ['env', targetName],
                  operator: 'set',
                  source: null,
                  value: createDefaultValueExpression(variable)
                };
              })
            }
          />
          {renderValueEditor(entry, index)}
          <Button
            danger
            type="text"
            onClick={() =>
              onChange(value.filter((_, itemIndex) => itemIndex !== index))
            }
          >
            {i18nText("agentFlow", "auto.delete")}</Button>
        </div>
      ))}
      <Button
        type="dashed"
        onClick={() =>
          onChange([
            ...value,
            {
              path: ['env', ''],
              operator: 'set',
              source: null,
              value: null
            }
          ])
        }
      >
        {i18nText("agentFlow", "auto.add_environment_variable_update")}</Button>
    </div>
  );
}
