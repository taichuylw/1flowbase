import { useEffect, useState } from 'react';
import {
  Button,
  Drawer,
  Form,
  Input,
  InputNumber,
  message,
  Select,
  Switch
} from 'antd';
import {
  createSettingsFileStorage,
  updateSettingsFileStorage,
  type SettingsFileStorage,
  type CreateSettingsFileStorageInput,
  type UpdateSettingsFileStorageInput
} from '../api/file-management';
import { useAuthStore } from '../../../state/auth-store';
import { i18nText } from '../../../shared/i18n/text';

type DrawerMode = 'create' | 'view' | 'edit';

interface FileStorageDrawerProps {
  open: boolean;
  mode: DrawerMode;
  record: SettingsFileStorage | null;
  onClose: () => void;
  onSuccess: () => void;
}

type StorageFormScalar = string | number | boolean | undefined;
type StorageFormObject = Record<string, StorageFormScalar>;

interface StorageFormValues {
  code: string;
  title: string;
  driver_type: string;
  enabled: boolean;
  is_default: boolean;
  config_json: StorageFormObject;
  rule_json: StorageFormObject;
}

const DRIVER_TYPE_OPTIONS = [
  { label: i18nText("settings", "auto.k_8d76933308"), value: 'local' },
  { label: i18nText("settings", "auto.k_6aff108343"), value: 's3' },
  { label: i18nText("settings", "auto.k_9a38a926a6"), value: 'oss' },
  { label: i18nText("settings", "auto.k_44bbd058d1"), value: 'cos' },
  { label: i18nText("settings", "auto.k_3900bffc22"), value: 'rustfs' }
];

const DRIVER_FIELDS: Record<string, { key: string; label: string; type: 'string' | 'number' }[]> = {
  local: [
    { key: 'root_path', label: i18nText("settings", "auto.k_946450ad2b"), type: 'string' }
  ],
  s3: [
    { key: 'endpoint', label: 'Endpoint', type: 'string' },
    { key: 'region', label: 'Region', type: 'string' },
    { key: 'bucket', label: 'Bucket', type: 'string' },
    { key: 'access_key_id', label: 'Access Key ID', type: 'string' },
    { key: 'secret_access_key', label: 'Secret Access Key', type: 'string' },
    { key: 'force_path_style', label: 'Force Path Style', type: 'string' }
  ],
  oss: [
    { key: 'endpoint', label: 'Endpoint', type: 'string' },
    { key: 'region', label: 'Region', type: 'string' },
    { key: 'bucket', label: 'Bucket', type: 'string' },
    { key: 'access_key_id', label: 'Access Key ID', type: 'string' },
    { key: 'secret_access_key', label: 'Secret Access Key', type: 'string' }
  ],
  cos: [
    { key: 'endpoint', label: 'Endpoint', type: 'string' },
    { key: 'region', label: 'Region', type: 'string' },
    { key: 'bucket', label: 'Bucket', type: 'string' },
    { key: 'access_key_id', label: 'Access Key ID', type: 'string' },
    { key: 'secret_access_key', label: 'Secret Access Key', type: 'string' }
  ],
  rustfs: [
    { key: 'endpoint', label: 'Endpoint', type: 'string' },
    { key: 'region', label: 'Region', type: 'string' },
    { key: 'bucket', label: 'Bucket', type: 'string' },
    { key: 'access_key_id', label: 'Access Key ID', type: 'string' },
    { key: 'secret_access_key', label: 'Secret Access Key', type: 'string' },
    { key: 'force_path_style', label: 'Force Path Style', type: 'string' }
  ]
};

function toStorageFormObject(
  value: Record<string, unknown> | null | undefined
): StorageFormObject {
  return (value ?? {}) as StorageFormObject;
}

export function FileStorageDrawer({
  open,
  mode,
  record,
  onClose,
  onSuccess
}: FileStorageDrawerProps) {
  const [form] = Form.useForm<StorageFormValues>();
  const [submitting, setSubmitting] = useState(false);
  const csrfToken = useAuthStore((state) => state.csrfToken);
  const isView = mode === 'view';

  useEffect(() => {
    if (open) {
      if (record && mode !== 'create') {
        form.setFieldsValue({
          code: record.code,
          title: record.title,
          driver_type: record.driver_type,
          enabled: record.enabled,
          is_default: record.is_default,
          config_json: toStorageFormObject(record.config_json),
          rule_json: toStorageFormObject(record.rule_json)
        });
      } else {
        form.resetFields();
      }
    }
  }, [open, record, mode, form]);

  const handleSubmit = async () => {
    try {
      const values = await form.validateFields();
      setSubmitting(true);

      if (!csrfToken) {
        throw new Error('missing csrf token');
      }

      const input: CreateSettingsFileStorageInput = {
        code: values.code,
        title: values.title,
        driver_type: values.driver_type,
        enabled: values.enabled,
        is_default: values.is_default,
        config_json: values.config_json ?? {},
        rule_json: values.rule_json ?? {}
      };

      if (mode === 'edit' && record) {
        const updateInput: UpdateSettingsFileStorageInput = {
          title: input.title,
          enabled: input.enabled,
          is_default: input.is_default,
          config_json: input.config_json,
          rule_json: input.rule_json
        };
        await updateSettingsFileStorage(record.id, updateInput, csrfToken);
        message.success(i18nText("settings", "auto.k_442ed441ae"));
      } else {
        await createSettingsFileStorage(input, csrfToken);
        message.success(i18nText("settings", "auto.k_3cc7d61ba4"));
      }

      onSuccess();
      onClose();
    } catch (err: unknown) {
      if (err && typeof err === 'object' && 'errorFields' in err) return;
      const msg =
        err instanceof Error ? err.message : i18nText("settings", "auto.k_51d3cb57e6");
      message.error(msg);
    } finally {
      setSubmitting(false);
    }
  };

  const currentDriver = Form.useWatch('driver_type', form);

  return (
    <Drawer
      title={
        mode === 'create'
          ? i18nText("settings", "auto.k_c6d2ecf562")
          : mode === 'edit'
            ? i18nText("settings", "auto.k_cca4df23d3")
            : i18nText("settings", "auto.k_891a7e446d")
      }
      open={open}
      onClose={onClose}
      width={520}
      extra={
        !isView ? (
          <Button type="primary" loading={submitting} onClick={handleSubmit}>
            {mode === 'create' ? i18nText("settings", "auto.k_fcbd093292") : i18nText("settings", "auto.k_fadf24dbc5")}
          </Button>
        ) : undefined
      }
    >
      <Form
        form={form}
        layout="vertical"
        disabled={isView}
        initialValues={{
          driver_type: 'local',
          enabled: true,
          is_default: false,
          config_json: {},
          rule_json: {}
        }}
      >
        <Form.Item
          name="code"
          label={i18nText("settings", "auto.k_ae17a131fe")}
          rules={[{ required: true, message: i18nText("settings", "auto.k_ac3a41d55f") }]}
        >
          <Input placeholder={i18nText("settings", "auto.k_3a288a7ebb")} disabled={mode === 'edit' || isView} />
        </Form.Item>

        <Form.Item
          name="title"
          label={i18nText("settings", "auto.k_1be7ae4fc2")}
          rules={[{ required: true, message: i18nText("settings", "auto.k_c2afb255a5") }]}
        >
          <Input placeholder={i18nText("settings", "auto.k_25437d105e")} />
        </Form.Item>

        <Form.Item
          name="driver_type"
          label={i18nText("settings", "auto.k_86414a5456")}
          rules={[{ required: true, message: i18nText("settings", "auto.k_580c8c8243") }]}
        >
          <Select options={DRIVER_TYPE_OPTIONS} disabled={mode === 'edit' || isView} />
        </Form.Item>

        <Form.Item name="enabled" label={i18nText("settings", "auto.k_d4e9ca3dd4")} valuePropName="checked">
          <Switch />
        </Form.Item>

        <Form.Item name="is_default" label={i18nText("settings", "auto.k_bb8a31330e")} valuePropName="checked">
          <Switch />
        </Form.Item>

        {currentDriver && DRIVER_FIELDS[currentDriver] && (
          <div className="storage-drawer-driver-config">
            <h4>{i18nText("settings", "auto.k_406af8cf5a")}</h4>
            {DRIVER_FIELDS[currentDriver].map((field) => (
              <Form.Item
                key={field.key}
                name={['config_json', field.key]}
                label={field.label}
              >
                {field.type === 'number' ? (
                  <InputNumber style={{ width: '100%' }} />
                ) : (
                  <Input placeholder={i18nText("settings", "auto.k_3de2d2bfc3", { value1: field.label })} />
                )}
              </Form.Item>
            ))}
          </div>
        )}

        <Form.Item name={['rule_json', 'description']} label={i18nText("settings", "auto.k_f3dc34f386")}>
          <Input.TextArea rows={2} placeholder={i18nText("settings", "auto.k_53e32830a5")} />
        </Form.Item>
      </Form>
    </Drawer>
  );
}
