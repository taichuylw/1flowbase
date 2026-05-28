import { Button, Drawer, List, Space, Tag, Typography } from 'antd';

import type { AgentFlowIssue } from '../../lib/validate-document';
import { i18nText } from '../../../../shared/i18n/text';

interface IssuesDrawerProps {
  open: boolean;
  onClose: () => void;
  issues: AgentFlowIssue[];
  onSelectIssue: (issue: AgentFlowIssue) => void;
}

export function IssuesDrawer({
  open,
  onClose,
  issues,
  onSelectIssue
}: IssuesDrawerProps) {
  return (
    <Drawer
      getContainer={false}
      open={open}
      placement="right"
      title="Issues"
      width={360}
      onClose={onClose}
    >
      <List
        dataSource={issues}
        locale={{ emptyText: i18nText("agentFlow", "auto.k_0a21937069") }}
        renderItem={(issue) => (
          <List.Item>
            <Space direction="vertical" size={4}>
              <Button type="link" onClick={() => onSelectIssue(issue)}>
                {issue.title}
              </Button>
              <Space size={8}>
                <Tag color={issue.level === 'error' ? 'red' : 'gold'}>
                  {issue.level === 'error' ? i18nText("agentFlow", "auto.k_b859c7be75") : i18nText("agentFlow", "auto.k_5521e368d8")}
                </Tag>
                {issue.sectionKey ? <Tag>{issue.sectionKey}</Tag> : null}
              </Space>
              <Typography.Text type="secondary">{issue.message}</Typography.Text>
            </Space>
          </List.Item>
        )}
      />
    </Drawer>
  );
}
