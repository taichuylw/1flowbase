import { useCallback, useMemo, useState } from 'react';

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import {
  Button,
  Form,
  Input,
  Modal,
  Popconfirm,
  Select,
  Space,
  Table,
  Tag,
  Typography,
  message,
  type TableProps
} from 'antd';
import { CopyOutlined, PlusOutlined, StopOutlined } from '@ant-design/icons';

import { useAuthStore } from '../../../state/auth-store';
import { copyTextToClipboard } from '../../../shared/ui/clipboard/copy-text';
import {
  createSettingsPersonalAccessToken,
  fetchSettingsPersonalAccessTokens,
  revokeSettingsPersonalAccessToken,
  settingsPersonalAccessTokensQueryKey,
  type CreateSettingsPersonalAccessTokenInput,
  type SettingsPersonalAccessToken
} from '../api/personal-access-tokens';
import { i18nText } from '../../../shared/i18n/text';
import { SettingsSectionSurface } from './SettingsSectionSurface';

interface CreatePersonalAccessTokenFormValues {
  name: string;
  expiration_policy: CreateSettingsPersonalAccessTokenInput['expiration_policy'];
}

function formatDateTime(value: string | null) {
  if (!value) {
    return i18nText('settings', 'auto.never_expires');
  }

  const date = new Date(value);

  if (Number.isNaN(date.getTime())) {
    return value;
  }

  return new Intl.DateTimeFormat(undefined, {
    dateStyle: 'medium',
    timeStyle: 'short'
  }).format(date);
}

function formatLastUsedAt(value: string | null) {
  return value
    ? formatDateTime(value)
    : i18nText('settings', 'auto.not_used_yet');
}

export function PersonalAccessTokensPanel() {
  const queryClient = useQueryClient();
  const csrfToken = useAuthStore((state) => state.csrfToken);
  const [createForm] = Form.useForm<CreatePersonalAccessTokenFormValues>();
  const [createModalOpen, setCreateModalOpen] = useState(false);
  const [createdToken, setCreatedToken] =
    useState<SettingsPersonalAccessToken | null>(null);

  const tokensQuery = useQuery({
    queryKey: settingsPersonalAccessTokensQueryKey,
    queryFn: fetchSettingsPersonalAccessTokens
  });

  const invalidateTokens = useCallback(
    () =>
      queryClient.invalidateQueries({
        queryKey: settingsPersonalAccessTokensQueryKey
      }),
    [queryClient]
  );

  const createMutation = useMutation({
    mutationFn: async (values: CreatePersonalAccessTokenFormValues) => {
      if (!csrfToken) {
        throw new Error('missing csrf token');
      }

      return createSettingsPersonalAccessToken(
        {
          name: values.name,
          expiration_policy: values.expiration_policy
        },
        csrfToken
      );
    },
    onSuccess: async (token) => {
      setCreatedToken(token);
      setCreateModalOpen(false);
      createForm.resetFields();
      await invalidateTokens();
    }
  });

  const revokeMutation = useMutation({
    mutationFn: async (apiKeyId: string) => {
      if (!csrfToken) {
        throw new Error('missing csrf token');
      }

      return revokeSettingsPersonalAccessToken(apiKeyId, csrfToken);
    },
    onSuccess: invalidateTokens
  });

  const expirationOptions = useMemo(
    () => [
      {
        value: '30d',
        label: i18nText('settings', 'auto.expiration_thirty_days')
      },
      {
        value: '1y',
        label: i18nText('settings', 'auto.expiration_one_year')
      },
      {
        value: '3y',
        label: i18nText('settings', 'auto.expiration_three_years')
      },
      {
        value: 'never',
        label: i18nText('settings', 'auto.never_expires')
      }
    ],
    []
  );

  const handleCreateSubmit = useCallback(
    (values: CreatePersonalAccessTokenFormValues) => {
      createMutation.mutate(values);
    },
    [createMutation]
  );

  const handleCopyCreatedToken = useCallback(async () => {
    if (!createdToken?.token) {
      return;
    }

    try {
      await copyTextToClipboard(createdToken.token);
      message.success(i18nText('settings', 'auto.copied'));
    } catch {
      message.error(i18nText('settings', 'auto.copy_failed_manual'));
    }
  }, [createdToken?.token]);

  const columns = useMemo<TableProps<SettingsPersonalAccessToken>['columns']>(
    () => [
      {
        title: i18nText('settings', 'auto.name'),
        dataIndex: 'name',
        key: 'name',
        render: (_: unknown, token) => (
          <Space direction="vertical" size={2}>
            <Typography.Text strong>{token.name}</Typography.Text>
            <Typography.Text type="secondary" style={{ fontSize: 12 }}>
              {token.id}
            </Typography.Text>
          </Space>
        )
      },
      {
        title: i18nText('settings', 'auto.token_prefix'),
        dataIndex: 'token_prefix',
        key: 'token_prefix',
        width: 160,
        render: (token_prefix: string) => (
          <Typography.Text code>{token_prefix}</Typography.Text>
        )
      },
      {
        title: i18nText('settings', 'auto.status'),
        key: 'status',
        width: 120,
        render: (_: unknown, token) =>
          token.revoked || !token.enabled ? (
            <Tag>{i18nText('settings', 'auto.revoked')}</Tag>
          ) : (
            <Tag color="green">{i18nText('settings', 'auto.enabled_alt')}</Tag>
          )
      },
      {
        title: i18nText('settings', 'auto.expires'),
        dataIndex: 'expires_at',
        key: 'expires_at',
        render: (expires_at: string | null) => formatDateTime(expires_at)
      },
      {
        title: i18nText('settings', 'auto.last_used_at'),
        dataIndex: 'last_used_at',
        key: 'last_used_at',
        render: (last_used_at: string | null) => formatLastUsedAt(last_used_at)
      },
      {
        title: i18nText('settings', 'auto.created'),
        dataIndex: 'created_at',
        key: 'created_at',
        render: (created_at: string) => formatDateTime(created_at)
      },
      {
        title: i18nText('settings', 'auto.operation'),
        key: 'action',
        width: 120,
        render: (_: unknown, token) =>
          token.revoked || !token.enabled ? null : (
            <Popconfirm
              title={i18nText('settings', 'auto.revoke_api_key')}
              description={i18nText(
                'settings',
                'auto.revoke_api_key_description',
                { value1: token.name }
              )}
              okText={i18nText('settings', 'auto.confirm_revoke')}
              cancelText={i18nText('settings', 'auto.cancel')}
              okButtonProps={{ danger: true }}
              onConfirm={() => revokeMutation.mutate(token.id)}
            >
              <Button
                danger
                size="small"
                icon={<StopOutlined />}
                loading={revokeMutation.isPending}
              >
                {i18nText('settings', 'auto.revoke')}
              </Button>
            </Popconfirm>
          )
      }
    ],
    [revokeMutation]
  );

  return (
    <SettingsSectionSurface
      title={i18nText('settings', 'auto.api_key_authentication')}
      description={i18nText('settings', 'auto.user_api_key_description')}
      titleLevel={3}
      hideHeader={false}
      heightMode="fill"
      headerActions={
        <Button
          type="primary"
          icon={<PlusOutlined />}
          onClick={() => setCreateModalOpen(true)}
        >
          {i18nText('settings', 'auto.create_api_key')}
        </Button>
      }
      status={
        <Typography.Text type="secondary">
          {i18nText('settings', 'auto.user_api_key_security_notice')}
        </Typography.Text>
      }
    >
      <Table<SettingsPersonalAccessToken>
        rowKey="id"
        loading={tokensQuery.isLoading}
        dataSource={tokensQuery.data ?? []}
        columns={columns}
        pagination={false}
        size="middle"
        locale={{ emptyText: i18nText('settings', 'auto.no_user_api_keys') }}
      />

      <Modal
        title={i18nText('settings', 'auto.create_api_key')}
        open={createModalOpen}
        onCancel={() => {
          setCreateModalOpen(false);
          createForm.resetFields();
        }}
        onOk={() => createForm.submit()}
        confirmLoading={createMutation.isPending}
        okText={i18nText('settings', 'auto.create')}
        cancelText={i18nText('settings', 'auto.cancel')}
        destroyOnHidden
      >
        <Form
          form={createForm}
          layout="vertical"
          onFinish={handleCreateSubmit}
          style={{ marginTop: 16 }}
          initialValues={{ expiration_policy: '1y' }}
        >
          <Form.Item
            label={i18nText('settings', 'auto.name')}
            name="name"
            rules={[
              {
                required: true,
                message: i18nText('settings', 'auto.fill_name')
              }
            ]}
          >
            <Input autoFocus />
          </Form.Item>
          <Form.Item
            label={i18nText('settings', 'auto.expiration_policy')}
            name="expiration_policy"
            rules={[
              {
                required: true,
                message: i18nText('settings', 'auto.please_select_expiration')
              }
            ]}
          >
            <Select options={expirationOptions} />
          </Form.Item>
        </Form>
      </Modal>

      <Modal
        title={i18nText('settings', 'auto.api_key_created')}
        open={Boolean(createdToken?.token)}
        onCancel={() => setCreatedToken(null)}
        onOk={() => setCreatedToken(null)}
        okText={i18nText('settings', 'auto.done')}
        cancelButtonProps={{ style: { display: 'none' } }}
        destroyOnHidden
      >
        <Space direction="vertical" size={12} style={{ width: '100%' }}>
          <Typography.Text type="secondary">
            {i18nText('settings', 'auto.api_key_created_once_notice')}
          </Typography.Text>
          <Input.TextArea
            aria-label={i18nText('settings', 'auto.full_token')}
            value={createdToken?.token ?? ''}
            readOnly
            autoSize={{ minRows: 2, maxRows: 4 }}
          />
          <Button icon={<CopyOutlined />} onClick={handleCopyCreatedToken}>
            {i18nText('settings', 'auto.copy_token')}
          </Button>
        </Space>
      </Modal>
    </SettingsSectionSurface>
  );
}
