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
  const title = mode === 'create' ? i18nText("agentFlow", "auto.key_knkfpfomfg") : i18nText("agentFlow", "auto.key_flbgmpjdll");
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
      closeLabel={i18nText("agentFlow", "auto.key_ikejchbplf", { value1: title })}
      triggerRef={triggerRef}
      className="agent-flow-start-input-fields__panel"
      defaultWidth={420}
      minWidth={360}
      initialHeight={520}
      gap={16}
      onClose={onClose}
      footer={
        <div className="agent-flow-start-input-fields__panel-footer">
          <Button onClick={onClose}>{i18nText("agentFlow", "auto.key_enalegiimh")}</Button>
          <Button aria-label={i18nText("agentFlow", "auto.key_ejklkjghlf")} type="primary" onClick={onSave}>
            {i18nText("agentFlow", "auto.key_pknpcenlmf")}</Button>
        </div>
      }
    >
      <div className="agent-flow-start-input-fields__form">
        <label className="agent-flow-start-input-fields__form-row">
          <span>{i18nText("agentFlow", "auto.key_hfammdnihd")}</span>
          <Select
            aria-label={i18nText("agentFlow", "auto.key_mjiplpfkab")}
            options={startInputTypeOptions}
            value={field.inputType}
            virtual={false}
            onChange={handleTypeChange}
          />
        </label>
        <label className="agent-flow-start-input-fields__form-row">
          <span>{i18nText("agentFlow", "auto.key_gdnfjhhnog")}</span>
          <Input
            aria-label={i18nText("agentFlow", "auto.key_abeohcnich")}
            value={field.key}
            onChange={(event) => onChange({ key: event.target.value })}
          />
        </label>
        <label className="agent-flow-start-input-fields__form-row">
          <span>{i18nText("agentFlow", "auto.key_mballpfnnn")}</span>
          <Input
            aria-label={i18nText("agentFlow", "auto.key_pdhjdkkiei")}
            value={field.label}
            onChange={(event) => onChange({ label: event.target.value })}
          />
        </label>
        {shouldShowMaxLength(field.inputType) ? (
          <label className="agent-flow-start-input-fields__form-row">
            <span>{i18nText("agentFlow", "auto.key_mgphkihlbo")}</span>
            <InputNumber
              aria-label={i18nText("agentFlow", "auto.key_ibppblejfp")}
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
            <span>{i18nText("agentFlow", "auto.key_gjaobkpklj")}</span>
            <div className="agent-flow-start-input-fields__option-list">
              {options.map((option, index) => (
                <div
                  className="agent-flow-start-input-fields__option-row"
                  key={index}
                >
                  <Input
                    aria-label={i18nText("agentFlow", "auto.key_mmfmjpefdp", { value1: index + 1 })}
                    value={option}
                    onChange={(event) =>
                      updateOption(index, event.target.value)
                    }
                  />
                  <Button
                    aria-label={i18nText("agentFlow", "auto.key_fffadkdhjd", { value1: index + 1 })}
                    icon={<DeleteOutlined />}
                    size="small"
                    type="text"
                    onClick={() => removeOption(index)}
                  />
                </div>
              ))}
              <Button
                aria-label={i18nText("agentFlow", "auto.key_hnnpkpcaio")}
                icon={<PlusOutlined />}
                size="small"
                onClick={() => onChange({ options: [...options, ''] })}
              >
                {i18nText("agentFlow", "auto.key_bkhelgpdin")}</Button>
            </div>
          </div>
        ) : null}

        {showDefaultValue ? (
          <div className="agent-flow-start-input-fields__form-row">
            <span>{i18nText("agentFlow", "auto.key_njdjbjmdhl")}</span>
            {field.inputType === 'paragraph' ? (
              <Input.TextArea
                aria-label={i18nText("agentFlow", "auto.key_fdcmkemegd")}
                autoSize={{ minRows: 2, maxRows: 4 }}
                value={String(field.defaultValue ?? '')}
                onChange={(event) =>
                  onChange({ defaultValue: event.target.value || undefined })
                }
              />
            ) : field.inputType === 'number' ? (
              <InputNumber
                aria-label={i18nText("agentFlow", "auto.key_fdcmkemegd")}
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
                aria-label={i18nText("agentFlow", "auto.key_fdcmkemegd")}
                options={[
                  { value: true, label: i18nText("agentFlow", "auto.key_cgolfmhfgg") },
                  { value: false, label: i18nText("agentFlow", "auto.key_oblkeeaabf") }
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
                aria-label={i18nText("agentFlow", "auto.key_fdcmkemegd")}
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
                aria-label={i18nText("agentFlow", "auto.key_fdcmkemegd")}
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
              <Typography.Text strong>{i18nText("agentFlow", "auto.key_dcjefndodg")}</Typography.Text>
              <Typography.Text type="secondary">
                {i18nText("agentFlow", "auto.key_ngadmcbhid")}</Typography.Text>
            </span>
            <Switch
              aria-label={i18nText("agentFlow", "auto.key_foohnfoffd")}
              checked={field.required}
              onChange={(required) =>
                onChange({ required, hidden: required ? false : field.hidden })
              }
            />
          </label>
          <label className="agent-flow-start-input-fields__toggle-row">
            <span>
              <Typography.Text strong>{i18nText("agentFlow", "auto.key_llaohoabkk")}</Typography.Text>
              <Typography.Text type="secondary">
                {i18nText("agentFlow", "auto.key_eajfdmfegj")}</Typography.Text>
            </span>
            <Switch
              aria-label={i18nText("agentFlow", "auto.key_oimcgkedla")}
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
