import { useMutation, useQueryClient } from '@tanstack/react-query';
import { Button, Form, Input, Radio, Space, Typography } from 'antd';
import { useTranslation } from 'react-i18next';

import { SchemaModalPanel } from '../../../shared/schema-ui/overlay-shell/SchemaModalPanel';
import { applicationsQueryKey, createApplication } from '../api/applications';

interface ApplicationCreateModalProps {
  open: boolean;
  csrfToken: string;
  onClose: () => void;
  onCreated: (applicationId: string) => void;
}

interface ApplicationCreateFormValues {
  application_type: 'agent_flow' | 'workflow';
  name: string;
  description: string;
}

const applicationCreateShell = {
  schemaVersion: '1.0.0',
  shellType: 'modal_panel',
  destroyOnHidden: true
} as const;

export function ApplicationCreateModal({
  open,
  csrfToken,
  onClose,
  onCreated
}: ApplicationCreateModalProps) {
  const { t } = useTranslation('applications');
  const queryClient = useQueryClient();
  const [form] = Form.useForm<ApplicationCreateFormValues>();
  const mutation = useMutation({
    mutationFn: (values: ApplicationCreateFormValues) =>
      createApplication(
        {
          application_type: values.application_type,
          name: values.name,
          description: values.description,
          icon: 'RobotOutlined',
          icon_type: 'iconfont',
          icon_background: '#E6F7F2'
        },
        csrfToken
      ),
    onSuccess: async (created) => {
      await queryClient.invalidateQueries({ queryKey: applicationsQueryKey });
      form.resetFields();
      onClose();
      onCreated(created.id);
    }
  });

  return (
    <SchemaModalPanel
      open={open}
      schema={{ ...applicationCreateShell, title: t('auto.new_application') }}
      onClose={onClose}
    >
      <Form<ApplicationCreateFormValues>
        form={form}
        layout="vertical"
        initialValues={{
          application_type: 'agent_flow',
          name: '',
          description: ''
        }}
        onFinish={(values) => mutation.mutate(values)}
      >
        <Form.Item label={t('auto.type')} name="application_type">
          <Radio.Group>
            <Space direction="vertical" size="small">
              <Radio value="agent_flow">{t('auto.application_type_agent_flow')}</Radio>
              <Radio value="workflow" disabled>
                {t('auto.application_type_workflow')}
              </Radio>
            </Space>
          </Radio.Group>
        </Form.Item>

        <Typography.Text type="secondary">{t('auto.not_open')}</Typography.Text>

        <Form.Item
          label={t('auto.name')}
          name="name"
          rules={[{ required: true, message: t('auto.name_required') }]}
        >
          <Input />
        </Form.Item>

        <Form.Item label={t('auto.description')} name="description">
          <Input.TextArea rows={3} />
        </Form.Item>

        <Button type="primary" htmlType="submit" loading={mutation.isPending}>
          {t('auto.create_application')}</Button>
      </Form>
    </SchemaModalPanel>
  );
}
