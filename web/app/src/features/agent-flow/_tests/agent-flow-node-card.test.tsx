import { readFileSync } from 'node:fs';

import { act, fireEvent, render, screen, within } from '@testing-library/react';
import { describe, expect, test, vi } from 'vitest';

import { AppProviders } from '../../../app/AppProviders';
import { AgentFlowNodeCard } from '../components/nodes/AgentFlowNodeCard';
import type { NodePickerOption } from '../lib/plugin-node-definitions';
import { resolveAgentFlowNodeSchema } from '../schema/node-schema-registry';

vi.mock('@xyflow/react', () => ({
  Handle: ({
    children,
    className,
    ...props
  }: {
    children?: React.ReactNode;
    className?: string;
    role?: string;
    tabIndex?: number;
    ['aria-label']?: string;
    onClick?: (event: React.MouseEvent<HTMLDivElement>) => void;
    onKeyDown?: (event: React.KeyboardEvent<HTMLDivElement>) => void;
  }) => {
    const domProps = { ...(props as Record<string, unknown>) };

    delete domProps.type;
    delete domProps.position;

    return (
      <div className={`react-flow__handle ${className ?? ''}`} {...domProps}>
        {children}
      </div>
    );
  },
  Position: {
    Left: 'left',
    Right: 'right'
  }
}));

describe('AgentFlowNodeCard', () => {
  test('keeps node color on the shell theme instead of per-type selectors', () => {
    const canvasStyles = readFileSync(
      'src/features/agent-flow/components/editor/styles/canvas.css',
      'utf8'
    );

    expect(canvasStyles).toContain('.agent-flow-node-card--theme-unified');
    expect(canvasStyles).toContain('--node-accent: #1677ff');

    render(
      <AppProviders>
        <AgentFlowNodeCard
          {...({
            data: {
              nodeId: 'node-data-model',
              nodeType: 'data_model_list',
              nodeSchema: resolveAgentFlowNodeSchema('data_model_list'),
              typeLabel: 'Data Model List',
              alias: 'Data Model List',
              description: 'List records from a Data Model runtime.',
              config: {},
              issueCount: 0,
              canEnterContainer: false,
              pickerOpen: false,
              showTargetHandle: true,
              showSourceHandle: true,
              isContainer: false,
              onOpenPicker: vi.fn(),
              onClosePicker: vi.fn(),
              onOpenContainer: vi.fn(),
              onSelectNode: vi.fn(),
              onInsertNode: vi.fn()
            },
            id: 'node-data-model',
            selected: false
          } as unknown as Parameters<typeof AgentFlowNodeCard>[0])}
        />
      </AppProviders>
    );

    const card = screen.getByRole('button', { name: /database Data Model List/ });

    expect(card).toHaveClass('agent-flow-node-card--theme-unified');
    expect(card).toHaveClass('agent-flow-node-card--type-data_model_list');
  });

  test('keeps the answer node on the unified blue node-card theme with a header icon', () => {
    const canvasStyles = readFileSync(
      'src/features/agent-flow/components/editor/styles/canvas.css',
      'utf8'
    );

    expect(canvasStyles).toContain('.agent-flow-node-card--theme-unified');

    render(
      <AppProviders>
        <AgentFlowNodeCard
          {...({
            data: {
              nodeId: 'node-answer',
              nodeType: 'answer',
              nodeSchema: resolveAgentFlowNodeSchema('answer'),
              typeLabel: 'Answer',
              alias: 'Answer',
              description: '向最终用户输出本轮工作流的回复结果。',
              config: {},
              issueCount: 3,
              canEnterContainer: false,
              pickerOpen: false,
              showTargetHandle: true,
              showSourceHandle: true,
              isContainer: false,
              onOpenPicker: vi.fn(),
              onClosePicker: vi.fn(),
              onOpenContainer: vi.fn(),
              onSelectNode: vi.fn(),
              onInsertNode: vi.fn()
            },
            id: 'node-answer',
            selected: false
          } as unknown as Parameters<typeof AgentFlowNodeCard>[0])}
        />
      </AppProviders>
    );

    const card = screen.getByRole('button', { name: /message Answer/ });

    expect(card).toHaveClass('agent-flow-node-card--theme-unified');
    expect(card).toHaveClass('agent-flow-node-card--type-answer');
    expect(
      within(card).getByRole('img', { name: 'message' })
    ).toBeInTheDocument();
    expect(screen.queryByText('3')).not.toBeInTheDocument();
  });

  test('renders Code nodes with a code SVG icon', () => {
    render(
      <AppProviders>
        <AgentFlowNodeCard
          {...({
            data: {
              nodeId: 'node-code',
              nodeType: 'code',
              nodeSchema: resolveAgentFlowNodeSchema('code'),
              typeLabel: 'Code',
              alias: 'Code',
              description: '执行自定义代码并返回结构化结果。',
              config: {},
              issueCount: 0,
              canEnterContainer: false,
              pickerOpen: false,
              showTargetHandle: true,
              showSourceHandle: true,
              isContainer: false,
              onOpenPicker: vi.fn(),
              onClosePicker: vi.fn(),
              onOpenContainer: vi.fn(),
              onSelectNode: vi.fn(),
              onInsertNode: vi.fn()
            },
            id: 'node-code',
            selected: false
          } as unknown as Parameters<typeof AgentFlowNodeCard>[0])}
        />
      </AppProviders>
    );

    const card = screen.getByRole('button', { name: /code Code/ });

    expect(card).toHaveClass('agent-flow-node-card--type-code');
    expect(within(card).getByRole('img', { name: 'code' })).toBeInTheDocument();
  });

  test('uses the source handle itself as the add-node trigger instead of nesting a separate button', () => {
    const onOpenPicker = vi.fn();

    render(
      <AppProviders>
        <AgentFlowNodeCard
          {...({
            data: {
              nodeId: 'node-llm',
              nodeType: 'llm',
              nodeSchema: resolveAgentFlowNodeSchema('llm'),
              typeLabel: 'LLM',
              alias: 'LLM',
              description: '选择并调用大语言模型',
              config: {
                model_provider: {
                  provider_code: 'openai_compatible',
                  model_id: 'gpt-4',
                  provider_label: 'OpenAI Prod',
                  model_label: 'GPT-4'
                }
              },
              issueCount: 0,
              canEnterContainer: false,
              pickerOpen: false,
              showTargetHandle: true,
              showSourceHandle: true,
              isContainer: false,
              onOpenPicker,
              onClosePicker: vi.fn(),
              onOpenContainer: vi.fn(),
              onSelectNode: vi.fn(),
              onInsertNode: vi.fn()
            },
            id: 'node-llm',
            selected: false
          } as unknown as Parameters<typeof AgentFlowNodeCard>[0])}
        />
      </AppProviders>
    );

    const card = screen.getByRole('button', { name: /LLM OpenAI Prod GPT-4/ });
    const trigger = screen.getByRole('button', { name: '在 LLM 后新增节点' });
    expect(card).toBeInTheDocument();
    expect(screen.getByText('LLM')).toBeInTheDocument();
    expect(screen.getByText('GPT-4')).toBeInTheDocument();
    expect(screen.getByText('OpenAI Prod')).toBeInTheDocument();
    expect(screen.queryByText('选择并调用大语言模型')).not.toBeInTheDocument();

    expect(trigger).toHaveClass('react-flow__handle');
    expect(within(trigger).queryByRole('button')).not.toBeInTheDocument();

    fireEvent.click(trigger);

    expect(onOpenPicker).toHaveBeenCalledWith('node-llm');
  });

  test('routes If / Else branch handles through picker open and insert callbacks', async () => {
    const onOpenPicker = vi.fn();
    const onInsertNode = vi.fn();
    const codeOption = {
      kind: 'builtin',
      type: 'code',
      label: 'Code',
      description: 'Run code',
      category: 'data',
      inputKeys: [],
      outputKeys: []
    } satisfies NodePickerOption;
    const baseData = {
      nodeId: 'node-if-else',
      nodeType: 'if_else',
      nodeSchema: resolveAgentFlowNodeSchema('if_else'),
      typeLabel: 'If / Else',
      alias: 'If / Else',
      description: '按条件分支继续执行工作流。',
      config: {},
      issueCount: 0,
      canEnterContainer: false,
      pickerOpen: false,
      pickerSourceHandleId: null,
      showTargetHandle: true,
      showSourceHandle: true,
      branchSourceHandles: [
        { id: 'if', title: 'If' },
        { id: 'else', title: 'Else' }
      ],
      isContainer: false,
      nodePickerOptions: [codeOption],
      onOpenPicker,
      onClosePicker: vi.fn(),
      onOpenContainer: vi.fn(),
      onSelectNode: vi.fn(),
      onInsertNode,
      onRunNode: vi.fn(),
      onReplaceNode: vi.fn(),
      onDeleteNode: vi.fn()
    };

    const { rerender } = render(
      <AppProviders>
        <AgentFlowNodeCard
          {...({
            data: baseData,
            id: 'node-if-else',
            selected: false
          } as unknown as Parameters<typeof AgentFlowNodeCard>[0])}
        />
      </AppProviders>
    );

    const ifHandle = screen.getByRole('button', {
      name: '在 If / Else 的 If 分支后新增节点'
    });
    const elseHandle = screen.getByRole('button', {
      name: '在 If / Else 的 Else 分支后新增节点'
    });

    expect(ifHandle).toHaveClass('agent-flow-node-handle--branch');
    expect(elseHandle).toHaveClass('agent-flow-node-handle--branch');
    expect(within(ifHandle).getByText('If')).toBeInTheDocument();
    expect(within(elseHandle).getByText('Else')).toBeInTheDocument();

    fireEvent.click(ifHandle);

    expect(onOpenPicker).toHaveBeenCalledWith('node-if-else', 'if');

    rerender(
      <AppProviders>
        <AgentFlowNodeCard
          {...({
            data: {
              ...baseData,
              pickerOpen: true,
              pickerSourceHandleId: 'if'
            },
            id: 'node-if-else',
            selected: false
          } as unknown as Parameters<typeof AgentFlowNodeCard>[0])}
        />
      </AppProviders>
    );

    fireEvent.click(await screen.findByRole('menuitem', { name: 'Code' }));

    expect(onInsertNode).toHaveBeenCalledWith(
      'node-if-else',
      codeOption,
      'if'
    );
  });

  test('shows hover quick actions for running, replacing and deleting a node', async () => {
    const onRunNode = vi.fn();
    const onReplaceNode = vi.fn();
    const onDeleteNode = vi.fn();

    render(
      <AppProviders>
        <AgentFlowNodeCard
          {...({
            data: {
              nodeId: 'node-tool',
              nodeType: 'tool',
              nodeSchema: resolveAgentFlowNodeSchema('tool'),
              typeLabel: 'Tool',
              alias: 'Tool',
              description: '调用外部工具能力并返回工具执行结果。',
              config: {},
              issueCount: 0,
              canEnterContainer: false,
              pickerOpen: false,
              showTargetHandle: true,
              showSourceHandle: true,
              isContainer: false,
              nodePickerOptions: [
                { kind: 'builtin', type: 'llm', label: 'LLM' },
                {
                  kind: 'builtin',
                  type: 'template_transform',
                  label: 'Template Transform'
                }
              ],
              onOpenPicker: vi.fn(),
              onClosePicker: vi.fn(),
              onOpenContainer: vi.fn(),
              onSelectNode: vi.fn(),
              onInsertNode: vi.fn(),
              onRunNode,
              onReplaceNode,
              onDeleteNode
            },
            id: 'node-tool',
            selected: false
          } as unknown as Parameters<typeof AgentFlowNodeCard>[0])}
        />
      </AppProviders>
    );

    fireEvent.click(screen.getByRole('button', { name: '执行 Tool' }));
    expect(onRunNode).toHaveBeenCalledWith('node-tool');

    fireEvent.click(screen.getByRole('button', { name: 'Tool 更多操作' }));
    expect(
      await screen.findByRole('menuitem', { name: /执行此节点/ })
    ).toBeInTheDocument();
    expect(
      screen.getByRole('menuitem', { name: /更换节点/ })
    ).toBeInTheDocument();

    fireEvent.mouseEnter(screen.getByRole('menuitem', { name: /更换节点/ }));
    fireEvent.click(await screen.findByRole('menuitem', { name: 'LLM' }));
    expect(onReplaceNode).toHaveBeenCalledWith('node-tool', {
      kind: 'builtin',
      type: 'llm',
      label: 'LLM'
    });

    fireEvent.click(screen.getByRole('button', { name: 'Tool 更多操作' }));
    fireEvent.click(await screen.findByRole('menuitem', { name: /删除节点/ }));
    expect(onDeleteNode).toHaveBeenCalledWith('node-tool');
  });

  test('keeps hover quick actions visible for one second after leaving the node', () => {
    vi.useFakeTimers();

    try {
      render(
        <AppProviders>
          <AgentFlowNodeCard
            {...({
              data: {
                nodeId: 'node-delay',
                nodeType: 'tool',
                nodeSchema: resolveAgentFlowNodeSchema('tool'),
                typeLabel: 'Tool',
                alias: 'Tool',
                description: '调用外部工具能力并返回工具执行结果。',
                config: {},
                issueCount: 0,
                canEnterContainer: false,
                pickerOpen: false,
                showTargetHandle: true,
                showSourceHandle: true,
                isContainer: false,
                nodePickerOptions: [],
                onOpenPicker: vi.fn(),
                onClosePicker: vi.fn(),
                onOpenContainer: vi.fn(),
                onSelectNode: vi.fn(),
                onInsertNode: vi.fn(),
                onRunNode: vi.fn(),
                onReplaceNode: vi.fn(),
                onDeleteNode: vi.fn()
              },
              id: 'node-delay',
              selected: false
            } as unknown as Parameters<typeof AgentFlowNodeCard>[0])}
          />
        </AppProviders>
      );

      const quickActions = screen.getByTestId(
        'agent-flow-node-quick-actions-node-delay'
      );
      const card = screen.getByRole('button', {
        name: /调用外部工具能力/
      });

      expect(quickActions).not.toHaveClass(
        'agent-flow-node-card__quick-actions--visible'
      );

      fireEvent.mouseEnter(card as HTMLElement);
      expect(quickActions).toHaveClass(
        'agent-flow-node-card__quick-actions--visible'
      );

      fireEvent.mouseLeave(card as HTMLElement);
      act(() => {
        vi.advanceTimersByTime(999);
      });
      expect(quickActions).toHaveClass(
        'agent-flow-node-card__quick-actions--visible'
      );

      act(() => {
        vi.advanceTimersByTime(1);
      });
      expect(quickActions).not.toHaveClass(
        'agent-flow-node-card__quick-actions--visible'
      );
    } finally {
      vi.useRealTimers();
    }
  });
});
