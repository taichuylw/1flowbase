import { DownloadOutlined, ReloadOutlined } from '@ant-design/icons';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import {
  Alert,
  Button,
  Space,
  Table,
  Tag,
  Typography,
  message,
  type TableProps
} from 'antd';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';

import {
  applicationsQueryKey,
  importAgentFlowTemplate,
  previewAgentFlowTemplate,
  type AgentFlowTemplatePackage,
  type AgentFlowTemplatePreview
} from '../../applications/api/applications';
import { ApplicationTemplateImportModal } from '../../applications/components/ApplicationTemplateImportModal';
import { formatDateTime } from '../../../shared/i18n/format';
import { useAuthStore } from '../../../state/auth-store';
import {
  downloadOfficialAgentFlowTemplate,
  fetchOfficialAgentFlowTemplateCatalog,
  officialAgentFlowTemplateCatalogQueryKey,
  officialAgentFlowTemplateCatalogStaleTimeMs,
  type OfficialAgentFlowTemplateCatalogEntry
} from '../api/templates';
import './templates-page.css';

interface PreparedOfficialTemplate {
  template: AgentFlowTemplatePackage;
  preview: AgentFlowTemplatePreview;
}

function renderTemplateIdentity(entry: OfficialAgentFlowTemplateCatalogEntry) {
  return (
    <Space align="center" size={12} className="templates-page__identity">
      <span
        className="templates-page__icon"
        style={{
          backgroundColor: entry.application.icon_background ?? undefined
        }}
        aria-hidden="true"
      >
        {entry.application.name.slice(0, 1).toUpperCase()}
      </span>
      <Space
        direction="vertical"
        size={0}
        className="templates-page__identity-text"
      >
        <Typography.Text strong>{entry.application.name}</Typography.Text>
        <Typography.Text type="secondary" copyable>
          {entry.workflow_id}
        </Typography.Text>
      </Space>
    </Space>
  );
}

export function TemplatesPage() {
  const { t } = useTranslation('templates');
  const csrfToken = useAuthStore((state) => state.csrfToken);
  const queryClient = useQueryClient();
  const [messageApi, messageContextHolder] = message.useMessage();
  const [importTemplate, setImportTemplate] =
    useState<AgentFlowTemplatePackage | null>(null);
  const [importPreview, setImportPreview] =
    useState<AgentFlowTemplatePreview | null>(null);
  const [importName, setImportName] = useState('');
  const [preparingWorkflowId, setPreparingWorkflowId] = useState<string | null>(
    null
  );

  const catalogQuery = useQuery({
    queryKey: officialAgentFlowTemplateCatalogQueryKey,
    queryFn: () => fetchOfficialAgentFlowTemplateCatalog(),
    staleTime: officialAgentFlowTemplateCatalogStaleTimeMs,
    gcTime: officialAgentFlowTemplateCatalogStaleTimeMs
  });

  async function prepareImportTemplate(
    entry: OfficialAgentFlowTemplateCatalogEntry
  ) {
    setPreparingWorkflowId(entry.workflow_id);

    try {
      const template = await downloadOfficialAgentFlowTemplate(
        entry.workflow_id
      );
      const preview = await previewAgentFlowTemplate(template);
      const preparedTemplate: PreparedOfficialTemplate = { template, preview };

      setImportTemplate(preparedTemplate.template);
      setImportPreview(preparedTemplate.preview);
      setImportName(preparedTemplate.preview.application.name);
    } catch {
      setImportTemplate(null);
      setImportPreview(null);
      messageApi.error(t('auto.template_prepare_failed'));
    } finally {
      setPreparingWorkflowId((currentWorkflowId) =>
        currentWorkflowId === entry.workflow_id ? null : currentWorkflowId
      );
    }
  }

  const importTemplateMutation = useMutation({
    mutationFn: () => {
      if (!importTemplate) {
        throw new Error('missing official agent flow template');
      }

      return importAgentFlowTemplate(
        {
          template: importTemplate,
          name: importName.trim(),
          description:
            importPreview?.application.description ??
            importTemplate.application.description
        },
        csrfToken ?? ''
      );
    },
    onSuccess: async (imported) => {
      await queryClient.invalidateQueries({ queryKey: applicationsQueryKey });
      messageApi.success(t('auto.template_imported'));
      setImportTemplate(null);
      setImportPreview(null);
      window.location.assign(
        `/applications/${imported.application.id}/orchestration`
      );
    },
    onError: () => {
      messageApi.error(t('auto.template_import_failed'));
    }
  });

  const columns: TableProps<OfficialAgentFlowTemplateCatalogEntry>['columns'] =
    [
      {
        title: t('auto.template_info'),
        key: 'template',
        width: 300,
        render: (_, entry) => renderTemplateIdentity(entry)
      },
      {
        title: t('auto.description'),
        dataIndex: ['application', 'description'],
        key: 'description',
        width: 320,
        render: (description: string) => (
          <Typography.Text type={description ? undefined : 'secondary'}>
            {description || t('auto.description_empty')}
          </Typography.Text>
        )
      },
      {
        title: t('auto.updated_at'),
        dataIndex: 'updated_at',
        key: 'updated_at',
        width: 180,
        render: (updatedAt: string) =>
          formatDateTime(updatedAt, {
            hour12: false
          })
      },
      {
        title: t('auto.template_hash'),
        dataIndex: 'template_sha256',
        key: 'template_sha256',
        width: 220,
        render: (value: string) => (
          <Typography.Text code copyable className="templates-page__hash">
            {value}
          </Typography.Text>
        )
      },
      {
        title: t('auto.actions'),
        key: 'actions',
        width: 140,
        fixed: 'right',
        render: (_, entry) => (
          <Button
            type="primary"
            icon={<DownloadOutlined />}
            loading={preparingWorkflowId === entry.workflow_id}
            aria-label={`${t('auto.import_template')}-${entry.application.name}`}
            onClick={() => void prepareImportTemplate(entry)}
          >
            {t('auto.import_template')}
          </Button>
        )
      }
    ];

  const catalog = catalogQuery.data;

  return (
    <div className="templates-page">
      {messageContextHolder}
      <div className="templates-page__header">
        <div className="templates-page__title">
          <Typography.Title level={2}>{t('auto.templates')}</Typography.Title>
          <Typography.Paragraph type="secondary">
            {t('auto.official_agent_flow_templates_description')}
          </Typography.Paragraph>
        </div>
        <Space className="templates-page__toolbar" size={12} wrap>
          {catalog ? (
            <Tag className="templates-page__source">
              {catalog.source.source_label}
            </Tag>
          ) : null}
          <Button
            icon={<ReloadOutlined />}
            onClick={() => void catalogQuery.refetch()}
            loading={catalogQuery.isFetching}
          >
            {t('auto.refresh_catalog')}
          </Button>
        </Space>
      </div>

      {catalogQuery.isError ? (
        <Alert
          type="error"
          showIcon
          message={t('auto.catalog_load_failed')}
          action={
            <Button size="small" onClick={() => void catalogQuery.refetch()}>
              {t('auto.retry')}
            </Button>
          }
        />
      ) : null}

      <div className="templates-page__table">
        <Table
          rowKey="workflow_id"
          columns={columns}
          dataSource={catalog?.entries ?? []}
          loading={catalogQuery.isLoading}
          pagination={false}
          scroll={{ x: 1160 }}
          locale={{ emptyText: t('auto.empty_catalog') }}
          footer={() =>
            catalog ? (
              <Typography.Text type="secondary">
                {t('auto.catalog_page_summary', {
                  value1: catalog.page.page,
                  value2: catalog.page.page_size
                })}
              </Typography.Text>
            ) : null
          }
        />
      </div>

      <ApplicationTemplateImportModal
        open={Boolean(importPreview)}
        preview={importPreview}
        name={importName}
        importing={importTemplateMutation.isPending}
        onNameChange={setImportName}
        onCancel={() => {
          setImportTemplate(null);
          setImportPreview(null);
        }}
        onImport={() => importTemplateMutation.mutate()}
      />
    </div>
  );
}
