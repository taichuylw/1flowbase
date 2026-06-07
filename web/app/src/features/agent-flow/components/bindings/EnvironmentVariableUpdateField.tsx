import { Button, Select } from 'antd';

import type { FlowSelectorOption } from '../../lib/selector-options';
import type { AgentFlowEnvironmentVariable } from '../../lib/variables/application-environment-variables';
import { SelectorField } from './SelectorField';
import { i18nText } from '../../../../shared/i18n/text';

interface EnvironmentVariableUpdateValue {
  path: string[];
  operator: 'set' | 'append' | 'clear' | 'increment';
  source: string[] | null;
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
              onChange(
                value.map((item, itemIndex) =>
                  itemIndex === index
                    ? {
                        ...item,
                        path: ['env', targetName],
                        operator: 'set'
                      }
                    : item
                )
              )
            }
          />
          <SelectorField
            ariaLabel={`${ariaLabel}-${index}-source`}
            options={selectorOptions}
            value={entry.source ?? []}
            onChange={(nextValue) =>
              onChange(
                value.map((item, itemIndex) =>
                  itemIndex === index
                    ? {
                        ...item,
                        operator: 'set',
                        source:
                          (nextValue as string[]).length > 0
                            ? (nextValue as string[])
                            : null
                      }
                    : item
                )
              )
            }
          />
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
              source: null
            }
          ])
        }
      >
        {i18nText("agentFlow", "auto.add_environment_variable_update")}</Button>
    </div>
  );
}
