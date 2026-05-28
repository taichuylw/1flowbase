import { useState } from 'react';

import {
  Button,
  Flex,
  Select,
  Table,
  Tabs,
  Tag,
  Typography,
  Space
} from 'antd';
import type { ColumnsType } from 'antd/es/table';
import {
  LockOutlined,
  TagOutlined,
  CodeOutlined,
  DatabaseOutlined,
  CloudServerOutlined,
  InteractionOutlined,
  TableOutlined,
  DeploymentUnitOutlined,
  PlusOutlined,
  EditOutlined
} from '@ant-design/icons';

import type {
  CreateSettingsDataModelFieldInput,
  SettingsDataModel,
  SettingsDataModelAdvisorFinding,
  SettingsDataModelField,
  SettingsDataModelScopeGrant,
  SettingsRuntimeRecordPreview,
  UpdateSettingsDataModelApiExposureInput,
  UpdateSettingsDataModelFieldInput,
  UpdateSettingsDataModelInput
} from '../../api/data-models';
import { DataModelAdvisorTab } from './DataModelAdvisorTab';
import { DataModelApiTab } from './DataModelApiTab';
import { DataModelFieldDrawer } from './DataModelFieldDrawer';
import { DataModelFormDrawer } from './DataModelFormDrawer';
import {
  DataModelHelpTooltip,
  dataModelStatusHelp
} from './DataModelHelpTooltip';
import { DataModelPermissionsTab } from './DataModelPermissionsTab';
import { DataModelRecordPreview } from './DataModelRecordPreview';
import { i18nText } from '../../../../shared/i18n/text';

const dataModelStatusOptions = ['draft', 'published', 'disabled', 'broken'].map(
  (value) => ({ label: value, value })
);

function getFieldKindTag(kind: string) {
  let color = 'default';

  const k = kind.toLowerCase();
  if (k.includes('string') || k.includes('text') || k.includes('varchar')) {
    color = 'blue';
  } else if (
    k.includes('int') ||
    k.includes('float') ||
    k.includes('number') ||
    k.includes('decimal')
  ) {
    color = 'cyan';
  } else if (k.includes('bool')) {
    color = 'purple';
  } else if (k.includes('date') || k.includes('time')) {
    color = 'orange';
  } else if (k.includes('relation') || k.includes('ref')) {
    color = 'magenta';
  } else if (k.includes('json')) {
    color = 'geekblue';
  }

  return (
    <Tag
      color={color}
      style={{ borderRadius: 6, margin: 0, textTransform: 'capitalize' }}
    >
      {kind}
    </Tag>
  );
}

export function DataModelDetail({
  model,
  allModels,
  canManage,
  grants,
  grantsLoading,
  grantsSaving,
  advisorFindings,
  advisorLoading,
  recordPreview,
  recordPreviewLoading,
  modelSaving,
  fieldSaving,
  onUpdateModelStatus,
  onUpdateModel,
  onCreateField,
  onUpdateField,
  onDeleteField,
  onUpdateApiExposure,
  onSaveGrant
}: {
  model: SettingsDataModel;
  allModels: SettingsDataModel[];
  canManage: boolean;
  grants: SettingsDataModelScopeGrant[];
  grantsLoading: boolean;
  grantsSaving: boolean;
  advisorFindings: SettingsDataModelAdvisorFinding[];
  advisorLoading: boolean;
  recordPreview: SettingsRuntimeRecordPreview | undefined;
  recordPreviewLoading: boolean;
  modelSaving: boolean;
  fieldSaving: boolean;
  onUpdateModelStatus: (status: SettingsDataModel['status']) => void;
  onUpdateModel: (input: UpdateSettingsDataModelInput) => void;
  onCreateField: (input: CreateSettingsDataModelFieldInput) => void;
  onUpdateField: (
    field: SettingsDataModelField,
    input: UpdateSettingsDataModelFieldInput
  ) => void;
  onDeleteField: (field: SettingsDataModelField) => void;
  onUpdateApiExposure: (input: UpdateSettingsDataModelApiExposureInput) => void;
  onSaveGrant: Parameters<typeof DataModelPermissionsTab>[0]['onSave'];
}) {
  const [modelDrawerOpen, setModelDrawerOpen] = useState(false);
  const [fieldDrawerState, setFieldDrawerState] = useState<
    | { open: false; mode: 'create'; field: null }
    | { open: true; mode: 'create'; field: null }
    | { open: true; mode: 'edit'; field: SettingsDataModelField }
  >({ open: false, mode: 'create', field: null });

  const fieldColumns: ColumnsType<SettingsDataModelField> = [
    {
      title: i18nText("settings", "auto.key_egmnolmiag"),
      dataIndex: 'title',
      key: 'title',
      render: (value: string, field) => {
        const disabled =
          field.is_system === true || field.is_writable === false;
        return (
          <Space size={8}>
            <button
              type="button"
              className={`data-model-panel__field-title-btn ${disabled ? 'disabled' : ''}`}
              disabled={disabled}
              onClick={() =>
                setFieldDrawerState({ open: true, mode: 'edit', field })
              }
            >
              <Typography.Text
                strong={!disabled}
                style={{
                  color: disabled
                    ? 'var(--ant-color-text-secondary)'
                    : 'var(--brand-primary)'
                }}
              >
                {value}
              </Typography.Text>
            </button>
            {disabled && (
              <Tag
                style={{ borderRadius: 6, margin: 0, fontSize: 10 }}
                icon={<LockOutlined style={{ fontSize: 10 }} />}
              >
                {i18nText("settings", "auto.key_ppmbnagfpl")}</Tag>
            )}
          </Space>
        );
      }
    },
    {
      title: 'Code',
      dataIndex: 'code',
      key: 'code',
      render: (value: string) => (
        <code className="data-model-panel__code-badge">{value}</code>
      )
    },
    {
      title: i18nText("settings", "auto.kind"),
      dataIndex: 'field_kind',
      key: 'field_kind',
      render: (value: string) => getFieldKindTag(value)
    },
    {
      title: i18nText("settings", "auto.key_dcjefndodg"),
      dataIndex: 'is_required',
      key: 'is_required',
      width: 80,
      render: (value: boolean) =>
        value ? (
          <Tag color="error" style={{ borderRadius: 4, margin: 0 }}>
            {i18nText("settings", "auto.key_dcjefndodg")}</Tag>
        ) : (
          <span style={{ color: 'var(--text-tertiary)' }}>-</span>
        )
    },
    {
      title: i18nText("settings", "auto.key_oiafabekkf"),
      dataIndex: 'is_unique',
      key: 'is_unique',
      width: 80,
      render: (value: boolean) =>
        value ? (
          <Tag color="warning" style={{ borderRadius: 4, margin: 0 }}>
            {i18nText("settings", "auto.key_oiafabekkf")}</Tag>
        ) : (
          <span style={{ color: 'var(--text-tertiary)' }}>-</span>
        )
    },
    {
      title: i18nText("settings", "auto.operation"),
      key: 'actions',
      width: 100,
      render: (_, field) => (
        <Button
          type="link"
          size="small"
          icon={<EditOutlined aria-hidden="true" />}
          style={{ padding: 0 }}
          disabled={
            !canManage ||
            field.is_system === true ||
            field.is_writable === false
          }
          onClick={() =>
            setFieldDrawerState({ open: true, mode: 'edit', field })
          }
        >
          {i18nText("settings", "auto.edit")}</Button>
      )
    }
  ];

  const relationFields = model.fields.filter(
    (field) => field.relation_target_model_id
  );
  const summaryItems = [
    {
      key: 'title',
      label: i18nText("settings", "auto.key_heinhnmhod"),
      value: model.title,
      strong: true,
      icon: <TagOutlined />
    },
    { key: 'code', label: 'Code', value: model.code, icon: <CodeOutlined /> },
    {
      key: 'source',
      label: i18nText("settings", "auto.key_mgdphjogdg"),
      value: model.source_kind === 'main_source' ? i18nText("settings", "auto.built_in_data_source") : i18nText("settings", "auto.external_data_source"),
      icon:
        model.source_kind === 'main_source' ? (
          <DatabaseOutlined />
        ) : (
          <CloudServerOutlined />
        )
    },
    {
      key: 'runtime',
      label: 'Runtime',
      value: model.runtime_availability,
      icon: <InteractionOutlined />
    },
    ...(model.source_kind === 'external_source'
      ? [
          {
            key: 'external_table_id',
            label: i18nText("settings", "auto.key_oimggkfpmn"),
            value: model.external_table_id ?? '-',
            icon: <TableOutlined />
          }
        ]
      : []),
    {
      key: 'table',
      label: i18nText("settings", "auto.key_blhcjdjhnl"),
      value: model.physical_table_name,
      icon: <DeploymentUnitOutlined />
    }
  ];

  return (
    <section className="data-model-panel__detail" aria-label={i18nText("settings", "auto.key_aofjndgane")}>
      <div
        className="data-model-panel__meta-grid"
        data-testid="data-model-detail-summary"
      >
        {summaryItems.map((item) => (
          <div
            key={item.key}
            className="data-model-panel__meta-card"
            data-testid="data-model-summary-item"
          >
            <div className="data-model-panel__meta-card-header">
              {item.icon}
              <span className="data-model-panel__meta-card-label">
                {item.label}：
              </span>
            </div>
            <Typography.Text
              strong
              className="data-model-panel__meta-card-value"
            >
              {item.value}
            </Typography.Text>
          </div>
        ))}
      </div>

      <div
        className="data-model-panel__detail-actions"
        data-testid="data-model-detail-actions"
      >
        <div className="data-model-panel__status-control">
          <div
            className="data-model-panel__status-label"
            data-testid="data-model-status-label"
          >
            <label htmlFor="data-model-status-select">{i18nText("settings", "auto.key_lpakmijhid")}</label>
            <DataModelHelpTooltip
              label={i18nText("settings", "auto.key_nplkhfpcjg")}
              title={dataModelStatusHelp}
            />
          </div>
          <Select
            id="data-model-status-select"
            value={model.status}
            options={dataModelStatusOptions}
            disabled={!canManage || modelSaving}
            virtual={false}
            onChange={(value) => onUpdateModelStatus(value)}
          />
        </div>
        <Button disabled={!canManage} onClick={() => setModelDrawerOpen(true)}>
          {i18nText("settings", "auto.edit")}</Button>
      </div>

      <Tabs
        items={[
          {
            key: 'fields',
            label: i18nText("settings", "auto.key_hhkejpcmdi"),
            children: (
              <Flex vertical gap={12}>
                <Flex justify="flex-end">
                  <Button
                    type="primary"
                    icon={<PlusOutlined aria-hidden="true" />}
                    disabled={!canManage}
                    onClick={() =>
                      setFieldDrawerState({
                        open: true,
                        mode: 'create',
                        field: null
                      })
                    }
                  >
                    {i18nText("settings", "auto.key_bgihmialje")}</Button>
                </Flex>
                <Table
                  rowKey="id"
                  size="small"
                  columns={fieldColumns}
                  dataSource={model.fields}
                  pagination={false}
                />
                <DataModelFieldDrawer
                  open={fieldDrawerState.open}
                  mode={fieldDrawerState.mode}
                  field={fieldDrawerState.field}
                  isExternalModel={model.source_kind === 'external_source'}
                  modelOptions={allModels}
                  saving={fieldSaving}
                  canManage={canManage}
                  onClose={() =>
                    setFieldDrawerState({
                      open: false,
                      mode: 'create',
                      field: null
                    })
                  }
                  onCreate={onCreateField}
                  onUpdate={onUpdateField}
                  onDelete={onDeleteField}
                />
              </Flex>
            )
          },
          {
            key: 'relations',
            label: i18nText("settings", "auto.key_nkpepcnmao"),
            children: (
              <Table
                rowKey="id"
                size="small"
                columns={[
                  { title: i18nText("settings", "auto.key_hhkejpcmdi"), dataIndex: 'title', key: 'title' },
                  {
                    title: i18nText("settings", "auto.key_edimdomibg"),
                    dataIndex: 'relation_target_model_id',
                    key: 'relation_target_model_id'
                  }
                ]}
                dataSource={relationFields}
                pagination={false}
              />
            )
          },
          {
            key: 'permissions',
            label: i18nText("settings", "auto.key_fgabgfkgnh"),
            children: (
              <DataModelPermissionsTab
                grants={grants}
                loading={grantsLoading}
                saving={grantsSaving}
                onSave={onSaveGrant}
              />
            )
          },
          {
            key: 'api',
            label: 'API',
            children: (
              <DataModelApiTab
                model={model}
                canManage={canManage}
                saving={modelSaving}
                onUpdateApiExposure={onUpdateApiExposure}
              />
            )
          },
          {
            key: 'records',
            label: i18nText("settings", "auto.key_nbfhpohbbd"),
            children: (
              <DataModelRecordPreview
                preview={recordPreview}
                loading={recordPreviewLoading}
              />
            )
          },
          {
            key: 'advisor',
            label: 'Advisor',
            children: (
              <DataModelAdvisorTab
                findings={advisorFindings}
                loading={advisorLoading}
              />
            )
          }
        ]}
      />
      <DataModelFormDrawer
        open={modelDrawerOpen}
        mode="edit"
        model={model}
        source={null}
        saving={modelSaving}
        onClose={() => setModelDrawerOpen(false)}
        onCreate={() => undefined}
        onUpdate={(_model, input) => {
          onUpdateModel(input);
          setModelDrawerOpen(false);
        }}
      />
    </section>
  );
}
