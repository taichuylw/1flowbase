import { useMutation, useQueryClient } from '@tanstack/react-query';
import { Button, Form, Input, Radio, Space, Typography } from 'antd';

import { SchemaModalPanel } from '../../../shared/schema-ui/overlay-shell/SchemaModalPanel';
import { applicationsQueryKey, createApplication } from '../api/applications';
import { i18nText } from '../../../shared/i18n/text';

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
  title: i18nText("applications", "auto.k_ac31c1170b"),
  destroyOnHidden: true
} as const;

export function ApplicationCreateModal({
  open,
  csrfToken,
  onClose,
  onCreated
}: ApplicationCreateModalProps) {
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
    <SchemaModalPanel open={open} schema={applicationCreateShell} onClose={onClose}>
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
        <Form.Item label={i18nText("applications", "auto.k_e4e46c7235")} name="application_type">
          <Radio.Group>
            <Space direction="vertical" size="small">
              <Radio value="agent_flow">AgentFlow</Radio>
              <Radio value="workflow" disabled>
                Workflow
              </Radio>
            </Space>
          </Radio.Group>
        </Form.Item>

        <Typography.Text type="secondary">{i18nText("applications", "auto.k_530b63f3e0")}</Typography.Text>

        <Form.Item
          label={i18nText("applications", "auto.k_1be7ae4fc2")}
          name="name"
          rules={[{ required: true, message: i18nText("applications", "auto.k_c2afb255a5") }]}
        >
          <Input />
        </Form.Item>

        <Form.Item label={i18nText("applications", "auto.k_5ea2e0cde2")} name="description">
          <Input.TextArea rows={3} />
        </Form.Item>

        <Button type="primary" htmlType="submit" loading={mutation.isPending}>
          {i18nText("applications", "auto.k_45dc181075")}</Button>
      </Form>
    </SchemaModalPanel>
  );
}
