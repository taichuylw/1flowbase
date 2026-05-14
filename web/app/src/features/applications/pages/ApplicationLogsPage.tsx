import { useQuery } from '@tanstack/react-query';
import { Empty, Result, Space, Typography } from 'antd';
import { useEffect, useRef, useState, type CSSProperties } from 'react';

import type { AgentFlowDebugMessage } from '../../agent-flow/api/runtime';
import { ConversationLogPanel } from '../../agent-flow/components/debug-console/ConversationLogPanel';
import {
  applicationRunsQueryKey,
  fetchApplicationRuns
} from '../api/runtime';
import { ApplicationRunDetailPanel } from '../components/logs/ApplicationRunDetailPanel';
import { ApplicationLogsFloatingWindow } from '../components/logs/ApplicationLogsFloatingWindow';
import { ApplicationRunsTable } from '../components/logs/ApplicationRunsTable';
import './application-logs-page.css';

const FLOATING_WINDOW_TOP = 112;
const FLOATING_WINDOW_GAP = 16;
const FLOATING_WINDOW_RIGHT = 32;
const RUN_DETAIL_WINDOW_WIDTH = 504;
const CONVERSATION_LOG_WINDOW_WIDTH = 520;
const FLOATING_WINDOW_MAX_HEIGHT = 720;

function getViewportSize() {
  if (typeof window === 'undefined') {
    return { width: 1280, height: 720 };
  }

  return {
    width: window.innerWidth,
    height: window.innerHeight
  };
}

function getFloatingWindowHeight() {
  const viewport = getViewportSize();

  return Math.max(
    320,
    Math.min(
      FLOATING_WINDOW_MAX_HEIGHT,
      viewport.height - FLOATING_WINDOW_TOP - FLOATING_WINDOW_RIGHT
    )
  );
}

function getRunDetailInitialRect() {
  const viewport = getViewportSize();

  return {
    left: viewport.width - RUN_DETAIL_WINDOW_WIDTH - FLOATING_WINDOW_RIGHT,
    top: FLOATING_WINDOW_TOP,
    width: RUN_DETAIL_WINDOW_WIDTH,
    height: getFloatingWindowHeight()
  };
}

function getConversationLogInitialRect() {
  const runDetailRect = getRunDetailInitialRect();

  return {
    left:
      runDetailRect.left -
      CONVERSATION_LOG_WINDOW_WIDTH -
      FLOATING_WINDOW_GAP,
    top: FLOATING_WINDOW_TOP,
    width: CONVERSATION_LOG_WINDOW_WIDTH,
    height: getFloatingWindowHeight()
  };
}

export function ApplicationLogsPage({
  applicationId
}: {
  applicationId: string;
}) {
  const [selectedRunId, setSelectedRunId] = useState<string | null>(null);
  const [openConversationLogMessage, setOpenConversationLogMessage] =
    useState<AgentFlowDebugMessage | null>(null);
  const [runsTableBodyHeight, setRunsTableBodyHeight] = useState<number | null>(
    null
  );
  const [activeFloatingWindow, setActiveFloatingWindow] = useState<
    'conversation-log' | 'run-detail'
  >('run-detail');
  const listRef = useRef<HTMLElement | null>(null);
  const runsQuery = useQuery({
    queryKey: applicationRunsQueryKey(applicationId),
    queryFn: () => fetchApplicationRuns(applicationId)
  });

  useEffect(() => {
    if (!selectedRunId) {
      setRunsTableBodyHeight(null);
      return;
    }

    function updateWorkspaceMeasurements() {
      const listElement = listRef.current;

      if (!listElement) {
        return;
      }

      const availableHeight = Math.floor(
        window.innerHeight - listElement.getBoundingClientRect().top
      );

      if (availableHeight <= 0) {
        return;
      }

      const tableHeaderElement = listElement.querySelector<HTMLElement>(
        '.application-logs-page__list .ant-table-thead'
      );
      const tableHeaderHeight =
        Math.ceil(tableHeaderElement?.getBoundingClientRect().height ?? 0) ||
        56;
      const nextRunsTableBodyHeight = Math.max(
        160,
        availableHeight - tableHeaderHeight
      );

      setRunsTableBodyHeight((currentHeight) =>
        currentHeight === nextRunsTableBodyHeight
          ? currentHeight
          : nextRunsTableBodyHeight
      );
    }

    updateWorkspaceMeasurements();

    const resizeObserver =
      typeof ResizeObserver === 'undefined'
        ? null
        : new ResizeObserver(updateWorkspaceMeasurements);

    if (resizeObserver && listRef.current) {
      resizeObserver.observe(listRef.current);
    }

    window.addEventListener('resize', updateWorkspaceMeasurements);

    return () => {
      resizeObserver?.disconnect();
      window.removeEventListener('resize', updateWorkspaceMeasurements);
    };
  }, [selectedRunId]);

  function selectRun(runId: string | null) {
    setSelectedRunId(runId);
    setOpenConversationLogMessage(null);
    setActiveFloatingWindow('run-detail');
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
        这里展示应用运行记录，点击后可用浮窗查看对话和节点输入输出。
      </Typography.Paragraph>
    </div>
  );
  const isDetailOpen = Boolean(selectedRunId);
  const logsListStyle =
    isDetailOpen && runsTableBodyHeight
      ? ({
          '--application-runs-table-body-height': `${runsTableBodyHeight}px`
        } as CSSProperties)
      : undefined;
  const logsList = (
    <section
      className="application-logs-page__list"
      data-testid="application-logs-list"
      ref={listRef}
      style={logsListStyle}
    >
      {runsQuery.data.length === 0 ? (
        <Empty
          description="当前应用还没有运行记录"
          image={Empty.PRESENTED_IMAGE_SIMPLE}
        />
      ) : (
        <ApplicationRunsTable
          runs={runsQuery.data}
          selectedRunId={selectedRunId}
          scrollY={
            isDetailOpen ? (runsTableBodyHeight ?? undefined) : undefined
          }
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
      {logsList}
      {openConversationLogMessage ? (
        <ApplicationLogsFloatingWindow
          active={activeFloatingWindow === 'conversation-log'}
          initialRect={getConversationLogInitialRect}
          testId="application-logs-floating-conversation-log"
          title="对话日志"
          onActivate={() => setActiveFloatingWindow('conversation-log')}
        >
          <div className="application-logs-page__conversation-log-panel">
            <ConversationLogPanel
              message={openConversationLogMessage}
              onClose={() => setOpenConversationLogMessage(null)}
            />
          </div>
        </ApplicationLogsFloatingWindow>
      ) : null}
      <ApplicationLogsFloatingWindow
        active={activeFloatingWindow === 'run-detail'}
        initialRect={getRunDetailInitialRect}
        testId="application-logs-floating-run-detail"
        title="运行详情"
        onActivate={() => setActiveFloatingWindow('run-detail')}
      >
        <ApplicationRunDetailPanel
          applicationId={applicationId}
          onClose={() => selectRun(null)}
          onOpenMessageLog={(message) => {
            setOpenConversationLogMessage(message);
            setActiveFloatingWindow('conversation-log');
          }}
          runId={selectedRunId}
        />
      </ApplicationLogsFloatingWindow>
    </div>
  );
}
