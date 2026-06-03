import { DeleteOutlined, PlusOutlined } from '@ant-design/icons';
import { Button, Input, Select } from 'antd';
import type {
  FlowConditionComparator,
  FlowConditionExpressionDocument,
  FlowConditionGroupDocument,
  FlowConditionRuleDocument,
  FlowConditionValue
} from '@1flowbase/flow-schema';

import type { FlowSelectorOption } from '../../lib/selector-options';
import { SelectorField } from './SelectorField';
import {
  createEmptyConditionGroup,
  isConditionGroup,
  isConditionRule
} from '../../lib/if-else-branches';
import { i18nText } from '../../../../shared/i18n/text';

interface ConditionGroupFieldProps {
  ariaLabel: string;
  value: FlowConditionGroupDocument;
  options: FlowSelectorOption[];
  onChange: (value: FlowConditionGroupDocument) => void;
}

const COMPARATOR_OPTIONS = [
  {
    label: i18nText("agentFlow", "auto.value_comparison"),
    options: [
      { label: i18nText("agentFlow", "auto.exists"), value: 'exists' },
      { label: i18nText("agentFlow", "auto.empty"), value: 'empty' },
      { label: i18nText("agentFlow", "auto.equals"), value: 'equals' },
      { label: i18nText("agentFlow", "auto.not_equals"), value: 'not_equals' },
      { label: '>', value: 'greater_than' },
      { label: '>=', value: 'greater_than_or_equals' },
      { label: '<', value: 'less_than' },
      { label: '<=', value: 'less_than_or_equals' }
    ]
  },
  {
    label: i18nText("agentFlow", "auto.string"),
    options: [
      { label: i18nText("agentFlow", "auto.contains"), value: 'contains' },
      { label: i18nText("agentFlow", "auto.starts_with"), value: 'starts_with' },
      { label: i18nText("agentFlow", "auto.ends_with"), value: 'ends_with' },
      { label: i18nText("agentFlow", "auto.matches_regex"), value: 'matches_regex' }
    ]
  }
] satisfies Array<{
  label: string;
  options: Array<{ label: string; value: FlowConditionComparator }>;
}>;
const conditionRenderKeys = new WeakMap<
  FlowConditionExpressionDocument,
  string
>();
let nextConditionRenderKey = 0;

function defaultRule(): FlowConditionRuleDocument {
  return { kind: 'rule', left: [], comparator: 'exists' };
}

function getConditionRenderKey(condition: FlowConditionExpressionDocument) {
  const existingKey = conditionRenderKeys.get(condition);
  if (existingKey) {
    return existingKey;
  }

  nextConditionRenderKey += 1;
  const nextKey = `condition-${nextConditionRenderKey}`;
  conditionRenderKeys.set(condition, nextKey);
  return nextKey;
}

function conditionValueKind(value: FlowConditionValue | undefined) {
  return value?.kind ?? 'constant';
}

function conditionValueText(value: FlowConditionValue | undefined) {
  if (!value || value.kind !== 'constant') {
    return '';
  }

  return typeof value.value === 'string' ? value.value : String(value.value ?? '');
}

function ensureRightValue(
  rule: FlowConditionRuleDocument,
  comparator: FlowConditionComparator
): FlowConditionRuleDocument {
  if (comparator === 'exists' || comparator === 'empty') {
    const nextRule = { ...rule, comparator };

    delete nextRule.right;

    return nextRule;
  }

  return {
    ...rule,
    comparator,
    right: rule.right ?? { kind: 'constant', value: '' }
  };
}

function replaceCondition(
  group: FlowConditionGroupDocument,
  index: number,
  condition: FlowConditionExpressionDocument
) {
  return {
    ...group,
    conditions: group.conditions.map((entry, entryIndex) =>
      entryIndex === index ? condition : entry
    )
  };
}

function removeCondition(group: FlowConditionGroupDocument, index: number) {
  return {
    ...group,
    conditions: group.conditions.filter((_, entryIndex) => entryIndex !== index)
  };
}

function RightValueField({
  ariaLabel,
  condition,
  options,
  onChange
}: {
  ariaLabel: string;
  condition: FlowConditionRuleDocument;
  options: FlowSelectorOption[];
  onChange: (condition: FlowConditionRuleDocument) => void;
}) {
  if (condition.comparator === 'exists' || condition.comparator === 'empty') {
    return null;
  }

  const valueKind = conditionValueKind(condition.right);

  return (
    <>
      <Select
        aria-label={`${ariaLabel}-right-kind`}
        className="agent-flow-condition-group__right-kind"
        options={[
          { label: i18nText("agentFlow", "auto.fixed_value"), value: 'constant' },
          { label: i18nText("agentFlow", "auto.select_upstream_output"), value: 'selector' }
        ]}
        value={valueKind}
        onChange={(kind) =>
          onChange({
            ...condition,
            right:
              kind === 'selector'
                ? { kind: 'selector', selector: [] }
                : { kind: 'constant', value: '' }
          })
        }
      />
      {valueKind === 'selector' ? (
        <SelectorField
          ariaLabel={`${ariaLabel}-right-selector`}
          options={options}
          value={
            condition.right?.kind === 'selector' ? condition.right.selector : []
          }
          onChange={(nextValue) =>
            onChange({
              ...condition,
              right: { kind: 'selector', selector: nextValue as string[] }
            })
          }
        />
      ) : (
        <Input
          aria-label={`${ariaLabel}-right`}
          placeholder={i18nText("agentFlow", "auto.comparison_value")}
          value={conditionValueText(condition.right)}
          onChange={(event) =>
            onChange({
              ...condition,
              right: { kind: 'constant', value: event.target.value }
            })
          }
        />
      )}
    </>
  );
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
      conditions: [...value.conditions, defaultRule()]
    });
  }

  function appendGroup() {
    onChange({
      ...value,
      conditions: [...value.conditions, createEmptyConditionGroup()]
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
          icon={<PlusOutlined />}
          type="dashed"
          onClick={appendCondition}
        >
          {i18nText("agentFlow", "auto.new_conditions")}
        </Button>
        <Button
          className="agent-flow-condition-group__add"
          icon={<PlusOutlined />}
          type="dashed"
          onClick={appendGroup}
        >
          {i18nText("agentFlow", "auto.add_condition_group")}
        </Button>
      </div>
      {value.conditions.map((condition, index) => {
        if (isConditionGroup(condition)) {
          return (
            <div
              key={getConditionRenderKey(condition)}
              className="agent-flow-condition-group__nested"
            >
              <ConditionGroupField
                ariaLabel={`${ariaLabel}-${index}`}
                options={options}
                value={condition}
                onChange={(nextGroup) =>
                  onChange(replaceCondition(value, index, nextGroup))
                }
              />
              <Button
                aria-label={i18nText("agentFlow", "auto.delete_condition_group")}
                className="agent-flow-binding-row__delete"
                danger
                icon={<DeleteOutlined />}
                type="text"
                onClick={() => onChange(removeCondition(value, index))}
              />
            </div>
          );
        }

        const rule = isConditionRule(condition) ? condition : defaultRule();

        return (
          <div
            key={getConditionRenderKey(condition)}
            className="agent-flow-binding-row agent-flow-condition-group__rule"
          >
            <SelectorField
              ariaLabel={`${ariaLabel}-${index}-left`}
              options={options}
              value={rule.left}
              onChange={(nextValue) =>
                onChange(
                  replaceCondition(value, index, {
                    ...rule,
                    left: nextValue as string[]
                  })
                )
              }
            />
            <Select
              aria-label={`${ariaLabel}-${index}-comparator`}
              options={COMPARATOR_OPTIONS}
              value={rule.comparator}
              onChange={(comparator) =>
                onChange(
                  replaceCondition(
                    value,
                    index,
                    ensureRightValue(rule, comparator)
                  )
                )
              }
            />
            <RightValueField
              ariaLabel={`${ariaLabel}-${index}`}
              condition={rule}
              options={options}
              onChange={(nextRule) =>
                onChange(replaceCondition(value, index, nextRule))
              }
            />
            <Button
              aria-label={i18nText("agentFlow", "auto.delete_condition")}
              className="agent-flow-binding-row__delete"
              danger
              icon={<DeleteOutlined />}
              type="text"
              onClick={() => onChange(removeCondition(value, index))}
            />
          </div>
        );
      })}
    </div>
  );
}
