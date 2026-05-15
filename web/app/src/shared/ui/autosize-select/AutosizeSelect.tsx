import type { ReactNode } from 'react';

import { Select } from 'antd';
import type { SelectProps } from 'antd';

import './autosize-select.css';

function getPlainMeasureLabel(label: ReactNode) {
  if (typeof label === 'string' || typeof label === 'number') {
    return String(label);
  }

  return null;
}

function getMeasureLabels<ValueType, OptionType extends { label?: ReactNode }>({
  autosizeLabels,
  options
}: {
  autosizeLabels?: string[];
  options?: SelectProps<ValueType, OptionType>['options'];
}) {
  if (autosizeLabels) {
    return autosizeLabels;
  }

  return (options ?? [])
    .map((option) => getPlainMeasureLabel(option.label))
    .filter((label): label is string => Boolean(label));
}

export type AutosizeSelectProps<
  ValueType = unknown,
  OptionType extends { label?: ReactNode } = { label?: ReactNode }
> = SelectProps<ValueType, OptionType> & {
  autosizeLabels?: string[];
};

export function AutosizeSelect<
  ValueType = unknown,
  OptionType extends { label?: ReactNode } = { label?: ReactNode }
>({
  autosizeLabels,
  className,
  options,
  ...selectProps
}: AutosizeSelectProps<ValueType, OptionType>) {
  const measureLabels = getMeasureLabels<ValueType, OptionType>({
    autosizeLabels,
    options
  });
  const selectClassName = ['autosize-select__control', className]
    .filter(Boolean)
    .join(' ');

  return (
    <span className="autosize-select">
      <span aria-hidden="true" className="autosize-select__measure">
        {measureLabels.map((label, index) => (
          <span
            key={`${label}-${index}`}
            className="autosize-select__measure-item"
            data-measure-label={label}
          />
        ))}
      </span>
      <Select<ValueType, OptionType>
        {...selectProps}
        className={selectClassName}
        options={options}
      />
    </span>
  );
}
