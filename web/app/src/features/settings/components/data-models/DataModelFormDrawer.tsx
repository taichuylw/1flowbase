import { useEffect } from 'react';

import { Button, Drawer, Form, Input, Select } from 'antd';

import type {
  CreateSettingsDataModelInput,
  SettingsDataModel,
  SettingsDataSourceInstance,
  UpdateSettingsDataModelInput
} from '../../api/data-models';
import {
  DataModelFieldLabel,
  dataModelCodeHelp,
  dataModelStatusHelp,
  dataModelTitleHelp
} from './DataModelHelpTooltip';
import { i18nText } from '../../../../shared/i18n/text';

const dataModelStatusOptions = ['draft', 'published', 'disabled', 'broken'].map(
  (value) => ({ label: value, value })
);

interface DataModelFormValues {
  code: string;
  title: string;
  status: SettingsDataModel['status'];
  data_source_instance_id: string;
  external_table_id: string;
}

export function DataModelFormDrawer({
  open,
  mode,
  model,
  source,
  saving,
  onClose,
  onCreate,
  onUpdate
}: {
  open: boolean;
  mode: 'create' | 'edit';
  model: SettingsDataModel | null;
  source: SettingsDataSourceInstance | null;
  saving: boolean;
  onClose: () => void;
  onCreate: (input: CreateSettingsDataModelInput) => void;
  onUpdate: (
    model: SettingsDataModel,
    input: UpdateSettingsDataModelInput
  ) => void;
}) {
  const [form] = Form.useForm<DataModelFormValues>();
  const isExternalModel =
    mode === 'edit'
      ? model?.source_kind === 'external_source'
      : source?.source_kind === 'external_source';

  useEffect(() => {
    if (!open) {
      return;
    }

    if (mode === 'edit' && model) {
      form.setFieldsValue({
        code: model.code,
        title: model.title,
        status: model.status,
        data_source_instance_id: model.data_source_instance_id ?? 'main_source',
        external_table_id: model.external_table_id ?? ''
      });
      return;
    }

    form.setFieldsValue({
      code: '',
      title: '',
      status: source?.default_data_model_status ?? 'published',
      data_source_instance_id: source?.id ?? 'main_source',
      external_table_id: ''
    });
  }, [form, mode, model, open, source]);

  const handleSubmit = async () => {
    const values = await form.validateFields();

    if (mode === 'edit' && model) {
      onUpdate(model, {
        title: values.title,
        status: values.status,
        external_table_id: isExternalModel ? values.external_table_id : null
      });
      onClose();
      return;
    }

    onCreate({
      scope_kind: 'workspace',
      code: values.code,
      title: values.title,
      status: values.status,
      data_source_instance_id:
        source?.source_kind === 'external_source' ? source.id : null,
      external_resource_key: isExternalModel ? values.external_table_id : null,
      external_table_id: isExternalModel ? values.external_table_id : null
    });
    onClose();
  };

  return (
    <Drawer
      title={mode === 'create' ? i18nText("settings", "auto.k_bcd246f770") : i18nText("settings", "auto.k_de5da45943")}
      open={open}
      width={520}
      onClose={onClose}
      extra={
        <Button
          type="primary"
          aria-label={mode === 'create' ? i18nText("settings", "auto.k_fcbd093292") : i18nText("settings", "auto.k_fadf24dbc5")}
          loading={saving}
          onClick={handleSubmit}
        >
          {mode === 'create' ? i18nText("settings", "auto.k_fcbd093292") : i18nText("settings", "auto.k_fadf24dbc5")}
        </Button>
      }
    >
      <Form
        form={form}
        layout="vertical"
        initialValues={{
          status: source?.default_data_model_status ?? 'published',
          data_source_instance_id: source?.id ?? 'main_source'
        }}
      >
        <Form.Item
          name="title"
          label={
            <DataModelFieldLabel label={i18nText("settings", "auto.k_748d7dc7e3")} title={dataModelTitleHelp} />
          }
          rules={[{ required: true, message: i18nText("settings", "auto.k_901722e5f3") }]}
        >
          <Input aria-label={i18nText("settings", "auto.k_748d7dc7e3")} />
        </Form.Item>
        <Form.Item
          name="code"
          label={
            <DataModelFieldLabel label="Code" title={dataModelCodeHelp} />
          }
          rules={[{ required: true, message: i18nText("settings", "auto.k_a443b39d6f") }]}
        >
          <Input aria-label="Code" disabled={mode === 'edit'} />
        </Form.Item>
        <Form.Item
          name="status"
          label={
            <DataModelFieldLabel label={i18nText("settings", "auto.k_62e951a692")} title={dataModelStatusHelp} />
          }
          rules={[{ required: true, message: i18nText("settings", "auto.k_dba277df58") }]}
        >
          <Select aria-label={i18nText("settings", "auto.k_62e951a692")} options={dataModelStatusOptions} />
        </Form.Item>
        <Form.Item name="data_source_instance_id" label={i18nText("settings", "auto.k_a3ccf702c5")}>
          <Input disabled />
        </Form.Item>
        {isExternalModel ? (
          <Form.Item
            name="external_table_id"
            label={i18nText("settings", "auto.k_e8c66a5fcd")}
            rules={[{ required: true, message: i18nText("settings", "auto.k_39d63adf01") }]}
          >
            <Input />
          </Form.Item>
        ) : null}
      </Form>
    </Drawer>
  );
}
