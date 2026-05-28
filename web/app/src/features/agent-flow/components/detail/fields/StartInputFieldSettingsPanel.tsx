import { DeleteOutlined, PlusOutlined } from '@ant-design/icons';
import { Button, Input, InputNumber, Select, Switch, Typography } from 'antd';
import type { RefObject } from 'react';

import type {
  FlowStartInputField,
  FlowStartInputType
} from '@1flowbase/flow-schema';

import {
  getStartInputValueType,
  startInputTypeOptions
} from '../../../lib/start-node-variables';
import { FloatingSettingsPanel } from '../FloatingSettingsPanel';
import { i18nText } from '../../../../../shared/i18n/text';

type StartInputFieldSettingsPanelProps = {
  mode: 'create' | 'edit';
  field: FlowStartInputField;
  triggerRef: RefObject<HTMLElement | null>;
  onChange: (patch: Partial<FlowStartInputField>) => void;
  onClose: () => void;
  onSave: () => void;
};

function isStringDefaultType(inputType: FlowStartInputType) {
  return (
    inputType === 'text' ||
    inputType === 'paragraph' ||
    inputType === 'select' ||
    inputType === 'url'
  );
}

function shouldShowMaxLength(inputType: FlowStartInputType) {
  return (
    inputType === 'text' || inputType === 'paragraph' || inputType === 'url'
  );
}

function normalizeOptions(options: string[] | undefined) {
  const nextOptions = options?.length ? options : [''];

  return nextOptions;
}

export function StartInputFieldSettingsPanel({
  mode,
  field,
  triggerRef,
  onChange,
  onClose,
  onSave
}: StartInputFieldSettingsPanelProps) {
  const title = mode === 'create' ? i18nText("agentFlow", "auto.k_ada5f5ec56") : i18nText("agentFlow", "auto.k_5b16cf93bb");
  const options = normalizeOptions(field.options);
  const showDefaultValue =
    isStringDefaultType(field.inputType) ||
    field.inputType === 'number' ||
    field.inputType === 'checkbox';

  function handleTypeChange(inputType: FlowStartInputType) {
    const valueType = getStartInputValueType(inputType);

    onChange({
      inputType,
      valueType,
      options: inputType === 'select' ? options : undefined,
      defaultValue: undefined,
      maxLength: shouldShowMaxLength(inputType) ? field.maxLength : undefined
    });
  }

  function updateOption(index: number, value: string) {
    onChange({
      options: options.map((option, optionIndex) =>
        optionIndex === index ? value : option
      )
    });
  }

  function removeOption(index: number) {
    const nextOptions = options.filter(
      (_, optionIndex) => optionIndex !== index
    );

    onChange({
      options: nextOptions.length > 0 ? nextOptions : [''],
      defaultValue: nextOptions.includes(String(field.defaultValue ?? ''))
        ? field.defaultValue
        : undefined
    });
  }

  return (
    <FloatingSettingsPanel
      open
      title={title}
      closeLabel={i18nText("agentFlow", "auto.k_8a49271fb5", { value1: title })}
      triggerRef={triggerRef}
      className="agent-flow-start-input-fields__panel"
      defaultWidth={420}
      minWidth={360}
      initialHeight={520}
      gap={16}
      onClose={onClose}
      footer={
        <div className="agent-flow-start-input-fields__panel-footer">
          <Button onClick={onClose}>{i18nText("agentFlow", "auto.k_4d0b4688c7")}</Button>
          <Button aria-label={i18nText("agentFlow", "auto.k_49aba967b5")} type="primary" onClick={onSave}>
            {i18nText("agentFlow", "auto.k_fadf24dbc5")}</Button>
        </div>
      }
    >
      <div className="agent-flow-start-input-fields__form">
        <label className="agent-flow-start-input-fields__form-row">
          <span>{i18nText("agentFlow", "auto.k_750cc3d873")}</span>
          <Select
            aria-label={i18nText("agentFlow", "auto.k_c98fbf5a01")}
            options={startInputTypeOptions}
            value={field.inputType}
            virtual={false}
            onChange={handleTypeChange}
          />
        </label>
        <label className="agent-flow-start-input-fields__form-row">
          <span>{i18nText("agentFlow", "auto.k_63d5977de6")}</span>
          <Input
            aria-label={i18nText("agentFlow", "auto.k_014e72d827")}
            value={field.key}
            onChange={(event) => onChange({ key: event.target.value })}
          />
        </label>
        <label className="agent-flow-start-input-fields__form-row">
          <span>{i18nText("agentFlow", "auto.k_c10bbf5ddd")}</span>
          <Input
            aria-label={i18nText("agentFlow", "auto.k_f3793aa848")}
            value={field.label}
            onChange={(event) => onChange({ label: event.target.value })}
          />
        </label>
        {shouldShowMaxLength(field.inputType) ? (
          <label className="agent-flow-start-input-fields__form-row">
            <span>{i18nText("agentFlow", "auto.k_c6f7a87b1e")}</span>
            <InputNumber
              aria-label={i18nText("agentFlow", "auto.k_81ff1b495f")}
              min={1}
              precision={0}
              value={field.maxLength}
              onChange={(maxLength) =>
                onChange({
                  maxLength:
                    typeof maxLength === 'number' ? maxLength : undefined
                })
              }
            />
          </label>
        ) : null}

        {field.inputType === 'select' ? (
          <div className="agent-flow-start-input-fields__form-row">
            <span>{i18nText("agentFlow", "auto.k_690e1afab9")}</span>
            <div className="agent-flow-start-input-fields__option-list">
              {options.map((option, index) => (
                <div
                  className="agent-flow-start-input-fields__option-row"
                  key={index}
                >
                  <Input
                    aria-label={i18nText("agentFlow", "auto.k_cc5c9f453f", { value1: index + 1 })}
                    value={option}
                    onChange={(event) =>
                      updateOption(index, event.target.value)
                    }
                  />
                  <Button
                    aria-label={i18nText("agentFlow", "auto.k_55503a3793", { value1: index + 1 })}
                    icon={<DeleteOutlined />}
                    size="small"
                    type="text"
                    onClick={() => removeOption(index)}
                  />
                </div>
              ))}
              <Button
                aria-label={i18nText("agentFlow", "auto.k_7ddfaf208e")}
                icon={<PlusOutlined />}
                size="small"
                onClick={() => onChange({ options: [...options, ''] })}
              >
                {i18nText("agentFlow", "auto.k_1a74b6f38d")}</Button>
            </div>
          </div>
        ) : null}

        {showDefaultValue ? (
          <div className="agent-flow-start-input-fields__form-row">
            <span>{i18nText("agentFlow", "auto.k_d93919c37b")}</span>
            {field.inputType === 'paragraph' ? (
              <Input.TextArea
                aria-label={i18nText("agentFlow", "auto.k_532ca4c463")}
                autoSize={{ minRows: 2, maxRows: 4 }}
                value={String(field.defaultValue ?? '')}
                onChange={(event) =>
                  onChange({ defaultValue: event.target.value || undefined })
                }
              />
            ) : field.inputType === 'number' ? (
              <InputNumber
                aria-label={i18nText("agentFlow", "auto.k_532ca4c463")}
                value={
                  typeof field.defaultValue === 'number'
                    ? field.defaultValue
                    : undefined
                }
                onChange={(defaultValue) =>
                  onChange({
                    defaultValue:
                      typeof defaultValue === 'number'
                        ? defaultValue
                        : undefined
                  })
                }
              />
            ) : field.inputType === 'checkbox' ? (
              <Select
                aria-label={i18nText("agentFlow", "auto.k_532ca4c463")}
                options={[
                  { value: true, label: i18nText("agentFlow", "auto.k_26eb5c7566") },
                  { value: false, label: i18nText("agentFlow", "auto.k_e1ba440015") }
                ]}
                value={
                  typeof field.defaultValue === 'boolean'
                    ? field.defaultValue
                    : undefined
                }
                virtual={false}
                onChange={(defaultValue: boolean) => onChange({ defaultValue })}
              />
            ) : field.inputType === 'select' ? (
              <Select
                allowClear
                aria-label={i18nText("agentFlow", "auto.k_532ca4c463")}
                options={options
                  .map((option) => option.trim())
                  .filter(Boolean)
                  .map((option) => ({ value: option, label: option }))}
                value={
                  typeof field.defaultValue === 'string'
                    ? field.defaultValue
                    : undefined
                }
                virtual={false}
                onChange={(defaultValue?: string) => onChange({ defaultValue })}
              />
            ) : (
              <Input
                aria-label={i18nText("agentFlow", "auto.k_532ca4c463")}
                value={String(field.defaultValue ?? '')}
                onChange={(event) =>
                  onChange({ defaultValue: event.target.value || undefined })
                }
              />
            )}
          </div>
        ) : null}

        <div className="agent-flow-start-input-fields__toggles">
          <label className="agent-flow-start-input-fields__toggle-row">
            <span>
              <Typography.Text strong>{i18nText("agentFlow", "auto.k_32945d3e36")}</Typography.Text>
              <Typography.Text type="secondary">
                {i18nText("agentFlow", "auto.k_d603c21783")}</Typography.Text>
            </span>
            <Switch
              aria-label={i18nText("agentFlow", "auto.k_5ee7d5e553")}
              checked={field.required}
              onChange={(required) =>
                onChange({ required, hidden: required ? false : field.hidden })
              }
            />
          </label>
          <label className="agent-flow-start-input-fields__toggle-row">
            <span>
              <Typography.Text strong>{i18nText("agentFlow", "auto.k_bb0e7e01aa")}</Typography.Text>
              <Typography.Text type="secondary">
                {i18nText("agentFlow", "auto.k_40953c5469")}</Typography.Text>
            </span>
            <Switch
              aria-label={i18nText("agentFlow", "auto.k_e8c26a43b0")}
              checked={field.hidden}
              disabled={field.required}
              onChange={(hidden) =>
                onChange({ hidden, required: hidden ? false : field.required })
              }
            />
          </label>
        </div>
      </div>
    </FloatingSettingsPanel>
  );
}
