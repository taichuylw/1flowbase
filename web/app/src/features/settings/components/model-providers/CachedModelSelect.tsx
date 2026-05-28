import type { CSSProperties } from 'react';

import { Select, Typography } from 'antd';
import { i18nText } from '../../../../shared/i18n/text';

type EmptyMode = 'text' | 'select';

export function CachedModelSelect({
  modelIds,
  ariaLabel,
  className,
  style,
  placeholder = i18nText("settings", "auto.cache_model"),
  value,
  values,
  defaultValue,
  onChange,
  emptyMode = 'text',
  disabled = false
}: {
  modelIds: string[];
  ariaLabel: string;
  className?: string;
  style?: CSSProperties;
  placeholder?: string;
  value?: string;
  values?: string[];
  defaultValue?: string;
  onChange?: (value: string | null) => void;
  emptyMode?: EmptyMode;
  disabled?: boolean;
}) {
  const options = modelIds.map((modelId) => ({
    value: modelId,
    label: modelId
  }));

  if (options.length === 0 && emptyMode === 'text') {
    return <Typography.Text type="secondary">{i18nText("settings", "auto.cache_model_yet")}</Typography.Text>;
  }

  return (
    <Select
      aria-label={ariaLabel}
      className={className}
      style={style}
      mode={values ? 'multiple' : undefined}
      value={values ?? value}
      defaultValue={defaultValue}
      options={options}
      placeholder={placeholder}
      disabled={disabled}
      filterOption={(input, option) =>
        String(option?.label ?? '')
          .toLowerCase()
          .includes(input.toLowerCase())
      }
      optionFilterProp="label"
      popupMatchSelectWidth={false}
      showSearch={!disabled}
      allowClear={Boolean(onChange)}
      onChange={(nextValue) => onChange?.(typeof nextValue === 'string' ? nextValue : null)}
    />
  );
}
