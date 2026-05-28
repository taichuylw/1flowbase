import { useEffect, useState } from 'react';
import {
  Button,
  Drawer,
  Form,
  Input,
  message,
  Select
} from 'antd';
import {
  createSettingsFileTable,
  type SettingsFileStorage,
  type SettingsFileTable
} from '../api/file-management';
import { useAuthStore } from '../../../state/auth-store';
import { i18nText } from '../../../shared/i18n/text';

type DrawerMode = 'create' | 'view' | 'edit';

interface FileTableDrawerProps {
  open: boolean;
  mode: DrawerMode;
  record: SettingsFileTable | null;
  storages: SettingsFileStorage[];
  onClose: () => void;
  onSuccess: () => void;
  onUpdateBinding: (tableId: string, storageId: string) => Promise<void>;
}

interface TableFormValues {
  code: string;
  title: string;
  bound_storage_id: string;
}

export function FileTableDrawer({
  open,
  mode,
  record,
  storages,
  onClose,
  onSuccess,
  onUpdateBinding
}: FileTableDrawerProps) {
  const [form] = Form.useForm<TableFormValues>();
  const [submitting, setSubmitting] = useState(false);
  const [bindingSubmitting, setBindingSubmitting] = useState(false);
  const csrfToken = useAuthStore((state) => state.csrfToken);
  const isView = mode === 'view';

  useEffect(() => {
    if (open) {
      if (record && mode !== 'create') {
        form.setFieldsValue({
          code: record.code,
          title: record.title,
          bound_storage_id: record.bound_storage_id || ''
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

      await createSettingsFileTable(
        { code: values.code, title: values.title },
        csrfToken
      );

      message.success(i18nText("settings", "auto.k_4955c06b3f"));
      onSuccess();
      onClose();
    } catch (err: unknown) {
      if (err && typeof err === 'object' && 'errorFields' in err) return;
      const msg =
        err instanceof Error ? err.message : i18nText("settings", "auto.k_af973c1b23");
      message.error(msg);
    } finally {
      setSubmitting(false);
    }
  };

  const handleBindingSave = async () => {
    if (!record) return;
    try {
      const values = form.getFieldsValue();
      setBindingSubmitting(true);
      await onUpdateBinding(record.id, values.bound_storage_id || '');
      message.success(i18nText("settings", "auto.k_e9306fa89e"));
      onSuccess();
      onClose();
    } catch (err: unknown) {
      const msg =
        err instanceof Error ? err.message : i18nText("settings", "auto.k_e984b83605");
      message.error(msg);
    } finally {
      setBindingSubmitting(false);
    }
  };

  const storageOptions = storages.map((s) => ({
    label: `${s.title} (${s.code})`,
    value: s.id
  }));

  return (
    <Drawer
      title={
        mode === 'create'
          ? i18nText("settings", "auto.k_97d5d4da3a")
          : mode === 'edit'
            ? i18nText("settings", "auto.k_05e5f28c5f")
            : i18nText("settings", "auto.k_2de570621b")
      }
      open={open}
      onClose={onClose}
      width={480}
      extra={
        !isView ? (
          mode === 'edit' ? (
            <Button
              type="primary"
              loading={bindingSubmitting}
              onClick={handleBindingSave}
            >
              {i18nText("settings", "auto.k_547fd26235")}</Button>
          ) : (
            <Button type="primary" loading={submitting} onClick={handleSubmit}>
              {i18nText("settings", "auto.k_fcbd093292")}</Button>
          )
        ) : undefined
      }
    >
      <Form
        form={form}
        layout="vertical"
        disabled={isView}
        initialValues={{ bound_storage_id: '' }}
      >
        <Form.Item
          name="code"
          label={i18nText("settings", "auto.k_469515fc79")}
          rules={[{ required: true, message: i18nText("settings", "auto.k_237ebdde3c") }]}
        >
          <Input placeholder={i18nText("settings", "auto.k_7f942f1b59")} disabled={mode !== 'create'} />
        </Form.Item>

        <Form.Item
          name="title"
          label={i18nText("settings", "auto.k_1be7ae4fc2")}
          rules={[{ required: true, message: i18nText("settings", "auto.k_c2afb255a5") }]}
        >
          <Input placeholder={i18nText("settings", "auto.k_6bf29a4b1e")} disabled={isView} />
        </Form.Item>

        {mode !== 'create' && (
          <>
            <Form.Item
              name="bound_storage_id"
              label={i18nText("settings", "auto.k_47224aad23")}
              rules={[{ required: true, message: i18nText("settings", "auto.k_53d6ae4774") }]}
            >
              <Select
                options={storageOptions}
                placeholder={i18nText("settings", "auto.k_3ef2c80e54")}
                allowClear
                disabled={isView}
              />
            </Form.Item>

            <Form.Item label={i18nText("settings", "auto.k_689434b4ec")}>
              <Input
                value={
                  record
                    ? `${record.scope_kind} / ${record.scope_id}`
                    : '-'
                }
                disabled
              />
            </Form.Item>

            {record && (
              <>
                <Form.Item label={i18nText("settings", "auto.k_83a5c20bbf")}>
                  <Input value={record.is_builtin ? i18nText("settings", "auto.k_30160a21b9") : i18nText("settings", "auto.k_8bf5c10ad9")} disabled />
                </Form.Item>
                <Form.Item label={i18nText("settings", "auto.k_62e951a692")}>
                  <Input value={record.status} disabled />
                </Form.Item>
              </>
            )}
          </>
        )}
      </Form>

      {mode === 'edit' && record && !isView && (
        <div style={{ marginTop: 16, color: '#888', fontSize: 13 }}>
          {i18nText("settings", "auto.k_f0e90f8eab")}</div>
      )}
    </Drawer>
  );
}
