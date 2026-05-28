import { Form, Input, Modal } from 'antd';
import { useEffect } from 'react';
import { i18nText } from '../../../shared/i18n/text';

interface ApplicationEditModalProps {
  open: boolean;
  application:
    | {
        name: string;
        description: string;
      }
    | null;
  saving?: boolean;
  onCancel: () => void;
  onSubmit: (values: { name: string; description: string }) => void;
}

export function ApplicationEditModal({
  open,
  application,
  saving = false,
  onCancel,
  onSubmit
}: ApplicationEditModalProps) {
  const [form] = Form.useForm<{ name: string; description: string }>();

  useEffect(() => {
    if (!open) {
      form.resetFields();
      return;
    }

    form.setFieldsValue({
      name: application?.name ?? '',
      description: application?.description ?? ''
    });
  }, [application, form, open]);

  return (
    <Modal
      open={open}
      title={i18nText("applications", "auto.k_4e38d03d7d")}
      okText={i18nText("applications", "auto.k_60b4ae9082")}
      cancelText={i18nText("applications", "auto.k_4d0b4688c7")}
      confirmLoading={saving}
      onCancel={onCancel}
      onOk={() => form.submit()}
      destroyOnHidden
      forceRender
    >
      <Form form={form} layout="vertical" onFinish={onSubmit}>
        <Form.Item
          label={i18nText("applications", "auto.k_2d87d51825")}
          name="name"
          rules={[{ required: true, message: i18nText("applications", "auto.k_183bb0289f") }]}
        >
          <Input maxLength={64} aria-label={i18nText("applications", "auto.k_2d87d51825")} />
        </Form.Item>
        <Form.Item label={i18nText("applications", "auto.k_9b58608132")} name="description">
          <Input.TextArea rows={4} maxLength={240} aria-label={i18nText("applications", "auto.k_9b58608132")} />
        </Form.Item>
      </Form>
    </Modal>
  );
}
