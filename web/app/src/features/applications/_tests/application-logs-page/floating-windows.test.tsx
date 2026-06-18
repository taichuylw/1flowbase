import {
  act,
  fireEvent,
  render,
  screen,
  waitFor,
  within
} from '@testing-library/react';
import { vi } from 'vitest';

type ConversationMessagePageItem = {
  id: string;
  flow_run_id: string | null;
  role: 'system' | 'user' | 'assistant';
  content: string;
  sequence: number;
  status?: string;
  started_at?: string | null;
  finished_at?: string | null;
};

const runtimeApi = vi.hoisted(() => ({
  applicationRunsQueryKey: (
    applicationId: string,
    input?: {
      page?: number;
      pageSize?: number;
      timeRangeDays?: number | null;
      sortBy?: 'started_at' | 'finished_at' | 'created_at';
      sortOrder?: 'asc' | 'desc';
      cacheMode?: 'default' | 'refresh';
    }
  ) =>
    [
      'applications',
      applicationId,
      'runtime',
      'runs',
      input?.page ?? 1,
      input?.pageSize ?? 20,
      input?.timeRangeDays ?? 'all',
      input?.sortBy ?? 'started_at',
      input?.sortOrder ?? 'desc'
    ] as const,
  applicationRunTraceTreeQueryKey: (applicationId: string, runId: string) =>
    [
      'applications',
      applicationId,
      'runtime',
      'runs',
      runId,
      'trace-tree'
    ] as const,
  applicationRunOverviewQueryKey: (applicationId: string, runId: string) =>
    [
      'applications',
      applicationId,
      'runtime',
      'runs',
      runId,
      'overview'
    ] as const,
  applicationRunTraceNodeChildrenQueryKey: (
    applicationId: string,
    runId: string,
    traceNodeId: string
  ) =>
    [
      'applications',
      applicationId,
      'runtime',
      'runs',
      runId,
      'trace-tree',
      traceNodeId,
      'children'
    ] as const,
  applicationRunTraceNodeContentQueryKey: (
    applicationId: string,
    runId: string,
    traceNodeId: string
  ) =>
    [
      'applications',
      applicationId,
      'runtime',
      'runs',
      runId,
      'trace-tree',
      traceNodeId,
      'content'
    ] as const,
  applicationRunResumeTimelineQueryKey: (
    applicationId: string,
    runId: string
  ) =>
    [
      'applications',
      applicationId,
      'runtime',
      'runs',
      runId,
      'resume-timeline'
    ] as const,
  applicationConversationMessagesQueryKey: (
    applicationId: string,
    runId: string
  ) =>
    [
      'applications',
      applicationId,
      'runtime',
      'runs',
      runId,
      'conversation',
      'around',
      runId
    ] as const,
  applicationRunConversationMessagesQueryKey: (
    applicationId: string,
    runId: string
  ) =>
    [
      'applications',
      applicationId,
      'runtime',
      'runs',
      runId,
      'conversation-messages'
    ] as const,
  fetchApplicationRuns: vi.fn(),
  fetchApplicationRunOverview: vi.fn(),
  fetchApplicationRunTraceTree: vi.fn(),
  fetchApplicationRunTraceNodeChildren: vi.fn(),
  fetchApplicationRunTraceNodeContent: vi.fn(),
  fetchApplicationRunResumeTimeline: vi.fn(),
  fetchApplicationConversationMessages: vi.fn(),
  fetchApplicationRunConversationMessages: vi.fn().mockImplementation(
    async (
      appId: string,
      runId: string,
      options?: {
        limit?: number;
      }
    ) => {
      const rawPage = await runtimeApi.fetchApplicationConversationMessages(
        appId,
        {
          flowRunId: runId,
          page: 1,
          pageSize: options?.limit ?? 5
        }
      );
      return {
        items: rawPage.items.map((item: ConversationMessagePageItem) => ({
          ...item,
          run_id: item.flow_run_id ?? `message:${item.id}`,
          detail_run_id: item.flow_run_id,
          can_open_detail: item.flow_run_id === 'run-0',
          status: item.status ?? 'succeeded'
        })),
        page: {
          has_before: false,
          has_after: false,
          before_cursor: null,
          after_cursor: null
        }
      };
    }
  ),
  fetchRuntimeDebugArtifact: vi.fn(),
  resumeFlowRun: vi.fn(),
  completeCallbackTask: vi.fn()
}));

vi.mock('../../api/runtime', () => runtimeApi);

import type { ConsoleApplicationRunDetail as ApplicationRunDetail } from '@1flowbase/api-client';
import { AppProviders } from '../../../../app/AppProviders';
import { appI18n } from '../../../../shared/i18n/app-i18n';
import { resetAuthStore } from '../../../../state/auth-store';
import { ApplicationLogsPage } from '../../pages/ApplicationLogsPage';

function applicationRunsPage<T>(
  items: T[],
  overrides?: Partial<{
    total: number;
    page: number;
    page_size: number;
  }>
) {
  return {
    items,
    total: overrides?.total ?? items.length,
    page: overrides?.page ?? 1,
    page_size: overrides?.page_size ?? 20
  };
}

function conversationMessagesPage(
  items: Array<{
    id: string;
    flow_run_id: string | null;
    role: 'system' | 'user' | 'assistant';
    content: string;
    sequence: number;
    status?: string;
    started_at?: string | null;
    finished_at?: string | null;
  }>
) {
  return {
    items: items.map((item) => ({
      run_id: item.flow_run_id ?? `message:${item.id}`,
      detail_run_id: item.flow_run_id,
      can_open_detail: Boolean(item.flow_run_id),
      role: item.role,
      content: item.content,
      started_at: item.started_at ?? '2026-04-17T09:00:00Z',
      finished_at: item.finished_at ?? '2026-04-17T09:00:01Z',
      status: item.status ?? 'succeeded',
      query: null,
      model: null,
      answer: null,
      is_current: item.flow_run_id === 'run-1'
    })),
    page: {
      has_before: false,
      has_after: false,
      before_cursor: null,
      after_cursor: null
    }
  };
}

function lastElement<T>(items: T[], message: string): T {
  const item = items.at(-1);
  if (!item) {
    throw new Error(message);
  }
  return item;
}

function sampleRunDetail(): ApplicationRunDetail {
  return {
    run: {
      id: 'run-1',
      application_id: 'app-1',
      application_type: 'agent_flow',
      run_object_kind: 'flow_run',
      run_kind: 'published_api_run',
      status: 'succeeded',
      title: '公开 API 退款总结',
      source: 'api_key',
      compatibility_mode: 'openai-responses-v1',
      subject: {
        kind: 'agent_flow',
        id: 'flow-1',
        draft_id: 'draft-1',
        target_node_id: 'node-llm'
      },
      actor: {
        kind: 'user',
        id: 'user-1',
        display_name: 'root'
      },
      correlation: {
        compatibility_mode: 'openai-responses-v1'
      },
      started_at: '2026-04-17T09:00:00Z',
      finished_at: '2026-04-17T09:00:01Z',
      created_at: '2026-04-17T09:00:00Z',
      updated_at: '2026-04-17T09:00:01Z'
    },
    statistics: {
      total_tokens: 50,
      input_tokens: 40,
      output_tokens: 10,
      input_cache_hit_tokens: 12,
      unique_node_count: 3,
      tool_callback_count: 20
    },
    flow_run: {
      id: 'run-1',
      application_id: 'app-1',
      flow_id: 'flow-1',
      draft_id: 'draft-1',
      compiled_plan_id: 'plan-1',
      run_mode: 'published_api_run' as const,
      status: 'succeeded',
      target_node_id: 'node-llm',
      title: '公开 API 退款总结',
      expand_id: 'customer-42',
      authorized_account: 'root',
      external_conversation_id: 'conversation-1',
      query: '总结退款政策',
      model: 'deepseek-chat',
      input_payload: {
        __runtime_debug_artifact: true,
        artifact_ref: 'artifact-flow-input',
        content_type: 'application/json',
        is_truncated: true,
        original_size_bytes: 54538,
        preview_size_bytes: 2048,
        preview:
          '{"node-start":{"compatibility":{"tools":[{"function":{"description":"path to the file to read."}}]}}}'
      } as Record<string, unknown>,
      output_payload: {
        answer: '退款政策摘要',
        resolved_inputs: {
          user_prompt: '总结退款政策'
        }
      },
      error_payload: null,
      created_by: 'user-1',
      started_at: '2026-04-17T09:00:00Z',
      finished_at: '2026-04-17T09:00:01Z',
      created_at: '2026-04-17T09:00:00Z',
      updated_at: '2026-04-17T09:00:01Z'
    },
    node_runs: [
      {
        id: 'node-run-1',
        flow_run_id: 'run-1',
        node_id: 'node-llm',
        node_type: 'llm',
        node_alias: 'LLM',
        status: 'succeeded',
        input_payload: {
          user_prompt: '总结退款政策'
        },
        output_payload: {
          answer: '退款政策摘要',
          rendered_templates: {}
        },
        error_payload: null,
        metrics_payload: {
          output_contract_count: 1
        },
        started_at: '2026-04-17T09:00:00Z',
        finished_at: '2026-04-17T09:00:01Z'
      }
    ],
    checkpoints: [],
    callback_tasks: [],
    events: [
      {
        id: 'event-1',
        flow_run_id: 'run-1',
        node_run_id: 'node-run-1',
        sequence: 1,
        event_type: 'node_preview_started',
        payload: {
          target_node_id: 'node-llm'
        },
        created_at: '2026-04-17T09:00:00Z'
      },
      {
        id: 'event-2',
        flow_run_id: 'run-1',
        node_run_id: 'node-run-1',
        sequence: 2,
        event_type: 'node_preview_completed',
        payload: {
          target_node_id: 'node-llm'
        },
        created_at: '2026-04-17T09:00:01Z'
      }
    ]
  };
}

function sampleTraceTree() {
  return {
    run: {
      id: 'run-1',
      application_id: 'app-1',
      application_type: 'agent_flow',
      run_object_kind: 'application_run',
      run_kind: 'published_api_run',
      status: 'succeeded',
      title: '公开 API 退款总结',
      source: 'api_key',
      compatibility_mode: 'openai-responses-v1',
      subject: {
        kind: 'agent_flow',
        id: 'flow-1',
        draft_id: 'draft-1',
        target_node_id: 'node-llm'
      },
      actor: {
        kind: 'user',
        id: 'user-1',
        display_name: 'root'
      },
      correlation: {
        compatibility_mode: 'openai-responses-v1'
      },
      started_at: '2026-04-17T09:00:00Z',
      finished_at: '2026-04-17T09:00:01Z',
      created_at: '2026-04-17T09:00:00Z',
      updated_at: '2026-04-17T09:00:01Z'
    },
    statistics: {
      total_tokens: 50,
      input_tokens: 40,
      output_tokens: 10,
      input_cache_hit_tokens: 12,
      unique_node_count: 3,
      tool_callback_count: 20
    },
    flow_run: sampleRunDetail().flow_run,
    answer_snapshot: null,
    nodes: [
      {
        trace_node_id: 'node_run:node-run-1',
        parent_trace_node_id: null,
        node_kind: 'node_run',
        flow_run_id: 'run-1',
        node_run_id: 'node-run-1',
        node_id: 'node-llm',
        node_type: 'llm',
        node_alias: 'LLM',
        status: 'succeeded',
        started_at: '2026-04-17T09:00:00Z',
        finished_at: '2026-04-17T09:00:01Z',
        duration_ms: 1000,
        metrics_payload: {
          output_contract_count: 1
        },
        has_children: false,
        has_content: true
      }
    ]
  };
}

function sampleRunOverview() {
  const detail = sampleRunDetail();

  return {
    run: detail.run,
    statistics: detail.statistics,
    flow_run: detail.flow_run,
    answer_snapshot: detail.answer_snapshot ?? null
  };
}

function sampleTraceNodeContent() {
  return {
    trace_node_id: 'node_run:node-run-1',
    node_kind: 'node_run',
    node_run: sampleRunDetail().node_runs[0],
    callback_task: null,
    flow_run: null,
    checkpoints: [],
    events: sampleRunDetail().events
  };
}

describe('ApplicationLogsPage - floating windows', () => {
  let getBoundingClientRectSpy: { mockRestore: () => void } | undefined;
  let innerHeightSpy: { mockRestore: () => void } | undefined;
  let innerWidthSpy: { mockRestore: () => void } | undefined;
  let dateNowSpy: { mockRestore: () => void } | undefined;

  beforeEach(async () => {
    window.localStorage.clear();
    window.localStorage.setItem('1flowbase.ui.locale_preference', 'zh_Hans');
    await appI18n.changeLanguage('zh_Hans');
    dateNowSpy = vi
      .spyOn(Date, 'now')
      .mockReturnValue(new Date('2026-04-18T00:00:00Z').getTime());
    runtimeApi.fetchApplicationRuns.mockReset();
    runtimeApi.fetchApplicationRunOverview.mockReset();
    runtimeApi.fetchApplicationRunTraceTree.mockReset();
    runtimeApi.fetchApplicationRunTraceNodeChildren.mockReset();
    runtimeApi.fetchApplicationRunTraceNodeContent.mockReset();
    runtimeApi.fetchApplicationRunResumeTimeline.mockReset();
    runtimeApi.fetchApplicationConversationMessages.mockReset();
    runtimeApi.fetchApplicationRunConversationMessages.mockReset();
    runtimeApi.fetchRuntimeDebugArtifact.mockReset();

    runtimeApi.fetchApplicationRuns.mockResolvedValue(
      applicationRunsPage([
        {
          id: 'run-1',
          run_mode: 'published_api_run' as const,
          status: 'succeeded',
          target_node_id: 'node-llm',
          title: '公开 API 退款总结',
          expand_id: 'customer-42',
          authorized_account: 'root',
          compatibility_mode: 'openai-responses-v1',
          total_tokens: 50,
          input_tokens: 40,
          output_tokens: 10,
          input_cache_hit_tokens: 12,
          unique_node_count: 3,
          tool_callback_count: 20,
          started_at: '2026-04-17T09:00:00Z',
          finished_at: '2026-04-17T09:00:01Z',
          created_at: '2026-04-17T09:00:00Z',
          updated_at: '2026-04-17T09:00:01Z'
        }
      ])
    );
    runtimeApi.fetchApplicationRunTraceTree.mockResolvedValue(
      sampleTraceTree()
    );
    runtimeApi.fetchApplicationRunOverview.mockResolvedValue(
      sampleRunOverview()
    );
    runtimeApi.fetchApplicationRunTraceNodeChildren.mockResolvedValue({
      items: [],
      page_info: {
        has_more: false,
        next_cursor: null,
        page_size: 20
      }
    });
    runtimeApi.fetchApplicationRunTraceNodeContent.mockResolvedValue(
      sampleTraceNodeContent()
    );
    runtimeApi.fetchApplicationRunResumeTimeline.mockResolvedValue({
      flow_run: sampleRunDetail().flow_run,
      callback_tasks: sampleRunDetail().callback_tasks,
      events: sampleRunDetail().events
    });
    runtimeApi.fetchApplicationRunConversationMessages.mockResolvedValue(
      conversationMessagesPage([
        {
          id: 'msg-history-system',
          flow_run_id: null,
          role: 'system',
          content: '你是项目助手',
          sequence: 1,
          started_at: '2026-04-17T08:59:00Z',
          finished_at: '2026-04-17T08:59:01Z'
        },
        {
          id: 'msg-run-1-user',
          flow_run_id: 'run-1',
          role: 'user',
          content: '总结退款政策',
          sequence: 2,
          started_at: '2026-04-17T09:00:00Z',
          finished_at: '2026-04-17T09:00:01Z'
        },
        {
          id: 'msg-run-1-assistant',
          flow_run_id: 'run-1',
          role: 'assistant',
          content: '退款政策摘要',
          sequence: 3,
          started_at: '2026-04-17T09:00:00Z',
          finished_at: '2026-04-17T09:00:01Z'
        }
      ])
    );
  });

  afterEach(() => {
    resetAuthStore();
    getBoundingClientRectSpy?.mockRestore();
    getBoundingClientRectSpy = undefined;
    innerHeightSpy?.mockRestore();
    innerHeightSpy = undefined;
    innerWidthSpy?.mockRestore();
    innerWidthSpy = undefined;
    dateNowSpy?.mockRestore();
    dateNowSpy = undefined;
  });

  test('opens run detail and conversation log as floating windows', async () => {
    render(
      <AppProviders>
        <ApplicationLogsPage applicationId="app-1" />
      </AppProviders>
    );

    expect((await screen.findAllByRole('table')).length).toBeGreaterThan(0);
    expect(screen.getByText('公开 API 退款总结')).toBeInTheDocument();
    expect(screen.getByText('customer-42')).toBeInTheDocument();
    expect(screen.getByText('root')).toBeInTheDocument();
    expect(
      screen.getByRole('columnheader', {
        name: '协议'
      })
    ).toBeInTheDocument();
    expect(screen.getByText('OpenAI Responses')).toBeInTheDocument();
    expect(
      screen.getByRole('columnheader', {
        name: 'expand_id'
      })
    ).toBeInTheDocument();
    expect(
      screen.getByRole('columnheader', { name: '总 tokens' })
    ).toBeInTheDocument();
    expect(
      screen.getByRole('columnheader', { name: '真实节点数' })
    ).toBeInTheDocument();
    expect(
      screen.getByRole('columnheader', { name: '工具回调次数' })
    ).toBeInTheDocument();
    expect(screen.getByText('50')).toBeInTheDocument();
    expect(screen.getByText('3')).toBeInTheDocument();
    expect(screen.getByText('20')).toBeInTheDocument();
    expect(runtimeApi.fetchApplicationRuns).toHaveBeenCalledWith('app-1', {
      page: 1,
      pageSize: 20,
      timeRangeDays: 7,
      sortBy: 'started_at',
      sortOrder: 'desc'
    });
    expect(
      screen.queryByRole('complementary', { name: '运行详情' })
    ).not.toBeInTheDocument();
    expect(
      screen.queryByRole('dialog', { name: '运行详情' })
    ).not.toBeInTheDocument();
    expect(
      screen.queryByTestId('application-logs-floating-run-detail')
    ).not.toBeInTheDocument();
    expect(
      screen.queryByTestId('application-logs-splitter')
    ).not.toBeInTheDocument();
    expect(screen.getByTestId('application-logs-page')).not.toHaveClass(
      'application-logs-page--detail-open'
    );
    expect(screen.getByTestId('application-logs-list')).not.toHaveAttribute(
      'style'
    );

    fireEvent.click(screen.getByRole('button', { name: '查看运行详情' }));

    await waitFor(() => {
      expect(
        runtimeApi.fetchApplicationRunConversationMessages
      ).toHaveBeenCalledWith('app-1', 'run-1', { limit: 5 });
    });
    const detailPane = await screen.findByRole('complementary', {
      name: '运行详情'
    });
    expect(detailPane).toBeInTheDocument();
    expect(
      screen.queryByTestId('application-run-detail-meta')
    ).not.toBeInTheDocument();
    expect(
      screen.getByRole('dialog', { name: '运行详情' })
    ).toBeInTheDocument();
    expect(screen.getAllByRole('table').length).toBeGreaterThan(0);
    expect(
      screen.queryByRole('button', { name: '返回日志' })
    ).not.toBeInTheDocument();
    expect(
      screen.queryByTestId('application-logs-splitter')
    ).not.toBeInTheDocument();
    expect(screen.getByTestId('application-logs-page')).not.toHaveClass(
      'application-logs-page--detail-open'
    );
    expect(screen.getByTestId('application-logs-list')).not.toHaveAttribute(
      'style'
    );
    expect(
      screen.getByTestId('application-logs-floating-run-detail')
    ).toBeInTheDocument();
    expect(
      within(
        screen.getByTestId('application-logs-floating-run-detail')
      ).getByRole('separator', { name: '从右侧调整运行详情宽度' })
    ).toBeInTheDocument();
    expect(
      within(
        screen.getByTestId('application-logs-floating-run-detail')
      ).getByRole('separator', { name: '从左侧调整运行详情宽度' })
    ).toBeInTheDocument();
    expect(
      within(
        screen.getByTestId('application-logs-floating-run-detail')
      ).getByRole('separator', { name: '向下调整运行详情高度' })
    ).toBeInTheDocument();

    const conversation = await screen.findByTestId(
      'debug-conversation-messages'
    );
    expect(within(conversation).getByText('System')).toBeInTheDocument();
    expect(within(conversation).getByText('你是项目助手')).toBeInTheDocument();
    expect(within(conversation).getAllByText('User')).toHaveLength(1);
    expect(
      within(conversation).queryByText('上一轮问题')
    ).not.toBeInTheDocument();
    expect(
      within(conversation).queryByText('上一轮回答')
    ).not.toBeInTheDocument();
    expect(within(conversation).getByText('总结退款政策')).toBeInTheDocument();
    expect(within(conversation).getByText('退款政策摘要')).toBeInTheDocument();
    const composerInput = screen.getByPlaceholderText('和 Bot 聊天');
    expect(composerInput).toBeInTheDocument();
    fireEvent.change(composerInput, {
      target: { value: '这只是日志页的输入 UI' }
    });
    expect(composerInput).toHaveValue('这只是日志页的输入 UI');
    fireEvent.click(screen.getByRole('button', { name: '发送调试消息' }));
    expect(composerInput).toHaveValue('');
    expect(runtimeApi.resumeFlowRun).not.toHaveBeenCalled();
    expect(runtimeApi.completeCallbackTask).not.toHaveBeenCalled();
    expect(screen.queryByText('功能已开启')).not.toBeInTheDocument();
    expect(
      screen.queryByRole('button', { name: '管理功能' })
    ).not.toBeInTheDocument();
    expect(screen.queryByText('运行摘要')).not.toBeInTheDocument();
    expect(screen.queryByText('运行输入输出')).not.toBeInTheDocument();
    expect(screen.queryByText('事件时间线')).not.toBeInTheDocument();

    expect(
      within(detailPane).queryByRole('button', { name: /LLM.*llm/ })
    ).not.toBeInTheDocument();
    expect(
      within(detailPane).queryByLabelText('输入 JSON')
    ).not.toBeInTheDocument();

    const openLogButton = lastElement(
      await screen.findAllByRole(
        'button',
        { name: '查看对话日志' },
        { timeout: 8_000 }
      ),
      'expected conversation log button'
    );
    fireEvent.click(openLogButton);

    const logPanel = await screen.findByRole('complementary', {
      name: '对话日志'
    });
    expect(logPanel).toBeInTheDocument();
    expect(
      screen.getByRole('dialog', { name: '对话日志' })
    ).toBeInTheDocument();
    expect(detailPane).not.toContainElement(logPanel);
    expect(detailPane).toContainElement(conversation);
    expect(
      screen.getByTestId('application-logs-floating-conversation-log')
    ).toBeInTheDocument();
    expect(within(logPanel).getByRole('tab', { name: '详情' })).toHaveAttribute(
      'aria-selected',
      'true'
    );
    expect(runtimeApi.fetchApplicationRunTraceTree).not.toHaveBeenCalled();
    expect(await within(logPanel).findByLabelText('输出 JSON')).toHaveTextContent(
      '退款政策摘要'
    );
    expect(within(logPanel).getByText('协议')).toBeInTheDocument();
    expect(within(logPanel).queryByText('节点数')).not.toBeInTheDocument();

    fireEvent.click(within(logPanel).getByRole('tab', { name: '追踪' }));
    await waitFor(() => {
      expect(runtimeApi.fetchApplicationRunTraceTree).toHaveBeenCalledWith(
        'app-1',
        'run-1'
      );
    });
    expect(
      runtimeApi.fetchApplicationRunTraceNodeContent
    ).not.toHaveBeenCalled();

    fireEvent.click(
      await within(logPanel).findByRole('button', { name: /LLM.*llm/ })
    );
    const nodeDetail = await within(logPanel).findByRole('region', {
      name: 'LLM 节点详情'
    });
    expect(
      within(nodeDetail).queryByRole('button', { name: '详情' })
    ).not.toBeInTheDocument();
    await waitFor(() => {
      expect(
        runtimeApi.fetchApplicationRunTraceNodeContent
      ).toHaveBeenCalledWith('app-1', 'run-1', 'node_run:node-run-1');
    });
    await waitFor(() => {
      expect(
        within(logPanel)
          .getAllByLabelText('输入 JSON')
          .some((element) => element.textContent?.includes('总结退款政策'))
      ).toBe(true);
    });

    fireEvent.click(screen.getByRole('button', { name: '关闭运行详情' }));

    expect(
      screen.queryByRole('complementary', { name: '运行详情' })
    ).not.toBeInTheDocument();
  }, 20_000);

  test('opens a waiting callback conversation log without active polling', async () => {
    runtimeApi.fetchApplicationRuns.mockReset();
    runtimeApi.fetchApplicationConversationMessages.mockReset();
    runtimeApi.fetchApplicationRunConversationMessages.mockReset();
    runtimeApi.fetchApplicationRuns.mockResolvedValue(
      applicationRunsPage([
        {
          id: 'run-active',
          run_mode: 'published_api_run' as const,
          status: 'waiting_callback',
          target_node_id: 'node-llm',
          title: '公开 API 工具调用',
          expand_id: 'customer-42',
          authorized_account: 'root',
          compatibility_mode: 'openai-chat-completions-v1',
          started_at: '2026-04-17T09:00:00Z',
          finished_at: null,
          created_at: '2026-04-17T09:00:00Z',
          updated_at: '2026-04-17T09:00:01Z'
        }
      ])
    );
    runtimeApi.fetchApplicationRunConversationMessages
      .mockResolvedValueOnce(
        conversationMessagesPage([
          {
            id: 'msg-run-active-user',
            flow_run_id: 'run-active',
            role: 'user',
            content: '读取 README',
            sequence: 1,
            started_at: '2026-04-17T09:00:00Z',
            finished_at: null,
            status: 'waiting_callback'
          },
          {
            id: 'msg-run-active-assistant',
            flow_run_id: 'run-active',
            role: 'assistant',
            content: '等待工具结果',
            sequence: 2,
            started_at: '2026-04-17T09:00:00Z',
            finished_at: null,
            status: 'waiting_callback'
          }
        ])
      )
      .mockResolvedValue(
        conversationMessagesPage([
          {
            id: 'msg-run-active-user',
            flow_run_id: 'run-active',
            role: 'user',
            content: '读取 README',
            sequence: 1,
            started_at: '2026-04-17T09:00:00Z',
            finished_at: '2026-04-17T09:00:05Z',
            status: 'succeeded'
          },
          {
            id: 'msg-run-active-assistant',
            flow_run_id: 'run-active',
            role: 'assistant',
            content: '最终回答',
            sequence: 2,
            started_at: '2026-04-17T09:00:00Z',
            finished_at: '2026-04-17T09:00:05Z',
            status: 'succeeded'
          }
        ])
      );

    render(
      <AppProviders>
        <ApplicationLogsPage applicationId="app-1" />
      </AppProviders>
    );

    expect(await screen.findByText('公开 API 工具调用')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: '查看运行详情' }));

    expect(
      runtimeApi.fetchApplicationRunConversationMessages.mock.calls.length
    ).toBe(1);
    expect(await screen.findByText('等待工具结果')).toBeInTheDocument();
    await act(async () => {
      await new Promise((resolve) => window.setTimeout(resolve, 1200));
    });
    expect(screen.queryByText('最终回答')).not.toBeInTheDocument();
    expect(
      runtimeApi.fetchApplicationRunConversationMessages.mock.calls.length
    ).toBe(1);
  }, 8_000);

  test('drags and resizes floating run detail window', async () => {
    innerWidthSpy = vi.spyOn(window, 'innerWidth', 'get').mockReturnValue(1280);
    innerHeightSpy = vi
      .spyOn(window, 'innerHeight', 'get')
      .mockReturnValue(900);

    render(
      <AppProviders>
        <ApplicationLogsPage applicationId="app-1" />
      </AppProviders>
    );

    expect((await screen.findAllByRole('table')).length).toBeGreaterThan(0);
    fireEvent.click(screen.getByRole('button', { name: '查看运行详情' }));

    const detailWindow = await screen.findByTestId(
      'application-logs-floating-run-detail'
    );
    expect(detailWindow).toHaveStyle({
      left: '888px',
      top: '112px',
      width: '360px',
      height: '720px'
    });

    fireEvent.mouseDown(within(detailWindow).getByText('运行详情'), {
      button: 0,
      clientX: 980,
      clientY: 130
    });
    fireEvent.mouseMove(window, {
      clientX: 880,
      clientY: 190
    });
    fireEvent.mouseUp(window);

    expect(detailWindow).toHaveStyle({
      left: '788px',
      top: '172px'
    });

    fireEvent.mouseDown(
      within(detailWindow).getByRole('separator', {
        name: '从右侧调整运行详情宽度'
      }),
      {
        button: 0,
        clientX: 1148,
        clientY: 240
      }
    );
    fireEvent.mouseMove(window, {
      clientX: 1218,
      clientY: 240
    });
    fireEvent.mouseUp(window);

    expect(detailWindow).toHaveStyle({ width: '430px' });
    expect(
      window.localStorage.getItem(
        'applicationLogsFloatingWindowWidth:application-logs-floating-run-detail'
      )
    ).toBe('430');

    fireEvent.mouseDown(
      within(detailWindow).getByRole('separator', {
        name: '从左侧调整运行详情宽度'
      }),
      {
        button: 0,
        clientX: 788,
        clientY: 240
      }
    );
    fireEvent.mouseMove(window, {
      clientX: 728,
      clientY: 240
    });
    fireEvent.mouseUp(window);

    expect(detailWindow).toHaveStyle({
      left: '728px',
      width: '490px'
    });
    expect(
      window.localStorage.getItem(
        'applicationLogsFloatingWindowWidth:application-logs-floating-run-detail'
      )
    ).toBe('490');

    fireEvent.mouseDown(
      within(detailWindow).getByRole('separator', {
        name: '向下调整运行详情高度'
      }),
      {
        button: 0,
        clientX: 840,
        clientY: 892
      }
    );
    fireEvent.mouseMove(window, {
      clientX: 840,
      clientY: 820
    });
    fireEvent.mouseUp(window);

    expect(detailWindow).toHaveStyle({ height: '648px' });
    expect(
      await screen.findByTestId('debug-conversation-messages')
    ).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: '关闭运行详情' }));
    fireEvent.click(screen.getByRole('button', { name: '查看运行详情' }));

    expect(
      await screen.findByTestId('application-logs-floating-run-detail')
    ).toHaveStyle({
      left: '758px',
      width: '490px'
    });
  }, 20_000);

  test('lets a floating window move past the viewport bottom while keeping its header reachable', async () => {
    innerWidthSpy = vi.spyOn(window, 'innerWidth', 'get').mockReturnValue(1280);
    innerHeightSpy = vi
      .spyOn(window, 'innerHeight', 'get')
      .mockReturnValue(900);

    render(
      <AppProviders>
        <ApplicationLogsPage applicationId="app-1" />
      </AppProviders>
    );

    expect((await screen.findAllByRole('table')).length).toBeGreaterThan(0);
    fireEvent.click(screen.getByRole('button', { name: '查看运行详情' }));

    const detailWindow = await screen.findByTestId(
      'application-logs-floating-run-detail'
    );
    expect(detailWindow).toHaveStyle({
      top: '112px',
      height: '720px'
    });

    fireEvent.mouseDown(within(detailWindow).getByText('运行详情'), {
      button: 0,
      clientX: 980,
      clientY: 130
    });
    fireEvent.mouseMove(window, {
      clientX: 980,
      clientY: 1100
    });
    fireEvent.mouseUp(window);

    expect(detailWindow).toHaveStyle({
      left: '888px',
      top: '852px',
      height: '720px'
    });
  }, 20_000);

  test('lets opened floating windows move independently after initial placement', async () => {
    innerWidthSpy = vi.spyOn(window, 'innerWidth', 'get').mockReturnValue(1280);
    innerHeightSpy = vi
      .spyOn(window, 'innerHeight', 'get')
      .mockReturnValue(900);

    render(
      <AppProviders>
        <ApplicationLogsPage applicationId="app-1" />
      </AppProviders>
    );

    expect((await screen.findAllByRole('table')).length).toBeGreaterThan(0);
    fireEvent.click(screen.getByRole('button', { name: '查看运行详情' }));

    const runDetailWindow = await screen.findByTestId(
      'application-logs-floating-run-detail'
    );
    const openLogButton = lastElement(
      await screen.findAllByRole(
        'button',
        { name: '查看对话日志' },
        { timeout: 8_000 }
      ),
      'expected conversation log button'
    );
    fireEvent.click(openLogButton);

    const conversationLogWindow = await screen.findByTestId(
      'application-logs-floating-conversation-log'
    );

    expect(runDetailWindow).toHaveStyle({
      left: '888px',
      top: '112px',
      width: '360px'
    });
    expect(conversationLogWindow).toHaveStyle({
      left: '512px',
      top: '112px',
      width: '360px'
    });

    fireEvent.mouseDown(within(conversationLogWindow).getByText('对话日志'), {
      button: 0,
      clientX: 560,
      clientY: 130
    });
    fireEvent.mouseMove(window, {
      clientX: 740,
      clientY: 170
    });
    fireEvent.mouseUp(window);

    expect(conversationLogWindow).toHaveStyle({
      left: '692px',
      top: '152px'
    });
    expect(runDetailWindow).toHaveStyle({
      left: '888px',
      top: '112px'
    });
  }, 20_000);

  test('opens resume timeline from run detail without covering existing floating windows', async () => {
    innerWidthSpy = vi.spyOn(window, 'innerWidth', 'get').mockReturnValue(1280);
    innerHeightSpy = vi
      .spyOn(window, 'innerHeight', 'get')
      .mockReturnValue(900);
    runtimeApi.fetchApplicationRunResumeTimeline.mockResolvedValue({
      flow_run: sampleRunDetail().flow_run,
      callback_tasks: [
        {
          id: 'callback-1',
          flow_run_id: 'run-1',
          node_run_id: 'node-run-1',
          callback_kind: 'llm_tool_calls',
          status: 'completed',
          request_payload: {},
          response_payload: {},
          external_ref_payload: null,
          created_at: '2026-04-17T09:00:01Z',
          completed_at: '2026-04-17T09:00:02Z'
        }
      ],
      events: [
        ...sampleRunDetail().events,
        {
          id: 'event-resume-requested',
          flow_run_id: 'run-1',
          node_run_id: 'node-run-1',
          sequence: 3,
          event_type: 'public_run_resume_requested',
          payload: {
            callback_task_id: 'callback-1'
          },
          created_at: '2026-04-17T09:00:01Z'
        }
      ]
    });

    render(
      <AppProviders>
        <ApplicationLogsPage applicationId="app-1" />
      </AppProviders>
    );

    expect((await screen.findAllByRole('table')).length).toBeGreaterThan(0);
    fireEvent.click(screen.getByRole('button', { name: '查看运行详情' }));

    const runDetailWindow = await screen.findByTestId(
      'application-logs-floating-run-detail'
    );
    const openLogButton = lastElement(
      await screen.findAllByRole(
        'button',
        { name: '查看对话日志' },
        { timeout: 8_000 }
      ),
      'expected conversation log button'
    );
    fireEvent.click(openLogButton);

    const conversationLogWindow = await screen.findByTestId(
      'application-logs-floating-conversation-log'
    );
    expect(conversationLogWindow).toHaveStyle({
      left: '512px',
      top: '112px',
      width: '360px'
    });

    const openResumeTimelineButton = lastElement(
      await screen.findAllByRole(
        'button',
        { name: '查看 Resume 时间线' },
        { timeout: 8_000 }
      ),
      'expected resume timeline button'
    );
    fireEvent.click(openResumeTimelineButton);

    const resumeTimelineWindow = await screen.findByTestId(
      'application-logs-floating-resume-timeline'
    );
    expect(runDetailWindow).toHaveStyle({
      left: '888px',
      top: '112px',
      width: '360px'
    });
    expect(conversationLogWindow).toHaveStyle({
      left: '512px',
      top: '112px',
      width: '360px'
    });
    expect(resumeTimelineWindow).toHaveStyle({
      left: '136px',
      top: '112px',
      width: '360px'
    });
    expect(
      await screen.findByRole('complementary', { name: 'Resume 时间线' })
    ).toBeInTheDocument();
    expect(screen.getByText('Resume 请求已接收')).toBeInTheDocument();
    expect(screen.getByText('工具调用回调')).toBeInTheDocument();
    expect(runtimeApi.fetchApplicationRunResumeTimeline).toHaveBeenCalledWith(
      'app-1',
      'run-1'
    );
  }, 20_000);

  test('opens resume timeline for the clicked historical conversation message run', async () => {
    runtimeApi.fetchApplicationRunConversationMessages.mockResolvedValue(
      conversationMessagesPage([
        {
          id: 'msg-run-0-assistant',
          flow_run_id: 'run-0',
          role: 'assistant',
          content: '上一轮回答',
          sequence: 1,
          started_at: '2026-04-17T08:59:00Z',
          finished_at: '2026-04-17T08:59:01Z'
        },
        {
          id: 'msg-run-1-assistant',
          flow_run_id: 'run-1',
          role: 'assistant',
          content: '退款政策摘要',
          sequence: 2,
          started_at: '2026-04-17T09:00:00Z',
          finished_at: '2026-04-17T09:00:01Z'
        }
      ])
    );

    render(
      <AppProviders>
        <ApplicationLogsPage applicationId="app-1" />
      </AppProviders>
    );

    expect((await screen.findAllByRole('table')).length).toBeGreaterThan(0);
    fireEvent.click(screen.getByRole('button', { name: '查看运行详情' }));

    const conversation = await screen.findByTestId(
      'debug-conversation-messages'
    );
    const openResumeTimelineButtons = await within(conversation).findAllByRole(
      'button',
      { name: '查看 Resume 时间线' },
      { timeout: 8_000 }
    );
    fireEvent.click(openResumeTimelineButtons[0]);

    await screen.findByTestId('application-logs-floating-resume-timeline');
    await waitFor(() => {
      expect(runtimeApi.fetchApplicationRunResumeTimeline).toHaveBeenCalledWith(
        'app-1',
        'run-0'
      );
    });
  }, 20_000);

  test('keeps the runs table layout unchanged while floating windows are open', async () => {
    innerHeightSpy = vi
      .spyOn(window, 'innerHeight', 'get')
      .mockReturnValue(920);
    getBoundingClientRectSpy = vi
      .spyOn(HTMLElement.prototype, 'getBoundingClientRect')
      .mockImplementation(function getBoundingClientRect(this: HTMLElement) {
        if (this.classList.contains('application-logs-page__list')) {
          return {
            bottom: 120,
            height: 0,
            left: 0,
            right: 0,
            top: 120,
            width: 1200,
            x: 0,
            y: 120,
            toJSON: () => ({})
          };
        }

        if (this.classList.contains('ant-table-thead')) {
          return {
            bottom: 176,
            height: 56,
            left: 0,
            right: 0,
            top: 120,
            width: 900,
            x: 0,
            y: 120,
            toJSON: () => ({})
          };
        }

        if (this.classList.contains('ant-table-wrapper')) {
          return {
            bottom: 760,
            height: 640,
            left: 0,
            right: 0,
            top: 120,
            width: 900,
            x: 0,
            y: 120,
            toJSON: () => ({})
          };
        }

        return {
          bottom: 0,
          height: 0,
          left: 0,
          right: 0,
          top: 0,
          width: 0,
          x: 0,
          y: 0,
          toJSON: () => ({})
        };
      });

    render(
      <AppProviders>
        <ApplicationLogsPage applicationId="app-1" />
      </AppProviders>
    );

    expect((await screen.findAllByRole('table')).length).toBeGreaterThan(0);

    fireEvent.click(screen.getByRole('button', { name: '查看运行详情' }));

    expect(
      await screen.findByRole('complementary', { name: '运行详情' })
    ).toBeInTheDocument();
    expect(screen.getByTestId('application-logs-page')).not.toHaveClass(
      'application-logs-page--detail-open'
    );
    expect(screen.getByTestId('application-logs-list')).not.toHaveAttribute(
      'style'
    );
  });
});
