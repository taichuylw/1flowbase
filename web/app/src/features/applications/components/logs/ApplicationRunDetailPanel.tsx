import { CheckOutlined, CopyOutlined } from '@ant-design/icons';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { App, Button, Tooltip } from 'antd';
import { useEffect, useMemo, useState } from 'react';

import { AgentFlowDebugConsole } from '../../../agent-flow/components/debug-console/AgentFlowDebugConsole';
import type {
  AgentFlowDebugMessage,
  AgentFlowDebugMessageStatus,
  AgentFlowRunContext
} from '../../../agent-flow/api/runtime';
import {
  extractAssistantOutputText,
  mapRunDetailToTrace
} from '../../../agent-flow/lib/debug-console/run-detail-mapper';
import type { AgentFlowDebugSessionStatus } from '../../../agent-flow/hooks/runtime/useAgentFlowDebugSession';
import { useClipboardCopy } from '../../../../shared/ui/clipboard/use-clipboard-copy';
import {
  applicationConversationMessagesQueryKey,
  applicationRunDetailQueryKey,
  fetchApplicationConversationMessages,
  fetchApplicationRunDetail,
  type ApplicationConversationMessagesPage,
  type ApplicationRunDetail
} from '../../api/runtime';
import { formatApplicationRunCompatibilityMode } from '../../lib/run-compatibility-mode';
import './application-run-detail-panel.css';

function isRecord(value: unknown): value is Record<string, unknown> {
  return Boolean(value && typeof value === 'object' && !Array.isArray(value));
}

function nonEmptyString(value: unknown): string | null {
  return typeof value === 'string' && value.trim().length > 0
    ? value
    : null;
}

function findFirstString(value: unknown): string | null {
  if (typeof value === 'string' && value.trim().length > 0) {
    return value;
  }

  if (Array.isArray(value)) {
    for (const entry of value) {
      const nestedValue = findFirstString(entry);

      if (nestedValue) {
        return nestedValue;
      }
    }

    return null;
  }

  if (isRecord(value)) {
    for (const nestedValue of Object.values(value)) {
      const firstString = findFirstString(nestedValue);

      if (firstString) {
        return firstString;
      }
    }
  }

  return null;
}

function mapRunStatusToMessageStatus(
  status: string
): AgentFlowDebugMessageStatus {
  switch (status) {
    case 'succeeded':
      return 'completed';
    case 'waiting_callback':
      return 'waiting_callback';
    case 'waiting_human':
      return 'waiting_human';
    case 'cancelled':
      return 'cancelled';
    case 'failed':
      return 'failed';
    default:
      return 'running';
  }
}

function mapRunStatusToSessionStatus(
  status: string
): AgentFlowDebugSessionStatus {
  switch (status) {
    case 'succeeded':
      return 'completed';
    case 'waiting_callback':
      return 'waiting_callback';
    case 'waiting_human':
      return 'waiting_human';
    case 'cancelled':
      return 'cancelled';
    case 'failed':
      return 'failed';
    case 'running':
      return 'running';
    default:
      return 'completed';
  }
}

const runConversationContext: AgentFlowRunContext = {
  environmentLabel: 'draft',
  remembered: false,
  fields: []
};

function RunIdSubtitle({ runId }: { runId: string }) {
  const { message } = App.useApp();
  const { copied, copy } = useClipboardCopy();

  async function handleCopyRunId() {
    try {
      await copy(runId);
      message.success('已复制 ID');
    } catch {
      message.error('复制失败');
    }
  }

  return (
    <span className="application-run-detail__run-id">
      <span className="application-run-detail__run-id-value">{runId}</span>
      <Tooltip title="复制 ID">
        <Button
          aria-label="复制运行 ID"
          className="application-run-detail__run-id-copy"
          icon={copied ? <CheckOutlined /> : <CopyOutlined />}
          onClick={handleCopyRunId}
          size="small"
          type="text"
        />
      </Tooltip>
    </span>
  );
}

function runDetailCompatibilityMode(detail: ApplicationRunDetail) {
  return (
    detail.run?.compatibility_mode ??
    detail.run?.correlation?.compatibility_mode ??
    null
  );
}

function buildConversationLogMessage(
  detail: ApplicationRunDetail
): AgentFlowDebugMessage {
  const assistantContent =
    extractAssistantOutputText(detail) ||
    findFirstString(detail.flow_run.output_payload) ||
    '暂无输出';
  const rawOutput =
    Object.keys(detail.flow_run.output_payload).length > 0
      ? detail.flow_run.output_payload
      : null;
  const compatibilityMode = runDetailCompatibilityMode(detail);

  return {
    id: `conversation-log-${detail.flow_run.id}`,
    role: 'assistant',
    content: assistantContent,
    status: mapRunStatusToMessageStatus(detail.flow_run.status),
    runId: detail.flow_run.id,
    detailRunId: detail.flow_run.id,
    canOpenDetail: true,
    compatibilityMode,
    compatibilityModeLabel:
      formatApplicationRunCompatibilityMode(compatibilityMode),
    rawOutput,
    traceSummary: mapRunDetailToTrace(detail)
  };
}

function conversationItemDetailRunId(
  item: ApplicationConversationMessagesPage['items'][number]
): string | null {
  if (item.can_open_detail === false) {
    return null;
  }

  return nonEmptyString(item.detail_run_id) ?? nonEmptyString(item.run_id);
}

function mapConversationItemToMessages(
  item: ApplicationConversationMessagesPage['items'][number]
): AgentFlowDebugMessage[] {
  const detailRunId = conversationItemDetailRunId(item);
  const canOpenDetail = Boolean(detailRunId) && item.can_open_detail !== false;
  const userContent = nonEmptyString(item.query) ?? '无';
  const assistantContent = nonEmptyString(item.answer) ?? '暂无输出';

  return [
    {
      id: `conversation-user-${item.run_id}`,
      role: 'user',
      content: userContent,
      status: 'completed',
      runId: item.run_id,
      detailRunId,
      canOpenDetail,
      rawOutput: null,
      traceSummary: []
    },
    {
      id: `conversation-assistant-${item.run_id}`,
      role: 'assistant',
      content: assistantContent,
      status: mapRunStatusToMessageStatus(item.status),
      runId: item.run_id,
      detailRunId,
      canOpenDetail,
      rawOutput: null,
      traceSummary: []
    }
  ];
}

function buildConversationPageMessages(
  page: ApplicationConversationMessagesPage | null
): AgentFlowDebugMessage[] {
  if (!page || page.items.length === 0) {
    return [];
  }

  return page.items.flatMap((item) => mapConversationItemToMessages(item));
}

function conversationSessionStatus(
  page: ApplicationConversationMessagesPage | null
): AgentFlowDebugSessionStatus {
  const currentItem =
    page?.items.find((item) => item.is_current) ?? page?.items.at(-1) ?? null;

  return mapRunStatusToSessionStatus(currentItem?.status ?? 'succeeded');
}

function RunConversation({
  applicationId,
  onClose,
  onOpenMessageLog,
  runId
}: {
  applicationId: string;
  onClose: () => void;
  onOpenMessageLog?: (message: AgentFlowDebugMessage) => void;
  runId: string;
}) {
  const queryClient = useQueryClient();
  const [conversationPage, setConversationPage] =
    useState<ApplicationConversationMessagesPage | null>(null);
  const initialConversationQuery = useQuery({
    queryKey: applicationConversationMessagesQueryKey(applicationId, runId),
    queryFn: () =>
      fetchApplicationConversationMessages(applicationId, runId, {
        limit: 5
      })
  });
  const loadPreviousConversationMutation = useMutation({
    mutationFn: async () => {
      if (!conversationPage?.page.before_cursor) {
        throw new Error('missing previous conversation cursor');
      }

      return fetchApplicationConversationMessages(applicationId, runId, {
        before: conversationPage.page.before_cursor,
        limit: 5
      });
    },
    onSuccess: (page) => {
      setConversationPage((current) => {
        if (!current) {
          return page;
        }

        const existingRunIds = new Set(
          current.items.map((item) => item.run_id)
        );
        const newItems = page.items.filter(
          (item) => !existingRunIds.has(item.run_id)
        );

        return {
          items: [...newItems, ...current.items],
          page: {
            has_before: page.page.has_before,
            has_after: current.page.has_after,
            before_cursor: page.page.before_cursor,
            after_cursor: current.page.after_cursor
          }
        };
      });
    }
  });
  const messages = useMemo(
    () => buildConversationPageMessages(conversationPage),
    [conversationPage]
  );

  useEffect(() => {
    setConversationPage(null);
  }, [runId]);

  useEffect(() => {
    if (initialConversationQuery.data) {
      setConversationPage(initialConversationQuery.data);
    }
  }, [initialConversationQuery.data]);

  async function handleOpenMessageLog(message: AgentFlowDebugMessage) {
    const detailRunId =
      nonEmptyString(message.detailRunId) ??
      (message.canOpenDetail === false ? null : nonEmptyString(message.runId));

    if (!detailRunId) {
      return;
    }

    const detail = await queryClient.fetchQuery({
      queryKey: applicationRunDetailQueryKey(applicationId, detailRunId),
      queryFn: () => fetchApplicationRunDetail(applicationId, detailRunId)
    });

    onOpenMessageLog?.(buildConversationLogMessage(detail));
  }

  return (
    <div className="application-run-detail__conversation-pane">
      <AgentFlowDebugConsole
        ariaLabel="运行详情预览"
        closeLabel="关闭运行详情"
        composerUiOnly
        messages={messages}
        runContext={runConversationContext}
        showClearAction={false}
        showComposer
        status={conversationSessionStatus(conversationPage)}
        stopping={false}
        subtitle={<RunIdSubtitle runId={runId} />}
        title="运行详情"
        onChangeRunContextValue={() => {}}
        onClearSession={() => {}}
        onClose={onClose}
        onOpenMessageLog={(message) => {
          void handleOpenMessageLog(message);
        }}
        onReachConversationTop={() => {
          if (
            conversationPage?.page.has_before &&
            !loadPreviousConversationMutation.isPending
          ) {
            loadPreviousConversationMutation.mutate();
          }
        }}
        onStopRun={() => {}}
        onSubmitPrompt={() => {}}
      />
    </div>
  );
}

function renderDetail({
  applicationId,
  onClose,
  onOpenMessageLog,
  runId
}: {
  applicationId: string;
  onClose: () => void;
  onOpenMessageLog?: (message: AgentFlowDebugMessage) => void;
  runId: string;
}) {
  return (
    <div className="application-run-detail__content">
      <RunConversation
        applicationId={applicationId}
        onClose={onClose}
        onOpenMessageLog={onOpenMessageLog}
        runId={runId}
      />
    </div>
  );
}

export function ApplicationRunDetailPanel({
  applicationId,
  onClose,
  onOpenMessageLog,
  runId
}: {
  applicationId: string;
  onClose: () => void;
  onOpenMessageLog?: (message: AgentFlowDebugMessage) => void;
  runId: string | null;
}) {
  if (!runId) {
    return null;
  }

  return (
    <aside
      aria-label="运行详情"
      className="application-run-detail application-run-detail--loaded"
    >
      <div className="application-run-detail__body">
        {renderDetail({
          applicationId,
          onClose,
          onOpenMessageLog,
          runId
        })}
      </div>
    </aside>
  );
}
