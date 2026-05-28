import { Button, Flex, List, Tag, Typography } from 'antd';

import type { Application } from '../api/applications';
import { i18nText } from '../../../shared/i18n/text';

interface ApplicationCardGridProps {
  applications: Application[];
}

function applicationTypeLabel(applicationType: Application['application_type']) {
  return applicationType === 'agent_flow' ? 'AgentFlow' : 'Workflow';
}

export function ApplicationCardGrid({ applications }: ApplicationCardGridProps) {
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
              {application.description || i18nText("applications", "auto.k_14e94c943d")}
            </Typography.Paragraph>

            <Typography.Text type="secondary">
              {i18nText("applications", "auto.k_8d8174e0f2")}{new Date(application.updated_at).toLocaleString('zh-CN')}
            </Typography.Text>

            <a href={`/applications/${application.id}/orchestration`}>
              <Button type="primary">{i18nText("applications", "auto.k_ba95e86694")}</Button>
            </a>
          </Flex>
        </List.Item>
      )}
    />
  );
}
