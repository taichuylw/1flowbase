import { useCallback, useMemo, useState } from 'react';

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import {
  Avatar,
  Button,
  Form,
  Input,
  Modal,
  Popconfirm,
  Select,
  Space,
  Switch,
  Table,
  Tag,
  Typography
} from 'antd';
import {
  EditOutlined,
  KeyOutlined,
  StopOutlined,
  UserAddOutlined
} from '@ant-design/icons';

import { useAuthStore } from '../../../state/auth-store';
import {
  createSettingsMember,
  disableSettingsMember,
  fetchSettingsMembers,
  replaceSettingsMemberRoles,
  resetSettingsMemberPassword,
  settingsMembersQueryKey,
  type SettingsMember
} from '../api/members';
import { fetchSettingsRoles, settingsRolesQueryKey } from '../api/roles';
import { SettingsSectionSurface } from './SettingsSectionSurface';
import { i18nText } from '../../../shared/i18n/text';

const TEMP_PASSWORD = 'Temp@123456';

export function MemberManagementPanel({
  canManageMembers,
  canManageRoleBindings
}: {
  canManageMembers: boolean;
  canManageRoleBindings: boolean;
}) {
  const queryClient = useQueryClient();
  const csrfToken = useAuthStore((state) => state.csrfToken);

  const [createForm] = Form.useForm();
  const [createModalOpen, setCreateModalOpen] = useState(false);
  const [roleEditMember, setRoleEditMember] = useState<SettingsMember | null>(
    null
  );
  const [editingRoleCodes, setEditingRoleCodes] = useState<string[]>([]);

  const membersQuery = useQuery({
    queryKey: settingsMembersQueryKey,
    queryFn: fetchSettingsMembers
  });
  const rolesQuery = useQuery({
    queryKey: settingsRolesQueryKey,
    queryFn: fetchSettingsRoles,
    enabled: canManageRoleBindings
  });

  const invalidateMembers = () =>
    queryClient.invalidateQueries({ queryKey: settingsMembersQueryKey });

  const createMutation = useMutation({
    mutationFn: async (values: Record<string, unknown>) => {
      if (!csrfToken) {
        throw new Error('missing csrf token');
      }

      return createSettingsMember(
        {
          account: String(values.account ?? ''),
          email: String(values.email ?? ''),
          phone: values.phone ? String(values.phone) : null,
          password: String(values.password ?? ''),
          name: String(values.name ?? ''),
          nickname: String(values.nickname ?? ''),
          introduction: String(values.introduction ?? ''),
          email_login_enabled: Boolean(values.email_login_enabled),
          phone_login_enabled: Boolean(values.phone_login_enabled)
        },
        csrfToken
      );
    },
    onSuccess: async () => {
      createForm.resetFields();
      setCreateModalOpen(false);
      await invalidateMembers();
    }
  });

  const disableMutation = useMutation({
    mutationFn: async (memberId: string) => {
      if (!csrfToken) {
        throw new Error('missing csrf token');
      }

      return disableSettingsMember(memberId, csrfToken);
    },
    onSuccess: invalidateMembers
  });

  const resetPasswordMutation = useMutation({
    mutationFn: async (memberId: string) => {
      if (!csrfToken) {
        throw new Error('missing csrf token');
      }

      return resetSettingsMemberPassword(
        memberId,
        { new_password: TEMP_PASSWORD },
        csrfToken
      );
    }
  });

  const replaceRolesMutation = useMutation({
    mutationFn: async ({
      memberId,
      roleCodes
    }: {
      memberId: string;
      roleCodes: string[];
    }) => {
      if (!csrfToken) {
        throw new Error('missing csrf token');
      }

      return replaceSettingsMemberRoles(
        memberId,
        { role_codes: roleCodes },
        csrfToken
      );
    },
    onSuccess: async () => {
      await invalidateMembers();
      setRoleEditMember(null);
    }
  });

  const handleCreateSubmit = useCallback(
    (values: Record<string, unknown>) => {
      createMutation.mutate(values);
    },
    [createMutation]
  );

  const handleOpenRoleEdit = useCallback((member: SettingsMember) => {
    setRoleEditMember(member);
    setEditingRoleCodes(member.role_codes);
  }, []);

  const handleRoleEditOk = useCallback(() => {
    if (roleEditMember) {
      replaceRolesMutation.mutate({
        memberId: roleEditMember.id,
        roleCodes: editingRoleCodes
      });
    }
  }, [roleEditMember, editingRoleCodes, replaceRolesMutation]);

  const roleOptions = useMemo(
    () =>
      (rolesQuery.data ?? []).map((role) => ({
        label: role.name,
        value: role.code
      })),
    [rolesQuery.data]
  );

  const columns = useMemo(
    () => [
      {
        title: i18nText("settings", "auto.k_9ba763ea34"),
        key: 'user',
        render: (_: unknown, member: SettingsMember) => (
          <Space>
            <Avatar
              size="small"
              style={{ backgroundColor: '#00d084', flexShrink: 0 }}
            >
              {(member.name ?? member.account).charAt(0).toUpperCase()}
            </Avatar>
            <div style={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
              <Typography.Text strong style={{ fontSize: 14 }}>
                {member.name}
              </Typography.Text>
              <Typography.Text type="secondary" style={{ fontSize: 12 }}>
                {member.account}
                {member.nickname && member.nickname !== member.name
                  ? ` · ${member.nickname}`
                  : ''}
              </Typography.Text>
            </div>
          </Space>
        )
      },
      {
        title: i18nText("settings", "auto.k_60beedc8f2"),
        key: 'contact',
        render: (_: unknown, member: SettingsMember) => (
          <div style={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
            <Typography.Text style={{ fontSize: 13 }}>
              {member.email}
            </Typography.Text>
            {member.phone ? (
              <Typography.Text type="secondary" style={{ fontSize: 12 }}>
                {member.phone}
              </Typography.Text>
            ) : null}
          </div>
        )
      },
      {
        title: i18nText("settings", "auto.k_62e951a692"),
        key: 'status',
        width: 80,
        render: (_: unknown, member: SettingsMember) => (
          <Tag color={member.status === 'active' ? 'green' : 'default'}>
            {member.status === 'active' ? i18nText("settings", "auto.k_d4e9ca3dd4") : i18nText("settings", "auto.k_d989e55188")}
          </Tag>
        )
      },
      {
        title: i18nText("settings", "auto.k_6b26695e4d"),
        key: 'roles',
        render: (_: unknown, member: SettingsMember) => {
          const isRootMember = member.role_codes.includes('root');

          return canManageRoleBindings ? (
            <Space wrap size={4}>
              {member.role_codes.map((roleCode) => (
                <Tag key={roleCode}>{roleCode}</Tag>
              ))}
              <Button
                type="link"
                size="small"
                icon={<EditOutlined />}
                style={{ padding: '0 4px', fontSize: 12 }}
                disabled={isRootMember}
                onClick={
                  isRootMember ? undefined : () => handleOpenRoleEdit(member)
                }
              >
                {i18nText("settings", "auto.k_a7f814c0a4")}</Button>
            </Space>
          ) : (
            <Space wrap size={4}>
              {member.role_codes.map((roleCode) => (
                <Tag key={roleCode}>{roleCode}</Tag>
              ))}
            </Space>
          );
        }
      },
      ...(canManageMembers
        ? [
            {
              title: i18nText("settings", "auto.k_f3ea6d345e"),
              key: 'action',
              width: 160,
              render: (_: unknown, member: SettingsMember) => {
                const isRootMember = member.role_codes.includes('root');

                return (
                  <Space size={4}>
                    {member.status === 'active' ? (
                      isRootMember ? (
                        <Button
                          size="small"
                          danger
                          icon={<StopOutlined />}
                          disabled
                        >
                          {i18nText("settings", "auto.k_d989e55188")}</Button>
                      ) : (
                        <Popconfirm
                          title={i18nText("settings", "auto.k_fcd9326404")}
                          description={i18nText("settings", "auto.k_20daa2e828", { value1: member.name })}
                          onConfirm={() => disableMutation.mutate(member.id)}
                          okText={i18nText("settings", "auto.k_f3abd89419")}
                          cancelText={i18nText("settings", "auto.k_4d0b4688c7")}
                          okButtonProps={{ danger: true }}
                        >
                          <Button
                            size="small"
                            danger
                            icon={<StopOutlined />}
                            loading={disableMutation.isPending}
                          >
                            {i18nText("settings", "auto.k_d989e55188")}</Button>
                        </Popconfirm>
                      )
                    ) : null}
                    {isRootMember ? (
                      <Button size="small" icon={<KeyOutlined />} disabled>
                        {i18nText("settings", "auto.k_7e422146dd")}</Button>
                    ) : (
                      <Popconfirm
                        title={i18nText("settings", "auto.k_7e422146dd")}
                        description={i18nText("settings", "auto.k_1320a9789d", { value1: member.name })}
                        onConfirm={() =>
                          resetPasswordMutation.mutate(member.id)
                        }
                        okText={i18nText("settings", "auto.k_30a6079ce3")}
                        cancelText={i18nText("settings", "auto.k_4d0b4688c7")}
                      >
                        <Button
                          size="small"
                          icon={<KeyOutlined />}
                          loading={resetPasswordMutation.isPending}
                        >
                          {i18nText("settings", "auto.k_7e422146dd")}</Button>
                      </Popconfirm>
                    )}
                  </Space>
                );
              }
            }
          ]
        : [])
    ],
    [
      canManageMembers,
      canManageRoleBindings,
      disableMutation,
      resetPasswordMutation,
      handleOpenRoleEdit
    ]
  );

  return (
    <SettingsSectionSurface title={i18nText("settings", "auto.k_baf84751a2")} hideHeader heightMode="fill">
      <div style={{ display: 'flex', flexDirection: 'column', gap: 20 }}>
        {canManageMembers ? (
          <div style={{ display: 'flex', justifyContent: 'flex-end' }}>
            <Button
              type="primary"
              icon={<UserAddOutlined />}
              onClick={() => setCreateModalOpen(true)}
            >
              {i18nText("settings", "auto.k_30416407e5")}</Button>
          </div>
        ) : null}

        <Table<SettingsMember>
          rowKey="id"
          loading={membersQuery.isLoading}
          dataSource={membersQuery.data ?? []}
          pagination={false}
          columns={columns}
          size="middle"
        />

        <Typography.Text type="secondary" style={{ fontSize: 12 }}>
          {i18nText("settings", "auto.k_98314eb9f7")}</Typography.Text>

        {/* Create Member Modal */}
        <Modal
          title={i18nText("settings", "auto.k_30416407e5")}
          open={createModalOpen}
          onCancel={() => {
            setCreateModalOpen(false);
            createForm.resetFields();
          }}
          onOk={() => createForm.submit()}
          confirmLoading={createMutation.isPending}
          okText={i18nText("settings", "auto.k_fcbd093292")}
          cancelText={i18nText("settings", "auto.k_4d0b4688c7")}
          width={600}
          destroyOnHidden
        >
          <Form
            form={createForm}
            layout="vertical"
            onFinish={handleCreateSubmit}
            style={{ marginTop: 16 }}
          >
            <div
              style={{
                display: 'grid',
                gridTemplateColumns: '1fr 1fr',
                gap: '0 16px'
              }}
            >
              <Form.Item
                label={i18nText("settings", "auto.k_9013849179")}
                name="account"
                rules={[{ required: true, message: i18nText("settings", "auto.k_dfcc84468b") }]}
              >
                <Input />
              </Form.Item>
              <Form.Item
                label={i18nText("settings", "auto.k_be4c2616b1")}
                name="name"
                rules={[{ required: true, message: i18nText("settings", "auto.k_675b3ee2af") }]}
              >
                <Input />
              </Form.Item>
              <Form.Item
                label={i18nText("settings", "auto.k_9ed627bcf6")}
                name="email"
                rules={[
                  { required: true, message: i18nText("settings", "auto.k_f2dc24deb8") },
                  { type: 'email', message: i18nText("settings", "auto.k_bc18737362") }
                ]}
              >
                <Input />
              </Form.Item>
              <Form.Item label={i18nText("settings", "auto.k_5a9cc5e891")} name="phone">
                <Input />
              </Form.Item>
              <Form.Item
                label={i18nText("settings", "auto.k_25124ed74c")}
                name="nickname"
                rules={[{ required: true, message: i18nText("settings", "auto.k_5dbe6b07eb") }]}
              >
                <Input />
              </Form.Item>
              <Form.Item
                label={i18nText("settings", "auto.k_961fcbb15b")}
                name="password"
                initialValue={TEMP_PASSWORD}
                rules={[{ required: true, message: i18nText("settings", "auto.k_aebb4bd95f") }]}
              >
                <Input.Password />
              </Form.Item>
            </div>
            <Form.Item label={i18nText("settings", "auto.k_eff7665554")} name="introduction">
              <Input.TextArea rows={2} />
            </Form.Item>
            <div style={{ display: 'flex', gap: 24 }}>
              <Form.Item
                label={i18nText("settings", "auto.k_35e18cc8fd")}
                name="email_login_enabled"
                valuePropName="checked"
                initialValue
              >
                <Switch />
              </Form.Item>
              <Form.Item
                label={i18nText("settings", "auto.k_20feae5e89")}
                name="phone_login_enabled"
                valuePropName="checked"
                initialValue={false}
              >
                <Switch />
              </Form.Item>
            </div>
          </Form>
        </Modal>

        {/* Role Edit Modal */}
        <Modal
          title={
            roleEditMember ? i18nText("settings", "auto.k_97c19a33ef", { value1: roleEditMember.name }) : i18nText("settings", "auto.k_b9dcd82a7b")
          }
          open={Boolean(roleEditMember)}
          onCancel={() => setRoleEditMember(null)}
          onOk={handleRoleEditOk}
          confirmLoading={replaceRolesMutation.isPending}
          okText={i18nText("settings", "auto.k_fadf24dbc5")}
          cancelText={i18nText("settings", "auto.k_4d0b4688c7")}
          width={480}
          destroyOnHidden
        >
          {roleEditMember ? (
            <div style={{ marginTop: 16 }}>
              <Typography.Text
                type="secondary"
                style={{ display: 'block', marginBottom: 12, fontSize: 13 }}
              >
                {i18nText("settings", "auto.k_02385e4b97")}{roleEditMember.name}（{roleEditMember.account}{i18nText("settings", "auto.k_43356d3ef8")}</Typography.Text>
              <Select
                mode="multiple"
                style={{ width: '100%' }}
                value={editingRoleCodes}
                onChange={setEditingRoleCodes}
                options={roleOptions}
                placeholder={i18nText("settings", "auto.k_2150939fb1")}
              />
            </div>
          ) : null}
        </Modal>
      </div>
    </SettingsSectionSurface>
  );
}
