import { useEffect, useMemo, useState } from 'react';

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import {
  Button,
  Checkbox,
  Form,
  Input,
  Modal,
  Popconfirm,
  Space,
  Tabs,
  Tag,
  Tree,
  Typography,
  message
} from 'antd';
import type { TreeDataNode } from 'antd';
import {
  SearchOutlined,
  PlusOutlined,
  EditOutlined,
  DeleteOutlined,
  TeamOutlined,
  SafetyCertificateOutlined
} from '@ant-design/icons';

import { useAuthStore } from '../../../state/auth-store';
import {
  fetchSettingsPermissions,
  settingsPermissionsQueryKey,
  type SettingsPermission
} from '../api/permissions';
import {
  createSettingsRole,
  deleteSettingsRole,
  fetchSettingsRolePermissions,
  fetchSettingsRoles,
  replaceSettingsRolePermissions,
  settingsRolePermissionsQueryKey,
  settingsRolesQueryKey,
  updateSettingsRole,
  type SettingsRole
} from '../api/roles';
import { SettingsSectionSurface } from './SettingsSectionSurface';
import { i18nText } from '../../../shared/i18n/text';

// 分类映射，根据要求
const RESOURCE_MAP: Record<
  string,
  { tab: string; label: string; order: number }
> = {
  role_permission: {
    tab: i18nText("settings", "auto.k_5095009346"),
    label: i18nText("settings", "auto.k_03607df4ad"),
    order: 1
  },
  user: { tab: i18nText("settings", "auto.k_5095009346"), label: i18nText("settings", "auto.k_0a61e64e97"), order: 2 },
  team: { tab: i18nText("settings", "auto.k_5095009346"), label: i18nText("settings", "auto.k_94104f1af2"), order: 3 },
  external_data_source: {
    tab: i18nText("settings", "auto.k_5095009346"),
    label: i18nText("settings", "auto.k_d30800b12a"),
    order: 4
  },

  application: { tab: i18nText("settings", "auto.k_04ca1cb5c7"), label: i18nText("settings", "auto.k_aeb8ae55e4"), order: 1 },
  embedded_app: { tab: i18nText("settings", "auto.k_04ca1cb5c7"), label: i18nText("settings", "auto.k_0a584580f5"), order: 2 },
  plugin_config: {
    tab: i18nText("settings", "auto.k_04ca1cb5c7"),
    label: i18nText("settings", "auto.k_8f0e60d30f"),
    order: 3
  },
  state_model: { tab: i18nText("settings", "auto.k_04ca1cb5c7"), label: i18nText("settings", "auto.k_f4ac0dd2ca"), order: 4 },

  route_page: { tab: i18nText("settings", "auto.k_590675cfea"), label: i18nText("settings", "auto.k_013f4dd181"), order: 1 },

  flow: { tab: i18nText("settings", "auto.k_4275796187"), label: i18nText("settings", "auto.k_4461d0d885"), order: 1 },
  publish_endpoint: {
    tab: i18nText("settings", "auto.k_4275796187"),
    label: i18nText("settings", "auto.k_da667c6ad2"),
    order: 2
  }
};

const TAB_ORDER = [i18nText("settings", "auto.k_5095009346"), i18nText("settings", "auto.k_04ca1cb5c7"), i18nText("settings", "auto.k_590675cfea"), i18nText("settings", "auto.k_4275796187"), i18nText("settings", "auto.k_1a26edf94a")];

export function RolePermissionPanel({
  canManageRoles
}: {
  canManageRoles: boolean;
}) {
  const csrfToken = useAuthStore((state) => state.csrfToken);
  const queryClient = useQueryClient();
  const [messageApi, contextHolder] = message.useMessage();

  const [searchQuery, setSearchQuery] = useState('');
  const [selectedRoleCode, setSelectedRoleCode] = useState<string | null>(null);

  const [isCreateModalOpen, setIsCreateModalOpen] = useState(false);
  const [editingRole, setEditingRole] = useState<SettingsRole | null>(null);

  const [createForm] = Form.useForm();
  const [editForm] = Form.useForm();

  // Queries
  const rolesQuery = useQuery({
    queryKey: settingsRolesQueryKey,
    queryFn: fetchSettingsRoles
  });

  const permissionsQuery = useQuery({
    queryKey: settingsPermissionsQueryKey,
    queryFn: fetchSettingsPermissions
  });

  const rolePermissionsQuery = useQuery({
    queryKey: settingsRolePermissionsQueryKey(selectedRoleCode ?? 'none'),
    queryFn: () => fetchSettingsRolePermissions(selectedRoleCode ?? ''),
    enabled: Boolean(selectedRoleCode)
  });

  // Local state for fast UI updates
  const [localCheckedCodes, setLocalCheckedCodes] = useState<string[]>([]);

  useEffect(() => {
    setLocalCheckedCodes(rolePermissionsQuery.data?.permission_codes ?? []);
  }, [rolePermissionsQuery.data?.permission_codes]);

  useEffect(() => {
    if (!selectedRoleCode && rolesQuery.data?.length) {
      setSelectedRoleCode(rolesQuery.data[0].code);
    }
  }, [rolesQuery.data, selectedRoleCode]);

  const filteredRoles = useMemo(() => {
    if (!rolesQuery.data) return [];
    return rolesQuery.data.filter(
      (r) =>
        r.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
        r.code.toLowerCase().includes(searchQuery.toLowerCase())
    );
  }, [rolesQuery.data, searchQuery]);

  const selectedRole = useMemo(() => {
    return rolesQuery.data?.find((r) => r.code === selectedRoleCode) || null;
  }, [rolesQuery.data, selectedRoleCode]);

  const tabsData = useMemo(() => {
    const allPerms = permissionsQuery.data ?? [];
    const tabsMap = new Map<string, Map<string, SettingsPermission[]>>();

    allPerms.forEach((p) => {
      const resKey = p.resource || 'other';
      const mapInfo = RESOURCE_MAP[resKey];
      const tabName = mapInfo ? mapInfo.tab : i18nText("settings", "auto.k_1a26edf94a");

      if (!tabsMap.has(tabName)) {
        tabsMap.set(tabName, new Map());
      }
      const resMap = tabsMap.get(tabName)!;
      if (!resMap.has(resKey)) {
        resMap.set(resKey, []);
      }
      resMap.get(resKey)!.push(p);
    });

    return TAB_ORDER.filter((t) => tabsMap.has(t)).map((tabName) => {
      const resMap = tabsMap.get(tabName)!;
      const resources = Array.from(resMap.entries())
        .map(([resKey, perms]) => {
          const mapInfo = RESOURCE_MAP[resKey];
          return {
            key: resKey,
            label: mapInfo ? mapInfo.label : resKey,
            order: mapInfo ? mapInfo.order : 99,
            permissions: perms
          };
        })
        .sort((a, b) => a.order - b.order);

      const treeData: TreeDataNode[] = resources.map((res) => ({
        title: res.label,
        key: `resource:${res.key}`,
        children: res.permissions.map((p) => ({
          title: <span title={p.code}>{p.name}</span>,
          key: p.code
        }))
      }));

      const tabLeafKeys = resources.flatMap((res) =>
        res.permissions.map((p) => p.code)
      );

      return {
        key: tabName,
        label: tabName,
        treeData,
        tabLeafKeys
      };
    });
  }, [permissionsQuery.data]);

  const invalidateRoles = async () => {
    await queryClient.invalidateQueries({ queryKey: settingsRolesQueryKey });
    if (selectedRoleCode) {
      await queryClient.invalidateQueries({
        queryKey: settingsRolePermissionsQueryKey(selectedRoleCode)
      });
    }
  };

  const replacePermissionsMutation = useMutation({
    mutationFn: async (permissionCodes: string[]) => {
      if (!csrfToken || !selectedRoleCode) throw new Error('missing selection');
      return replaceSettingsRolePermissions(
        selectedRoleCode,
        { permission_codes: permissionCodes },
        csrfToken
      );
    },
    onSuccess: async () => {
      messageApi.success(i18nText("settings", "auto.k_f4bee77a73"));
      await invalidateRoles();
    },
    onError: () => {
      messageApi.error(i18nText("settings", "auto.k_0f18e624e8"));
      // revert local state on error
      setLocalCheckedCodes(rolePermissionsQuery.data?.permission_codes ?? []);
    }
  });

  const createMutation = useMutation({
    mutationFn: async (values: Record<string, unknown>) => {
      if (!csrfToken) throw new Error('missing csrf token');
      return createSettingsRole(
        {
          code: String(values.code ?? ''),
          name: String(values.name ?? ''),
          introduction: String(values.introduction ?? ''),
          auto_grant_new_permissions: Boolean(
            values.auto_grant_new_permissions
          ),
          is_default_member_role: Boolean(values.is_default_member_role)
        },
        csrfToken
      );
    },
    onSuccess: async () => {
      messageApi.success(i18nText("settings", "auto.k_90d651cbba"));
      createForm.resetFields();
      setIsCreateModalOpen(false);
      await invalidateRoles();
    },
    onError: () => messageApi.error(i18nText("settings", "auto.k_ed49361253"))
  });

  const updateMutation = useMutation({
    mutationFn: async (values: Record<string, unknown>) => {
      if (!csrfToken || !editingRole)
        throw new Error('missing csrf token or editing role');
      return updateSettingsRole(
        editingRole.code,
        {
          name: String(values.name ?? ''),
          introduction: String(values.introduction ?? ''),
          auto_grant_new_permissions: Boolean(
            values.auto_grant_new_permissions
          ),
          is_default_member_role: Boolean(values.is_default_member_role)
        },
        csrfToken
      );
    },
    onSuccess: async () => {
      messageApi.success(i18nText("settings", "auto.k_e6b74b0057"));
      setEditingRole(null);
      await invalidateRoles();
    },
    onError: () => messageApi.error(i18nText("settings", "auto.k_0d0f9319f5"))
  });

  const deleteMutation = useMutation({
    mutationFn: async (roleCode: string) => {
      if (!csrfToken) throw new Error('missing csrf token');
      return deleteSettingsRole(roleCode, csrfToken);
    },
    onSuccess: async (_, variables) => {
      messageApi.success(i18nText("settings", "auto.k_980b39d722"));
      if (selectedRoleCode === variables) {
        setSelectedRoleCode(rolesQuery.data?.[0]?.code ?? null);
      }
      await invalidateRoles();
    },
    onError: () => messageApi.error(i18nText("settings", "auto.k_af3639bb51"))
  });

  const handleEditClick = (role: SettingsRole) => {
    setEditingRole(role);
    editForm.setFieldsValue({
      name: role.name,
      introduction: role.introduction ?? '',
      auto_grant_new_permissions: role.auto_grant_new_permissions,
      is_default_member_role: role.is_default_member_role
    });
  };

  return (
    <SettingsSectionSurface title={i18nText("settings", "auto.permission_management")} hideHeader heightMode="fill">
      <div
        style={{
          display: 'flex',
          flexDirection: 'column',
          gap: '24px',
          width: '100%',
          minHeight: 'calc(100vh - 120px)'
        }}
      >
        {contextHolder}

        <div
          style={{
            flex: 1,
            minHeight: 0,
            display: 'flex',
            border: '1px solid #f0f0f0',
            borderRadius: '8px',
            background: '#fff',
            overflow: 'hidden'
          }}
        >
          {/* 左侧：角色列表 */}
          <div
            style={{
              width: 280,
              borderRight: '1px solid #f0f0f0',
              display: 'flex',
              flexDirection: 'column',
              background: '#fafafa',
              flexShrink: 0
            }}
          >
            <div
              style={{
                padding: 16,
                borderBottom: '1px solid #f0f0f0',
                background: '#fff'
              }}
            >
              <Space
                direction="vertical"
                size="middle"
                style={{ width: '100%' }}
              >
                {canManageRoles && (
                  <Button
                    type="primary"
                    icon={<PlusOutlined />}
                    block
                    onClick={() => setIsCreateModalOpen(true)}
                  >
                    {i18nText("settings", "auto.k_7a0524c3a7")}</Button>
                )}
                <Input
                  placeholder={i18nText("settings", "auto.k_93e4a5c9c6")}
                  prefix={<SearchOutlined style={{ color: '#bfbfbf' }} />}
                  value={searchQuery}
                  onChange={(e) => setSearchQuery(e.target.value)}
                  allowClear
                />
              </Space>
            </div>

            <div style={{ flex: 1, overflowY: 'auto' }}>
              {rolesQuery.isLoading ? (
                <div
                  style={{ padding: 16, textAlign: 'center', color: '#bfbfbf' }}
                >
                  {i18nText("settings", "auto.k_514c33af5c")}</div>
              ) : filteredRoles.length === 0 ? (
                <div
                  style={{ padding: 32, textAlign: 'center', color: '#bfbfbf' }}
                >
                  {i18nText("settings", "auto.k_6103376362")}</div>
              ) : (
                <div style={{ padding: '8px 0' }}>
                  {filteredRoles.map((role) => {
                    const isActive = selectedRoleCode === role.code;
                    return (
                      <div
                        key={role.code}
                        onClick={() => setSelectedRoleCode(role.code)}
                        style={{
                          padding: '12px 16px',
                          cursor: 'pointer',
                          background: isActive ? '#e6f4ff' : 'transparent',
                          borderRight: isActive
                            ? '3px solid #1677ff'
                            : '3px solid transparent',
                          transition: 'all 0.2s'
                        }}
                      >
                        <div
                          style={{
                            display: 'flex',
                            justifyContent: 'space-between',
                            alignItems: 'center',
                            marginBottom: 4
                          }}
                        >
                          <Typography.Text
                            strong={isActive}
                            style={{ color: isActive ? '#1677ff' : 'inherit' }}
                          >
                            {role.name}
                          </Typography.Text>
                          {role.is_builtin && (
                            <Tag
                              color="gold"
                              style={{ margin: 0, border: 'none' }}
                            >
                              {i18nText("settings", "auto.k_09ceea7644")}</Tag>
                          )}
                        </div>
                        <div style={{ fontSize: '12px', color: '#8c8c8c' }}>
                          {role.code}
                        </div>
                      </div>
                    );
                  })}
                </div>
              )}
            </div>
          </div>

          {/* 右侧：权限配置详情 */}
          <div
            style={{
              flex: 1,
              display: 'flex',
              flexDirection: 'column',
              overflow: 'hidden'
            }}
          >
            {selectedRole ? (
              <>
                {/* 头部信息 */}
                <div
                  style={{
                    padding: '20px 24px',
                    borderBottom: '1px solid #f0f0f0',
                    display: 'flex',
                    justifyContent: 'space-between',
                    alignItems: 'flex-start',
                    flexShrink: 0
                  }}
                >
                  <div>
                    <Typography.Title
                      level={4}
                      style={{ margin: 0, marginBottom: 8 }}
                    >
                      <SafetyCertificateOutlined
                        style={{ marginRight: 8, color: '#1677ff' }}
                      />
                      {selectedRole.name}
                    </Typography.Title>
                    <Space size="large" style={{ color: '#595959' }}>
                      <span>{i18nText("settings", "auto.k_bd19099baf")}{selectedRole.code}</span>
                      <span>{i18nText("settings", "auto.k_81de160a82")}{selectedRole.scope_kind}</span>
                      {selectedRole.introduction && (
                        <span>{i18nText("settings", "auto.k_a2c8f89312")}{selectedRole.introduction}</span>
                      )}
                      {selectedRole.auto_grant_new_permissions ? (
                        <Tag color="blue">{i18nText("settings", "auto.k_f6e1a8129f")}</Tag>
                      ) : null}
                      {selectedRole.is_default_member_role ? (
                        <Tag color="green">{i18nText("settings", "auto.k_a8e1023e12")}</Tag>
                      ) : null}
                    </Space>
                  </div>
                  {canManageRoles && selectedRole.is_editable && (
                    <Space>
                      <Button
                        icon={<EditOutlined />}
                        onClick={() => handleEditClick(selectedRole)}
                      >
                        {i18nText("settings", "auto.k_52d4d230e3")}</Button>
                      <Popconfirm
                        title={i18nText("settings", "auto.k_84dd4898fe")}
                        onConfirm={() =>
                          deleteMutation.mutate(selectedRole.code)
                        }
                        okText={i18nText("settings", "auto.delete")}
                        okButtonProps={{ danger: true }}
                      >
                        <Button danger icon={<DeleteOutlined />}>
                          {i18nText("settings", "auto.k_89dab40d48")}</Button>
                      </Popconfirm>
                    </Space>
                  )}
                </div>

                {/* 权限多 Tab 配置 */}
                <div
                  style={{ flex: 1, overflowY: 'auto', padding: '16px 24px' }}
                >
                  {permissionsQuery.isLoading ||
                  rolePermissionsQuery.isLoading ? (
                    <div style={{ padding: 32, textAlign: 'center' }}>
                      {i18nText("settings", "auto.k_cdca1a02f2")}</div>
                  ) : (
                    <Tabs
                      defaultActiveKey={TAB_ORDER[0]}
                      items={tabsData.map((tab) => ({
                        key: tab.key,
                        label: tab.label,
                        children: (
                          <div style={{ paddingBottom: 32 }}>
                            <Tree
                              checkable
                              disabled={
                                !canManageRoles || !selectedRole.is_editable
                              }
                              checkedKeys={localCheckedCodes.filter((code) =>
                                tab.tabLeafKeys.includes(code)
                              )}
                              onCheck={(checkedKeysValue) => {
                                const keys = Array.isArray(checkedKeysValue)
                                  ? checkedKeysValue
                                  : checkedKeysValue.checked;
                                const newlyCheckedLeaves = keys
                                  .map(String)
                                  .filter((k) => !k.startsWith('resource:'));

                                const otherCheckedCodes =
                                  localCheckedCodes.filter(
                                    (c) => !tab.tabLeafKeys.includes(c)
                                  );
                                const newCodes = [
                                  ...otherCheckedCodes,
                                  ...newlyCheckedLeaves
                                ];

                                setLocalCheckedCodes(newCodes);
                                replacePermissionsMutation.mutate(newCodes);
                              }}
                              treeData={tab.treeData}
                              defaultExpandAll={false}
                            />
                          </div>
                        )
                      }))}
                    />
                  )}
                </div>
              </>
            ) : (
              <div
                style={{
                  flex: 1,
                  display: 'flex',
                  justifyContent: 'center',
                  alignItems: 'center',
                  color: '#bfbfbf'
                }}
              >
                <Space direction="vertical" align="center">
                  <TeamOutlined style={{ fontSize: 48 }} />
                  <Typography.Text type="secondary">
                    {i18nText("settings", "auto.k_8516a37f19")}</Typography.Text>
                </Space>
              </div>
            )}
          </div>
        </div>

        <Modal
          title={i18nText("settings", "auto.k_7a0524c3a7")}
          open={isCreateModalOpen}
          onCancel={() => {
            setIsCreateModalOpen(false);
            createForm.resetFields();
          }}
          onOk={() => createForm.submit()}
          confirmLoading={createMutation.isPending}
          destroyOnHidden
        >
          <Form
            form={createForm}
            layout="vertical"
            onFinish={(values) => createMutation.mutate(values)}
            initialValues={{
              auto_grant_new_permissions: false,
              is_default_member_role: false
            }}
            style={{ marginTop: 24 }}
          >
            <Form.Item
              label={i18nText("settings", "auto.k_3aa1f085b2")}
              name="name"
              rules={[{ required: true, message: i18nText("settings", "auto.k_b7c17b9e6e") }]}
            >
              <Input placeholder={i18nText("settings", "auto.k_4d6f8223c8")} />
            </Form.Item>
            <Form.Item
              label={i18nText("settings", "auto.k_c12ace673d")}
              name="code"
              rules={[{ required: true, message: i18nText("settings", "auto.k_67819cee9b") }]}
              extra={i18nText("settings", "auto.k_0eaf475ae9")}
            >
              <Input placeholder={i18nText("settings", "auto.k_09149af273")} />
            </Form.Item>
            <Form.Item label={i18nText("settings", "auto.k_9ae5aa988d")} name="introduction">
              <Input.TextArea
                placeholder={i18nText("settings", "auto.k_8f86210c27")}
                rows={3}
              />
            </Form.Item>
            <Form.Item
              name="auto_grant_new_permissions"
              valuePropName="checked"
              extra={i18nText("settings", "auto.k_a305ece229")}
            >
              <Checkbox>{i18nText("settings", "auto.k_17cb542374")}</Checkbox>
            </Form.Item>
            <Form.Item
              name="is_default_member_role"
              valuePropName="checked"
              extra={i18nText("settings", "auto.k_7fead96aef")}
            >
              <Checkbox>{i18nText("settings", "auto.k_a8e1023e12")}</Checkbox>
            </Form.Item>
          </Form>
        </Modal>

        <Modal
          title={i18nText("settings", "auto.k_b9dcd82a7b")}
          open={!!editingRole}
          onCancel={() => setEditingRole(null)}
          onOk={() => editForm.submit()}
          confirmLoading={updateMutation.isPending}
          destroyOnHidden
        >
          <Form
            form={editForm}
            layout="vertical"
            onFinish={(values) => updateMutation.mutate(values)}
            style={{ marginTop: 24 }}
          >
            <Form.Item
              label={i18nText("settings", "auto.k_3aa1f085b2")}
              name="name"
              rules={[{ required: true, message: i18nText("settings", "auto.k_b7c17b9e6e") }]}
            >
              <Input />
            </Form.Item>
            <Form.Item label={i18nText("settings", "auto.k_9ae5aa988d")} name="introduction">
              <Input.TextArea rows={3} />
            </Form.Item>
            <Form.Item
              name="auto_grant_new_permissions"
              valuePropName="checked"
              extra={i18nText("settings", "auto.k_a305ece229")}
            >
              <Checkbox>{i18nText("settings", "auto.k_17cb542374")}</Checkbox>
            </Form.Item>
            <Form.Item
              name="is_default_member_role"
              valuePropName="checked"
              extra={i18nText("settings", "auto.k_7fead96aef")}
            >
              <Checkbox>{i18nText("settings", "auto.k_a8e1023e12")}</Checkbox>
            </Form.Item>
          </Form>
        </Modal>
      </div>
    </SettingsSectionSurface>
  );
}
