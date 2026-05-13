import { useQuery } from '@tanstack/react-query';
import { Empty, Result, Space, Splitter, Typography } from 'antd';
import { useState } from 'react';

import type { AgentFlowDebugMessage } from '../../agent-flow/api/runtime';
import { ConversationLogPanel } from '../../agent-flow/components/debug-console/ConversationLogPanel';
import {
  applicationRunsQueryKey,
  fetchApplicationRuns
} from '../api/runtime';
import { ApplicationRunDetailPanel } from '../components/logs/ApplicationRunDetailPanel';
import { ApplicationRunsTable } from '../components/logs/ApplicationRunsTable';
import './application-logs-page.css';

export function ApplicationLogsPage({
  applicationId
}: {
  applicationId: string;
}) {
  const [selectedRunId, setSelectedRunId] = useState<string | null>(null);
  const [openConversationLogMessage, setOpenConversationLogMessage] =
    useState<AgentFlowDebugMessage | null>(null);
  const runsQuery = useQuery({
    queryKey: applicationRunsQueryKey(applicationId),
    queryFn: () => fetchApplicationRuns(applicationId)
  });

  function selectRun(runId: string | null) {
    setSelectedRunId(runId);
    setOpenConversationLogMessage(null);
  }

  if (runsQuery.isPending) {
    return <Result status="info" title="正在加载运行日志" />;
  }

  if (runsQuery.isError) {
    return <Result status="error" title="运行日志加载失败" />;
  }

  const logsHeader = (
    <div className="application-logs-page__header">
      <Typography.Title level={4}>运行日志</Typography.Title>
      <Typography.Paragraph type="secondary">
        这里展示应用运行记录，点击后可在右侧直接查看对话和节点输入输出。
      </Typography.Paragraph>
    </div>
  );
  const logsList = (
    <section className="application-logs-page__list">
      {runsQuery.data.length === 0 ? (
        <Empty
          description="当前应用还没有运行记录"
          image={Empty.PRESENTED_IMAGE_SIMPLE}
        />
      ) : (
        <ApplicationRunsTable
          runs={runsQuery.data}
          selectedRunId={selectedRunId}
          onSelectRun={selectRun}
        />
      )}
    </section>
  );

  if (!selectedRunId) {
    return (
      <div className="application-logs-page">
        <Space direction="vertical" size="large" style={{ width: '100%' }}>
          {logsHeader}
          {logsList}
        </Space>
      </div>
    );
  }

  return (
    <div className="application-logs-page application-logs-page--detail-open">
      {logsHeader}
      <div
        className="application-logs-page__splitter"
        data-testid="application-logs-splitter"
      >
        <Splitter className="application-logs-page__splitter-control">
          <Splitter.Panel
            className="application-logs-page__splitter-panel"
            min={openConversationLogMessage ? 360 : 480}
          >
            {logsList}
          </Splitter.Panel>
          {openConversationLogMessage ? (
            <Splitter.Panel
              className="application-logs-page__splitter-panel"
              defaultSize={480}
              max="50%"
              min={360}
            >
              <div
                className="application-logs-page__conversation-log-panel"
                data-testid="application-logs-conversation-log-panel"
              >
                <ConversationLogPanel
                  message={openConversationLogMessage}
                  onClose={() => setOpenConversationLogMessage(null)}
                />
              </div>
            </Splitter.Panel>
          ) : null}
          <Splitter.Panel
            className="application-logs-page__splitter-panel"
            defaultSize={480}
            max={openConversationLogMessage ? '45%' : '60%'}
            min={360}
          >
            <ApplicationRunDetailPanel
              applicationId={applicationId}
              onClose={() => selectRun(null)}
              onOpenMessageLog={setOpenConversationLogMessage}
              runId={selectedRunId}
            />
          </Splitter.Panel>
        </Splitter>
      </div>
    </div>
  );
}
