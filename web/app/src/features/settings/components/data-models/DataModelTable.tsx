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
  i18nText("settings", "auto.key_iebdbikpal");

const defaultApiExposureStatusHelp =
  i18nText("settings", "auto.key_jkofikpneb");

type DefaultDataModelStatus =
  UpdateSettingsDataSourceDefaultsInput['default_data_model_status'];
type DefaultApiExposureStatus =
  UpdateSettingsDataSourceDefaultsInput['default_api_exposure_status'];

const dataModelStatusOptions = [
  { label: i18nText("settings", "auto.key_elnbjbplan"), value: 'draft' },
  { label: i18nText("settings", "auto.key_gfkfcfmbai"), value: 'published' },
  { label: i18nText("settings", "auto.key_dglnggnghl"), value: 'disabled' },
  { label: i18nText("settings", "auto.key_albnfileka"), value: 'broken' }
] satisfies Array<{ label: string; value: DefaultDataModelStatus }>;

const apiExposureOptions = [
  { label: i18nText("settings", "auto.key_lcmcjlmbma"), value: 'draft' },
  { label: i18nText("settings", "auto.key_lhflknglcn"), value: 'published_not_exposed' },
  { label: i18nText("settings", "auto.key_ogekommlha"), value: 'api_exposed_no_permission' }
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
          {i18nText("settings", "auto.key_bhgkcoleol")}</Tag>
      );
    case 'draft':
      return (
        <Tag
          color="default"
          style={{ borderRadius: 6, margin: 0 }}
          icon={<EditOutlined />}
        >
          {i18nText("settings", "auto.key_apedgibima")}</Tag>
      );
    case 'disabled':
      return (
        <Tag
          color="warning"
          style={{ borderRadius: 6, margin: 0 }}
          icon={<StopOutlined />}
        >
          {i18nText("settings", "auto.key_gmhnmllhdk")}</Tag>
      );
    case 'broken':
      return (
        <Tag
          color="error"
          style={{ borderRadius: 6, margin: 0 }}
          icon={<ExclamationCircleOutlined />}
        >
          {i18nText("settings", "auto.key_fmkpchjddj")}</Tag>
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
          {i18nText("settings", "auto.key_cfmbhlbcpb")}</Tag>
      );
    case 'api_exposed_no_permission':
      return (
        <Tag
          color="warning"
          style={{ borderRadius: 6, margin: 0 }}
          icon={<InfoCircleOutlined />}
        >
          {i18nText("settings", "auto.key_genffomglg")}</Tag>
      );
    case 'published_not_exposed':
      return (
        <Tag
          color="blue"
          style={{ borderRadius: 6, margin: 0 }}
          icon={<FileTextOutlined />}
        >
          {i18nText("settings", "auto.key_dgfmlmjdke")}</Tag>
      );
    case 'draft':
      return (
        <Tag
          color="default"
          style={{ borderRadius: 6, margin: 0 }}
          icon={<EditOutlined />}
        >
          {i18nText("settings", "auto.key_apedgibima")}</Tag>
      );
    case 'unsafe_external_source':
      return (
        <Tag
          color="error"
          style={{ borderRadius: 6, margin: 0 }}
          icon={<ExclamationCircleOutlined />}
        >
          {i18nText("settings", "auto.key_eejfgebmai")}</Tag>
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
      message.success(i18nText("settings", "auto.key_ghkdnchjig"));
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
      title: i18nText("settings", "auto.key_oimggkfpmn"),
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
      title: i18nText("settings", "auto.key_lceaelnnef"),
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
                aria-label={i18nText("settings", "auto.key_celhihelci", { value1: model.title })}
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
        <span className="data-model-panel__sr-only">{i18nText("settings", "auto.key_gbkllkkbma")}</span>
        <Button
          type="primary"
          icon={<PlusOutlined aria-hidden="true" />}
          disabled={!canManage || !selectedSource}
          onClick={() =>
            setDrawerState({ open: true, mode: 'create', model: null })
          }
        >
          {i18nText("settings", "auto.key_cdojcegonj")}</Button>

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
                  {i18nText("settings", "auto.key_pjahacdmni")}</label>
                <Select
                  id="data-source-default-model-status"
                  value={selectedSource.default_data_model_status}
                  options={dataModelStatusOptions}
                  disabled={updateDefaultsMutation.isPending}
                  style={{ minWidth: 140 }}
                  placeholder={i18nText("settings", "auto.key_inkccndeba")}
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
                  label={i18nText("settings", "auto.key_pjahacdmni")}
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
                  {i18nText("settings", "auto.key_bkoelhghch")}</label>
                <Select
                  id="data-source-default-api-status"
                  value={toDefaultApiExposureStatus(
                    selectedSource.default_api_exposure_status
                  )}
                  options={apiExposureOptions}
                  disabled={updateDefaultsMutation.isPending}
                  style={{ minWidth: 140 }}
                  placeholder={i18nText("settings", "auto.key_bkoelhghch")}
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
                  label={i18nText("settings", "auto.key_bkoelhghch")}
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
                      {i18nText("settings", "auto.key_ihaeahpjdo")}{model.external_table_id}
                    </Typography.Text>
                  ) : null}
                </Space>
                <Tag style={{ borderRadius: 6, margin: 0 }}>
                  {model.fields.length} {i18nText("settings", "auto.key_hhkejpcmdi")}</Tag>
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
                        aria-label={i18nText("settings", "auto.key_celhihelci", { value1: model.title })}
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
        title={i18nText("settings", "auto.key_ckmnaklkcc")}
        open={Boolean(deleteTarget)}
        okText={i18nText("settings", "auto.key_lfgnjkmgmf")}
        okType="danger"
        cancelText={i18nText("settings", "auto.cancel")}
        okButtonProps={{ 'aria-label': i18nText("settings", "auto.key_lfgnjkmgmf") }}
        onCancel={() => setDeleteTarget(null)}
        onOk={() => {
          if (deleteTarget) {
            onDeleteModel(deleteTarget);
          }
          setDeleteTarget(null);
        }}
      >
        {deleteTarget
          ? i18nText("settings", "auto.key_cpdbgmkbmb", { value1: deleteTarget.title, value2: deleteTarget.code })
          : null}
      </Modal>
    </Flex>
  );
}
