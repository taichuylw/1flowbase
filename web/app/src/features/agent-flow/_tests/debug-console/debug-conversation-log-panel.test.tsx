import { fireEvent, render, screen, within } from '@testing-library/react';
import { StrictMode, type ComponentProps } from 'react';
import { describe, expect, test, vi } from 'vitest';

import type {
  AgentFlowDebugMessage,
  AgentFlowRunContext
} from '../../api/runtime';
import { AgentFlowDebugConsole } from '../../components/debug-console/AgentFlowDebugConsole';

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
    expect(within(panel).queryByText('数据处理')).not.toBeInTheDocument();
    expect(within(panel).queryByText('provider')).not.toBeInTheDocument();
  });

  test('shows clickable trace nodes and reuses node run detail sections', () => {
    renderConsole();

    fireEvent.click(screen.getByRole('button', { name: '查看对话日志' }));
    const panel = screen.getByRole('complementary', { name: '对话日志' });
    fireEvent.click(within(panel).getByRole('tab', { name: '追踪' }));
    expect(within(panel).getByText('4.3 s')).toBeInTheDocument();
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
      name: /Tools.*1 次工具回调/
    });
    expect(toolsNode).toHaveAttribute('aria-expanded', 'false');
    expect(
      within(nodeDetail).queryByText('temperature')
    ).not.toBeInTheDocument();

    fireEvent.click(toolsNode);

    expect(toolsNode).toHaveAttribute('aria-expanded', 'true');
    expect(
      within(nodeDetail).queryByLabelText('工具回调索引 JSON')
    ).not.toBeInTheDocument();

    const toolCallback = within(nodeDetail).getByRole('button', {
      name: /lookup_weather.*14 tokens/
    });
    const toolMain = toolCallback.querySelector(
      '.agent-flow-editor__debug-llm-tool-main'
    ) as HTMLElement;
    expect(toolMain).toHaveTextContent('lookup_weather');
    expect(toolMain).toHaveTextContent('14 tokens');
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
    expect(within(nodeDetail).getByLabelText('工具调用 JSON')).toHaveTextContent(
      'total_tokens'
    );
    expect(within(nodeDetail).getByLabelText('完整回调 JSON')).toHaveTextContent(
      'result_context_usage'
    );
    expect(nodeDetail).toHaveTextContent('已返回');
    expect(nodeDetail).not.toHaveTextContent('执行未知');
    expect(nodeDetail).toHaveTextContent('weather is clear');
    within(nodeDetail)
      .getAllByLabelText('数据处理 JSON')
      .forEach((block) => {
        expect(block).not.toHaveTextContent('llm_rounds');
      });
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
      name: /Tools.*2 次工具回调/
    });
    fireEvent.click(toolsNode);

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
      name: /Tools.*1 次工具回调/
    });
    fireEvent.click(toolsNode);

    expect(
      within(nodeDetail).queryByRole('button', { name: '加载完整工具' })
    ).not.toBeInTheDocument();
    expect(onLoadArtifact).not.toHaveBeenCalled();
    const toolCallback = within(nodeDetail).getByRole('button', {
      name: /lookup_weather.*14 tokens/
    });
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
    expect(within(nodeDetail).getByLabelText('工具调用 JSON')).toHaveTextContent(
      'call_usage'
    );
    expect(within(nodeDetail).getByLabelText('完整回调 JSON')).toHaveTextContent(
      'result_context_usage'
    );
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
