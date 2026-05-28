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
import { i18nText } from '../../../../shared/i18n/text';

const dataModelStatusHelp =
  i18nText("settings", "auto.k_841318af0b");

const defaultApiExposureStatusHelp =
  i18nText("settings", "auto.k_9ae58afd41");

type DefaultDataModelStatus =
  UpdateSettingsDataSourceDefaultsInput['default_data_model_status'];
type DefaultApiExposureStatus =
  UpdateSettingsDataSourceDefaultsInput['default_api_exposure_status'];

const dataModelStatusOptions = [
  { label: i18nText("settings", "auto.k_4bd191fb0d"), value: 'draft' },
  { label: i18nText("settings", "auto.k_65a525c108"), value: 'published' },
  { label: i18nText("settings", "auto.k_36bd66d67b"), value: 'disabled' },
  { label: i18nText("settings", "auto.k_0b1d58b4a0"), value: 'broken' }
] satisfies Array<{ label: string; value: DefaultDataModelStatus }>;

const apiExposureOptions = [
  { label: i18nText("settings", "auto.k_b2c29bc1c0"), value: 'draft' },
  { label: i18nText("settings", "auto.k_b75bad6b2d"), value: 'published_not_exposed' },
  { label: i18nText("settings", "auto.k_e64aeccb70"), value: 'api_exposed_no_permission' }
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
          {i18nText("settings", "auto.k_176a2eb4eb")}</Tag>
      );
    case 'draft':
      return (
        <Tag
          color="default"
          style={{ borderRadius: 6, margin: 0 }}
          icon={<EditOutlined />}
        >
          {i18nText("settings", "auto.k_0f436818c0")}</Tag>
      );
    case 'disabled':
      return (
        <Tag
          color="warning"
          style={{ borderRadius: 6, margin: 0 }}
          icon={<StopOutlined />}
        >
          {i18nText("settings", "auto.k_6c7dcbb73a")}</Tag>
      );
    case 'broken':
      return (
        <Tag
          color="error"
          style={{ borderRadius: 6, margin: 0 }}
          icon={<ExclamationCircleOutlined />}
        >
          {i18nText("settings", "auto.k_5caf279339")}</Tag>
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
          {i18nText("settings", "auto.k_25c17b12f1")}</Tag>
      );
    case 'api_exposed_no_permission':
      return (
        <Tag
          color="warning"
          style={{ borderRadius: 6, margin: 0 }}
          icon={<InfoCircleOutlined />}
        >
          {i18nText("settings", "auto.k_64d55ec6b6")}</Tag>
      );
    case 'published_not_exposed':
      return (
        <Tag
          color="blue"
          style={{ borderRadius: 6, margin: 0 }}
          icon={<FileTextOutlined />}
        >
          {i18nText("settings", "auto.k_365cbc93a4")}</Tag>
      );
    case 'draft':
      return (
        <Tag
          color="default"
          style={{ borderRadius: 6, margin: 0 }}
          icon={<EditOutlined />}
        >
          {i18nText("settings", "auto.k_0f436818c0")}</Tag>
      );
    case 'unsafe_external_source':
      return (
        <Tag
          color="error"
          style={{ borderRadius: 6, margin: 0 }}
          icon={<ExclamationCircleOutlined />}
        >
          {i18nText("settings", "auto.k_4495641c08")}</Tag>
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
      message.success(i18nText("settings", "auto.k_67a3d27986"));
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
      title: i18nText("settings", "auto.status"),
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
      title: i18nText("settings", "auto.k_e8c66a5fcd"),
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
      title: i18nText("settings", "auto.k_b2404bdd45"),
      key: 'fields',
      width: 96,
      render: (_, model) => (
        <Tag style={{ borderRadius: 6, margin: 0 }}>{model.fields.length}</Tag>
      )
    },
    {
      title: i18nText("settings", "auto.operation"),
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
              {i18nText("settings", "auto.edit")}</Button>
            {canDeleteModel ? (
              <Button
                danger
                type="link"
                size="small"
                icon={<DeleteOutlined aria-hidden="true" />}
                style={{ padding: 0 }}
                aria-label={i18nText("settings", "auto.k_24b7874b28", { value1: model.title })}
                disabled={!canManage}
                onClick={(event) => {
                  event.stopPropagation();
                  setDeleteTarget(model);
                }}
              >
                {i18nText("settings", "auto.delete")}</Button>
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
        <span className="data-model-panel__sr-only">{i18nText("settings", "auto.k_61abbaa1c0")}</span>
        <Button
          type="primary"
          icon={<PlusOutlined aria-hidden="true" />}
          disabled={!canManage || !selectedSource}
          onClick={() =>
            setDrawerState({ open: true, mode: 'create', model: null })
          }
        >
          {i18nText("settings", "auto.k_23e9246ed9")}</Button>

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
                  {i18nText("settings", "auto.k_f907023cd8")}</label>
                <Select
                  id="data-source-default-model-status"
                  value={selectedSource.default_data_model_status}
                  options={dataModelStatusOptions}
                  disabled={updateDefaultsMutation.isPending}
                  style={{ minWidth: 140 }}
                  placeholder={i18nText("settings", "auto.k_8da22d3410")}
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
                  label={i18nText("settings", "auto.k_f907023cd8")}
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
                  {i18nText("settings", "auto.k_1ae4b76727")}</label>
                <Select
                  id="data-source-default-api-status"
                  value={toDefaultApiExposureStatus(
                    selectedSource.default_api_exposure_status
                  )}
                  options={apiExposureOptions}
                  disabled={updateDefaultsMutation.isPending}
                  style={{ minWidth: 140 }}
                  placeholder={i18nText("settings", "auto.k_1ae4b76727")}
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
                  label={i18nText("settings", "auto.k_1ae4b76727")}
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
                      {i18nText("settings", "auto.k_870407f93e")}{model.external_table_id}
                    </Typography.Text>
                  ) : null}
                </Space>
                <Tag style={{ borderRadius: 6, margin: 0 }}>
                  {model.fields.length} {i18nText("settings", "auto.k_77a49f2c38")}</Tag>
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
                      {i18nText("settings", "auto.edit")}</Button>
                    {!isBuiltinMainSourceModel(model) ? (
                      <Button
                        danger
                        type="link"
                        size="small"
                        icon={<DeleteOutlined aria-hidden="true" />}
                        style={{ padding: 0 }}
                        aria-label={i18nText("settings", "auto.k_24b7874b28", { value1: model.title })}
                        onClick={(event) => {
                          event.stopPropagation();
                          setDeleteTarget(model);
                        }}
                      >
                        {i18nText("settings", "auto.delete")}</Button>
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
        title={i18nText("settings", "auto.k_2acd0aba22")}
        open={Boolean(deleteTarget)}
        okText={i18nText("settings", "auto.k_b56d9ac6c5")}
        okType="danger"
        cancelText={i18nText("settings", "auto.cancel")}
        okButtonProps={{ 'aria-label': i18nText("settings", "auto.k_b56d9ac6c5") }}
        onCancel={() => setDeleteTarget(null)}
        onOk={() => {
          if (deleteTarget) {
            onDeleteModel(deleteTarget);
          }
          setDeleteTarget(null);
        }}
      >
        {deleteTarget
          ? i18nText("settings", "auto.k_2f316ca1c1", { value1: deleteTarget.title, value2: deleteTarget.code })
          : null}
      </Modal>
    </Flex>
  );
}
