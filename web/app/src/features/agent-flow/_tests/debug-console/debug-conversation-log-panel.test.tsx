import {
  fireEvent,
  render,
  screen,
  waitFor,
  within
} from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { StrictMode, type ComponentProps, type ReactNode } from 'react';
import { beforeEach, describe, expect, test, vi } from 'vitest';

import type {
  AgentFlowDebugMessage,
  AgentFlowRunContext
} from '../../api/runtime';
import { AgentFlowDebugConsole } from '../../components/debug-console/AgentFlowDebugConsole';
import { ConversationLogPanel } from '../../components/debug-console/ConversationLogPanel';
import { appI18n } from '../../../../shared/i18n/app-i18n';

const runContext: AgentFlowRunContext = {
  environmentLabel: 'draft',
  remembered: false,
  fields: [
    {
      nodeId: 'node-start',
      nodeLabel: 'Start',
      key: 'query',
      title: '问题',
      valueType: 'string',
      value: '你好?'
    }
  ]
};

function createQueryClient() {
  return new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false }
    }
  });
}

function renderWithQueryClient(children: ReactNode) {
  const queryClient = createQueryClient();

  return render(
    <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
  );
}

const assistantMessage: AgentFlowDebugMessage = {
  id: 'assistant-1',
  role: 'assistant',
  status: 'running',
  runId: 'run-1',
  compatibilityModeLabel: 'OpenAI Responses',
  content: '你好，我可以帮你。',
  rawOutput: {
    answer: '你好，我可以帮你。'
  },
  statistics: {
    total_tokens: 154,
    unique_node_count: 2,
    tool_callback_count: 0
  },
  traceSummary: [
    {
      nodeId: 'node-start',
      nodeRunId: 'node-run-start',
      nodeAlias: 'Start',
      nodeType: 'start',
      status: 'succeeded',
      startedAt: '2026-04-25T10:00:00Z',
      finishedAt: '2026-04-25T10:00:00Z',
      durationMs: 79,
      inputPayload: {
        query: '你好?'
      },
      outputPayload: {
        query: '你好?'
      },
      errorPayload: null,
      metricsPayload: {},
      debugPayload: {}
    },
    {
      nodeId: 'node-llm',
      nodeRunId: 'node-run-llm',
      nodeAlias: 'LLM',
      nodeType: 'llm',
      status: 'succeeded',
      startedAt: '2026-04-25T10:00:01Z',
      finishedAt: '2026-04-25T10:00:05Z',
      durationMs: 4257,
      inputPayload: {
        prompt: '你好?'
      },
      outputPayload: {
        answer: '你好，我可以帮你。'
      },
      errorPayload: null,
      metricsPayload: {
        total_tokens: 154
      },
      debugPayload: {
        provider: 'openai'
      }
    }
  ]
};

const llmRoundAssistantMessage: AgentFlowDebugMessage = {
  ...assistantMessage,
  traceSummary: assistantMessage.traceSummary.map((item) =>
    item.nodeId === 'node-llm'
      ? {
          ...item,
          outputPayload: {
            answer: 'weather is clear'
          },
          debugPayload: {
            provider: 'openai',
            llm_rounds: [
              {
                round_index: 0,
                usage: {
                  input_tokens: 11,
                  input_cache_hit_tokens: 5,
                  output_tokens: 3,
                  total_tokens: 14
                },
                assistant: {
                  role: 'assistant',
                  content: 'need tool',
                  tool_calls: [
                    {
                      id: 'call_weather',
                      name: 'lookup_weather',
                      call_usage: {
                        input_tokens: 11,
                        input_cache_hit_tokens: 5,
                        output_tokens: 3,
                        total_tokens: 14
                      },
                      arguments: {
                        city: 'Shanghai'
                      }
                    }
                  ]
                },
                finish_reason: 'tool_call'
              },
              {
                round_index: 1,
                assistant: {
                  role: 'assistant',
                  content: 'need tool'
                },
                tool_results: [
                  {
                    role: 'tool',
                    tool_call_id: 'call_weather',
                    token_delta: 10,
                    duration_ms: 1234,
                    result_context_usage: {
                      input_tokens: 20,
                      input_cache_hit_tokens: 8,
                      output_tokens: 4,
                      total_tokens: 24
                    },
                    content: '{"temperature":21}'
                  }
                ]
              },
              {
                round_index: 2,
                usage: {
                  input_tokens: 20,
                  input_cache_hit_tokens: 8,
                  output_tokens: 4,
                  total_tokens: 24
                },
                assistant: {
                  role: 'assistant',
                  content: 'weather is clear'
                },
                finish_reason: 'stop'
              }
            ]
          }
        }
      : item
  )
};

const truncatedLlmRoundsAssistantMessage: AgentFlowDebugMessage = {
  ...assistantMessage,
  traceSummary: assistantMessage.traceSummary.map((item) =>
    item.nodeId === 'node-llm'
      ? {
          ...item,
          outputPayload: {
            answer: 'weather is clear'
          },
          debugPayload: {
            provider: 'openai',
            llm_rounds: {
              __runtime_debug_artifact: true,
              artifact_ref: 'artifact-llm-rounds',
              is_truncated: true,
              original_size_bytes: 2000,
              preview_size_bytes: 120,
              content_type: 'application/json',
              preview: '["call_weather"]',
              tool_callbacks: [
                {
                  id: 'call_weather',
                  name: 'lookup_weather',
                  callback_status: 'returned',
                  execution_status: 'unknown',
                  request_round_index: 0,
                  result_round_index: 0,
                  call_usage: {
                    input_tokens: 11,
                    input_cache_hit_tokens: 5,
                    output_tokens: 3,
                    total_tokens: 14
                  },
                  result_context_usage: {
                    input_tokens: 20,
                    input_cache_hit_tokens: 8,
                    output_tokens: 4,
                    total_tokens: 24
                  },
                  token_delta: 10,
                  duration_ms: 1234,
                  artifact_ref: 'artifact-tool-call-weather'
                }
              ]
            }
          }
        }
      : item
  )
};

const toolCallbackDetailPayload = {
  id: 'call_weather',
  name: 'lookup_weather',
  callback_status: 'returned',
  execution_status: 'unknown',
  call_usage: {
    input_tokens: 11,
    input_cache_hit_tokens: 5,
    output_tokens: 3,
    total_tokens: 14
  },
  result_context_usage: {
    input_tokens: 20,
    input_cache_hit_tokens: 8,
    output_tokens: 4,
    total_tokens: 24
  },
  token_delta: 10,
  duration_ms: 1234,
  request_payload: {
    id: 'call_weather',
    name: 'lookup_weather',
    call_usage: {
      input_tokens: 11,
      input_cache_hit_tokens: 5,
      output_tokens: 3,
      total_tokens: 14
    },
    arguments: {
      city: 'Shanghai'
    }
  },
  callback_payload: {
    role: 'tool',
    tool_call_id: 'call_weather',
    token_delta: 10,
    duration_ms: 1234,
    result_context_usage: {
      input_tokens: 20,
      input_cache_hit_tokens: 8,
      output_tokens: 4,
      total_tokens: 24
    },
    content: '{"temperature":21}',
    adapter_trace_id: 'trace-weather-1'
  },
  parsed_result: {
    tool_call_id: 'call_weather',
    content: '{"temperature":21}'
  },
  request_round_index: 0,
  result_round_index: 0
};

const multiLlmRunAssistantMessage: AgentFlowDebugMessage = {
  ...assistantMessage,
  traceSummary: [
    assistantMessage.traceSummary[0],
    {
      ...assistantMessage.traceSummary[1],
      nodeRunId: 'node-run-llm-1',
      status: 'succeeded',
      durationMs: 5400,
      outputPayload: {
        usage: { total_tokens: 8035 },
        tool_calls: [{ id: 'call_weather', name: 'lookup_weather' }]
      },
      debugPayload: {
        llm_rounds: [
          {
            round_index: 0,
            assistant: {
              role: 'assistant',
              content: 'need weather',
              tool_calls: [
                {
                  id: 'call_weather',
                  name: 'lookup_weather',
                  arguments: { city: 'Shanghai' }
                }
              ]
            },
            finish_reason: 'tool_call'
          }
        ]
      }
    },
    {
      ...assistantMessage.traceSummary[1],
      nodeRunId: 'node-run-llm-2',
      status: 'succeeded',
      durationMs: 6900,
      outputPayload: {
        usage: { total_tokens: 8259 },
        tool_calls: [{ id: 'call_policy', name: 'read_policy' }]
      },
      debugPayload: {
        llm_rounds: [
          {
            round_index: 0,
            assistant: { role: 'assistant', content: 'continue' },
            tool_results: [
              {
                role: 'tool',
                tool_call_id: 'call_weather',
                content: '{"temperature":21}'
              }
            ]
          },
          {
            round_index: 1,
            assistant: {
              role: 'assistant',
              content: 'need policy',
              tool_calls: [
                {
                  id: 'call_policy',
                  name: 'read_policy',
                  arguments: { path: '.memory/user-memory.md' }
                }
              ]
            },
            finish_reason: 'tool_call'
          }
        ]
      }
    },
    {
      ...assistantMessage.traceSummary[1],
      nodeRunId: 'node-run-llm-3',
      status: 'succeeded',
      durationMs: 8500,
      outputPayload: {
        answer: 'weather is clear'
      },
      debugPayload: {
        llm_rounds: [
          {
            round_index: 0,
            assistant: { role: 'assistant', content: 'finish' },
            tool_results: [
              {
                role: 'tool',
                tool_call_id: 'call_policy',
                content: 'memory loaded'
              }
            ]
          },
          {
            round_index: 1,
            assistant: {
              role: 'assistant',
              content: 'weather is clear'
            },
            finish_reason: 'stop'
          }
        ]
      }
    }
  ]
};

const fusionSummaryOnlyAssistantMessage: AgentFlowDebugMessage = {
  ...assistantMessage,
  traceSummary: [
    {
      ...assistantMessage.traceSummary[1],
      nodeId: 'node-main-llm',
      nodeRunId: 'node-run-main-llm',
      nodeAlias: 'LLM',
      debugPayload: {
        llm_rounds: [
          {
            round_index: 0,
            assistant: {
              role: 'assistant',
              tool_calls: [
                {
                  id: 'call_fusion',
                  name: 'fusion_review'
                }
              ]
            }
          }
        ],
        visible_internal_llm_tool_trace: [
          {
            kind: 'visible_internal_llm_tool_trace',
            preview_kind: 'visible_internal_llm_tool_trace',
            route_kind: 'fusion',
            tool_call_id: 'call_fusion',
            tool_name: 'fusion_review',
            status: 'succeeded',
            branch_summaries: [
              {
                node_id: 'node-panel-a',
                node_alias: 'LLM2',
                node_type: 'llm',
                status: 'succeeded',
                output_summary: {
                  kind: 'text',
                  preview: 'summary only',
                  char_count: 12,
                  truncated: false
                }
              }
            ]
          }
        ]
      }
    }
  ]
};

const fusionHistoricalBranchDetailAssistantMessage: AgentFlowDebugMessage = {
  ...assistantMessage,
  traceSummary: [
    {
      ...assistantMessage.traceSummary[1],
      nodeId: 'node-main-llm',
      nodeRunId: 'node-run-main-llm',
      nodeAlias: 'LLM',
      debugPayload: {
        llm_rounds: [
          {
            round_index: 0,
            assistant: {
              role: 'assistant',
              tool_calls: [
                {
                  id: 'call_fusion',
                  name: 'fusion_review'
                }
              ]
            }
          }
        ],
        visible_internal_llm_tool_trace: [
          {
            kind: 'visible_internal_llm_tool_trace',
            preview_kind: 'visible_internal_llm_tool_trace',
            route_kind: 'fusion',
            tool_call_id: 'call_fusion',
            tool_name: 'fusion_review',
            status: 'succeeded',
            branch_count: 1,
            branch_traces: [
              {
                node_id: 'node-judge',
                node_alias: 'LLM5',
                node_type: 'llm',
                status: 'succeeded',
                route_model: 'gpt-5.4-mini',
                input_payload: {
                  prompt_messages: [
                    {
                      role: 'system',
                      content: 'You are the fusion judge.'
                    },
                    {
                      role: 'user',
                      content: 'Merge panel answers.'
                    }
                  ]
                },
                output_payload: {
                  text: 'judge merged answer'
                },
                metrics_payload: {
                  usage: {
                    input_tokens: 5513,
                    output_tokens: 2455,
                    total_tokens: 7968
                  }
                },
                debug_payload: {
                  assistant_message: {
                    role: 'assistant',
                    content: 'judge merged answer'
                  }
                },
                output_summary: {
                  kind: 'text',
                  preview: 'judge merged answer',
                  char_count: 19,
                  truncated: false
                }
              }
            ]
          }
        ]
      }
    }
  ]
};

const answerSnapshotAssistantMessage: AgentFlowDebugMessage = {
  ...assistantMessage,
  status: 'waiting_callback',
  content: '',
  rawOutput: {
    answer: 'LLM1 final\n----\n'
  },
  traceSummary: [
    {
      nodeId: 'node-llm-2',
      nodeRunId: 'node-run-llm-2',
      nodeAlias: 'LLM2',
      nodeType: 'llm',
      status: 'waiting_callback',
      startedAt: '2026-04-25T10:00:01Z',
      finishedAt: null,
      durationMs: null,
      inputPayload: {
        prompt: 'continue'
      },
      outputPayload: {
        tool_calls: []
      },
      errorPayload: null,
      metricsPayload: {},
      debugPayload: {},
      answerSnapshot: {
        kind: 'answer',
        text: 'LLM1 final\n----\n',
        outputPayload: {
          answer: 'LLM1 final\n----\n'
        },
        complete: false,
        materializedFrom: 'waiting_prefix',
        answerNodeId: 'node-answer',
        answerNodeRunId: 'node-run-answer-snapshot',
        waitingNodeId: 'node-llm-2',
        waitingNodeRunId: 'node-run-llm-2'
      }
    }
  ]
};

function renderConsole(
  props: Partial<ComponentProps<typeof AgentFlowDebugConsole>> = {}
) {
  return render(
    <StrictMode>
      <AgentFlowDebugConsole
        messages={[
          {
            id: 'user-1',
            role: 'user',
            status: 'completed',
            runId: 'run-1',
            content: '你好?',
            rawOutput: null,
            traceSummary: []
          },
          assistantMessage
        ]}
        runContext={runContext}
        status="completed"
        stopping={false}
        onChangeRunContextValue={vi.fn()}
        onClearSession={vi.fn()}
        onClose={vi.fn()}
        onStopRun={vi.fn()}
        onSubmitPrompt={vi.fn()}
        {...props}
      />
    </StrictMode>
  );
}

describe('debug conversation log panel', () => {
  beforeEach(async () => {
    window.localStorage.setItem('1flowbase.ui.locale_preference', 'zh_Hans');
    await appI18n.changeLanguage('zh_Hans');
  });

  test('opens from an assistant message and keeps detail limited to input, output and metadata', () => {
    renderConsole();

    expect(
      screen.queryByRole('complementary', { name: '对话日志' })
    ).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: '查看对话日志' }));

    const panel = screen.getByRole('complementary', { name: '对话日志' });
    expect(panel).toBeInTheDocument();
    expect(within(panel).getByRole('tab', { name: '详情' })).toHaveAttribute(
      'aria-selected',
      'true'
    );
    expect(within(panel).getByLabelText('输入 JSON')).toHaveTextContent(
      '你好?'
    );
    expect(within(panel).getByLabelText('输出 JSON')).toHaveTextContent(
      '你好，我可以帮你。'
    );
    expect(within(panel).getByText('元数据')).toBeInTheDocument();
    expect(within(panel).getByText('run-1')).toBeInTheDocument();
    expect(within(panel).getByText('协议')).toBeInTheDocument();
    expect(within(panel).getByText('OpenAI Responses')).toBeInTheDocument();
    expect(within(panel).getByText('总 tokens')).toBeInTheDocument();
    expect(within(panel).getByText('154')).toBeInTheDocument();
    expect(within(panel).getByText('真实节点数')).toBeInTheDocument();
    expect(within(panel).getByText('2')).toBeInTheDocument();
    expect(within(panel).getByText('工具回调次数')).toBeInTheDocument();
    expect(within(panel).getByText('0')).toBeInTheDocument();
    expect(within(panel).queryByText('节点数')).not.toBeInTheDocument();
    expect(within(panel).queryByText('数据处理')).not.toBeInTheDocument();
    expect(within(panel).queryByText('provider')).not.toBeInTheDocument();
  });

  test('loads lazy overview for application log details before trace root', async () => {
    const loadOverview = vi.fn().mockResolvedValue({
      run: {
        id: 'run-application-log',
        compatibility_mode: 'openai-responses-v1',
        started_at: '2026-04-25T10:00:00Z',
        finished_at: '2026-04-25T10:00:05Z'
      },
      statistics: {
        total_tokens: 154,
        unique_node_count: 2,
        tool_callback_count: 0
      },
      flow_run: {
        id: 'run-application-log',
        status: 'succeeded',
        input_payload: {
          'node-start': {
            query: '总结退款政策',
            model: 'deepseek-chat'
          }
        },
        output_payload: {
          answer: '退款政策摘要'
        },
        error_payload: null,
        started_at: '2026-04-25T10:00:00Z',
        finished_at: '2026-04-25T10:00:05Z'
      },
      answer_snapshot: null
    });
    const traceLoader = {
      loadTree: vi.fn().mockResolvedValue({ nodes: [] }),
      loadChildren: vi.fn(),
      loadContent: vi.fn()
    };

    renderWithQueryClient(
      <ConversationLogPanel
        message={{
          id: 'conversation-assistant-run-application-log',
          role: 'assistant',
          content: '退款政策摘要',
          status: 'completed',
          runId: 'run-application-log',
          detailRunId: 'run-application-log',
          rawOutput: null,
          traceSummary: []
        }}
        overviewLoader={{ loadOverview }}
        traceLoader={traceLoader}
        onClose={vi.fn()}
      />
    );

    await waitFor(() =>
      expect(screen.getByLabelText('输入 JSON')).toHaveTextContent('query')
    );
    expect(screen.getByLabelText('输入 JSON')).toHaveTextContent(
      '总结退款政策'
    );
    expect(screen.getByLabelText('输出 JSON')).toHaveTextContent(
      '退款政策摘要'
    );
    expect(screen.getByText('run-application-log')).toBeInTheDocument();
    expect(screen.getByText('154')).toBeInTheDocument();
    expect(loadOverview).toHaveBeenCalledWith('run-application-log');
    expect(traceLoader.loadTree).not.toHaveBeenCalled();

    fireEvent.click(screen.getByRole('tab', { name: '追踪' }));

    await waitFor(() =>
      expect(traceLoader.loadTree).toHaveBeenCalledWith('run-application-log')
    );
  });

  test('shows projection status instead of empty trace while the lazy trace index is pending', async () => {
    const traceLoader = {
      loadTree: vi.fn().mockResolvedValue({
        projection_status: {
          projection_status: 'pending',
          projection_version: 1,
          source_watermark: 'run-application-log:1',
          attempt_count: 0,
          last_attempt_at: null,
          last_success_at: null,
          last_error_code: null,
          last_error_stage: null,
          last_error_source_kind: null,
          last_error_source_locator: null,
          last_error_ref: null,
          retriable: true
        },
        nodes: []
      }),
      loadChildren: vi.fn(),
      loadContent: vi.fn()
    };

    renderWithQueryClient(
      <ConversationLogPanel
        message={{
          id: 'conversation-assistant-run-application-log',
          role: 'assistant',
          content: '退款政策摘要',
          status: 'completed',
          runId: 'run-application-log',
          detailRunId: 'run-application-log',
          rawOutput: null,
          traceSummary: []
        }}
        traceLoader={traceLoader}
        onClose={vi.fn()}
      />
    );

    fireEvent.click(screen.getByRole('tab', { name: '追踪' }));

    expect(await screen.findByText('追踪索引等待生成')).toBeInTheDocument();
    expect(screen.queryByText('暂无追踪记录')).not.toBeInTheDocument();
    expect(traceLoader.loadChildren).not.toHaveBeenCalled();
    expect(traceLoader.loadContent).not.toHaveBeenCalled();
  });

  test('loads lazy trace children and node content when a node expands', async () => {
    const rootNode = {
      trace_node_id: 'node_run:node-run-llm',
      node_kind: 'node_run',
      node_run_id: 'node-run-llm',
      node_id: 'node-llm',
      node_type: 'llm',
      node_alias: 'LLM',
      status: 'succeeded',
      started_at: '2026-04-25T10:00:01Z',
      finished_at: '2026-04-25T10:00:05Z',
      duration_ms: 4000,
      metrics_payload: {
        total_tokens: 154
      },
      has_children: true,
      has_content: true
    };
    const childNode = {
      trace_node_id: 'callback_task:callback-weather',
      node_kind: 'callback_task',
      node_run_id: null,
      node_id: null,
      node_type: 'callback_task',
      node_alias: 'lookup_weather',
      status: 'succeeded',
      started_at: '2026-04-25T10:00:02Z',
      finished_at: '2026-04-25T10:00:03Z',
      duration_ms: 1000,
      metrics_payload: {},
      has_children: false,
      has_content: false
    };
    const traceLoader = {
      loadTree: vi.fn().mockResolvedValue({ nodes: [rootNode] }),
      loadChildren: vi.fn().mockResolvedValue({ items: [childNode] }),
      loadContent: vi.fn().mockResolvedValue({
        trace_node_id: 'node_run:node-run-llm',
        node_kind: 'node_run',
        node_run: {
          id: 'node-run-llm',
          node_id: 'node-llm',
          node_type: 'llm',
          node_alias: 'LLM',
          status: 'succeeded',
          input_payload: {
            prompt: '总结退款政策'
          },
          output_payload: {
            answer: '退款政策摘要'
          },
          error_payload: null,
          metrics_payload: {
            total_tokens: 154
          },
          debug_payload: {
            provider: 'deepseek'
          },
          started_at: '2026-04-25T10:00:01Z',
          finished_at: '2026-04-25T10:00:05Z'
        }
      })
    };

    renderWithQueryClient(
      <ConversationLogPanel
        message={{
          id: 'conversation-assistant-run-application-log',
          role: 'assistant',
          content: '退款政策摘要',
          status: 'completed',
          runId: 'run-application-log',
          detailRunId: 'run-application-log',
          rawOutput: null,
          traceSummary: []
        }}
        traceLoader={traceLoader}
        onClose={vi.fn()}
      />
    );

    fireEvent.click(screen.getByRole('tab', { name: '追踪' }));
    await waitFor(() =>
      expect(traceLoader.loadTree).toHaveBeenCalledWith('run-application-log')
    );
    expect(traceLoader.loadChildren).not.toHaveBeenCalled();
    expect(traceLoader.loadContent).not.toHaveBeenCalled();

    const llmTraceNode = await screen.findByRole('button', { name: /LLM/ });
    fireEvent.click(llmTraceNode);

    await waitFor(() =>
      expect(traceLoader.loadChildren).toHaveBeenCalledWith(
        'run-application-log',
        'node_run:node-run-llm'
      )
    );
    await waitFor(() =>
      expect(traceLoader.loadContent).toHaveBeenCalledWith(
        'run-application-log',
        'node_run:node-run-llm'
      )
    );
    const nodeDetail = await screen.findByRole('region', {
      name: 'LLM 节点详情'
    });
    expect(
      within(nodeDetail).queryByRole('button', { name: '详情' })
    ).not.toBeInTheDocument();
    expect(
      await within(nodeDetail).findByRole('button', { name: /lookup_weather/ })
    ).toBeInTheDocument();
    await waitFor(() =>
      expect(within(nodeDetail).getByLabelText('输入 JSON')).toHaveTextContent(
        '总结退款政策'
      )
    );
  });

  test('loads lazy trace tool details only when a tool callback expands', async () => {
    const rootNode = {
      trace_node_id: 'node_run:node-run-llm',
      node_kind: 'node_run',
      node_run_id: 'node-run-llm',
      node_id: 'node-llm',
      node_type: 'llm',
      node_alias: 'LLM',
      status: 'succeeded',
      started_at: '2026-04-25T10:00:01Z',
      finished_at: '2026-04-25T10:00:05Z',
      duration_ms: 4000,
      metrics_payload: {},
      has_children: false,
      has_content: true
    };
    const traceLoader = {
      loadTree: vi.fn().mockResolvedValue({ nodes: [rootNode] }),
      loadChildren: vi.fn().mockResolvedValue({ items: [] }),
      loadContent: vi.fn().mockResolvedValue({
        trace_node_id: 'node_run:node-run-llm',
        node_kind: 'node_run',
        node_run: {
          id: 'node-run-llm',
          node_id: 'node-llm',
          node_type: 'llm',
          node_alias: 'LLM',
          status: 'succeeded',
          input_payload: {
            prompt: '总结退款政策'
          },
          output_payload: {
            answer: '退款政策摘要'
          },
          error_payload: null,
          metrics_payload: {},
          debug_payload: {
            tool_callbacks: [
              {
                id: 'call-refund-policy',
                name: 'refund_policy_lookup',
                callback_status: 'returned',
                execution_status: 'succeeded',
                request_round_index: 0,
                result_round_index: 1,
                duration_ms: 1234,
                detail_ref: 'call-refund-policy'
              }
            ]
          },
          started_at: '2026-04-25T10:00:01Z',
          finished_at: '2026-04-25T10:00:05Z'
        }
      }),
      loadToolCallbackDetail: vi.fn().mockResolvedValue({
        id: 'call-refund-policy',
        name: 'refund_policy_lookup',
        callback_status: 'returned',
        execution_status: 'succeeded',
        request_payload: {
          arguments: {
            topic: 'refund'
          }
        },
        callback_payload: {
          content: '30 days refund window'
        },
        parsed_result: {
          content: '30 days refund window'
        },
        request_round_index: 0,
        result_round_index: 1,
        duration_ms: 1234
      })
    };

    renderWithQueryClient(
      <ConversationLogPanel
        message={{
          id: 'conversation-assistant-run-application-log',
          role: 'assistant',
          content: '退款政策摘要',
          status: 'completed',
          runId: 'run-application-log',
          detailRunId: 'run-application-log',
          rawOutput: null,
          traceSummary: []
        }}
        traceLoader={traceLoader}
        onClose={vi.fn()}
      />
    );

    fireEvent.click(screen.getByRole('tab', { name: '追踪' }));
    const llmTraceNode = await screen.findByRole('button', { name: /LLM/ });
    fireEvent.click(llmTraceNode);
    const nodeDetail = await screen.findByRole('region', {
      name: 'LLM 节点详情'
    });
    const toolsNode = await within(nodeDetail).findByRole('button', {
      name: /工具.*1 次工具回调/
    });
    expect(toolsNode).toHaveAttribute('aria-expanded', 'true');

    expect(traceLoader.loadToolCallbackDetail).not.toHaveBeenCalled();
    const toolCallback = within(nodeDetail).getByRole('button', {
      name: /refund_policy_lookup/
    });
    expect(toolCallback).toHaveTextContent('1.23 s');
    expect(
      within(nodeDetail).queryByLabelText('工具调用 JSON')
    ).not.toBeInTheDocument();

    fireEvent.click(toolCallback);

    await waitFor(() =>
      expect(traceLoader.loadToolCallbackDetail).toHaveBeenCalledWith(
        'run-application-log',
        'node_run:node-run-llm',
        'call-refund-policy'
      )
    );
    expect(
      await within(nodeDetail).findByLabelText('工具调用 JSON')
    ).toHaveTextContent('refund');
    expect(
      within(nodeDetail).getByLabelText('完整回调 JSON')
    ).toHaveTextContent('30 days refund window');
  });

  test('shows clickable trace nodes and reuses node run detail sections', () => {
    renderConsole();

    fireEvent.click(screen.getByRole('button', { name: '查看对话日志' }));
    const panel = screen.getByRole('complementary', { name: '对话日志' });
    fireEvent.click(within(panel).getByRole('tab', { name: '追踪' }));
    expect(within(panel).getByText('4.26 s')).toBeInTheDocument();
    expect(within(panel).queryByText('4257 ms')).not.toBeInTheDocument();
    const llmTraceNode = within(panel).getByRole('button', { name: /LLM/ });

    expect(llmTraceNode).toHaveAttribute('aria-expanded', 'false');

    fireEvent.click(llmTraceNode);

    expect(llmTraceNode).toHaveAttribute('aria-expanded', 'true');
    expect(
      within(panel).getAllByTestId('debug-workflow-node-item')[1]
    ).toHaveAttribute('data-selected', 'false');

    const nodeDetail = within(panel).getByRole('region', {
      name: 'LLM 节点详情'
    });
    expect(nodeDetail).toBeInTheDocument();
    expect(within(nodeDetail).queryByText('LLM')).not.toBeInTheDocument();
    expect(within(nodeDetail).queryByText('llm')).not.toBeInTheDocument();
    expect(within(nodeDetail).getByLabelText('输入 JSON')).toHaveTextContent(
      'prompt'
    );
    expect(
      within(nodeDetail).getByLabelText('数据处理 JSON')
    ).toHaveTextContent('provider');
    expect(within(nodeDetail).getByLabelText('输出 JSON')).toHaveTextContent(
      '你好，我可以帮你。'
    );
    expect(
      within(panel).getAllByTestId('debug-workflow-node-row')
    ).toHaveLength(2);
  }, 10_000);

  test('groups LLM tool callbacks behind a virtual Tools child node', () => {
    renderConsole({
      messages: [
        {
          id: 'user-1',
          role: 'user',
          status: 'completed',
          runId: 'run-1',
          content: '天气?',
          rawOutput: null,
          traceSummary: []
        },
        llmRoundAssistantMessage
      ]
    });

    fireEvent.click(screen.getByRole('button', { name: '查看对话日志' }));
    const panel = screen.getByRole('complementary', { name: '对话日志' });
    fireEvent.click(within(panel).getByRole('tab', { name: '追踪' }));
    fireEvent.click(within(panel).getByRole('button', { name: /LLM/ }));

    const nodeDetail = within(panel).getByRole('region', {
      name: 'LLM 节点详情'
    });
    expect(within(nodeDetail).queryByText('Round #1')).not.toBeInTheDocument();
    expect(
      within(nodeDetail).queryByText('Tool Callback #1')
    ).not.toBeInTheDocument();

    const toolsNode = within(nodeDetail).getByRole('button', {
      name: /工具.*1 次工具回调/
    });
    expect(toolsNode).toHaveAttribute('aria-expanded', 'true');
    expect(
      within(nodeDetail).queryByText('temperature')
    ).not.toBeInTheDocument();

    expect(
      within(nodeDetail).queryByLabelText('工具回调索引 JSON')
    ).not.toBeInTheDocument();

    const toolCallback = within(nodeDetail).getByRole('button', {
      name: /lookup_weather.*14 tokens.*1\.23 s/
    });
    expect(toolCallback).toHaveTextContent('lookup_weather');
    expect(toolCallback).toHaveTextContent('14 tokens · 1.23 s');
    expect(toolCallback).not.toHaveTextContent('+10 tokens');
    expect(toolCallback).toHaveAttribute('aria-expanded', 'false');
    expect(
      within(nodeDetail).queryByText('call_weather')
    ).not.toBeInTheDocument();
    expect(
      within(nodeDetail).queryByText('temperature')
    ).not.toBeInTheDocument();

    fireEvent.click(toolCallback);

    expect(toolCallback).toHaveAttribute('aria-expanded', 'true');
    expect(
      within(nodeDetail).getByLabelText('工具调用 JSON')
    ).toHaveTextContent('Shanghai');
    expect(
      within(nodeDetail).getByLabelText('完整回调 JSON')
    ).toHaveTextContent('temperature');
    expect(nodeDetail).not.toHaveTextContent('工具 token 归因');
    expect(
      within(nodeDetail).getByLabelText('工具调用 JSON')
    ).toHaveTextContent('total_tokens');
    expect(
      within(nodeDetail).getByLabelText('完整回调 JSON')
    ).toHaveTextContent('result_context_usage');
    expect(nodeDetail).toHaveTextContent('已返回');
    expect(nodeDetail).not.toHaveTextContent('执行未知');
    expect(nodeDetail).toHaveTextContent('weather is clear');
    within(nodeDetail)
      .getAllByLabelText('数据处理 JSON')
      .forEach((block) => {
        expect(block).not.toHaveTextContent('llm_rounds');
      });
  }, 10_000);

  test('shows a summary-only state instead of empty JSON for route branch nodes without detail', () => {
    renderConsole({
      messages: [
        {
          id: 'user-1',
          role: 'user',
          status: 'completed',
          runId: 'run-1',
          content: '做 fusion 评审',
          rawOutput: null,
          traceSummary: []
        },
        fusionSummaryOnlyAssistantMessage
      ]
    });

    fireEvent.click(screen.getByRole('button', { name: '查看对话日志' }));
    const panel = screen.getByRole('complementary', { name: '对话日志' });
    fireEvent.click(within(panel).getByRole('tab', { name: '追踪' }));
    fireEvent.click(within(panel).getByRole('button', { name: /LLM/ }));
    expect(
      within(panel).getByRole('button', { name: /工具.*1 次工具回调/ })
    ).toHaveAttribute('aria-expanded', 'true');
    fireEvent.click(
      within(panel).getByRole('button', { name: /fusion_review/ })
    );

    const branchNode = within(panel).getByTestId('debug-llm-route-branch-node');
    fireEvent.click(within(branchNode).getByRole('button', { name: /LLM2/ }));

    expect(branchNode).toHaveTextContent('仅有摘要，节点详情未生成');
    expect(
      within(branchNode).queryByLabelText('输入 JSON')
    ).not.toBeInTheDocument();
    expect(
      within(branchNode).queryByLabelText('数据处理 JSON')
    ).not.toBeInTheDocument();
    expect(
      within(branchNode).queryByLabelText('输出 JSON')
    ).not.toBeInTheDocument();
  }, 10_000);

  test('shows fusion branch LLM tokens from metrics payload and reuses node detail sections', () => {
    renderConsole({
      messages: [
        {
          id: 'user-1',
          role: 'user',
          status: 'completed',
          runId: 'run-1',
          content: '做 fusion 评审',
          rawOutput: null,
          traceSummary: []
        },
        fusionHistoricalBranchDetailAssistantMessage
      ]
    });

    fireEvent.click(screen.getByRole('button', { name: '查看对话日志' }));
    const panel = screen.getByRole('complementary', { name: '对话日志' });
    fireEvent.click(within(panel).getByRole('tab', { name: '追踪' }));
    fireEvent.click(within(panel).getByRole('button', { name: /LLM/ }));
    expect(
      within(panel).getByRole('button', { name: /工具.*1 次工具回调/ })
    ).toHaveAttribute('aria-expanded', 'true');
    fireEvent.click(
      within(panel).getByRole('button', { name: /fusion_review/ })
    );

    const branchNode = within(panel).getByTestId('debug-llm-route-branch-node');
    const branchButton = within(branchNode).getByRole('button', {
      name: /LLM5/
    });
    expect(branchButton).toHaveTextContent('7.96 K tokens');
    expect(branchButton).not.toHaveTextContent('执行成功');

    fireEvent.click(branchButton);

    expect(within(branchNode).getByLabelText('输入 JSON')).toHaveTextContent(
      'Merge panel answers.'
    );
    expect(
      within(branchNode).getByLabelText('数据处理 JSON')
    ).toHaveTextContent('assistant_message');
    expect(within(branchNode).getByLabelText('输出 JSON')).toHaveTextContent(
      'judge merged answer'
    );
  }, 10_000);

  test('collapses repeated LLM node runs into one trace row', () => {
    renderConsole({
      messages: [
        {
          id: 'user-1',
          role: 'user',
          status: 'completed',
          runId: 'run-1',
          content: '天气?',
          rawOutput: null,
          traceSummary: []
        },
        multiLlmRunAssistantMessage
      ]
    });

    fireEvent.click(screen.getByRole('button', { name: '查看对话日志' }));
    const panel = screen.getByRole('complementary', { name: '对话日志' });
    fireEvent.click(within(panel).getByRole('tab', { name: '追踪' }));

    expect(
      within(panel).getAllByTestId('debug-workflow-node-row')
    ).toHaveLength(2);

    const llmTraceNode = within(panel).getByRole('button', { name: /LLM/ });
    expect(llmTraceNode).toHaveTextContent('工具 2');

    fireEvent.click(llmTraceNode);

    const nodeDetail = within(panel).getByRole('region', {
      name: 'LLM 节点详情'
    });
    const toolsNode = within(nodeDetail).getByRole('button', {
      name: /工具.*2 次工具回调/
    });
    expect(toolsNode).toHaveAttribute('aria-expanded', 'true');

    expect(
      within(nodeDetail).queryByLabelText('工具回调索引 JSON')
    ).not.toBeInTheDocument();
    expect(
      within(nodeDetail).getByRole('button', {
        name: /lookup_weather/
      })
    ).toBeInTheDocument();
    expect(
      within(nodeDetail).getByRole('button', {
        name: /read_policy/
      })
    ).toBeInTheDocument();
    expect(
      within(nodeDetail).queryByText('call_weather')
    ).not.toBeInTheDocument();
    expect(
      within(nodeDetail).queryByText('call_policy')
    ).not.toBeInTheDocument();
    expect(within(nodeDetail).getByLabelText('输出 JSON')).toHaveTextContent(
      'weather is clear'
    );
  }, 10_000);

  test('renders waiting answer snapshots inside the waiting LLM trace row', () => {
    renderConsole({
      messages: [
        {
          id: 'user-1',
          role: 'user',
          status: 'completed',
          runId: 'run-1',
          content: '继续?',
          rawOutput: null,
          traceSummary: []
        },
        answerSnapshotAssistantMessage
      ]
    });

    fireEvent.click(screen.getByRole('button', { name: '查看对话日志' }));
    const panel = screen.getByRole('complementary', { name: '对话日志' });
    fireEvent.click(within(panel).getByRole('tab', { name: '追踪' }));

    expect(
      within(panel).getAllByTestId('debug-workflow-node-row')
    ).toHaveLength(1);
    expect(within(panel).queryByText('直接回复')).not.toBeInTheDocument();

    const llmTraceNode = within(panel).getByRole('button', { name: /LLM2/ });
    fireEvent.click(llmTraceNode);

    const nodeDetail = within(panel).getByRole('region', {
      name: 'LLM2 节点详情'
    });
    const answerSnapshot = within(nodeDetail).getByRole('button', {
      name: /answer快照/
    });
    expect(answerSnapshot).toHaveAttribute('aria-expanded', 'false');
    expect(
      within(nodeDetail).queryByText('LLM1 final')
    ).not.toBeInTheDocument();

    fireEvent.click(answerSnapshot);

    expect(answerSnapshot).toHaveAttribute('aria-expanded', 'true');
    expect(
      within(nodeDetail).getByLabelText('answer快照 JSON')
    ).toHaveTextContent('LLM1 final');
  }, 10_000);

  test('loads full LLM tool callbacks when the rounds payload is truncated', async () => {
    const onLoadArtifact = vi.fn().mockResolvedValue(toolCallbackDetailPayload);

    renderConsole({
      messages: [
        {
          id: 'user-1',
          role: 'user',
          status: 'completed',
          runId: 'run-1',
          content: '天气?',
          rawOutput: null,
          traceSummary: []
        },
        truncatedLlmRoundsAssistantMessage
      ],
      onLoadArtifact
    });

    fireEvent.click(screen.getByRole('button', { name: '查看对话日志' }));
    const panel = screen.getByRole('complementary', { name: '对话日志' });
    fireEvent.click(within(panel).getByRole('tab', { name: '追踪' }));
    fireEvent.click(within(panel).getByRole('button', { name: /LLM/ }));

    const nodeDetail = within(panel).getByRole('region', {
      name: 'LLM 节点详情'
    });
    const toolsNode = within(nodeDetail).getByRole('button', {
      name: /工具.*1 次工具回调/
    });
    expect(toolsNode).toHaveAttribute('aria-expanded', 'true');

    expect(
      within(nodeDetail).queryByRole('button', { name: '加载完整工具' })
    ).not.toBeInTheDocument();
    expect(onLoadArtifact).not.toHaveBeenCalled();
    const toolCallback = within(nodeDetail).getByRole('button', {
      name: /lookup_weather.*14 tokens.*1\.23 s/
    });
    expect(toolCallback).toHaveTextContent('14 tokens · 1.23 s');
    expect(toolCallback).not.toHaveTextContent('+10 tokens');
    expect(
      within(nodeDetail).queryByLabelText('工具回调索引 JSON')
    ).not.toBeInTheDocument();
    expect(within(nodeDetail).queryByText('Shanghai')).not.toBeInTheDocument();

    fireEvent.click(toolCallback);

    expect(onLoadArtifact).toHaveBeenCalledWith('artifact-tool-call-weather');
    expect(
      await within(nodeDetail).findByLabelText('工具调用 JSON')
    ).toHaveTextContent('Shanghai');
    expect(
      within(nodeDetail).getByLabelText('完整回调 JSON')
    ).toHaveTextContent('trace-weather-1');
    expect(
      within(nodeDetail).getByLabelText('解析结果 JSON')
    ).toHaveTextContent('temperature');
    expect(nodeDetail).not.toHaveTextContent('工具 token 归因');
    expect(
      within(nodeDetail).getByLabelText('工具调用 JSON')
    ).toHaveTextContent('call_usage');
    expect(
      within(nodeDetail).getByLabelText('完整回调 JSON')
    ).toHaveTextContent('result_context_usage');
  }, 10_000);

  test('delegates log opening when the canvas shell controls the log panel', () => {
    const onOpenMessageLog = vi.fn();

    renderConsole({ onOpenMessageLog });

    fireEvent.click(screen.getByRole('button', { name: '查看对话日志' }));

    expect(onOpenMessageLog).toHaveBeenCalledWith(assistantMessage);
    expect(
      screen.queryByRole('complementary', { name: '对话日志' })
    ).not.toBeInTheDocument();
  });
});
