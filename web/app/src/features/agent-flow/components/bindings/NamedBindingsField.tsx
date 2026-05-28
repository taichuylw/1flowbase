import { DeleteOutlined } from '@ant-design/icons';
import { Button, Input, Select } from 'antd';

import type { FlowSelectorOption } from '../../lib/selector-options';
import { SelectorField } from './SelectorField';
import { i18nText } from '../../../../shared/i18n/text';

interface NamedBindingValue {
  name: string;
  selector: string[];
}

interface NamedBindingsFieldProps {
  ariaLabel: string;
  value: NamedBindingValue[];
  options: FlowSelectorOption[];
  nameOptions?: Array<{ value: string; label: string; disabled?: boolean }>;
  namePlaceholder?: string;
  selectorLabel?: string;
  addButtonLabel?: string;
  onChange: (value: NamedBindingValue[]) => void;
}

export function NamedBindingsField({
  ariaLabel,
  value,
  options,
  nameOptions,
  namePlaceholder = i18nText("agentFlow", "auto.key_gdnfjhhnog"),
  selectorLabel = 'selector',
  addButtonLabel = i18nText("agentFlow", "auto.key_libkhndodm"),
  onChange
}: NamedBindingsFieldProps) {
  return (
    <div className="agent-flow-binding-list">
      {value.map((entry, index) => (
        <div key={`${entry.name}-${index}`} className="agent-flow-binding-row">
          <div className="agent-flow-binding-row__name">
            {nameOptions ? (
              <Select
                aria-label={`${ariaLabel}-${index}-field`}
                options={nameOptions}
                placeholder={namePlaceholder}
                value={entry.name || undefined}
                onChange={(nextName) =>
                  onChange(
                    value.map((item, itemIndex) =>
                      itemIndex === index ? { ...item, name: nextName } : item
                    )
                  )
                }
              />
            ) : (
              <Input
                aria-label={`${ariaLabel}-${index}-name`}
                placeholder={namePlaceholder}
                value={entry.name}
                onChange={(event) =>
                  onChange(
                    value.map((item, itemIndex) =>
                      itemIndex === index
                        ? { ...item, name: event.target.value }
                        : item
                    )
                  )
                }
              />
            )}
          </div>
          <div className="agent-flow-binding-row__selector">
            <SelectorField
              ariaLabel={`${ariaLabel}-${index}-${selectorLabel}`}
              options={options}
              value={entry.selector}
              onChange={(nextValue) =>
                onChange(
                  value.map((item, itemIndex) =>
                    itemIndex === index
                      ? { ...item, selector: nextValue as string[] }
                      : item
                  )
                )
              }
            />
          </div>
          <Button
            aria-label={i18nText("agentFlow", "auto.key_ekigejjmna", { value1: entry.name || index + 1 })}
            className="agent-flow-binding-row__delete"
            danger
            icon={<DeleteOutlined />}
            size="small"
            type="text"
            onClick={() =>
              onChange(value.filter((_, itemIndex) => itemIndex !== index))
            }
          />
        </div>
      ))}
      <Button
        type="dashed"
        onClick={() => onChange([...value, { name: '', selector: [] }])}
      >
        {addButtonLabel}
      </Button>
    </div>
  );
}
