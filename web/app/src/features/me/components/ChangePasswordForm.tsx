import { Alert, Button, Form, Input, Space, Typography } from 'antd';

import type { ChangeMyPasswordInput } from '../api/me';
import { i18nText } from '../../../shared/i18n/text';

interface ChangePasswordValues {
  old_password: string;
  new_password: string;
  confirm_password: string;
}

export function ChangePasswordForm({
  className,
  submitting,
  errorMessage,
  onSubmit
}: {
  className?: string;
  submitting: boolean;
  errorMessage: string | null;
  onSubmit: (input: ChangeMyPasswordInput) => Promise<void> | void;
}) {
  const [form] = Form.useForm<ChangePasswordValues>();

  return (
    <Space className={className} direction="vertical" size="large">
      <div>
        <Typography.Title level={3}>{i18nText("me", "auto.k_8bf435e8a8")}</Typography.Title>
        <Typography.Paragraph>
          {i18nText("me", "auto.k_f10b511381")}</Typography.Paragraph>
      </div>

      {errorMessage ? <Alert type="error" message={errorMessage} showIcon /> : null}

      <Form<ChangePasswordValues>
        form={form}
        layout="vertical"
        onFinish={async (values) => {
          await onSubmit({
            old_password: values.old_password,
            new_password: values.new_password
          });
          form.resetFields();
        }}
      >
        <Form.Item
          label={i18nText("me", "auto.k_c839a8ff17")}
          name="old_password"
          rules={[{ required: true, message: i18nText("me", "auto.k_5ccbfd6edd") }]}
        >
          <Input.Password />
        </Form.Item>
        <Form.Item
          label={i18nText("me", "auto.k_d22c9c0085")}
          name="new_password"
          rules={[{ required: true, message: i18nText("me", "auto.k_7543b01a32") }]}
        >
          <Input.Password />
        </Form.Item>
        <Form.Item
          label={i18nText("me", "auto.k_d4477adb6f")}
          name="confirm_password"
          dependencies={['new_password']}
          rules={[
            { required: true, message: i18nText("me", "auto.k_4850ecf733") },
            ({ getFieldValue }) => ({
              validator(_, value) {
                if (!value || value === getFieldValue('new_password')) {
                  return Promise.resolve();
                }

                return Promise.reject(new Error(i18nText("me", "auto.k_00348363de")));
              }
            })
          ]}
        >
          <Input.Password />
        </Form.Item>
        <Button type="primary" htmlType="submit" loading={submitting}>
          {i18nText("me", "auto.k_2f3ed4c7a4")}</Button>
      </Form>
    </Space>
  );
}
