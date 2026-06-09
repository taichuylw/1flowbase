import {
  fireEvent,
  render,
  screen,
  waitFor,
  within
} from '@testing-library/react';
import { vi } from 'vitest';

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
  fetchApplicationRunDetail: vi.fn(),
  fetchApplicationConversationMessages: vi.fn(),
  fetchApplicationRunConversationMessages: vi.fn(),
  fetchRuntimeDebugArtifact: vi.fn(),
  resumeFlowRun: vi.fn(),
  completeCallbackTask: vi.fn()
}));

vi.mock('../../api/runtime', () => runtimeApi);

import type { ApplicationRunDetail } from '../../api/runtime';
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
      run_id: item.flow_run_id ?? item.id,
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

describe('ApplicationLogsPage - artifacts and trace', () => {
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
    runtimeApi.fetchApplicationRunDetail.mockReset();
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
          started_at: '2026-04-17T09:00:00Z',
          finished_at: '2026-04-17T09:00:01Z',
          created_at: '2026-04-17T09:00:00Z',
          updated_at: '2026-04-17T09:00:01Z'
        }
      ])
    );
    runtimeApi.fetchApplicationRunDetail.mockResolvedValue(sampleRunDetail());
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
    fireEvent.click(within(logPanel).getByRole('tab', { name: '追踪' }));

    expect(
      within(logPanel).getAllByTestId('debug-workflow-node-row')
    ).toHaveLength(1);

    const llmTraceNode = within(logPanel).getByRole('button', { name: /LLM/ });
    expect(llmTraceNode).toHaveTextContent('工具 2');
    fireEvent.click(llmTraceNode);

    const toolsNode = within(logPanel).getByRole('button', {
      name: /工具 2 次工具回调/
    });
    expect(toolsNode).toHaveAttribute('aria-expanded', 'false');
    expect(
      within(logPanel).queryByText('call_weather')
    ).not.toBeInTheDocument();

    fireEvent.click(toolsNode);
    expect(
      within(logPanel).queryByLabelText('工具回调索引 JSON')
    ).not.toBeInTheDocument();
    expect(
      within(logPanel).getByRole('button', {
        name: /lookup_weather/
      })
    ).toBeInTheDocument();
    expect(
      within(logPanel).getByRole('button', {
        name: /read_policy/
      })
    ).toBeInTheDocument();
    expect(
      within(logPanel).queryByText('call_weather')
    ).not.toBeInTheDocument();
    expect(within(logPanel).queryByText('call_policy')).not.toBeInTheDocument();
  }, 20_000);

  test('keeps expanded trace tools and loaded tool details across floating window activation', async () => {
    const detail = sampleRunDetail();
    const llmNodeRun = detail.node_runs[0]!;
    detail.node_runs = [
      {
        ...llmNodeRun,
        id: 'node-run-llm-1',
        debug_payload: {
          llm_rounds: {
            __runtime_debug_artifact: true,
            artifact_ref: 'artifact-llm-rounds',
            tool_callbacks: [
              {
                id: 'call_weather',
                name: 'lookup_weather',
                callback_status: 'returned',
                execution_status: 'succeeded',
                artifact_ref: 'artifact-tool-weather'
              }
            ]
          },
          visible_internal_llm_tool_trace: [
            {
              __runtime_debug_artifact: true,
              kind: 'visible_internal_llm_tool_trace',
              preview_kind: 'visible_internal_llm_tool_trace',
              artifact_ref: 'artifact-route-weather',
              tool_call_id: 'call_weather',
              tool_name: 'lookup_weather',
              route_model: 'mimo-v2.5',
              returned_to_main: true,
              main_resume: true,
              route_output_summary: {
                kind: 'text',
                preview: 'weather route said warm',
                char_count: 23,
                truncated: false
              }
            }
          ]
        }
      },
      {
        ...llmNodeRun,
        id: 'node-run-llm-2',
        debug_payload: {},
        started_at: '2026-04-17T09:00:01Z',
        finished_at: '2026-04-17T09:00:02Z'
      }
    ];
    runtimeApi.fetchApplicationRunDetail.mockResolvedValue(detail);
    runtimeApi.fetchRuntimeDebugArtifact.mockImplementation(
      async (_applicationId: string, artifactRef: string) => {
        if (artifactRef === 'artifact-tool-weather') {
          return {
            id: 'call_weather',
            name: 'lookup_weather',
            callback_status: 'returned',
            execution_status: 'succeeded',
            request_payload: {
              city: 'Shanghai'
            },
            callback_payload: {
              temperature: 'warm'
            },
            parsed_result: {
              ok: true
            }
          };
        }
        if (artifactRef === 'artifact-route-weather') {
          return {
            kind: 'visible_internal_llm_tool_trace',
            tool_call_id: 'call_weather',
            route: {
              model: 'mimo-v2.5'
            },
            returned_to_main: true,
            main_resume: true,
            main_resume_output: {
              content: 'main saw weather route'
            }
          };
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
    fireEvent.click(within(logPanel).getByRole('tab', { name: '追踪' }));
    fireEvent.click(within(logPanel).getByRole('button', { name: /LLM/ }));

    const toolsNode = within(logPanel).getByRole('button', {
      name: /工具 1 次工具回调/
    });
    fireEvent.click(toolsNode);

    const toolCallbackNode = within(logPanel).getByRole('button', {
      name: /lookup_weather/
    });
    expect(toolCallbackNode).toHaveTextContent('route');
    expect(toolCallbackNode).not.toHaveTextContent('路由模型 mimo-v2.5');
    expect(toolCallbackNode).not.toHaveTextContent('weather route said warm');
    fireEvent.click(toolCallbackNode);

    await waitFor(() =>
      expect(runtimeApi.fetchRuntimeDebugArtifact).toHaveBeenCalledTimes(1)
    );
    expect(runtimeApi.fetchRuntimeDebugArtifact).toHaveBeenCalledWith(
      'app-1',
      'artifact-tool-weather'
    );
    const routeNode = within(logPanel).getByTestId('debug-llm-route-node');
    expect(routeNode).toHaveTextContent('LLM');
    expect(routeNode).toHaveTextContent('llm');
    expect(
      within(logPanel).queryByLabelText('智能路由 JSON')
    ).not.toBeInTheDocument();
    const routeTraceJson = within(routeNode).getByLabelText('route JSON');
    expect(routeTraceJson).toHaveTextContent('weather route said warm');
    const routeTraceBlock = routeTraceJson.closest('section');
    expect(routeTraceBlock).not.toBeNull();
    fireEvent.click(
      within(routeTraceBlock as HTMLElement).getByRole('button', {
        name: '加载完整值'
      })
    );
    await waitFor(() =>
      expect(runtimeApi.fetchRuntimeDebugArtifact).toHaveBeenCalledWith(
        'app-1',
        'artifact-route-weather'
      )
    );
    await waitFor(() =>
      expect(within(routeNode).getByLabelText('route JSON')).toHaveTextContent(
        'main saw weather route'
      )
    );

    fireEvent.mouseDown(
      screen.getByTestId('application-logs-floating-run-detail')
    );

    expect(
      within(logPanel).getByRole('button', {
        name: /工具 1 次工具回调/
      })
    ).toHaveAttribute('aria-expanded', 'true');
    expect(
      within(logPanel).getByRole('button', {
        name: /lookup_weather/
      })
    ).toHaveAttribute('aria-expanded', 'true');

    fireEvent.click(within(logPanel).getByRole('button', { name: /LLM/ }));
    expect(
      within(logPanel).queryByRole('button', {
        name: /lookup_weather/
      })
    ).not.toBeInTheDocument();

    fireEvent.click(within(logPanel).getByRole('button', { name: /LLM/ }));
    fireEvent.click(
      within(logPanel).getByRole('button', {
        name: /工具 1 次工具回调/
      })
    );
    fireEvent.click(
      within(logPanel).getByRole('button', {
        name: /lookup_weather/
      })
    );

    await waitFor(() =>
      expect(runtimeApi.fetchRuntimeDebugArtifact).toHaveBeenCalledTimes(2)
    );
  }, 20_000);

  test('does not offer run log details for imported context messages', async () => {
    runtimeApi.fetchApplicationRunConversationMessages.mockResolvedValue(
      conversationMessagesPage([
        {
          id: 'msg-history-system',
          flow_run_id: null,
          role: 'system',
          content: '你是项目助手',
          sequence: 1,
          started_at: '2026-04-17T08:58:59Z',
          finished_at: '2026-04-17T08:59:00Z'
        },
        {
          id: 'msg-history-user',
          flow_run_id: null,
          role: 'user',
          content: '外部传入的问题',
          sequence: 2,
          started_at: '2026-04-17T08:59:00Z',
          finished_at: '2026-04-17T08:59:01Z'
        },
        {
          id: 'msg-history-assistant',
          flow_run_id: null,
          role: 'assistant',
          content: '外部传入的回答',
          sequence: 3,
          started_at: '2026-04-17T08:59:01Z',
          finished_at: '2026-04-17T08:59:02Z'
        },
        {
          id: 'msg-run-1-user',
          flow_run_id: 'run-1',
          role: 'user',
          content: '总结退款政策',
          sequence: 4,
          started_at: '2026-04-17T09:00:00Z',
          finished_at: '2026-04-17T09:00:01Z'
        },
        {
          id: 'msg-run-1-assistant',
          flow_run_id: 'run-1',
          role: 'assistant',
          content: '退款政策摘要',
          sequence: 5,
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

    expect(await screen.findByText('run-1')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: '查看运行详情' }));

    const conversation = await screen.findByTestId(
      'debug-conversation-messages'
    );
    expect(await within(conversation).findByText('System')).toBeInTheDocument();
    expect(within(conversation).getByText('你是项目助手')).toBeInTheDocument();
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
});
