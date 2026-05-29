import { Form, Input, Modal } from 'antd';
import { useEffect } from 'react';
import { useTranslation } from 'react-i18next';

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
  const { t } = useTranslation('applications');
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
      title={t('auto.edit_application_information')}
      okText={t('auto.save_changes')}
      cancelText={t('auto.cancel')}
      confirmLoading={saving}
      onCancel={onCancel}
      onOk={() => form.submit()}
      destroyOnHidden
      forceRender
    >
      <Form form={form} layout="vertical" onFinish={onSubmit}>
        <Form.Item
          label={t('auto.application_name')}
          name="name"
          rules={[{ required: true, message: t('auto.application_name_required') }]}
        >
          <Input maxLength={64} aria-label={t('auto.application_name')} />
        </Form.Item>
        <Form.Item label={t('auto.application_description')} name="description">
          <Input.TextArea rows={4} maxLength={240} aria-label={t('auto.application_description')} />
        </Form.Item>
      </Form>
    </Modal>
  );
}
