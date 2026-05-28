import { DeleteOutlined } from '@ant-design/icons';
import { Button, Input } from 'antd';

import type { FlowSelectorOption } from '../../lib/selector-options';
import { TemplatedTextField } from './TemplatedTextField';
import { i18nText } from '../../../../shared/i18n/text';

export interface TemplatedNamedBindingValue {
  name: string;
  content: {
    kind: 'templated_text';
    value: string;
  };
  selector?: string[];
}

interface TemplatedNamedBindingsFieldProps {
  ariaLabel: string;
  value: TemplatedNamedBindingValue[];
  options: FlowSelectorOption[];
  onChange: (value: TemplatedNamedBindingValue[]) => void;
}

export function TemplatedNamedBindingsField({
  ariaLabel,
  value,
  options,
  onChange
}: TemplatedNamedBindingsFieldProps) {
  return (
    <div className="agent-flow-templated-binding-list">
      {value.map((entry, index) => {
        const entryLabel = entry.name || i18nText("agentFlow", "auto.k_9fdebca240", { value1: index + 1 });

        return (
          <div
            key={`${entry.name}-${index}`}
            className="agent-flow-templated-binding-row"
          >
            <div className="agent-flow-templated-binding-row__name">
              <Input
                aria-label={`${ariaLabel}-${index}-name`}
                placeholder={i18nText("agentFlow", "auto.k_63d5977de6")}
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
            </div>
            <div className="agent-flow-templated-binding-row__value">
              <TemplatedTextField
                ariaLabel={`${ariaLabel}-${index}-value`}
                displayMode="input"
                label={entryLabel}
                options={options}
                placeholder={i18nText("agentFlow", "auto.k_faa6bb45af")}
                value={entry.content.value}
                onChange={(nextValue) =>
                  onChange(
                    value.map((item, itemIndex) =>
                      itemIndex === index
                        ? {
                            ...item,
                            content: {
                              kind: 'templated_text',
                              value: nextValue
                            }
                          }
                        : item
                    )
                  )
                }
              />
            </div>
            <Button
              aria-label={i18nText("agentFlow", "auto.k_4a86499cd0", { value1: entry.name || index + 1 })}
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
            { name: '', content: { kind: 'templated_text', value: '' } }
          ])
        }
      >
        {i18nText("agentFlow", "auto.k_b81a7d3e3c")}</Button>
    </div>
  );
}
