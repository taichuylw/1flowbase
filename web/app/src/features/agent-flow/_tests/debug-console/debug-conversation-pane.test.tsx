import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, test, vi } from 'vitest';

import type {
  AgentFlowDebugMessage,
  AgentFlowRunContext
} from '../../api/runtime';
import { DebugConversationPane } from '../../components/debug-console/conversation/DebugConversationPane';

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
      value: '你好'
    }
  ]
};

function assistantMessage(content: string): AgentFlowDebugMessage {
  return {
    id: 'assistant-1',
    role: 'assistant',
    status: 'running',
    runId: 'run-1',
    content,
    rawOutput: null,
    traceSummary: []
  };
}

function renderPane(messages: AgentFlowDebugMessage[]) {
  return render(
    <DebugConversationPane
      messages={messages}
      runContext={runContext}
      status="running"
      stopping={false}
      onChangeQuery={vi.fn()}
      onStopRun={vi.fn()}
      onSubmitPrompt={vi.fn()}
    />
  );
}

function configureScrollMetrics(element: HTMLElement) {
  Object.defineProperty(element, 'clientHeight', {
    configurable: true,
    value: 120
  });
  Object.defineProperty(element, 'scrollHeight', {
    configurable: true,
    value: 360
  });
}

describe('DebugConversationPane auto scroll', () => {
  test('keeps streamed output pinned to the bottom until the user scrolls', () => {
    const { rerender } = renderPane([assistantMessage('你好')]);
    const messagesElement = screen.getByTestId(
      'debug-conversation-messages'
    );
    configureScrollMetrics(messagesElement);
    const bottomElement = screen.getByTestId(
      'debug-conversation-bottom-sentinel'
    );
    const scrollIntoViewSpy = vi.fn();
    bottomElement.scrollIntoView = scrollIntoViewSpy;

    rerender(
      <DebugConversationPane
        messages={[assistantMessage('你好，正在输出更多内容')]}
        runContext={runContext}
        status="running"
        stopping={false}
        onChangeQuery={vi.fn()}
        onStopRun={vi.fn()}
        onSubmitPrompt={vi.fn()}
      />
    );

    expect(scrollIntoViewSpy).toHaveBeenCalled();

    messagesElement.scrollTop = 40;
    messagesElement.dispatchEvent(new Event('scroll', { bubbles: true }));

    rerender(
      <DebugConversationPane
        messages={[assistantMessage('你好，正在输出更多内容，继续追加')]}
        runContext={runContext}
        status="running"
        stopping={false}
        onChangeQuery={vi.fn()}
        onStopRun={vi.fn()}
        onSubmitPrompt={vi.fn()}
      />
    );

    expect(scrollIntoViewSpy).toHaveBeenCalledTimes(2);

    messagesElement.dispatchEvent(new WheelEvent('wheel', { bubbles: true }));

    rerender(
      <DebugConversationPane
        messages={[assistantMessage('你好，正在输出更多内容，继续追加')]}
        runContext={runContext}
        status="running"
        stopping={false}
        onChangeQuery={vi.fn()}
        onStopRun={vi.fn()}
        onSubmitPrompt={vi.fn()}
      />
    );

    expect(scrollIntoViewSpy).toHaveBeenCalledTimes(2);
  });
});

describe('DebugConversationPane workflow trace', () => {
  test('renders LLM tool callback rounds from trace debug payload', () => {
    renderPane([
      {
        ...assistantMessage('等待工具结果'),
        status: 'waiting_callback',
        traceSummary: [
          {
            nodeId: 'node-llm',
            nodeRunId: 'node-run-llm',
            nodeAlias: 'LLM',
            nodeType: 'llm',
            status: 'waiting_callback',
            startedAt: '2026-04-25T10:00:01Z',
            finishedAt: null,
            durationMs: null,
            inputPayload: {
              prompt: '天气?'
            },
            outputPayload: {
              tool_calls: [
                {
                  id: 'call_weather',
                  name: 'lookup_weather'
                }
              ]
            },
            errorPayload: null,
            metricsPayload: {},
            debugPayload: {
              llm_rounds: [
                {
                  round_index: 0,
                  assistant: {
                    role: 'assistant',
                    content: 'need tool',
                    tool_calls: [
                      {
                        id: 'call_weather',
                        name: 'lookup_weather'
                      }
                    ]
                  },
                  finish_reason: 'tool_call'
                }
              ]
            }
          }
        ]
      }
    ]);

    expect(screen.getByText('工作流')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: /LLM/ }));

    expect(screen.getByText('Round #1')).toBeInTheDocument();
    expect(screen.getByLabelText('LLM 回合')).toHaveTextContent(
      'lookup_weather'
    );
  });
});
