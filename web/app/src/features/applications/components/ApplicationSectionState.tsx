import { Descriptions, Result, Space, Tag, Typography } from 'antd';

import type { ApplicationDetail } from '../api/applications';
import type { ApplicationSectionKey } from '../lib/application-sections';
import { i18nText } from '../../../shared/i18n/text';

function renderStatusTag(status: string) {
  return <Tag color={status === 'planned' ? 'gold' : 'default'}>{status}</Tag>;
}

export function ApplicationSectionState({
  application,
  sectionKey
}: {
  application: ApplicationDetail;
  sectionKey: ApplicationSectionKey;
}) {
  if (sectionKey === 'orchestration') {
    return (
      <Space direction="vertical" size="middle">
        <Typography.Title level={4}>{i18nText("applications", "auto.k_63881557e3")}</Typography.Title>
        <Typography.Paragraph>
          {i18nText("applications", "auto.k_61d16fc410")}</Typography.Paragraph>
        <Descriptions
          bordered
          column={1}
          items={[
            {
              key: 'status',
              label: i18nText("applications", "auto.k_959c8d535f"),
              children: renderStatusTag(application.sections.orchestration.status)
            },
            {
              key: 'subject_kind',
              label: i18nText("applications", "auto.k_a8d3d15b65"),
              children: application.sections.orchestration.subject_kind
            },
            {
              key: 'subject_status',
              label: i18nText("applications", "auto.k_1e93fc0379"),
              children: application.sections.orchestration.subject_status
            },
            {
              key: 'subject_id',
              label: i18nText("applications", "auto.k_dec3ef2ce4"),
              children: application.sections.orchestration.current_subject_id ?? i18nText("applications", "auto.k_3bf179d8d0")
            },
            {
              key: 'draft_id',
              label: i18nText("applications", "auto.k_f9cd767c98"),
              children: application.sections.orchestration.current_draft_id ?? i18nText("applications", "auto.k_3c04f9eb8b")
            }
          ]}
        />
      </Space>
    );
  }

  if (sectionKey === 'api') {
    return (
      <Space direction="vertical" size="middle">
        <Typography.Title level={4}>API</Typography.Title>
        <Typography.Paragraph>
          {i18nText("applications", "auto.k_34dea221b0")}</Typography.Paragraph>
        <Descriptions
          bordered
          column={1}
          items={[
            {
              key: 'status',
              label: i18nText("applications", "auto.k_959c8d535f"),
              children: renderStatusTag(application.sections.api.status)
            },
            {
              key: 'credential_kind',
              label: i18nText("applications", "auto.k_3265c93172"),
              children: application.sections.api.credential_kind
            },
            {
              key: 'routing_mode',
              label: i18nText("applications", "auto.k_ddea5353a7"),
              children: application.sections.api.invoke_routing_mode
            },
            {
              key: 'path_template',
              label: i18nText("applications", "auto.k_4e396712b3"),
              children:
                application.sections.api.invoke_path_template ??
                i18nText("applications", "auto.k_04232bf146")
            },
            {
              key: 'credentials_status',
              label: i18nText("applications", "auto.k_21b7de945b"),
              children: application.sections.api.credentials_status
            }
          ]}
        />
      </Space>
    );
  }

  if (sectionKey === 'monitoring') {
    return (
      <Space direction="vertical" size="middle">
        <Typography.Title level={4}>{i18nText("applications", "auto.k_c87cbd5fc8")}</Typography.Title>
        <Typography.Paragraph>
          {i18nText("applications", "auto.k_52cd6392ce")}</Typography.Paragraph>
        <Descriptions
          bordered
          column={1}
          items={[
            {
              key: 'status',
              label: i18nText("applications", "auto.k_959c8d535f"),
              children: renderStatusTag(application.sections.monitoring.status)
            },
            {
              key: 'metrics_kind',
              label: i18nText("applications", "auto.k_bc9d472b4c"),
              children: application.sections.monitoring.metrics_object_kind
            },
            {
              key: 'metrics_status',
              label: i18nText("applications", "auto.k_803a4eca27"),
              children: application.sections.monitoring.metrics_capability_status
            },
            {
              key: 'tracing_status',
              label: i18nText("applications", "auto.k_6163bf0d94"),
              children: application.sections.monitoring.tracing_config_status
            }
          ]}
        />
      </Space>
    );
  }

  return <Result status="info" title={i18nText("applications", "auto.k_e1ccb97b46")} />;
}
