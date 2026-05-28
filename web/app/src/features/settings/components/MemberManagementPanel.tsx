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
        title: i18nText("settings", "auto.key_jlkhgdokde"),
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
        title: i18nText("settings", "auto.key_galoonmipc"),
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
            {member.status === 'active' ? i18nText("settings", "auto.enabled") : i18nText("settings", "auto.key_njijoffbii")}
          </Tag>
        )
      },
      {
        title: i18nText("settings", "auto.key_glcggjfoen"),
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
                          {i18nText("settings", "auto.key_njijoffbii")}</Button>
                      ) : (
                        <Popconfirm
                          title={i18nText("settings", "auto.key_pmnjdcgeae")}
                          description={i18nText("settings", "auto.key_cankkcoici", { value1: member.name })}
                          onConfirm={() => disableMutation.mutate(member.id)}
                          okText={i18nText("settings", "auto.key_pdklnijebj")}
                          cancelText={i18nText("settings", "auto.cancel")}
                          okButtonProps={{ danger: true }}
                        >
                          <Button
                            size="small"
                            danger
                            icon={<StopOutlined />}
                            loading={disableMutation.isPending}
                          >
                            {i18nText("settings", "auto.key_njijoffbii")}</Button>
                        </Popconfirm>
                      )
                    ) : null}
                    {isRootMember ? (
                      <Button size="small" icon={<KeyOutlined />} disabled>
                        {i18nText("settings", "auto.key_hoeccbegnn")}</Button>
                    ) : (
                      <Popconfirm
                        title={i18nText("settings", "auto.key_hoeccbegnn")}
                        description={i18nText("settings", "auto.key_bdcakjhijn", { value1: member.name })}
                        onConfirm={() =>
                          resetPasswordMutation.mutate(member.id)
                        }
                        okText={i18nText("settings", "auto.key_dakgahjmod")}
                        cancelText={i18nText("settings", "auto.cancel")}
                      >
                        <Button
                          size="small"
                          icon={<KeyOutlined />}
                          loading={resetPasswordMutation.isPending}
                        >
                          {i18nText("settings", "auto.key_hoeccbegnn")}</Button>
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
    <SettingsSectionSurface title={i18nText("settings", "auto.user_management")} hideHeader heightMode="fill">
      <div style={{ display: 'flex', flexDirection: 'column', gap: 20 }}>
        {canManageMembers ? (
          <div style={{ display: 'flex', justifyContent: 'flex-end' }}>
            <Button
              type="primary"
              icon={<UserAddOutlined />}
              onClick={() => setCreateModalOpen(true)}
            >
              {i18nText("settings", "auto.key_daebgeahof")}</Button>
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
          {i18nText("settings", "auto.key_jidbeoljph")}</Typography.Text>

        {/* Create Member Modal */}
        <Modal
          title={i18nText("settings", "auto.key_daebgeahof")}
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
                label={i18nText("settings", "auto.key_jabdiejbhj")}
                name="account"
                rules={[{ required: true, message: i18nText("settings", "auto.key_npmmieegil") }]}
              >
                <Input />
              </Form.Item>
              <Form.Item
                label={i18nText("settings", "auto.key_loemcgbglb")}
                name="name"
                rules={[{ required: true, message: i18nText("settings", "auto.key_ghfldoockp") }]}
              >
                <Input />
              </Form.Item>
              <Form.Item
                label={i18nText("settings", "auto.key_jongchlmpg")}
                name="email"
                rules={[
                  { required: true, message: i18nText("settings", "auto.key_pcnmcenoli") },
                  { type: 'email', message: i18nText("settings", "auto.key_lmbihdhdgc") }
                ]}
              >
                <Input />
              </Form.Item>
              <Form.Item label={i18nText("settings", "auto.key_fkjmmfoijb")} name="phone">
                <Input />
              </Form.Item>
              <Form.Item
                label={i18nText("settings", "auto.key_cfbceonhem")}
                name="nickname"
                rules={[{ required: true, message: i18nText("settings", "auto.key_fnloglahol") }]}
              >
                <Input />
              </Form.Item>
              <Form.Item
                label={i18nText("settings", "auto.key_jgbpmllbfl")}
                name="password"
                initialValue={TEMP_PASSWORD}
                rules={[{ required: true, message: i18nText("settings", "auto.key_kollelnjfp") }]}
              >
                <Input.Password />
              </Form.Item>
            </div>
            <Form.Item label={i18nText("settings", "auto.key_opphggfffe")} name="introduction">
              <Input.TextArea rows={2} />
            </Form.Item>
            <div style={{ display: 'flex', gap: 24 }}>
              <Form.Item
                label={i18nText("settings", "auto.key_dfobimmipn")}
                name="email_login_enabled"
                valuePropName="checked"
                initialValue
              >
                <Switch />
              </Form.Item>
              <Form.Item
                label={i18nText("settings", "auto.key_capokofoij")}
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
            roleEditMember ? i18nText("settings", "auto.key_jhmbjkddop", { value1: roleEditMember.name }) : i18nText("settings", "auto.key_ljnmnickhl")
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
                {i18nText("settings", "auto.key_acdifoeljh")}{roleEditMember.name}（{roleEditMember.account}{i18nText("settings", "auto.key_eddfgndopi")}</Typography.Text>
              <Select
                mode="multiple"
                style={{ width: '100%' }}
                value={editingRoleCodes}
                onChange={setEditingRoleCodes}
                options={roleOptions}
                placeholder={i18nText("settings", "auto.key_cbfajdjplb")}
              />
            </div>
          ) : null}
        </Modal>
      </div>
    </SettingsSectionSurface>
  );
}
