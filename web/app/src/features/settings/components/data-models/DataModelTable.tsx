import { useState } from 'react';

import {
  Button,
  Flex,
  Form,
  Grid,
  Modal,
  Select,
  Space,
  Table,
  Tag,
  Typography,
  message
} from 'antd';
import type { ColumnsType } from 'antd/es/table';
import {
  PlusOutlined,
  EditOutlined,
  DeleteOutlined,
  CheckCircleOutlined,
  ExclamationCircleOutlined,
  InfoCircleOutlined,
  StopOutlined,
  FileTextOutlined
} from '@ant-design/icons';
import { useMutation, useQueryClient } from '@tanstack/react-query';

import { useAuthStore } from '../../../../state/auth-store';
import {
  updateSettingsDataSourceDefaults,
  settingsDataSourcesQueryKey,
  type UpdateSettingsDataSourceDefaultsInput,
  type CreateSettingsDataModelInput,
  type SettingsDataModel,
  type SettingsDataSourceInstance,
  type UpdateSettingsDataModelInput
} from '../../api/data-models';
import { DataModelFormDrawer } from './DataModelFormDrawer';
import { DataModelHelpTooltip } from './DataModelHelpTooltip';

const dataModelStatusHelp =
  'draft: 草稿，默认新建为未发布状态；published: 已发布，允许进入运行可用性和 API 暴露判断；disabled: 已停用，不进入运行面；broken: 当前定义、运行依赖或外部资源异常，需要修复后再发布。';

const defaultApiExposureStatusHelp =
  'draft: API 暴露草稿；published_not_exposed: 默认不生成 API 访问面；api_exposed_no_permission: 已请求生成 API 访问面，但默认不授予访问权限。';

type DefaultDataModelStatus =
  UpdateSettingsDataSourceDefaultsInput['default_data_model_status'];
type DefaultApiExposureStatus =
  UpdateSettingsDataSourceDefaultsInput['default_api_exposure_status'];

const dataModelStatusOptions = [
  { label: 'Draft (草稿)', value: 'draft' },
  { label: 'Published (已发布)', value: 'published' },
  { label: 'Disabled (已停用)', value: 'disabled' },
  { label: 'Broken (异常)', value: 'broken' }
] satisfies Array<{ label: string; value: DefaultDataModelStatus }>;

const apiExposureOptions = [
  { label: 'Draft (API 草稿)', value: 'draft' },
  { label: 'Published (无公开 API)', value: 'published_not_exposed' },
  { label: 'API Exposed (未授权公开)', value: 'api_exposed_no_permission' }
] satisfies Array<{ label: string; value: DefaultApiExposureStatus }>;

const builtinMainSourceModelCodes = new Set(['attachments', 'users', 'roles']);

function toDefaultApiExposureStatus(
  status: SettingsDataSourceInstance['default_api_exposure_status']
): DefaultApiExposureStatus {
  return status === 'api_exposed_ready' ? 'published_not_exposed' : status;
}

function isBuiltinMainSourceModel(model: SettingsDataModel) {
  return (
    model.source_kind === 'main_source' &&
    builtinMainSourceModelCodes.has(model.code)
  );
}

function getStatusTag(status: string) {
  switch (status) {
    case 'published':
      return (
        <Tag
          color="success"
          style={{ borderRadius: 6, margin: 0 }}
          icon={<CheckCircleOutlined />}
        >
          已发布
        </Tag>
      );
    case 'draft':
      return (
        <Tag
          color="default"
          style={{ borderRadius: 6, margin: 0 }}
          icon={<EditOutlined />}
        >
          草稿
        </Tag>
      );
    case 'disabled':
      return (
        <Tag
          color="warning"
          style={{ borderRadius: 6, margin: 0 }}
          icon={<StopOutlined />}
        >
          已停用
        </Tag>
      );
    case 'broken':
      return (
        <Tag
          color="error"
          style={{ borderRadius: 6, margin: 0 }}
          icon={<ExclamationCircleOutlined />}
        >
          异常
        </Tag>
      );
    default:
      return <Tag style={{ borderRadius: 6, margin: 0 }}>{status}</Tag>;
  }
}

function getApiExposureTag(status: string) {
  switch (status) {
    case 'api_exposed_ready':
      return (
        <Tag
          color="success"
          style={{ borderRadius: 6, margin: 0 }}
          icon={<CheckCircleOutlined />}
        >
          已公开
        </Tag>
      );
    case 'api_exposed_no_permission':
      return (
        <Tag
          color="warning"
          style={{ borderRadius: 6, margin: 0 }}
          icon={<InfoCircleOutlined />}
        >
          已公开 (未授权)
        </Tag>
      );
    case 'published_not_exposed':
      return (
        <Tag
          color="blue"
          style={{ borderRadius: 6, margin: 0 }}
          icon={<FileTextOutlined />}
        >
          未公开
        </Tag>
      );
    case 'draft':
      return (
        <Tag
          color="default"
          style={{ borderRadius: 6, margin: 0 }}
          icon={<EditOutlined />}
        >
          草稿
        </Tag>
      );
    case 'unsafe_external_source':
      return (
        <Tag
          color="error"
          style={{ borderRadius: 6, margin: 0 }}
          icon={<ExclamationCircleOutlined />}
        >
          不安全外部源
        </Tag>
      );
    default:
      return <Tag style={{ borderRadius: 6, margin: 0 }}>API {status}</Tag>;
  }
}

export function DataModelTable({
  models,
  selectedSource,
  selectedModelId,
  loading,
  saving,
  canManage,
  onSelectModel,
  onEditModel,
  onDeleteModel,
  onCreateModel,
  onUpdateModel
}: {
  models: SettingsDataModel[];
  selectedSource: SettingsDataSourceInstance | null;
  selectedModelId: string | null;
  loading: boolean;
  saving: boolean;
  canManage: boolean;
  onSelectModel: (model: SettingsDataModel) => void;
  onEditModel: (model: SettingsDataModel) => void;
  onDeleteModel: (model: SettingsDataModel) => void;
  onCreateModel: (input: CreateSettingsDataModelInput) => void;
  onUpdateModel: (
    model: SettingsDataModel,
    input: UpdateSettingsDataModelInput
  ) => void;
}) {
  const screens = Grid.useBreakpoint();
  const useMobileList = Boolean(screens.xs && !screens.md);

  const csrfToken = useAuthStore((state) => state.csrfToken);
  const queryClient = useQueryClient();

  const updateDefaultsMutation = useMutation({
    mutationFn: ({
      source,
      patch
    }: {
      source: SettingsDataSourceInstance;
      patch: Pick<
        UpdateSettingsDataSourceDefaultsInput,
        'default_data_model_status' | 'default_api_exposure_status'
      >;
    }) => {
      if (!csrfToken) {
        throw new Error('missing csrf token');
      }
      return updateSettingsDataSourceDefaults(source.id, patch, csrfToken);
    },
    onSuccess: async () => {
      message.success('默认状态已保存');
      await queryClient.invalidateQueries({
        queryKey: settingsDataSourcesQueryKey
      });
    }
  });

  const [drawerState, setDrawerState] = useState<
    | { open: false; mode: 'create'; model: null }
    | { open: true; mode: 'create'; model: null }
  >({ open: false, mode: 'create', model: null });
  const [deleteTarget, setDeleteTarget] = useState<SettingsDataModel | null>(
    null
  );

  const columns: ColumnsType<SettingsDataModel> = [
    {
      title: 'Data Model',
      dataIndex: 'title',
      key: 'title',
      width: 220,
      render: (_, model) => (
        <Space direction="vertical" size={2}>
          <Typography.Text strong className="data-model-panel__model-title">
            {model.title}
          </Typography.Text>
          <Typography.Text type="secondary" style={{ fontSize: 12 }}>
            <code className="data-model-panel__code-badge">{model.code}</code>
          </Typography.Text>
        </Space>
      )
    },
    {
      title: '状态',
      dataIndex: 'status',
      key: 'status',
      width: 140,
      render: (value: string) => getStatusTag(value)
    },
    {
      title: 'API',
      dataIndex: 'api_exposure_status',
      key: 'api_exposure_status',
      width: 200,
      render: (value: string) => getApiExposureTag(value)
    },
    {
      title: '表 ID',
      dataIndex: 'external_table_id',
      key: 'external_table_id',
      width: 180,
      render: (_, model) =>
        model.source_kind === 'external_source' ? (
          <Typography.Text type="secondary">
            {model.external_table_id ?? '-'}
          </Typography.Text>
        ) : (
          <Typography.Text type="secondary">-</Typography.Text>
        )
    },
    {
      title: '字段数',
      key: 'fields',
      width: 96,
      render: (_, model) => (
        <Tag style={{ borderRadius: 6, margin: 0 }}>{model.fields.length}</Tag>
      )
    },
    {
      title: '操作',
      key: 'actions',
      width: 160,
      render: (_, model) => {
        const canDeleteModel = !isBuiltinMainSourceModel(model);

        return (
          <Space size={12}>
            <Button
              type="link"
              size="small"
              icon={<EditOutlined aria-hidden="true" />}
              style={{ padding: 0 }}
              disabled={!canManage}
              onClick={(event) => {
                event.stopPropagation();
                onEditModel(model);
              }}
            >
              编辑
            </Button>
            {canDeleteModel ? (
              <Button
                danger
                type="link"
                size="small"
                icon={<DeleteOutlined aria-hidden="true" />}
                style={{ padding: 0 }}
                aria-label={`删除数据表 ${model.title}`}
                disabled={!canManage}
                onClick={(event) => {
                  event.stopPropagation();
                  setDeleteTarget(model);
                }}
              >
                删除
              </Button>
            ) : null}
          </Space>
        );
      }
    }
  ];

  return (
    <Flex vertical gap={16} className="data-model-panel__table-container">
      <Flex
        align="center"
        justify="flex-start"
        className="data-model-panel__table-head"
        wrap="wrap"
        gap={16}
      >
        <span className="data-model-panel__sr-only">数据表</span>
        <Button
          type="primary"
          icon={<PlusOutlined aria-hidden="true" />}
          disabled={!canManage || !selectedSource}
          onClick={() =>
            setDrawerState({ open: true, mode: 'create', model: null })
          }
        >
          新建数据表
        </Button>

        {selectedSource && (
          <Form
            layout="inline"
            style={{
              margin: 0,
              display: 'inline-flex',
              alignItems: 'center',
              gap: 12
            }}
          >
            <Form.Item style={{ margin: 0 }}>
              <Flex align="center" gap={6}>
                <label
                  htmlFor="data-source-default-model-status"
                  className="data-model-panel__sr-only"
                >
                  默认 Data Model 状态
                </label>
                <Select
                  id="data-source-default-model-status"
                  value={selectedSource.default_data_model_status}
                  options={dataModelStatusOptions}
                  disabled={updateDefaultsMutation.isPending}
                  style={{ minWidth: 140 }}
                  placeholder="默认建模状态"
                  onChange={(value) =>
                    updateDefaultsMutation.mutate({
                      source: selectedSource,
                      patch: {
                        default_data_model_status: value,
                        default_api_exposure_status: toDefaultApiExposureStatus(
                          selectedSource.default_api_exposure_status
                        )
                      }
                    })
                  }
                />
                <DataModelHelpTooltip
                  decorative
                  label="默认 Data Model 状态"
                  title={dataModelStatusHelp}
                />
              </Flex>
            </Form.Item>

            <Form.Item style={{ margin: 0 }}>
              <Flex align="center" gap={6}>
                <label
                  htmlFor="data-source-default-api-status"
                  className="data-model-panel__sr-only"
                >
                  默认 API 暴露状态
                </label>
                <Select
                  id="data-source-default-api-status"
                  value={toDefaultApiExposureStatus(
                    selectedSource.default_api_exposure_status
                  )}
                  options={apiExposureOptions}
                  disabled={updateDefaultsMutation.isPending}
                  style={{ minWidth: 140 }}
                  placeholder="默认 API 暴露状态"
                  onChange={(value: DefaultApiExposureStatus) =>
                    updateDefaultsMutation.mutate({
                      source: selectedSource,
                      patch: {
                        default_data_model_status:
                          selectedSource.default_data_model_status,
                        default_api_exposure_status: value
                      }
                    })
                  }
                />
                <DataModelHelpTooltip
                  decorative
                  label="默认 API 暴露状态"
                  title={defaultApiExposureStatusHelp}
                />
              </Flex>
            </Form.Item>
          </Form>
        )}
      </Flex>
      {!useMobileList ? (
        <Table
          rowKey="id"
          size="middle"
          loading={loading}
          columns={columns}
          dataSource={models}
          pagination={false}
          scroll={{ x: 840 }}
          className="data-model-panel__models-table"
          rowSelection={{
            type: 'radio',
            selectedRowKeys: selectedModelId ? [selectedModelId] : [],
            onChange: ([modelId]) => {
              const model = models.find((item) => item.id === modelId);
              if (model) {
                onSelectModel(model);
              }
            }
          }}
          onRow={(model) => ({
            onClick: () => onSelectModel(model),
            style: { cursor: 'pointer' }
          })}
        />
      ) : null}
      {useMobileList ? (
        <div className="data-model-panel__mobile-list">
          {models.map((model) => (
            <div
              key={model.id}
              role="button"
              tabIndex={0}
              className="data-model-panel__mobile-item data-model-panel__mobile-item--clickable"
              onClick={() => onSelectModel(model)}
              onKeyDown={(event) => {
                if (event.key === 'Enter' || event.key === ' ') {
                  onSelectModel(model);
                }
              }}
            >
              <Flex
                align="center"
                justify="space-between"
                style={{ width: '100%' }}
              >
                <Space direction="vertical" size={2}>
                  <Typography.Text strong>{model.title}</Typography.Text>
                  <Typography.Text type="secondary" style={{ fontSize: 12 }}>
                    {model.code}
                  </Typography.Text>
                  {model.source_kind === 'external_source' &&
                  model.external_table_id ? (
                    <Typography.Text type="secondary" style={{ fontSize: 11 }}>
                      表 ID: {model.external_table_id}
                    </Typography.Text>
                  ) : null}
                </Space>
                <Tag style={{ borderRadius: 6, margin: 0 }}>
                  {model.fields.length} 字段
                </Tag>
              </Flex>
              <Flex gap={6} style={{ marginTop: 12 }} wrap="wrap">
                {getStatusTag(model.status)}
                {getApiExposureTag(model.api_exposure_status)}
              </Flex>
              <span
                className="data-model-panel__mobile-actions"
                style={{
                  marginTop: 12,
                  display: 'flex',
                  justifyContent: 'flex-end',
                  width: '100%'
                }}
              >
                {canManage ? (
                  <Space size={16}>
                    <Button
                      type="link"
                      size="small"
                      icon={<EditOutlined aria-hidden="true" />}
                      style={{ padding: 0 }}
                      onClick={(event) => {
                        event.stopPropagation();
                        onEditModel(model);
                      }}
                    >
                      编辑
                    </Button>
                    {!isBuiltinMainSourceModel(model) ? (
                      <Button
                        danger
                        type="link"
                        size="small"
                        icon={<DeleteOutlined aria-hidden="true" />}
                        style={{ padding: 0 }}
                        aria-label={`删除数据表 ${model.title}`}
                        onClick={(event) => {
                          event.stopPropagation();
                          setDeleteTarget(model);
                        }}
                      >
                        删除
                      </Button>
                    ) : null}
                  </Space>
                ) : null}
              </span>
            </div>
          ))}
        </div>
      ) : null}
      <DataModelFormDrawer
        open={drawerState.open}
        mode={drawerState.mode}
        model={drawerState.model}
        source={selectedSource}
        saving={saving}
        onClose={() =>
          setDrawerState({ open: false, mode: 'create', model: null })
        }
        onCreate={onCreateModel}
        onUpdate={onUpdateModel}
      />
      <Modal
        title="确认删除数据表"
        open={Boolean(deleteTarget)}
        okText="确认"
        okType="danger"
        cancelText="取消"
        okButtonProps={{ 'aria-label': '确认' }}
        onCancel={() => setDeleteTarget(null)}
        onOk={() => {
          if (deleteTarget) {
            onDeleteModel(deleteTarget);
          }
          setDeleteTarget(null);
        }}
      >
        {deleteTarget
          ? `确定删除数据表 "${deleteTarget.title}" (${deleteTarget.code}) 吗？此操作会同步删除运行表和字段配置。`
          : null}
      </Modal>
    </Flex>
  );
}
