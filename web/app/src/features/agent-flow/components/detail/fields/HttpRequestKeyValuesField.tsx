import { DeleteOutlined, PlusOutlined } from '@ant-design/icons';
import { Button, Input } from 'antd';
import type { FlowBinding, NamedBindingEntry } from '@1flowbase/flow-schema';

import type { FlowSelectorOption } from '../../../lib/selector-options';
import { TemplatedTextField } from '../../bindings/TemplatedTextField';
import { i18nText } from '../../../../../shared/i18n/text';

export type HttpRequestKeyValueEntry = Pick<
  NamedBindingEntry,
  'name' | 'value' | 'valueType'
>;

export function namedBindingEntriesFromValue(
  value: unknown
): HttpRequestKeyValueEntry[] {
  if (
    typeof value === 'object' &&
    value !== null &&
    (value as { kind?: unknown }).kind === 'named_bindings' &&
    Array.isArray((value as { value?: unknown }).value)
  ) {
    return ((value as { value: unknown[] }).value as HttpRequestKeyValueEntry[]);
  }

  return [];
}

export function toNamedBinding(entries: HttpRequestKeyValueEntry[]): FlowBinding {
  return {
    kind: 'named_bindings',
    value: entries.map((entry) => ({
      name: entry.name,
      valueType: entry.valueType,
      value: entry.value ?? { kind: 'templated_text', value: '' }
    }))
  };
}

function getTemplatedValue(entry: HttpRequestKeyValueEntry) {
  return entry.value?.kind === 'templated_text' ? entry.value.value : '';
}

export function HttpRequestKeyValuesField({
  ariaLabel,
  value,
  options,
  addButtonLabel,
  onChange
}: {
  ariaLabel: string;
  value: unknown;
  options: FlowSelectorOption[];
  addButtonLabel?: string;
  onChange: (value: FlowBinding) => void;
}) {
  const entries = namedBindingEntriesFromValue(value);

  function emit(nextEntries: HttpRequestKeyValueEntry[]) {
    onChange(toNamedBinding(nextEntries));
  }

  return (
    <div className="agent-flow-http-request-key-values">
      {entries.map((entry, index) => (
        <div
          key={`${entry.name}-${index}`}
          className="agent-flow-http-request-key-values__row"
        >
          <Input
            aria-label={`${ariaLabel}-${index}-key`}
            placeholder={i18nText('agentFlow', 'auto.field_key')}
            value={entry.name}
            onChange={(event) =>
              emit(
                entries.map((candidate, candidateIndex) =>
                  candidateIndex === index
                    ? { ...candidate, name: event.target.value }
                    : candidate
                )
              )
            }
          />
          <TemplatedTextField
            ariaLabel={`${ariaLabel}-${index}-value`}
            displayMode="input"
            label={`${ariaLabel}-${index}-value`}
            options={options}
            placeholder={i18nText(
              'agentFlow',
              'auto.support_text_variable_block_enter_left_curly_bracket_quick_reference'
            )}
            value={getTemplatedValue(entry)}
            onChange={(nextValue) =>
              emit(
                entries.map((candidate, candidateIndex) =>
                  candidateIndex === index
                    ? {
                        ...candidate,
                        value: { kind: 'templated_text', value: nextValue }
                      }
                    : candidate
                )
              )
            }
          />
          <Button
            aria-label={i18nText('agentFlow', 'auto.delete_variable', {
              value1: entry.name || index + 1
            })}
            danger
            icon={<DeleteOutlined />}
            type="text"
            onClick={() =>
              emit(entries.filter((_, candidateIndex) => candidateIndex !== index))
            }
          />
        </div>
      ))}
      <Button
        icon={<PlusOutlined />}
        type="dashed"
        onClick={() =>
          emit([
            ...entries,
            { name: '', value: { kind: 'templated_text', value: '' } }
          ])
        }
      >
        {addButtonLabel ?? i18nText('agentFlow', 'auto.add_new_parameter')}
      </Button>
    </div>
  );
}
