import { Button, Flex, List, Tag, Typography } from 'antd';
import { useTranslation } from 'react-i18next';

import { formatDateTime } from '../../../shared/i18n/format';
import type { Application } from '../api/applications';

interface ApplicationCardGridProps {
  applications: Application[];
}

function applicationTypeLabel(applicationType: Application['application_type']) {
  return applicationType === 'agent_flow' ? 'AgentFlow' : 'Workflow';
}

export function ApplicationCardGrid({ applications }: ApplicationCardGridProps) {
  const { t } = useTranslation('applications');

  return (
    <List
      grid={{ gutter: 16, column: 2 }}
      dataSource={applications}
      renderItem={(application) => (
        <List.Item>
          <Flex
            vertical
            gap={12}
            style={{
              padding: 16,
              border: '1px solid rgba(15, 23, 42, 0.08)',
              borderRadius: 16,
              background: '#ffffff'
            }}
          >
            <Flex justify="space-between" align="center" gap={12}>
              <Typography.Title level={4} style={{ margin: 0 }}>
                {application.name}
              </Typography.Title>
              <Tag>{applicationTypeLabel(application.application_type)}</Tag>
            </Flex>

            <Typography.Paragraph style={{ marginBottom: 0 }}>
              {application.description || t('auto.application_description_empty')}
            </Typography.Paragraph>

            <Typography.Text type="secondary">
              {t('auto.recently_updated')}{formatDateTime(application.updated_at)}
            </Typography.Text>

            <a href={`/applications/${application.id}/orchestration`}>
              <Button type="primary">{t('auto.enter_application')}</Button>
            </a>
          </Flex>
        </List.Item>
      )}
    />
  );
}
