import {
  fireEvent,
  render,
  screen,
  waitFor,
  within
} from '@testing-library/react';
import { readFile } from 'node:fs/promises';
import path from 'node:path';
import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';

const runtimeApi = vi.hoisted(() => ({
  applicationRunsQueryKey: (
    applicationId: string,
    input?: {
      page?: number;
      pageSize?: number;
      timeRangeDays?: number | null;
      sortBy?: 'started_at' | 'finished_at' | 'created_at';
      sortOrder?: 'asc' | 'desc';
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
  applicationRunDetailQueryKey: (applicationId: string, runId: string) =>
    ['applications', applicationId, 'runtime', 'runs', runId] as const,
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
  fetchApplicationRuns: vi.fn(),
  fetchApplicationRunDetail: vi.fn(),
  fetchApplicationConversationMessages: vi.fn(),
  fetchRuntimeDebugArtifact: vi.fn(),
  resumeFlowRun: vi.fn(),
  completeCallbackTask: vi.fn()
}));

vi.mock('../api/runtime', () => runtimeApi);

import type { ApplicationRunDetail } from '../api/runtime';
import { AppProviders } from '../../../app/AppProviders';
import { resetAuthStore, useAuthStore } from '../../../state/auth-store';
import { ApplicationLogsPage } from '../pages/ApplicationLogsPage';

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

describe('ApplicationLogsPage', () => {
  let getBoundingClientRectSpy: { mockRestore: () => void } | undefined;
  let innerHeightSpy: { mockRestore: () => void } | undefined;
  let innerWidthSpy: { mockRestore: () => void } | undefined;
  let dateNowSpy: { mockRestore: () => void } | undefined;

  beforeEach(() => {
    window.localStorage.clear();
    dateNowSpy = vi
      .spyOn(Date, 'now')
      .mockReturnValue(new Date('2026-04-18T00:00:00Z').getTime());
    runtimeApi.fetchApplicationRuns.mockReset();
    runtimeApi.fetchApplicationRunDetail.mockReset();
    runtimeApi.fetchApplicationConversationMessages.mockReset();
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
          started_at: '2026-04-17T09:00:00Z',
          finished_at: '2026-04-17T09:00:01Z',
          created_at: '2026-04-17T09:00:00Z',
          updated_at: '2026-04-17T09:00:01Z'
        }
      ])
    );
    runtimeApi.fetchApplicationRunDetail.mockResolvedValue(sampleRunDetail());
    runtimeApi.fetchApplicationConversationMessages.mockResolvedValue({
      items: [
        {
          run_id: 'run-0',
          detail_run_id: 'run-0',
          can_open_detail: true,
          started_at: '2026-04-17T08:59:00Z',
          finished_at: '2026-04-17T08:59:01Z',
          status: 'succeeded',
          query: '上一轮问题',
          model: 'deepseek-chat',
          answer: '上一轮回答',
          is_current: false
        },
        {
          run_id: 'run-1',
          detail_run_id: 'run-1',
          can_open_detail: true,
          started_at: '2026-04-17T09:00:00Z',
          finished_at: '2026-04-17T09:00:01Z',
          status: 'succeeded',
          query: '总结退款政策',
          model: 'deepseek-chat',
          answer: '退款政策摘要',
          is_current: true
        }
      ],
      page: {
        has_before: false,
        has_after: false,
        before_cursor: 'run-0',
        after_cursor: 'run-1'
      }
    });
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
        runtimeApi.fetchApplicationConversationMessages
      ).toHaveBeenCalledWith('app-1', 'run-1', {
        limit: 5
      });
    });
    expect(runtimeApi.fetchApplicationRunDetail).not.toHaveBeenCalled();
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
    expect(within(conversation).getAllByText('User')).toHaveLength(2);
    expect(within(conversation).getByText('上一轮问题')).toBeInTheDocument();
    expect(within(conversation).getByText('上一轮回答')).toBeInTheDocument();
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

    const openLogButton = screen
      .getAllByRole('button', {
        name: '查看对话日志'
      })
      .at(-1);
    expect(openLogButton).toBeDefined();
    fireEvent.click(openLogButton!);

    await waitFor(() => {
      expect(runtimeApi.fetchApplicationRunDetail).toHaveBeenCalledWith(
        'app-1',
        'run-1'
      );
    });

    const logPanel = screen.getByRole('complementary', { name: '对话日志' });
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
    expect(within(logPanel).getByLabelText('输入 JSON')).toHaveTextContent(
      'user_prompt'
    );
    expect(within(logPanel).getByLabelText('输出 JSON')).toHaveTextContent(
      '退款政策摘要'
    );
    expect(within(logPanel).getByText('协议')).toBeInTheDocument();
    expect(within(logPanel).getByText('OpenAI Responses')).toBeInTheDocument();

    fireEvent.click(within(logPanel).getByRole('tab', { name: '追踪' }));
    const logTraceNode = within(logPanel).getByRole('button', { name: /LLM/ });
    fireEvent.click(logTraceNode);
    expect(logTraceNode).toHaveAttribute('aria-expanded', 'true');
    expect(
      within(logPanel).getByRole('region', { name: 'LLM 节点详情' })
    ).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: '关闭运行详情' }));

    expect(
      screen.queryByRole('complementary', { name: '运行详情' })
    ).not.toBeInTheDocument();
  }, 20_000);

  test('refreshes an active conversation log until the run reaches a terminal status', async () => {
    runtimeApi.fetchApplicationRuns.mockReset();
    runtimeApi.fetchApplicationConversationMessages.mockReset();
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
    runtimeApi.fetchApplicationConversationMessages
      .mockResolvedValueOnce({
        items: [
          {
            run_id: 'run-active',
            detail_run_id: 'run-active',
            can_open_detail: true,
            started_at: '2026-04-17T09:00:00Z',
            finished_at: null,
            status: 'waiting_callback',
            query: '读取 README',
            model: 'deepseek-chat',
            answer: '等待工具结果',
            is_current: true
          }
        ],
        page: {
          has_before: false,
          has_after: false,
          before_cursor: null,
          after_cursor: 'run-active'
        }
      })
      .mockResolvedValue({
        items: [
          {
            run_id: 'run-active',
            detail_run_id: 'run-active',
            can_open_detail: true,
            started_at: '2026-04-17T09:00:00Z',
            finished_at: '2026-04-17T09:00:05Z',
            status: 'succeeded',
            query: '读取 README',
            model: 'deepseek-chat',
            answer: '最终回答',
            is_current: true
          }
        ],
        page: {
          has_before: false,
          has_after: false,
          before_cursor: null,
          after_cursor: 'run-active'
        }
      });

    render(
      <AppProviders>
        <ApplicationLogsPage applicationId="app-1" />
      </AppProviders>
    );

    expect(await screen.findByText('公开 API 工具调用')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: '查看运行详情' }));

    expect(await screen.findByText('等待工具结果')).toBeInTheDocument();
    await waitFor(
      () => {
        expect(screen.getByText('最终回答')).toBeInTheDocument();
      },
      { timeout: 4_000 }
    );
    expect(
      runtimeApi.fetchApplicationConversationMessages.mock.calls.length
    ).toBeGreaterThanOrEqual(2);
  }, 8_000);

  test('loads conversation log detail and trace artifacts from application logs', async () => {
    const detail = sampleRunDetail();
    detail.flow_run.output_payload = {
      answer: {
        __runtime_debug_artifact: true,
        artifact_scope: 'field',
        field_path: ['answer'],
        is_truncated: true,
        original_size_bytes: 4096,
        preview_size_bytes: 128,
        content_type: 'application/json',
        artifact_ref: 'artifact-detail-answer',
        preview: '详情截断回答'
      }
    };
    detail.node_runs[0]!.output_payload = {
      answer: {
        __runtime_debug_artifact: true,
        artifact_scope: 'field',
        field_path: ['answer'],
        is_truncated: true,
        original_size_bytes: 4096,
        preview_size_bytes: 128,
        content_type: 'application/json',
        artifact_ref: 'artifact-trace-answer',
        preview: '追踪截断回答'
      },
      rendered_templates: {}
    };
    runtimeApi.fetchApplicationRunDetail.mockResolvedValue(detail);
    runtimeApi.fetchRuntimeDebugArtifact.mockImplementation(
      async (_applicationId: string, artifactRef: string) => {
        if (artifactRef === 'artifact-detail-answer') {
          return '详情完整回答';
        }

        if (artifactRef === 'artifact-trace-answer') {
          return '追踪完整回答';
        }

        throw new Error(`unexpected artifact: ${artifactRef}`);
      }
    );

    render(
      <AppProviders>
        <ApplicationLogsPage applicationId="app-1" />
      </AppProviders>
    );

    expect(await screen.findByText('run-1')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: '查看运行详情' }));

    const openLogButton = (
      await screen.findAllByRole('button', { name: '查看对话日志' })
    ).at(-1);
    expect(openLogButton).toBeDefined();
    fireEvent.click(openLogButton!);

    const logPanel = await screen.findByRole('complementary', {
      name: '对话日志'
    });
    const detailLoadButton = within(logPanel).getByRole('button', {
      name: '加载完整值'
    });
    expect(detailLoadButton).toBeEnabled();
    fireEvent.click(detailLoadButton);

    expect(runtimeApi.fetchRuntimeDebugArtifact).toHaveBeenCalledWith(
      'app-1',
      'artifact-detail-answer'
    );
    await waitFor(() =>
      expect(within(logPanel).getByLabelText('输出 JSON')).toHaveTextContent(
        '详情完整回答'
      )
    );

    fireEvent.click(within(logPanel).getByRole('tab', { name: '追踪' }));
    fireEvent.click(within(logPanel).getByRole('button', { name: /LLM/ }));
    const traceLoadButton = within(logPanel).getByRole('button', {
      name: '加载完整值'
    });
    expect(traceLoadButton).toBeEnabled();
    fireEvent.click(traceLoadButton);

    expect(runtimeApi.fetchRuntimeDebugArtifact).toHaveBeenCalledWith(
      'app-1',
      'artifact-trace-answer'
    );
    await waitFor(() =>
      expect(
        within(logPanel)
          .getAllByLabelText('输出 JSON')
          .some((element) => element.textContent?.includes('追踪完整回答'))
      ).toBe(true)
    );
  }, 20_000);

  test('groups repeated LLM tool callbacks under Tools from application logs', async () => {
    const detail = sampleRunDetail();
    const llmNodeRun = detail.node_runs[0]!;
    detail.flow_run.status = 'waiting_callback';
    detail.node_runs = [
      {
        ...llmNodeRun,
        id: 'node-run-llm-1',
        status: 'succeeded',
        output_payload: {
          usage: {
            total_tokens: 8035
          }
        },
        debug_payload: {
          llm_rounds: [
            {
              round_index: 0,
              assistant: {
                role: 'assistant',
                content: 'need weather',
                tool_calls: [
                  {
                    id: 'call_weather',
                    name: 'lookup_weather'
                  }
                ]
              }
            }
          ]
        },
        started_at: '2026-04-17T09:00:00Z',
        finished_at: '2026-04-17T09:00:03Z'
      },
      {
        ...llmNodeRun,
        id: 'node-run-llm-2',
        status: 'waiting_callback',
        output_payload: {
          tool_calls: [
            {
              id: 'call_policy'
            }
          ]
        },
        debug_payload: {
          llm_rounds: [
            {
              round_index: 1,
              assistant: {
                role: 'assistant',
                content: 'need policy',
                tool_calls: [
                  {
                    id: 'call_policy',
                    name: 'read_policy'
                  }
                ]
              }
            }
          ]
        },
        started_at: '2026-04-17T09:00:04Z',
        finished_at: null
      }
    ];
    runtimeApi.fetchApplicationRunDetail.mockResolvedValue(detail);

    render(
      <AppProviders>
        <ApplicationLogsPage applicationId="app-1" />
      </AppProviders>
    );

    expect(await screen.findByText('run-1')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: '查看运行详情' }));

    const openLogButton = (
      await screen.findAllByRole('button', { name: '查看对话日志' })
    ).at(-1);
    expect(openLogButton).toBeDefined();
    fireEvent.click(openLogButton!);

    const logPanel = await screen.findByRole('complementary', {
      name: '对话日志'
    });
    fireEvent.click(within(logPanel).getByRole('tab', { name: '追踪' }));

    expect(
      within(logPanel).getAllByTestId('debug-workflow-node-row')
    ).toHaveLength(1);

    const llmTraceNode = within(logPanel).getByRole('button', { name: /LLM/ });
    expect(llmTraceNode).toHaveTextContent('工具 2');
    fireEvent.click(llmTraceNode);

    const toolsNode = within(logPanel).getByRole('button', {
      name: /Tools.*2 次工具回调/
    });
    expect(toolsNode).toHaveAttribute('aria-expanded', 'false');
    expect(within(logPanel).queryByText('call_weather')).not.toBeInTheDocument();

    fireEvent.click(toolsNode);
    const toolIndex = within(logPanel).getByLabelText('工具回调索引 JSON');
    expect(toolIndex).toHaveTextContent('call_weather');
    expect(toolIndex).toHaveTextContent('call_policy');
    expect(
      within(logPanel).getByRole('button', {
        name: /lookup_weather.*call_weather/
      })
    ).toBeInTheDocument();
    expect(
      within(logPanel).getByRole('button', {
        name: /read_policy.*call_policy/
      })
    ).toBeInTheDocument();
  }, 20_000);

  test('does not offer run log details for imported context messages', async () => {
    runtimeApi.fetchApplicationConversationMessages.mockResolvedValue({
      items: [
        {
          run_id: 'run-1:history:0',
          detail_run_id: null,
          can_open_detail: false,
          started_at: '2026-04-17T08:59:00Z',
          finished_at: '2026-04-17T08:59:01Z',
          status: 'succeeded',
          query: '外部传入的问题',
          model: 'deepseek-chat',
          answer: '外部传入的回答',
          is_current: false
        },
        {
          run_id: 'run-1',
          detail_run_id: 'run-1',
          can_open_detail: true,
          started_at: '2026-04-17T09:00:00Z',
          finished_at: '2026-04-17T09:00:01Z',
          status: 'succeeded',
          query: '总结退款政策',
          model: 'deepseek-chat',
          answer: '退款政策摘要',
          is_current: true
        }
      ],
      page: {
        has_before: false,
        has_after: false,
        before_cursor: null,
        after_cursor: 'run-1'
      }
    });

    render(
      <AppProviders>
        <ApplicationLogsPage applicationId="app-1" />
      </AppProviders>
    );

    expect(await screen.findByText('run-1')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: '查看运行详情' }));

    const conversation = await screen.findByTestId(
      'debug-conversation-messages'
    );
    expect(
      await within(conversation).findByText('外部传入的问题')
    ).toBeInTheDocument();
    expect(
      within(conversation).getByText('外部传入的回答')
    ).toBeInTheDocument();
    expect(
      within(conversation).getAllByRole('button', {
        name: '查看对话日志'
      })
    ).toHaveLength(1);
  }, 20_000);

  test('persists table column visibility in user preferences meta', async () => {
    useAuthStore.getState().setAuthenticated({
      csrfToken: 'csrf-123',
      actor: {
        id: 'user-1',
        account: 'root',
        effective_display_role: 'root',
        current_workspace_id: 'workspace-1'
      },
      me: {
        id: 'user-1',
        account: 'root',
        email: 'root@example.com',
        phone: null,
        nickname: 'Root',
        name: 'Root',
        avatar_url: null,
        introduction: '',
        preferred_locale: null,
        effective_display_role: 'root',
        permissions: [],
        meta: {}
      }
    });
    const fetchMock = vi.spyOn(globalThis, 'fetch').mockResolvedValue(
      new Response(
        JSON.stringify({
          data: {
            id: 'user-1',
            account: 'root',
            email: 'root@example.com',
            phone: null,
            nickname: 'Root',
            name: 'Root',
            avatar_url: null,
            introduction: '',
            preferred_locale: null,
            effective_display_role: 'root',
            permissions: [],
            meta: {
              ui: {
                data_tables: {
                  'applications.logs.runs': {
                    visibleColumnKeys: [
                      'title',
                      'status',
                      'run_mode',
                      'authorized_account',
                      'started_at',
                      'duration',
                      'action'
                    ],
                    columnWidths: {}
                  }
                }
              }
            }
          },
          meta: null
        }),
        {
          status: 200,
          headers: { 'content-type': 'application/json' }
        }
      )
    );
    const { unmount } = render(
      <AppProviders>
        <ApplicationLogsPage applicationId="app-1" />
      </AppProviders>
    );

    expect((await screen.findAllByRole('table')).length).toBeGreaterThan(0);
    expect(
      screen.getByRole('columnheader', {
        name: 'expand_id'
      })
    ).toBeInTheDocument();

    fireEvent.mouseDown(screen.getByRole('combobox', { name: '字段配置' }));
    fireEvent.click(
      await screen.findByText('expand_id', {
        selector: '.ant-select-item-option-content'
      })
    );

    await waitFor(() => {
      const meta = useAuthStore.getState().me?.meta as
        | {
            ui?: {
              data_tables?: {
                'applications.logs.runs'?: {
                  visibleColumnKeys?: string[];
                };
              };
            };
          }
        | undefined;

      expect(fetchMock).toHaveBeenCalledWith(
        expect.stringContaining('/api/console/me/meta'),
        expect.objectContaining({
          method: 'PATCH',
          headers: expect.objectContaining({
            'x-csrf-token': 'csrf-123'
          }),
          body: expect.stringContaining('"applications.logs.runs"')
        })
      );
      expect(
        meta?.ui?.data_tables?.['applications.logs.runs']?.visibleColumnKeys
      ).not.toContain('expand_id');
    });

    unmount();
    render(
      <AppProviders>
        <ApplicationLogsPage applicationId="app-1" />
      </AppProviders>
    );

    await waitFor(() => {
      expect(
        screen.queryByRole('columnheader', {
          name: 'expand_id'
        })
      ).not.toBeInTheDocument();
    });
    fetchMock.mockRestore();
  });

  test('places table field configuration with the filters', async () => {
    render(
      <AppProviders>
        <ApplicationLogsPage applicationId="app-1" />
      </AppProviders>
    );

    expect((await screen.findAllByRole('table')).length).toBeGreaterThan(0);
    const filters = screen.getByRole('search');

    expect(
      within(filters).getByRole('combobox', { name: '字段配置' })
    ).toBeInTheDocument();
  });

  test('sizes log filter selects from their longest option label', async () => {
    render(
      <AppProviders>
        <ApplicationLogsPage applicationId="app-1" />
      </AppProviders>
    );

    expect((await screen.findAllByRole('table')).length).toBeGreaterThan(0);

    const expectedMeasuredLabels = [
      [
        '时间间隔',
        [
          '今天',
          '过去 7 天',
          '过去 4 周',
          '过去 3 月',
          '过去 12 月',
          '所有时间'
        ]
      ],
      ['排序字段', ['排序：开始时间', '排序：更新时间']]
    ] as const;

    expectedMeasuredLabels.forEach(([ariaLabel, measuredLabels]) => {
      expect(
        screen.getByRole('combobox', { name: ariaLabel })
      ).toBeInTheDocument();

      measuredLabels.forEach((label) => {
        // Hidden measurement spans are intentionally aria-hidden and have no text content.
        // eslint-disable-next-line testing-library/no-node-access
        const measureItem = document.querySelector(
          `.autosize-select__measure-item[data-measure-label="${label}"]`
        );

        expect(measureItem).toBeInTheDocument();
      });
    });

    const cssSource = await readFile(
      path.resolve(
        process.cwd(),
        'src/shared/ui/autosize-select/autosize-select.css'
      ),
      'utf8'
    );

    expect(cssSource).toContain('.autosize-select {');
    expect(cssSource).toContain('grid-template-columns: max-content;');
    expect(cssSource).toContain('.autosize-select__control {');
    expect(cssSource).toContain('width: 100%;');
  });

  test('combines run sort field and direction into one sort control', async () => {
    render(
      <AppProviders>
        <ApplicationLogsPage applicationId="app-1" />
      </AppProviders>
    );

    expect((await screen.findAllByRole('table')).length).toBeGreaterThan(0);

    const filters = screen.getByRole('search');
    const sortControl = within(filters).getByTestId(
      'application-logs-sort-control'
    );

    expect(
      within(sortControl).getByRole('combobox', { name: '排序字段' })
    ).toBeInTheDocument();
    expect(within(sortControl).getByText('排序：')).toBeInTheDocument();
    expect(
      within(sortControl).getByRole('button', {
        name: '当前降序，切换为升序'
      })
    ).toBeInTheDocument();
    expect(
      within(filters).queryByRole('combobox', { name: '排序方向' })
    ).not.toBeInTheDocument();
  });

  test('toggles run sort direction from the merged sort control', async () => {
    render(
      <AppProviders>
        <ApplicationLogsPage applicationId="app-1" />
      </AppProviders>
    );

    expect((await screen.findAllByRole('table')).length).toBeGreaterThan(0);

    fireEvent.click(
      screen.getByRole('button', { name: '当前降序，切换为升序' })
    );

    await waitFor(() => {
      expect(runtimeApi.fetchApplicationRuns).toHaveBeenLastCalledWith(
        'app-1',
        {
          page: 1,
          pageSize: 20,
          timeRangeDays: 7,
          sortBy: 'started_at',
          sortOrder: 'asc'
        }
      );
    });
    expect(
      await screen.findByRole('button', { name: '当前升序，切换为降序' })
    ).toBeInTheDocument();
  });

  test('refetches runs when selecting a different sort field', async () => {
    render(
      <AppProviders>
        <ApplicationLogsPage applicationId="app-1" />
      </AppProviders>
    );

    expect((await screen.findAllByRole('table')).length).toBeGreaterThan(0);

    fireEvent.mouseDown(screen.getByRole('combobox', { name: '排序字段' }));
    fireEvent.click(
      await screen.findByText('更新时间', {
        selector: '.ant-select-item-option-content'
      })
    );

    await waitFor(() => {
      expect(runtimeApi.fetchApplicationRuns).toHaveBeenLastCalledWith(
        'app-1',
        {
          page: 1,
          pageSize: 20,
          timeRangeDays: 7,
          sortBy: 'updated_at',
          sortOrder: 'desc'
        }
      );
    });
  });

  test('renders table field configuration with Ant Design multiple select', async () => {
    render(
      <AppProviders>
        <ApplicationLogsPage applicationId="app-1" />
      </AppProviders>
    );

    expect((await screen.findAllByRole('table')).length).toBeGreaterThan(0);

    const trigger = screen.getByRole('combobox', { name: '字段配置' });

    expect(trigger).toHaveAttribute('aria-haspopup', 'listbox');
    expect(
      screen.queryByText('字段配置', {
        selector: '.application-runs-table__column-selector-trigger-caret'
      })
    ).not.toBeInTheDocument();
  });

  test('keeps table field configuration as a native responsive multiple select', async () => {
    const tableSource = await readFile(
      path.resolve(process.cwd(), 'src/shared/ui/data-table/DataTable.tsx'),
      'utf8'
    );

    expect(tableSource).toContain('mode="multiple"');
    expect(tableSource).toContain('maxTagCount="responsive"');
    expect(tableSource).toContain('popupMatchSelectWidth');
    expect(tableSource).not.toContain('maxTagCount={0}');
    expect(tableSource).not.toContain("maxTagPlaceholder={() => '字段配置'}");
  });

  test('opens table field configuration as a dropdown menu', async () => {
    render(
      <AppProviders>
        <ApplicationLogsPage applicationId="app-1" />
      </AppProviders>
    );

    expect((await screen.findAllByRole('table')).length).toBeGreaterThan(0);

    fireEvent.mouseDown(screen.getByRole('combobox', { name: '字段配置' }));

    const fieldListbox = await screen.findByRole('listbox');

    expect(
      within(fieldListbox).getByRole('option', { name: 'expand_id' })
    ).toBeInTheDocument();
    expect(
      within(fieldListbox).getByRole('option', { name: '运行 ID' })
    ).toBeInTheDocument();
    expect(
      screen.getByRole('button', { name: '重置默认字段' })
    ).toBeInTheDocument();
  });

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

  test('filters application logs by time range and keyword within the current page', async () => {
    runtimeApi.fetchApplicationRuns
      .mockResolvedValueOnce(
        applicationRunsPage([
          {
            id: 'run-refund',
            run_mode: 'debug_flow_run' as const,
            status: 'succeeded',
            target_node_id: null,
            started_at: '2026-04-17T10:00:00Z',
            finished_at: '2026-04-17T10:05:00Z',
            created_at: '2026-04-17T10:00:00Z',
            updated_at: '2026-04-17T10:05:00Z'
          },
          {
            id: 'run-weather',
            run_mode: 'debug_flow_run' as const,
            status: 'succeeded',
            target_node_id: null,
            started_at: '2026-04-17T09:00:00Z',
            finished_at: '2026-04-17T12:00:00Z',
            created_at: '2026-04-17T09:00:00Z',
            updated_at: '2026-04-17T12:00:00Z'
          }
        ])
      )
      .mockResolvedValueOnce(
        applicationRunsPage([
          {
            id: 'run-refund',
            run_mode: 'debug_flow_run' as const,
            status: 'succeeded',
            target_node_id: null,
            started_at: '2026-04-17T10:00:00Z',
            finished_at: '2026-04-17T10:05:00Z',
            created_at: '2026-04-17T10:00:00Z',
            updated_at: '2026-04-17T10:05:00Z'
          },
          {
            id: 'run-weather',
            run_mode: 'debug_flow_run' as const,
            status: 'succeeded',
            target_node_id: null,
            started_at: '2026-04-17T09:00:00Z',
            finished_at: '2026-04-17T12:00:00Z',
            created_at: '2026-04-17T09:00:00Z',
            updated_at: '2026-04-17T12:00:00Z'
          },
          {
            id: 'run-old',
            run_mode: 'debug_flow_run' as const,
            status: 'succeeded',
            target_node_id: null,
            started_at: '2026-03-01T09:00:00Z',
            finished_at: '2026-03-01T09:02:00Z',
            created_at: '2026-03-01T09:00:00Z',
            updated_at: '2026-03-01T09:02:00Z'
          }
        ])
      );
    runtimeApi.fetchApplicationRunDetail.mockImplementation(
      async (_applicationId: string, runId: string) => {
        const detail = sampleRunDetail();

        if (runId === 'run-refund') {
          detail.flow_run.id = runId;
          detail.flow_run.query = '我想查退款规则';
          detail.flow_run.input_payload = {
            'node-start.query': '我想查退款规则'
          };
          detail.flow_run.output_payload = {
            answer: '可以在 7 天内退款',
            resolved_inputs: {
              user_prompt: '我想查退款规则'
            }
          };
          return detail;
        }

        detail.flow_run.id = runId;
        detail.flow_run.query = '今天天气怎么样';
        detail.flow_run.input_payload = {
          'node-start.query': '今天天气怎么样'
        };
        detail.flow_run.output_payload = {
          answer: '天气晴朗',
          resolved_inputs: {
            user_prompt: '今天天气怎么样'
          }
        };
        detail.node_runs = detail.node_runs.map((nodeRun) => ({
          ...nodeRun,
          input_payload: { user_prompt: '今天天气怎么样' },
          output_payload: { answer: '天气晴朗', rendered_templates: {} }
        }));
        return detail;
      }
    );

    render(
      <AppProviders>
        <ApplicationLogsPage applicationId="app-1" />
      </AppProviders>
    );

    expect(await screen.findByText('run-refund')).toBeInTheDocument();
    expect(screen.getByText('run-weather')).toBeInTheDocument();
    expect(screen.queryByText('run-old')).not.toBeInTheDocument();
    expect(
      screen.getByRole('combobox', { name: '时间间隔' })
    ).toBeInTheDocument();
    expect(
      screen.getByText('过去 7 天', {
        selector: '.ant-select-selection-item'
      })
    ).toBeInTheDocument();
    expect(
      screen.getByRole('columnheader', {
        name: '更新时间'
      })
    ).toBeInTheDocument();
    expect(screen.getByText('2026/4/17 18:05:00')).toBeInTheDocument();
    expect(screen.getByText('2026/4/17 20:00:00')).toBeInTheDocument();

    const searchInput = screen.getByPlaceholderText('搜索对话和回答');
    fireEvent.change(searchInput, { target: { value: '退款' } });

    await waitFor(() => {
      expect(screen.getByText('run-refund')).toBeInTheDocument();
      expect(screen.queryByText('run-weather')).not.toBeInTheDocument();
    });
    expect(runtimeApi.fetchApplicationRunDetail).toHaveBeenCalledWith(
      'app-1',
      'run-refund'
    );

    fireEvent.change(searchInput, { target: { value: '' } });
    fireEvent.mouseDown(screen.getByRole('combobox', { name: '时间间隔' }));
    fireEvent.click(
      await screen.findByText('所有时间', {
        selector: '.ant-select-item-option-content'
      })
    );

    await waitFor(() => {
      expect(runtimeApi.fetchApplicationRuns).toHaveBeenLastCalledWith(
        'app-1',
        {
          page: 1,
          pageSize: 20,
          timeRangeDays: null,
          sortBy: 'started_at',
          sortOrder: 'desc'
        }
      );
    });
    expect(await screen.findByText('run-old')).toBeInTheDocument();
  });

  test('requests 20 runs per page and refetches when pagination changes', async () => {
    runtimeApi.fetchApplicationRuns
      .mockResolvedValueOnce(
        applicationRunsPage(
          Array.from({ length: 20 }, (_, index) => ({
            id: `run-${index + 1}`,
            run_mode: 'debug_flow_run' as const,
            status: 'succeeded',
            target_node_id: null,
            title: `title-${index + 1}`,
            expand_id: null,
            authorized_account: 'root',
            started_at: `2026-04-17T09:${String(index).padStart(2, '0')}:00Z`,
            finished_at: `2026-04-17T09:${String(index).padStart(2, '0')}:30Z`,
            created_at: `2026-04-17T09:${String(index).padStart(2, '0')}:00Z`,
            updated_at: `2026-04-17T09:${String(index).padStart(2, '0')}:30Z`
          })),
          { total: 42, page: 1, page_size: 20 }
        )
      )
      .mockResolvedValueOnce(
        applicationRunsPage(
          Array.from({ length: 20 }, (_, index) => ({
            id: `run-${index + 21}`,
            run_mode: 'debug_flow_run' as const,
            status: 'succeeded',
            target_node_id: null,
            title: `title-${index + 21}`,
            expand_id: null,
            authorized_account: 'root',
            started_at: `2026-04-16T09:${String(index).padStart(2, '0')}:00Z`,
            finished_at: `2026-04-16T09:${String(index).padStart(2, '0')}:30Z`,
            created_at: `2026-04-16T09:${String(index).padStart(2, '0')}:00Z`,
            updated_at: `2026-04-16T09:${String(index).padStart(2, '0')}:30Z`
          })),
          { total: 42, page: 2, page_size: 20 }
        )
      );

    render(
      <AppProviders>
        <ApplicationLogsPage applicationId="app-1" />
      </AppProviders>
    );

    expect(await screen.findByText('title-1')).toBeInTheDocument();
    expect(runtimeApi.fetchApplicationRuns).toHaveBeenNthCalledWith(
      1,
      'app-1',
      {
        page: 1,
        pageSize: 20,
        timeRangeDays: 7,
        sortBy: 'started_at',
        sortOrder: 'desc'
      }
    );
    expect(screen.getByText('共 42 条')).toBeInTheDocument();

    fireEvent.click(screen.getByTitle('2'));

    await waitFor(() => {
      expect(runtimeApi.fetchApplicationRuns).toHaveBeenNthCalledWith(
        2,
        'app-1',
        {
          page: 2,
          pageSize: 20,
          timeRangeDays: 7,
          sortBy: 'started_at',
          sortOrder: 'desc'
        }
      );
    });
    expect(await screen.findByText('title-21')).toBeInTheDocument();
  });

  test('uses floating window CSS instead of a docked splitter override', async () => {
    const cssSource = await readFile(
      path.resolve(
        process.cwd(),
        'src/features/applications/pages/application-logs-page.css'
      ),
      'utf8'
    );

    expect(cssSource).not.toContain('application-logs-page--detail-open');
    expect(cssSource).not.toContain('--application-runs-table-body-height');
    expect(cssSource).toContain('flex: 1 1 auto;');
    expect(cssSource).toContain('width: 100%;');
    expect(cssSource).toContain('.application-logs-floating-window');
    expect(cssSource).toContain('position: fixed;');
    expect(cssSource).toContain(
      '.application-logs-floating-window__resize--left'
    );
    expect(cssSource).toContain('cursor: move;');
    expect(cssSource).not.toContain('position: static;');
  });

  test('pins the logs page list to the parent full-height layout instead of a nested viewport calc', async () => {
    const cssSource = await readFile(
      path.resolve(
        process.cwd(),
        'src/features/applications/pages/application-logs-page.css'
      ),
      'utf8'
    );

    expect(cssSource).not.toContain('calc(100vh - 120px)');
    expect(cssSource).toMatch(
      /\.application-logs-page\s*\{[^}]*height:\s*100%;[^}]*min-height:\s*0;[^}]*box-sizing:\s*border-box;/s
    );
    expect(cssSource).toMatch(
      /\.application-logs-page\s*\{[^}]*padding:\s*32px 0;/s
    );
    expect(cssSource).toMatch(
      /\.application-logs-page__stack\s*\{[^}]*display:\s*flex;[^}]*flex-direction:\s*column;[^}]*height:\s*100%;/s
    );
    expect(cssSource).toMatch(
      /\.application-logs-page__list\s*\{[^}]*display:\s*flex;[^}]*flex-direction:\s*column;[^}]*flex:\s*1 1 auto;[^}]*min-height:\s*0;[^}]*overflow-x:\s*hidden;[^}]*overflow-y:\s*hidden;/s
    );
  });

  test('keeps the table header and pagination fixed around the row scroll area', async () => {
    const cssSource = await readFile(
      path.resolve(process.cwd(), 'src/shared/ui/data-table/data-table.css'),
      'utf8'
    );
    const pageCssSource = await readFile(
      path.resolve(
        process.cwd(),
        'src/features/applications/pages/application-logs-page.css'
      ),
      'utf8'
    );
    const tableSource = await readFile(
      path.resolve(
        process.cwd(),
        'src/features/applications/components/logs/ApplicationRunsTable.tsx'
      ),
      'utf8'
    );

    expect(cssSource).toMatch(
      /\.data-table\s*\{[^}]*flex:\s*1 1 auto;[^}]*\}/s
    );
    expect(cssSource).toMatch(
      /\.data-table__scroll-area\s*\{[^}]*flex:\s*1 1 auto;[^}]*overflow-x:\s*auto;[^}]*overflow-y:\s*auto;[^}]*\}/s
    );
    expect(cssSource).toMatch(
      /\.data-table \.ant-table-thead > tr > th\s*\{[^}]*position:\s*sticky;[^}]*top:\s*0;[^}]*z-index:\s*2;[^}]*\}/s
    );
    expect(cssSource).toMatch(
      /\.data-table__pagination\s*\{[^}]*flex:\s*0 0 auto;[^}]*\}/s
    );
    expect(pageCssSource).toMatch(
      /\.application-logs-page__list\s*\{[^}]*overflow-y:\s*hidden;[^}]*\}/s
    );
    expect(tableSource).not.toContain("y: '100%'");
  });

  test('keeps horizontal scrolling on the runs table wrapper instead of the Ant Design body', async () => {
    const cssSource = await readFile(
      path.resolve(process.cwd(), 'src/shared/ui/data-table/data-table.css'),
      'utf8'
    );
    const tableSource = await readFile(
      path.resolve(process.cwd(), 'src/shared/ui/data-table/DataTable.tsx'),
      'utf8'
    );

    expect(tableSource).not.toMatch(/\s+sticky(?:\s|\n|\/?>)/);
    expect(tableSource).not.toContain('x: fixedTableWidth');
    expect(tableSource).toContain('minWidth: fixedTableWidth');
    expect(cssSource).toMatch(
      /\.data-table__scroll-area\s*\{[^}]*overflow-x:\s*auto;[^}]*overflow-y:\s*auto;/s
    );
    expect(cssSource).toMatch(
      /\.data-table \.ant-table-body\s*\{[^}]*overflow-x:\s*hidden !important;[^}]*\}/s
    );
  });

  test('renders logs inside the full section layout height chain', async () => {
    const pageSource = await readFile(
      path.resolve(
        process.cwd(),
        'src/features/applications/pages/ApplicationDetailPage.tsx'
      ),
      'utf8'
    );

    expect(pageSource).toMatch(
      /contentWidth=\{[\s\S]*requestedSectionKey === 'orchestration' \|\| requestedSectionKey === 'logs'[\s\S]*\? 'full'[\s\S]*: 'wide'[\s\S]*\}/
    );
  });

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
