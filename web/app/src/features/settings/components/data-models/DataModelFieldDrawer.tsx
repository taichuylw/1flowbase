import { useEffect, useState } from 'react';

import { DeleteOutlined, PlusOutlined } from '@ant-design/icons';
import {
  Button,
  Checkbox,
  Divider,
  Drawer,
  Form,
  Input,
  Modal,
  Radio,
  Select,
  Space,
  Typography
} from 'antd';

import type {
  CreateSettingsDataModelFieldInput,
  SettingsDataModel,
  SettingsDataModelField,
  UpdateSettingsDataModelFieldInput
} from '../../api/data-models';
import { DataModelHelpTooltip } from './DataModelHelpTooltip';
import { i18nText } from '../../../../shared/i18n/text';

const fieldKindOptions = [
  { label: i18nText("settings", "auto.k_16a1a33cec"), value: 'string' },
  { label: i18nText("settings", "auto.k_7a4dc8253f"), value: 'number' },
  { label: i18nText("settings", "auto.k_420ec7f43d"), value: 'boolean' },
  { label: i18nText("settings", "auto.k_82d790005c"), value: 'datetime' },
  { label: i18nText("settings", "auto.k_d7f8312638"), value: 'enum' },
  { label: i18nText("settings", "auto.k_a7bab8d8aa"), value: 'text' },
  { label: 'JSON', value: 'json' },
  { label: i18nText("settings", "auto.k_2551c0888c"), value: 'many_to_one' },
  { label: i18nText("settings", "auto.k_cc8eeabc10"), value: 'one_to_many' },
  { label: i18nText("settings", "auto.k_6a27e04333"), value: 'many_to_many' }
];

const displayInterfaceOptions = [
  { label: 'input', value: 'input' },
  { label: 'textarea', value: 'textarea' },
  { label: 'select', value: 'select' },
  { label: 'radio', value: 'radio' },
  { label: 'checkbox_group', value: 'checkbox_group' },
  { label: 'multi_select', value: 'multi_select' },
  { label: 'switch', value: 'switch' },
  { label: 'date_picker', value: 'date_picker' },
  { label: 'json_editor', value: 'json_editor' }
];

const enumDisplayFormatOptions = [
  { label: i18nText("settings", "auto.k_3e886a9ca6"), value: 'radio' },
  { label: i18nText("settings", "auto.k_83c68005f4"), value: 'checkbox_group' },
  { label: i18nText("settings", "auto.k_41f4c0be10"), value: 'select' },
  { label: i18nText("settings", "auto.k_02d351d901"), value: 'multi_select' }
];

const externalFieldKeyHelp = i18nText("settings", "auto.k_b27fd8f436");
const enumOptionValueHelp = i18nText("settings", "auto.k_a1da2b4c44");
const enumOptionLabelHelp = i18nText("settings", "auto.k_4f68c00ecc");

interface FieldFormValues {
  code: string;
  title: string;
  external_field_key?: string;
  field_kind: string;
  is_required: boolean;
  is_unique: boolean;
  default_value_input?: string | string[] | boolean;
  enum_display_format?: string;
  enum_options?: Array<{
    label?: string;
    value?: string;
  }>;
  display_interface: string | null;
  display_options_json: string;
  relation_target_model_id: string | null;
  relation_options_json: string;
}

const relationFieldKinds = new Set(['many_to_one', 'one_to_many', 'many_to_many']);

function isRelationFieldKind(fieldKind: string | null | undefined) {
  return fieldKind ? relationFieldKinds.has(fieldKind) : false;
}

function defaultDisplayInterfaceForKind(fieldKind: string | null | undefined) {
  switch (fieldKind) {
    case 'text':
      return 'textarea';
    case 'boolean':
      return 'switch';
    case 'datetime':
      return 'date_picker';
    case 'enum':
      return 'select';
    case 'json':
      return 'json_editor';
    default:
      return 'input';
  }
}

function stringifyJson(value: unknown, fallback = '{}') {
  if (value === null || value === undefined) {
    return fallback;
  }

  return JSON.stringify(value, null, 2);
}

function parseJson(raw: string, fallback: unknown) {
  const trimmed = (raw ?? '').trim();

  if (!trimmed) {
    return fallback;
  }

  return JSON.parse(trimmed) as unknown;
}

function formatDefaultValueForForm(
  fieldKind: string | null | undefined,
  value: unknown,
  enumDisplayFormat?: string | null
) {
  if (value === null || value === undefined) {
    return undefined;
  }

  if (fieldKind === 'enum') {
    if (isMultipleEnumDisplayFormat(enumDisplayFormat)) {
      return Array.isArray(value) ? value.map(String) : [String(value)];
    }
    return Array.isArray(value) ? String(value[0] ?? '') : String(value);
  }

  if (fieldKind === 'boolean') {
    return value === true;
  }

  if (fieldKind === 'json') {
    return stringifyJson(value);
  }

  return String(value);
}

function isMultipleEnumDisplayFormat(value: string | null | undefined) {
  return value === 'checkbox_group' || value === 'multi_select';
}

function parseDefaultValue(
  fieldKind: string,
  raw: unknown,
  enumDisplayFormat?: string | null
) {
  if (
    raw === null ||
    raw === undefined ||
    raw === '' ||
    (Array.isArray(raw) && raw.length === 0)
  ) {
    return null;
  }

  if (fieldKind === 'enum') {
    if (isMultipleEnumDisplayFormat(enumDisplayFormat)) {
      return Array.isArray(raw) ? raw.map(String) : [String(raw)];
    }
    return Array.isArray(raw) ? String(raw[0] ?? '') : String(raw);
  }

  if (fieldKind === 'boolean') {
    return raw === true;
  }

  if (fieldKind === 'number') {
    const parsed = Number(raw);
    if (!Number.isFinite(parsed)) {
      throw new Error('number');
    }
    return parsed;
  }

  if (fieldKind === 'json') {
    return parseJson(String(raw), null);
  }

  return String(raw);
}

function readEnumOptions(displayOptions: Record<string, unknown>) {
  const options = displayOptions.options;
  if (!Array.isArray(options)) {
    return [{ label: '', value: '' }];
  }

  const normalized = options
    .map((option) => {
      if (typeof option === 'string') {
        return { label: option, value: option };
      }
      if (typeof option === 'object' && option !== null) {
        const record = option as Record<string, unknown>;
        const value = typeof record.value === 'string' ? record.value : '';
        return {
          label:
            typeof record.label === 'string'
              ? record.label
              : value,
          value
        };
      }
      return null;
    })
    .filter((option): option is { label: string; value: string } => option !== null);

  return normalized.length > 0 ? normalized : [{ label: '', value: '' }];
}

function parseEnumOptions(
  options: FieldFormValues['enum_options']
) {
  return (options ?? [])
    .map((option) => ({
      label: option.label?.trim() ?? '',
      value: option.value?.trim() ?? ''
    }))
    .filter((option) => option.label || option.value)
    .map((option) => ({
      label: option.label || option.value,
      value: option.value || option.label
    }));
}

function normalizeEnumDisplayFormat(value: string | null | undefined): string {
  return value && enumDisplayFormatOptions.some((option) => option.value === value)
    ? value
    : 'select';
}

function createDefaultEnumOption() {
  const suffix = Math.random().toString(36).slice(2, 10).padEnd(8, '0');
  return { label: '', value: `enum_${suffix}` };
}

export function DataModelFieldDrawer({
  open,
  mode,
  field,
  isExternalModel,
  modelOptions,
  saving,
  canManage,
  onClose,
  onCreate,
  onUpdate,
  onDelete
}: {
  open: boolean;
  mode: 'create' | 'edit';
  field: SettingsDataModelField | null;
  isExternalModel: boolean;
  modelOptions: SettingsDataModel[];
  saving: boolean;
  canManage: boolean;
  onClose: () => void;
  onCreate: (input: CreateSettingsDataModelFieldInput) => void;
  onUpdate: (
    field: SettingsDataModelField,
    input: UpdateSettingsDataModelFieldInput
  ) => void;
  onDelete: (field: SettingsDataModelField) => void;
}) {
  const [form] = Form.useForm<FieldFormValues>();
  const [deleteConfirmOpen, setDeleteConfirmOpen] = useState(false);
  const [advancedOpen, setAdvancedOpen] = useState(false);
  const selectedFieldKind = Form.useWatch('field_kind', form) ?? 'string';
  const selectedEnumDisplayFormat =
    Form.useWatch('enum_display_format', form) ?? 'select';
  const watchedEnumOptions = Form.useWatch('enum_options', form) ?? [];
  const showsRelationSettings = isRelationFieldKind(selectedFieldKind);
  const showsEnumSettings = selectedFieldKind === 'enum';
  const showsDefaultValue = !showsRelationSettings;

  useEffect(() => {
    if (!open) {
      return;
    }
    setAdvancedOpen(false);

    if (mode === 'edit' && field) {
      form.setFieldsValue({
        code: field.code,
        title: field.title,
        external_field_key: field.external_field_key ?? '',
        field_kind: field.field_kind,
        is_required: field.is_required,
        is_unique: field.is_unique,
        default_value_input: formatDefaultValueForForm(
          field.field_kind,
          field.default_value,
          normalizeEnumDisplayFormat(field.display_interface)
        ),
        enum_display_format: normalizeEnumDisplayFormat(field.display_interface),
        enum_options: readEnumOptions(field.display_options),
        display_interface:
          field.display_interface ?? defaultDisplayInterfaceForKind(field.field_kind),
        display_options_json: stringifyJson(field.display_options),
        relation_target_model_id: field.relation_target_model_id,
        relation_options_json: stringifyJson(field.relation_options)
      });
      return;
    }

    form.setFieldsValue({
      code: '',
      title: '',
      external_field_key: '',
      field_kind: 'string',
      is_required: false,
      is_unique: false,
      default_value_input: undefined,
      enum_display_format: 'select',
      enum_options: [createDefaultEnumOption()],
      display_interface: 'input',
      display_options_json: '{}',
      relation_target_model_id: null,
      relation_options_json: '{}'
    });
  }, [field, form, mode, open]);

  const handleSubmit = async () => {
    const values = await form.validateFields();
    let defaultValue: unknown | null = null;
    let displayOptions: Record<string, unknown> = {};
    let relationOptions: Record<string, unknown> = {};

    try {
      defaultValue = parseDefaultValue(
        values.field_kind,
        values.default_value_input,
        values.enum_display_format
      );
    } catch {
      form.setFields([
        {
          name: 'default_value_input',
          errors:
            values.field_kind === 'json'
              ? [i18nText("settings", "auto.k_4b78932c56")]
              : [i18nText("settings", "auto.k_ba9c1ebd30")]
        }
      ]);
      return;
    }

    try {
      displayOptions = parseJson(
        values.display_options_json,
        {}
      ) as Record<string, unknown>;
    } catch {
      form.setFields([
        {
          name: 'display_options_json',
          errors: [i18nText("settings", "auto.k_4b78932c56")]
        }
      ]);
      return;
    }

    try {
      relationOptions = parseJson(
        values.relation_options_json,
        {}
      ) as Record<string, unknown>;
    } catch {
      form.setFields([
        {
          name: 'relation_options_json',
          errors: [i18nText("settings", "auto.k_4b78932c56")]
        }
      ]);
      return;
    }

    if (values.field_kind === 'enum') {
      displayOptions = {
        ...displayOptions,
        options: parseEnumOptions(values.enum_options)
      };
    }

    const displayInterface =
      values.field_kind === 'enum'
        ? values.enum_display_format || 'select'
        : values.display_interface || defaultDisplayInterfaceForKind(values.field_kind);
    const relationTargetModelId = isRelationFieldKind(values.field_kind)
      ? values.relation_target_model_id || null
      : null;
    const normalizedRelationOptions = isRelationFieldKind(values.field_kind)
      ? relationOptions
      : {};

    if (mode === 'edit' && field) {
      onUpdate(field, {
        title: values.title,
        is_required: values.is_required,
        is_unique: values.is_unique,
        default_value: defaultValue,
        display_interface: displayInterface,
        display_options: displayOptions,
        relation_options: normalizedRelationOptions
      });
      onClose();
      return;
    }

    onCreate({
      code: values.code,
      title: values.title,
      external_field_key: isExternalModel ? values.external_field_key || null : null,
      field_kind: values.field_kind,
      is_required: values.is_required,
      is_unique: values.is_unique,
      default_value: defaultValue,
      display_interface: displayInterface,
      display_options: displayOptions,
      relation_target_model_id: relationTargetModelId,
      relation_options: normalizedRelationOptions
    });
    onClose();
  };

  const confirmDelete = () => {
    if (!field) {
      return;
    }

    setDeleteConfirmOpen(true);
  };

  const relationTargetOptions = modelOptions.map((model) => ({
    label: `${model.title} (${model.code})`,
    value: model.id
  }));
  const defaultEnumOptions = parseEnumOptions(watchedEnumOptions).map((option) => ({
    label: option.label,
    value: option.value
  }));

  function renderDefaultValueControl() {
    if (selectedFieldKind === 'enum') {
      if (selectedEnumDisplayFormat === 'radio') {
        return <Radio.Group options={defaultEnumOptions} />;
      }

      if (selectedEnumDisplayFormat === 'checkbox_group') {
        return <Checkbox.Group options={defaultEnumOptions} />;
      }

      return (
        <Select
          allowClear
          mode={selectedEnumDisplayFormat === 'multi_select' ? 'multiple' : undefined}
          options={defaultEnumOptions}
        />
      );
    }

    if (selectedFieldKind === 'boolean') {
      return (
        <Select
          allowClear
          options={[
            { label: i18nText("settings", "auto.yes"), value: true },
            { label: i18nText("settings", "auto.no"), value: false }
          ]}
        />
      );
    }

    if (selectedFieldKind === 'json') {
      return <Input.TextArea rows={3} placeholder='{ "key": "value" }' />;
    }

    return (
      <Input
        type={selectedFieldKind === 'number' ? 'number' : undefined}
        placeholder={
          selectedFieldKind === 'datetime'
            ? i18nText("settings", "auto.k_12b8bd73e1")
            : undefined
        }
      />
    );
  }

  function renderRuleSettings() {
    return (
      <>
        <Divider />
        <Typography.Title level={5}>{i18nText("settings", "auto.k_ed904c685a")}</Typography.Title>
        <Space size="large">
          <Form.Item name="is_required" valuePropName="checked">
            <Checkbox>{i18nText("settings", "auto.k_32945d3e36")}</Checkbox>
          </Form.Item>
          <Form.Item name="is_unique" valuePropName="checked">
            <Checkbox>{i18nText("settings", "auto.k_e805014aa5")}</Checkbox>
          </Form.Item>
        </Space>
        {showsDefaultValue ? (
          <Form.Item
            name="default_value_input"
            label={selectedFieldKind === 'json' ? i18nText("settings", "auto.k_08ba521ddb") : i18nText("settings", "auto.k_d93919c37b")}
          >
            {renderDefaultValueControl()}
          </Form.Item>
        ) : null}
      </>
    );
  }

  return (
    <>
      <Drawer
        title={mode === 'create' ? i18nText("settings", "auto.k_1687c80b94") : i18nText("settings", "auto.k_0f88443ec3")}
        open={open}
        width={560}
        onClose={onClose}
        extra={
          <Space>
            {mode === 'edit' ? (
              <Button danger disabled={!canManage || saving} onClick={confirmDelete}>
                {i18nText("settings", "auto.k_b9cee43029")}</Button>
            ) : null}
            <Button
              type="primary"
              loading={saving}
              disabled={!canManage}
              onClick={handleSubmit}
            >
              {mode === 'create' ? i18nText("settings", "auto.k_f28bc54bb8") : i18nText("settings", "auto.k_dbf54adae7")}
            </Button>
          </Space>
        }
      >
        <Form
          form={form}
          layout="vertical"
          disabled={!canManage}
          initialValues={{
            field_kind: 'string',
            is_required: false,
            is_unique: false,
            display_interface: 'input',
            display_options_json: '{}',
            relation_options_json: '{}'
          }}
        >
          <Typography.Title level={5}>{i18nText("settings", "auto.k_41654e0268")}</Typography.Title>
          <Form.Item
            name="title"
            label={i18nText("settings", "auto.k_46cdebc806")}
            rules={[{ required: true, message: i18nText("settings", "auto.k_f8ebf7cbc1") }]}
          >
            <Input />
          </Form.Item>
          <Form.Item
            name="code"
            label={i18nText("settings", "auto.k_453b9cc528")}
            rules={[{ required: true, message: i18nText("settings", "auto.k_64ef817b08") }]}
          >
            <Input disabled={mode === 'edit'} />
          </Form.Item>
          <Form.Item
            name="field_kind"
            label={i18nText("settings", "auto.k_750cc3d873")}
            rules={[{ required: true, message: i18nText("settings", "auto.k_c466e23e7d") }]}
          >
            <Select options={fieldKindOptions} disabled={mode === 'edit'} />
          </Form.Item>

          {isExternalModel ? (
            <Form.Item
              name="external_field_key"
              label={i18nText("settings", "auto.k_05c0c54e56")}
              tooltip={externalFieldKeyHelp}
              rules={[
                {
                  required: mode === 'create',
                  message: i18nText("settings", "auto.k_df19e6f3a3")
                }
              ]}
            >
              <Input disabled={mode === 'edit'} />
            </Form.Item>
          ) : null}

          {showsEnumSettings ? null : renderRuleSettings()}

          {showsEnumSettings ? (
            <>
              <Divider />
              <Typography.Title level={5}>{i18nText("settings", "auto.k_0220fdd6e7")}</Typography.Title>
              <Form.Item
                name="enum_display_format"
                label={i18nText("settings", "auto.k_799e2cb348")}
                rules={[{ required: true, message: i18nText("settings", "auto.k_a4d7209e0a") }]}
              >
                <Select
                  options={enumDisplayFormatOptions}
                  onChange={(value) => {
                    const currentDefaultValue = form.getFieldValue('default_value_input');
                    if (isMultipleEnumDisplayFormat(value)) {
                      form.setFieldValue(
                        'default_value_input',
                        Array.isArray(currentDefaultValue)
                          ? currentDefaultValue
                          : currentDefaultValue
                            ? [currentDefaultValue]
                            : []
                      );
                      return;
                    }

                    form.setFieldValue(
                      'default_value_input',
                      Array.isArray(currentDefaultValue)
                        ? currentDefaultValue[0]
                        : currentDefaultValue
                    );
                  }}
                />
              </Form.Item>
              <Form.Item
                label={i18nText("settings", "auto.k_05b9e0400b")}
              >
                <Form.List
                  name="enum_options"
                  initialValue={[createDefaultEnumOption()]}
                >
                  {(fields, { add, remove }) => (
                    <div className="data-model-panel__enum-options">
                      <div className="data-model-panel__enum-options-head">
                        <span className="data-model-panel__enum-options-index" />
                        <span className="data-model-panel__enum-options-heading">
                          <span>{i18nText("settings", "auto.k_b9ccd17def")}</span>
                          <DataModelHelpTooltip
                            label={i18nText("settings", "auto.k_b9ccd17def")}
                            title={enumOptionValueHelp}
                          />
                        </span>
                        <span className="data-model-panel__enum-options-heading">
                          <span>{i18nText("settings", "auto.k_5545cfbfc4")}</span>
                          <DataModelHelpTooltip
                            label={i18nText("settings", "auto.k_5545cfbfc4")}
                            title={enumOptionLabelHelp}
                          />
                        </span>
                        <span className="data-model-panel__enum-options-action" />
                      </div>
                      {fields.map(({ key, name, ...restField }, index) => (
                        <div key={key} className="data-model-panel__enum-option-row">
                          <span className="data-model-panel__enum-options-index">
                            {index + 1}
                          </span>
                          <div className="data-model-panel__enum-option-cell">
                            <Form.Item
                              {...restField}
                              name={[name, 'value']}
                              rules={[{ required: true, message: i18nText("settings", "auto.k_5ee990cf78") }]}
                            >
                              <Input
                                aria-label={i18nText("settings", "auto.k_8ce7e7251a", { value1: index + 1 })}
                                placeholder="value"
                              />
                            </Form.Item>
                          </div>
                          <div className="data-model-panel__enum-option-cell">
                            <Form.Item
                              {...restField}
                              name={[name, 'label']}
                              rules={[{ required: true, message: i18nText("settings", "auto.k_aaeeb0b501") }]}
                            >
                              <Input
                                aria-label={i18nText("settings", "auto.k_dd4767e81e", { value1: index + 1 })}
                                placeholder="label"
                              />
                            </Form.Item>
                          </div>
                          <Button
                            danger
                            type="text"
                            aria-label={i18nText("settings", "auto.k_bb7dfc9e93", { value1: index + 1 })}
                            icon={<DeleteOutlined />}
                            disabled={fields.length <= 1}
                            onClick={() => remove(name)}
                            className="data-model-panel__enum-options-action"
                          />
                        </div>
                      ))}
                      <Button
                        block
                        aria-label={i18nText("settings", "auto.k_aff6de897c")}
                        icon={<PlusOutlined />}
                        onClick={() => add(createDefaultEnumOption())}
                        className="data-model-panel__enum-add"
                      >
                        {i18nText("settings", "auto.k_aff6de897c")}</Button>
                    </div>
                  )}
                </Form.List>
              </Form.Item>
            </>
          ) : null}

          {showsEnumSettings ? renderRuleSettings() : null}

          {showsRelationSettings ? (
            <>
              <Divider />
              <Typography.Title level={5}>{i18nText("settings", "auto.k_a92f6c7954")}</Typography.Title>
              <Form.Item
                name="relation_target_model_id"
                label={i18nText("settings", "auto.k_b26bd97f10")}
                rules={[
                  {
                    required: mode === 'create',
                    message: i18nText("settings", "auto.k_9ca8e8919c")
                  }
                ]}
              >
                <Select
                  allowClear
                  disabled={mode === 'edit'}
                  options={relationTargetOptions}
                />
              </Form.Item>
            </>
          ) : null}

          <Divider />
          <Button type="link" onClick={() => setAdvancedOpen((value) => !value)}>
            {i18nText("settings", "auto.k_67f9884dca")}</Button>
          {advancedOpen ? (
            <>
              {showsEnumSettings ? null : (
                <Form.Item name="display_interface" label={i18nText("settings", "auto.k_6a7f6e274a")}>
                  <Select allowClear options={displayInterfaceOptions} />
                </Form.Item>
              )}
              <Form.Item
                name="display_options_json"
                label={i18nText("settings", "auto.k_2903646c74")}
              >
                <Input.TextArea rows={3} />
              </Form.Item>
              {showsRelationSettings ? (
                <Form.Item
                  name="relation_options_json"
                  label={i18nText("settings", "auto.k_5270506f58")}
                >
                  <Input.TextArea rows={3} />
                </Form.Item>
              ) : null}
            </>
          ) : null}
        </Form>
      </Drawer>
      <Modal
        title={i18nText("settings", "auto.k_d3173d7269")}
        open={deleteConfirmOpen}
        okText={i18nText("settings", "auto.delete")}
        okType="danger"
        cancelText={i18nText("settings", "auto.cancel")}
        okButtonProps={{ 'aria-label': i18nText("settings", "auto.delete") }}
        onCancel={() => setDeleteConfirmOpen(false)}
        onOk={() => {
          if (field) {
            onDelete(field);
          }
          setDeleteConfirmOpen(false);
          onClose();
        }}
      >
        {field
          ? i18nText("settings", "auto.k_e93bd93466", { value1: field.title, value2: field.code })
          : null}
      </Modal>
    </>
  );
}
