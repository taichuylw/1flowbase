import { Button, Input, Select } from 'antd';

import type { FlowSelectorOption } from '../../lib/selector-options';
import { SelectorField } from './SelectorField';
import { i18nText } from '../../../../shared/i18n/text';

interface ConditionGroupFieldValue {
  operator: 'and' | 'or';
  conditions: Array<{
    left: string[];
    comparator: 'exists' | 'equals' | 'contains';
    right?: string | string[];
  }>;
}

interface ConditionGroupFieldProps {
  ariaLabel: string;
  value: ConditionGroupFieldValue;
  options: FlowSelectorOption[];
  onChange: (value: ConditionGroupFieldValue) => void;
}

export function ConditionGroupField({
  ariaLabel,
  value,
  options,
  onChange
}: ConditionGroupFieldProps) {
  function appendCondition() {
    onChange({
      ...value,
      conditions: [...value.conditions, { left: [], comparator: 'exists' }]
    });
  }

  return (
    <div className="agent-flow-binding-list agent-flow-condition-group">
      <div
        className="agent-flow-condition-group__toolbar"
        data-testid="condition-group-toolbar"
      >
        <Select
          aria-label={`${ariaLabel}-operator`}
          className="agent-flow-condition-group__operator"
          options={[
            { label: 'AND', value: 'and' },
            { label: 'OR', value: 'or' }
          ]}
          value={value.operator}
          onChange={(operator) =>
            onChange({
              ...value,
              operator: operator as 'and' | 'or'
            })
          }
        />
        <Button
          className="agent-flow-condition-group__add"
          type="dashed"
          onClick={appendCondition}
        >
          {i18nText("agentFlow", "auto.k_a20ad5803b")}</Button>
      </div>
      {value.conditions.map((condition, index) => (
        <div key={`${condition.comparator}-${index}`} className="agent-flow-binding-row">
          <SelectorField
            ariaLabel={`${ariaLabel}-${index}-left`}
            options={options}
            value={condition.left}
            onChange={(nextValue) =>
              onChange({
                ...value,
                conditions: value.conditions.map((item, itemIndex) =>
                  itemIndex === index
                    ? { ...item, left: nextValue as string[] }
                    : item
                )
              })
            }
          />
          <Select
            aria-label={`${ariaLabel}-${index}-comparator`}
            options={[
              { label: 'Exists', value: 'exists' },
              { label: 'Equals', value: 'equals' },
              { label: 'Contains', value: 'contains' }
            ]}
            value={condition.comparator}
            onChange={(comparator) =>
              onChange({
                ...value,
                conditions: value.conditions.map((item, itemIndex) =>
                  itemIndex === index
                    ? {
                        ...item,
                        comparator: comparator as 'exists' | 'equals' | 'contains'
                      }
                    : item
                )
              })
            }
          />
          {condition.comparator === 'exists' ? null : (
            <Input
              aria-label={`${ariaLabel}-${index}-right`}
              placeholder={i18nText("agentFlow", "auto.k_f4643126b8")}
              value={
                Array.isArray(condition.right)
                  ? condition.right.join(' / ')
                  : (condition.right ?? '')
              }
              onChange={(event) =>
                onChange({
                  ...value,
                  conditions: value.conditions.map((item, itemIndex) =>
                    itemIndex === index
                      ? { ...item, right: event.target.value }
                      : item
                  )
                })
              }
            />
          )}
          <Button
            danger
            type="text"
            onClick={() =>
              onChange({
                ...value,
                conditions: value.conditions.filter((_, itemIndex) => itemIndex !== index)
              })
            }
          >
            {i18nText("agentFlow", "auto.k_3755f56f2f")}</Button>
        </div>
      ))}
    </div>
  );
}
