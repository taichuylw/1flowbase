import { useState } from 'react';

import { DeleteOutlined, KeyOutlined, QuestionCircleOutlined } from '@ant-design/icons';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { App, Button, Form, Input, Modal, Space, Table, Tooltip, Typography } from 'antd';

import { copyTextToClipboard } from '../../../../shared/ui/clipboard/copy-text';
import { applicationDetailQueryKey } from '../../api/applications';
import {
  applicationApiKeysQueryKey,
  createApplicationApiKey,
  fetchApplicationApiKeys,
  revokeApplicationApiKey,
  type ApplicationApiKey,
  type CreatedApplicationApiKey
} from '../../api/public-api';
import { i18nText } from '../../../../shared/i18n/text';

const SHANGHAI_DATE_TIME_FORMATTER = new Intl.DateTimeFormat('en-US', {
  timeZone: 'Asia/Shanghai',
  year: 'numeric',
  month: '2-digit',
  day: '2-digit',
  hour: '2-digit',
  minute: '2-digit',
  second: '2-digit',
  hourCycle: 'h23'
});

function formatDateTime(value: string) {
  const date = new Date(value);

  if (Number.isNaN(date.getTime())) {
    return value;
  }

  const parts = SHANGHAI_DATE_TIME_FORMATTER.formatToParts(date);
  const partByType = new Map(parts.map((part) => [part.type, part.value]));

  return [
    `${partByType.get('year')}-${partByType.get('month')}-${partByType.get('day')}`,
    `${partByType.get('hour')}:${partByType.get('minute')}:${partByType.get('second')}`
  ].join(' ');
}

function formatOptionalDateTime(value: string | null | undefined) {
  return value ? formatDateTime(value) : i18nText("applications", "auto.k_13cec83596");
}

export function ApplicationApiKeysPanel({
  applicationId,
  csrfToken,
  onCreatedToken,
  variant = 'panel'
}: {
  applicationId: string;
  csrfToken: string;
  onCreatedToken: (token: string | null) => void;
  variant?: 'panel' | 'embedded';
}) {
  const queryClient = useQueryClient();
  const { message } = App.useApp();
  const [createOpen, setCreateOpen] = useState(false);
  const [listOpen, setListOpen] = useState(false);
  const [createdKey, setCreatedKey] = useState<CreatedApplicationApiKey | null>(null);
  const [form] = Form.useForm<{ name: string }>();
  const keysQuery = useQuery({
    queryKey: applicationApiKeysQueryKey(applicationId),
    queryFn: () => fetchApplicationApiKeys(applicationId)
  });
  const createMutation = useMutation({
    mutationFn: (name: string) => createApplicationApiKey(applicationId, name, csrfToken),
    onSuccess: (key) => {
      setCreatedKey(key);
      onCreatedToken(key.token);
      setCreateOpen(false);
      form.resetFields();
      void queryClient.invalidateQueries({
        queryKey: applicationApiKeysQueryKey(applicationId)
      });
      void queryClient.invalidateQueries({
        queryKey: applicationDetailQueryKey(applicationId)
      });
    }
  });
  const revokeMutation = useMutation({
    mutationFn: (keyId: string) => revokeApplicationApiKey(applicationId, keyId, csrfToken),
    onSuccess: () => {
      message.success(i18nText("applications", "auto.k_574dd3f1cc"));
      void queryClient.invalidateQueries({
        queryKey: applicationApiKeysQueryKey(applicationId)
      });
      void queryClient.invalidateQueries({
        queryKey: applicationDetailQueryKey(applicationId)
      });
    }
  });

  const keys = keysQuery.data ?? [];
  const maskTokenPreview = (value: string) => {
    return `${value}****`;
  };
  const copyCreatedToken = () => {
    const token = createdKey?.token;
    if (!token) {
      return;
    }
    copyTextToClipboard(token).then(
      () => message.success(i18nText("applications", "auto.k_8b68a9f539")),
      () => message.error(i18nText("applications", "auto.k_f2c6b5167b"))
    );
  };
  const keyTable = (
    <Table<ApplicationApiKey>
      rowKey="id"
      loading={keysQuery.isLoading}
      dataSource={keys}
      pagination={false}
      columns={[
        { title: i18nText("applications", "auto.k_1be7ae4fc2"), dataIndex: 'name' },
        {
          title: (
            <Space size={6}>
              <span>{i18nText("applications", "auto.k_0d1965e139")}</span>
              <Tooltip title={i18nText("applications", "auto.k_bf490b9cf1")}>
                <QuestionCircleOutlined aria-label={i18nText("applications", "auto.k_984b8989e3")} />
              </Tooltip>
            </Space>
          ),
          dataIndex: 'token_prefix',
          width: 210,
          render: (value: string) => (
            <Typography.Text code>{maskTokenPreview(value)}</Typography.Text>
          )
        },
        {
          title: i18nText("applications", "auto.k_84e3802f60"),
          dataIndex: 'created_at',
          width: 200,
          render: (value: string) => formatDateTime(value)
        },
        {
          title: i18nText("applications", "auto.k_8ccd127a3b"),
          dataIndex: 'last_used_at',
          width: 200,
          render: (value: string | null | undefined) => formatOptionalDateTime(value)
        },
        {
          title: i18nText("applications", "auto.k_f3ea6d345e"),
          key: 'actions',
          width: 96,
          render: (_, record) => (
            <Button
              danger
              icon={<DeleteOutlined />}
              size="small"
              type="text"
              aria-label={i18nText("applications", "auto.k_3755f56f2f")}
              loading={revokeMutation.isPending}
              onClick={() => revokeMutation.mutate(record.id)}
            />
          )
        }
      ]}
    />
  );

  if (variant === 'embedded') {
    return (
      <>
        <Button
          aria-label={i18nText("applications", "auto.k_5df1d83e18")}
          className="application-api-key-trigger"
          icon={<KeyOutlined />}
          onClick={() => setListOpen(true)}
        >
          {i18nText("applications", "auto.k_5df1d83e18")}</Button>
        <Modal
          title="API Keys"
          open={listOpen}
          destroyOnHidden
          width={840}
          footer={null}
          onCancel={() => setListOpen(false)}
        >
          <Space direction="vertical" size={16} className="application-api-key-list-modal">
            <div className="application-api-panel__header">
              <Typography.Text type="secondary">
                {i18nText("applications", "auto.k_62cfc53516")}{keys.length} {i18nText("applications", "auto.k_1bcb780957")}</Typography.Text>
              <Button type="primary" onClick={() => setCreateOpen(true)}>
                {i18nText("applications", "auto.k_bd80d911f1")}</Button>
            </div>
            {keyTable}
          </Space>
        </Modal>
        <Modal
          title={i18nText("applications", "auto.k_f02ce7394c")}
          open={createOpen}
          destroyOnHidden
          okText={i18nText("applications", "auto.k_fcbd093292")}
          confirmLoading={createMutation.isPending}
          onCancel={() => setCreateOpen(false)}
          onOk={() => form.submit()}
        >
          <Form
            form={form}
            layout="vertical"
            onFinish={(values) => createMutation.mutate(values.name)}
          >
            <Form.Item
              name="name"
              label={i18nText("applications", "auto.k_e8a3d906a8")}
              rules={[{ required: true, message: i18nText("applications", "auto.k_45ee57a35e") }]}
            >
              <Input />
            </Form.Item>
          </Form>
        </Modal>
        <Modal
          title={i18nText("applications", "auto.k_4bcdd71eeb")}
          open={Boolean(createdKey)}
          className="application-api-created-key-modal"
          destroyOnHidden
          onCancel={() => setCreatedKey(null)}
          footer={[
            <Button key="close" type="text" onClick={() => setCreatedKey(null)}>
              {i18nText("applications", "auto.k_6c14bd7f6f")}</Button>,
            <Button
              key="copy"
              aria-label={i18nText("applications", "auto.k_4edd1d0087")}
              className="application-api-created-token-copy"
              onClick={copyCreatedToken}
            >
              {i18nText("applications", "auto.k_4edd1d0087")}</Button>
          ]}
        >
          <Space direction="vertical" className="application-api-token-modal">
            <Typography.Text>{i18nText("applications", "auto.k_79d9d1edd7")}</Typography.Text>
            <Typography.Text type="secondary">
              {i18nText("applications", "auto.k_97afb20b3b")}</Typography.Text>
            <Typography.Text className="application-api-created-token">
              {createdKey?.token}
            </Typography.Text>
          </Space>
        </Modal>
      </>
    );
  }

  return (
    <section className="application-api-panel">
      <div className="application-api-panel__header">
        <div>
          <Typography.Title level={4}>API Keys</Typography.Title>
          <Typography.Text type="secondary">{i18nText("applications", "auto.k_391a547696")}</Typography.Text>
        </div>
        <Button type="primary" onClick={() => setCreateOpen(true)}>
          {i18nText("applications", "auto.k_bd80d911f1")}</Button>
      </div>
      {keyTable}
      <Modal
        title={i18nText("applications", "auto.k_f02ce7394c")}
        open={createOpen}
        destroyOnHidden
        okText={i18nText("applications", "auto.k_fcbd093292")}
        confirmLoading={createMutation.isPending}
        onCancel={() => setCreateOpen(false)}
        onOk={() => form.submit()}
      >
        <Form
          form={form}
          layout="vertical"
          onFinish={(values) => createMutation.mutate(values.name)}
        >
          <Form.Item
            name="name"
            label={i18nText("applications", "auto.k_e8a3d906a8")}
            rules={[{ required: true, message: i18nText("applications", "auto.k_45ee57a35e") }]}
          >
            <Input />
          </Form.Item>
        </Form>
      </Modal>
      <Modal
        title={i18nText("applications", "auto.k_4bcdd71eeb")}
        open={Boolean(createdKey)}
        className="application-api-created-key-modal"
        destroyOnHidden
        onCancel={() => setCreatedKey(null)}
        footer={[
          <Button key="close" type="text" onClick={() => setCreatedKey(null)}>
            {i18nText("applications", "auto.k_6c14bd7f6f")}</Button>,
          <Button
            key="copy"
            aria-label={i18nText("applications", "auto.k_4edd1d0087")}
            className="application-api-created-token-copy"
            onClick={copyCreatedToken}
          >
            {i18nText("applications", "auto.k_4edd1d0087")}</Button>
        ]}
      >
        <Space direction="vertical" className="application-api-token-modal">
          <Typography.Text>{i18nText("applications", "auto.k_79d9d1edd7")}</Typography.Text>
          <Typography.Text type="secondary">
            {i18nText("applications", "auto.k_97afb20b3b")}</Typography.Text>
          <Typography.Text className="application-api-created-token">
            {createdKey?.token}
          </Typography.Text>
        </Space>
      </Modal>
    </section>
  );
}
