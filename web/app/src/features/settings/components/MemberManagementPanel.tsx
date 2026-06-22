import { useCallback, useMemo, useState } from 'react';

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useNavigate } from '@tanstack/react-router';
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
  changeCurrentUserPassword,
  createSettingsMember,
  disableSettingsMember,
  fetchSettingsMembers,
  replaceSettingsMemberRoles,
  resetSettingsMemberPassword,
  settingsMembersQueryKey,
  updateSettingsMember,
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
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const csrfToken = useAuthStore((state) => state.csrfToken);
  const actor = useAuthStore((state) => state.actor);
  const setAnonymous = useAuthStore((state) => state.setAnonymous);

  const [createForm] = Form.useForm();
  const [profileForm] = Form.useForm();
  const [passwordForm] = Form.useForm();
  const [createModalOpen, setCreateModalOpen] = useState(false);
  const [profileEditMember, setProfileEditMember] =
    useState<SettingsMember | null>(null);
  const [passwordEditMember, setPasswordEditMember] =
    useState<SettingsMember | null>(null);
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

  const updateMemberMutation = useMutation({
    mutationFn: async ({
      memberId,
      values
    }: {
      memberId: string;
      values: Record<string, unknown>;
    }) => {
      if (!csrfToken) {
        throw new Error('missing csrf token');
      }

      return updateSettingsMember(
        memberId,
        {
          name: String(values.name ?? ''),
          nickname: String(values.nickname ?? ''),
          email: String(values.email ?? ''),
          phone: values.phone ? String(values.phone) : null,
          introduction: String(values.introduction ?? '')
        },
        csrfToken
      );
    },
    onSuccess: async () => {
      await invalidateMembers();
      setProfileEditMember(null);
      profileForm.resetFields();
    }
  });

  const changePasswordMutation = useMutation({
    mutationFn: async (values: Record<string, unknown>) => {
      if (!csrfToken) {
        throw new Error('missing csrf token');
      }

      return changeCurrentUserPassword(
        {
          old_password: String(values.old_password ?? ''),
          new_password: String(values.new_password ?? '')
        },
        csrfToken
      );
    },
    onSuccess: async () => {
      passwordForm.resetFields();
      setPasswordEditMember(null);
      setAnonymous();
      await navigate({ to: '/sign-in' });
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

  const handleOpenProfileEdit = useCallback(
    (member: SettingsMember) => {
      setProfileEditMember(member);
      profileForm.setFieldsValue({
        name: member.name,
        nickname: member.nickname,
        email: member.email,
        phone: member.phone,
        introduction: member.introduction
      });
    },
    [profileForm]
  );

  const handleProfileEditSubmit = useCallback(
    (values: Record<string, unknown>) => {
      if (profileEditMember) {
        updateMemberMutation.mutate({
          memberId: profileEditMember.id,
          values
        });
      }
    },
    [profileEditMember, updateMemberMutation]
  );

  const handleOpenPasswordEdit = useCallback(
    (member: SettingsMember) => {
      setPasswordEditMember(member);
      passwordForm.resetFields();
    },
    [passwordForm]
  );

  const handleRoleEditOk = useCallback(() => {
    if (roleEditMember) {
      const nextRoleCodes = roleEditMember.role_codes.includes('root')
        ? Array.from(new Set(['root', ...editingRoleCodes]))
        : editingRoleCodes;
      replaceRolesMutation.mutate({
        memberId: roleEditMember.id,
        roleCodes: nextRoleCodes
      });
    }
  }, [roleEditMember, editingRoleCodes, replaceRolesMutation]);

  const roleOptions = useMemo(
    () => {
      const options = (rolesQuery.data ?? []).map((role) => ({
        label: role.name,
        value: role.code
      }));

      if (
        roleEditMember?.role_codes.includes('root') &&
        !options.some((option) => option.value === 'root')
      ) {
        return [
          { label: 'root', value: 'root', disabled: true },
          ...options
        ];
      }

      return options;
    },
    [roleEditMember, rolesQuery.data]
  );

  const handleRoleSelectionChange = useCallback(
    (roleCodes: string[]) => {
      if (roleEditMember?.role_codes.includes('root')) {
        setEditingRoleCodes(Array.from(new Set(['root', ...roleCodes])));
        return;
      }

      setEditingRoleCodes(roleCodes);
    },
    [roleEditMember]
  );

  const columns = useMemo(
    () => [
      {
        title: i18nText("settings", "auto.user_alt"),
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
        title: i18nText("settings", "auto.contact_information"),
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
        title: i18nText("settings", "auto.status"),
        key: 'status',
        width: 80,
        render: (_: unknown, member: SettingsMember) => (
          <Tag color={member.status === 'active' ? 'green' : 'default'}>
            {member.status === 'active' ? i18nText("settings", "auto.enabled") : i18nText("settings", "auto.deactivate")}
          </Tag>
        )
      },
      {
        title: i18nText("settings", "auto.role"),
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
                onClick={() => handleOpenRoleEdit(member)}
              >
                {i18nText("settings", "auto.edit")}</Button>
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
              title: i18nText("settings", "auto.operation"),
              key: 'action',
              width: 160,
              render: (_: unknown, member: SettingsMember) => {
                const isRootMember = member.role_codes.includes('root');
                const isCurrentUser = member.id === actor?.id;

                return (
                  <Space size={4}>
                    <Button
                      size="small"
                      icon={<EditOutlined />}
                      onClick={() => handleOpenProfileEdit(member)}
                    >
                      {i18nText("settings", "auto.edit_profile")}</Button>
                    {member.status === 'active' ? (
                      isRootMember ? (
                        <Button
                          size="small"
                          danger
                          icon={<StopOutlined />}
                          disabled
                        >
                          {i18nText("settings", "auto.deactivate")}</Button>
                      ) : (
                        <Popconfirm
                          title={i18nText("settings", "auto.deactivate_account")}
                          description={i18nText("settings", "auto.sure_want_deactivate_s_account_deactivation_user_able_log", { value1: member.name })}
                          onConfirm={() => disableMutation.mutate(member.id)}
                          okText={i18nText("settings", "auto.confirm_deactivation")}
                          cancelText={i18nText("settings", "auto.cancel")}
                          okButtonProps={{ danger: true }}
                        >
                          <Button
                            size="small"
                            danger
                            icon={<StopOutlined />}
                            loading={disableMutation.isPending}
                          >
                            {i18nText("settings", "auto.deactivate")}</Button>
                        </Popconfirm>
                      )
                    ) : null}
                    {isRootMember ? (
                      <Button
                        size="small"
                        icon={<KeyOutlined />}
                        disabled={!isCurrentUser}
                        loading={changePasswordMutation.isPending}
                        onClick={
                          isCurrentUser
                            ? () => handleOpenPasswordEdit(member)
                            : undefined
                        }
                      >
                        {i18nText("settings", "auto.change_login_password")}</Button>
                    ) : (
                      <Popconfirm
                        title={i18nText("settings", "auto.reset_password")}
                        description={i18nText("settings", "auto.reset_password_temporary_password_needs_changed_immediately_user_logs", { value1: member.name })}
                        onConfirm={() =>
                          resetPasswordMutation.mutate(member.id)
                        }
                        okText={i18nText("settings", "auto.confirm_reset")}
                        cancelText={i18nText("settings", "auto.cancel")}
                      >
                        <Button
                          size="small"
                          icon={<KeyOutlined />}
                          loading={resetPasswordMutation.isPending}
                        >
                          {i18nText("settings", "auto.reset_password")}</Button>
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
      changePasswordMutation,
      actor?.id,
      handleOpenProfileEdit,
      handleOpenPasswordEdit,
      handleOpenRoleEdit
    ]
  );

  return (
    <SettingsSectionSurface title={i18nText("settings", "auto.user_management")} hideHeader heightMode="fill">
      <div style={{ display: 'flex', flexDirection: 'column', gap: 20 }}>
        {canManageMembers ? (
          <div style={{ display: 'flex', justifyContent: 'flex-end' }}>
            <Button
              type="primary"
              icon={<UserAddOutlined />}
              onClick={() => setCreateModalOpen(true)}
            >
              {i18nText("settings", "auto.create_new_user")}</Button>
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
          {i18nText("settings", "auto.resetting_password_reset_target_account_password_temporary_password_require_user")}</Typography.Text>

        {/* Create Member Modal */}
        <Modal
          title={i18nText("settings", "auto.create_new_user")}
          open={createModalOpen}
          onCancel={() => {
            setCreateModalOpen(false);
            createForm.resetFields();
          }}
          onOk={() => createForm.submit()}
          confirmLoading={createMutation.isPending}
          okText={i18nText("settings", "auto.create")}
          cancelText={i18nText("settings", "auto.cancel")}
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
                label={i18nText("settings", "auto.account_number")}
                name="account"
                rules={[{ required: true, message: i18nText("settings", "auto.enter_account_number") }]}
              >
                <Input />
              </Form.Item>
              <Form.Item
                label={i18nText("settings", "auto.name_alt")}
                name="name"
                rules={[{ required: true, message: i18nText("settings", "auto.enter_full_name") }]}
              >
                <Input />
              </Form.Item>
              <Form.Item
                label={i18nText("settings", "auto.email")}
                name="email"
                rules={[
                  { required: true, message: i18nText("settings", "auto.enter_email") },
                  { type: 'email', message: i18nText("settings", "auto.enter_valid_email_address") }
                ]}
              >
                <Input />
              </Form.Item>
              <Form.Item label={i18nText("settings", "auto.mobile_phone_number")} name="phone">
                <Input />
              </Form.Item>
              <Form.Item
                label={i18nText("settings", "auto.nickname")}
                name="nickname"
                rules={[{ required: true, message: i18nText("settings", "auto.enter_nickname") }]}
              >
                <Input />
              </Form.Item>
              <Form.Item
                label={i18nText("settings", "auto.initial_password")}
                name="password"
                initialValue={TEMP_PASSWORD}
                rules={[{ required: true, message: i18nText("settings", "auto.enter_initial_password") }]}
              >
                <Input.Password />
              </Form.Item>
            </div>
            <Form.Item label={i18nText("settings", "auto.personal_introduction")} name="introduction">
              <Input.TextArea rows={2} />
            </Form.Item>
            <div style={{ display: 'flex', gap: 24 }}>
              <Form.Item
                label={i18nText("settings", "auto.email_login")}
                name="email_login_enabled"
                valuePropName="checked"
                initialValue
              >
                <Switch />
              </Form.Item>
              <Form.Item
                label={i18nText("settings", "auto.mobile_login")}
                name="phone_login_enabled"
                valuePropName="checked"
                initialValue={false}
              >
                <Switch />
              </Form.Item>
            </div>
          </Form>
        </Modal>

        {/* Profile Edit Modal */}
        <Modal
          title={
            profileEditMember
              ? i18nText("settings", "auto.edit_user_profile", { value1: profileEditMember.name })
              : i18nText("settings", "auto.edit_profile")
          }
          open={Boolean(profileEditMember)}
          onCancel={() => {
            setProfileEditMember(null);
            profileForm.resetFields();
          }}
          onOk={() => profileForm.submit()}
          confirmLoading={updateMemberMutation.isPending}
          okText={i18nText("settings", "auto.save")}
          cancelText={i18nText("settings", "auto.cancel")}
          width={560}
          destroyOnHidden
        >
          <Form
            form={profileForm}
            layout="vertical"
            onFinish={handleProfileEditSubmit}
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
                label={i18nText("settings", "auto.name_alt")}
                name="name"
                rules={[{ required: true, message: i18nText("settings", "auto.enter_full_name") }]}
              >
                <Input />
              </Form.Item>
              <Form.Item
                label={i18nText("settings", "auto.nickname")}
                name="nickname"
                rules={[{ required: true, message: i18nText("settings", "auto.enter_nickname") }]}
              >
                <Input />
              </Form.Item>
              <Form.Item
                label={i18nText("settings", "auto.email")}
                name="email"
                rules={[
                  { required: true, message: i18nText("settings", "auto.enter_email") },
                  { type: 'email', message: i18nText("settings", "auto.enter_valid_email_address") }
                ]}
              >
                <Input />
              </Form.Item>
              <Form.Item label={i18nText("settings", "auto.mobile_phone_number")} name="phone">
                <Input />
              </Form.Item>
            </div>
            <Form.Item label={i18nText("settings", "auto.personal_introduction")} name="introduction">
              <Input.TextArea rows={3} />
            </Form.Item>
          </Form>
        </Modal>

        {/* Change Password Modal */}
        <Modal
          title={
            passwordEditMember
              ? i18nText("settings", "auto.change_login_password_for_user", { value1: passwordEditMember.name })
              : i18nText("settings", "auto.change_login_password")
          }
          open={Boolean(passwordEditMember)}
          onCancel={() => {
            setPasswordEditMember(null);
            passwordForm.resetFields();
          }}
          onOk={() => passwordForm.submit()}
          confirmLoading={changePasswordMutation.isPending}
          okText={i18nText("settings", "auto.confirm_change")}
          cancelText={i18nText("settings", "auto.cancel")}
          width={480}
          destroyOnHidden
        >
          <Form
            form={passwordForm}
            layout="vertical"
            onFinish={(values) => changePasswordMutation.mutate(values)}
            style={{ marginTop: 16 }}
          >
            <Form.Item
              label={i18nText("settings", "auto.current_password")}
              name="old_password"
              rules={[{ required: true, message: i18nText("settings", "auto.enter_current_password") }]}
            >
              <Input.Password />
            </Form.Item>
            <Form.Item
              label={i18nText("settings", "auto.new_password")}
              name="new_password"
              rules={[{ required: true, message: i18nText("settings", "auto.enter_new_password") }]}
            >
              <Input.Password />
            </Form.Item>
            <Form.Item
              label={i18nText("settings", "auto.confirm_new_password")}
              name="confirm_password"
              dependencies={['new_password']}
              rules={[
                { required: true, message: i18nText("settings", "auto.enter_new_password_again") },
                ({ getFieldValue }) => ({
                  validator(_, value) {
                    if (!value || value === getFieldValue('new_password')) {
                      return Promise.resolve();
                    }

                    return Promise.reject(new Error(i18nText("settings", "auto.passwords_do_not_match")));
                  }
                })
              ]}
            >
              <Input.Password />
            </Form.Item>
          </Form>
        </Modal>

        {/* Role Edit Modal */}
        <Modal
          title={
            roleEditMember ? i18nText("settings", "auto.edit_role", { value1: roleEditMember.name }) : i18nText("settings", "auto.edit_role_alt")
          }
          open={Boolean(roleEditMember)}
          onCancel={() => setRoleEditMember(null)}
          onOk={handleRoleEditOk}
          confirmLoading={replaceRolesMutation.isPending}
          okText={i18nText("settings", "auto.save")}
          cancelText={i18nText("settings", "auto.cancel")}
          width={480}
          destroyOnHidden
        >
          {roleEditMember ? (
            <div style={{ marginTop: 16 }}>
              <Typography.Text
                type="secondary"
                style={{ display: 'block', marginBottom: 12, fontSize: 13 }}
              >
                {i18nText("settings", "auto.for_users")}{roleEditMember.name}（{roleEditMember.account}{i18nText("settings", "auto.assign_roles")}</Typography.Text>
              <Select
                mode="multiple"
                style={{ width: '100%' }}
                value={editingRoleCodes}
                onChange={handleRoleSelectionChange}
                options={roleOptions}
                placeholder={i18nText("settings", "auto.select_role")}
              />
            </div>
          ) : null}
        </Modal>
      </div>
    </SettingsSectionSurface>
  );
}
