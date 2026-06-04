import { fireEvent, screen, waitFor, within } from '@testing-library/react';
import { beforeEach, describe, expect, test, vi } from 'vitest';

import { createDefaultAgentFlowDocument } from '@1flowbase/flow-schema';
import { modelProviderOptionsContract } from '../../../../test/model-provider-contract-fixtures';
import { TemplatedNamedBindingsField } from '../../components/bindings/TemplatedNamedBindingsField';
import { NodeDetailPanel } from '../../components/detail/NodeDetailPanel';
import { NodeConfigTab } from '../../components/detail/tabs/NodeConfigTab';
import { NodeInspector } from '../../components/inspector/NodeInspector';
import * as nodeSchemaAdapterApi from '../../schema/node-schema-adapter';
import * as nodeSchemaRegistry from '../../schema/node-schema-registry';
import { validateDocument } from '../../lib/validate-document';
import { AgentFlowEditorStoreProvider } from '../../store/editor/AgentFlowEditorStoreProvider';
import {
  DocumentObserver,
  FocusIssueSeed,
  SelectionSeed,
  createInitialState,
  createInitialStateWithCodeNode,
  createAgentFlowNodeSchemaAdapterSpy,
  fetchModelProviderOptionsSpy,
  getLlmNodeConfig,
  primaryProviderFirstModel,
  primaryProviderOption,
  renderWithProviders,
  resolveAgentFlowNodeSchemaSpy,
  setupNodeInspectorTest
} from './support';

beforeEach(setupNodeInspectorTest);

describe('NodeInspector core', () => {
  test('reads config sections through the node schema registry and adapter bridge', async () => {
    renderWithProviders(
      <AgentFlowEditorStoreProvider initialState={createInitialState()}>
        <SelectionSeed nodeId="node-llm" />
        <NodeInspector />
      </AgentFlowEditorStoreProvider>
    );

    await waitFor(() => {
      expect(screen.getByLabelText('USER 消息内容')).toHaveAttribute(
        'contenteditable',
        'true'
      );
    });
    expect(resolveAgentFlowNodeSchemaSpy).toHaveBeenCalledWith('llm');
    expect(createAgentFlowNodeSchemaAdapterSpy).toHaveBeenCalledTimes(1);
  });

  test('renders config sections as always-open blocks without repeating basics once summary content moves out', async () => {
    renderWithProviders(
      <AgentFlowEditorStoreProvider initialState={createInitialState()}>
        <SelectionSeed nodeId="node-llm" />
        <NodeInspector />
      </AgentFlowEditorStoreProvider>
    );

    await waitFor(() => {
      expect(screen.getByLabelText('USER 消息内容')).toHaveAttribute(
        'contenteditable',
        'true'
      );
    });
    expect(
      screen.queryByRole('button', { name: 'Inputs' })
    ).not.toBeInTheDocument();
    expect(
      screen.queryByRole('button', { name: 'Policy' })
    ).not.toBeInTheDocument();
    expect(
      screen.queryByRole('button', { name: 'Advanced' })
    ).not.toBeInTheDocument();
    expect(
      screen.queryByRole('combobox', { name: 'USER 消息内容' })
    ).not.toBeInTheDocument();
    expect(screen.queryByText('Basics')).not.toBeInTheDocument();
    expect(screen.queryByLabelText('节点别名')).not.toBeInTheDocument();
    expect(screen.queryByLabelText('节点简介')).not.toBeInTheDocument();
    expect(screen.queryByText('Inputs')).not.toBeInTheDocument();
    expect(screen.queryByText('Outputs')).not.toBeInTheDocument();
    expect(screen.queryByText('Advanced')).not.toBeInTheDocument();
    expect(screen.queryByText('LLM 参数')).not.toBeInTheDocument();
    expect(screen.queryByText('返回格式')).not.toBeInTheDocument();
    expect(screen.getByLabelText('失败重试')).toBeInTheDocument();
    expect(
      screen.getByRole('combobox', { name: '异常处理' })
    ).toBeInTheDocument();
    expect(screen.getByLabelText('SYSTEM 消息内容')).toBeInTheDocument();
    expect(screen.getByLabelText('USER 消息内容').tagName).toBe('DIV');
    expect(screen.getByLabelText('USER 消息内容')).toHaveAttribute(
      'contenteditable',
      'true'
    );
  }, 10000);

  test('updates node identity through header interactions instead of mutating document inline', () => {
    let latestDocument = createDefaultAgentFlowDocument({ flowId: 'flow-1' });

    renderWithProviders(
      <AgentFlowEditorStoreProvider initialState={createInitialState()}>
        <SelectionSeed nodeId="node-start" />
        <DocumentObserver
          onChange={(document) => {
            latestDocument = document;
          }}
        />
        <NodeDetailPanel onClose={vi.fn()} onRunNode={undefined} />
      </AgentFlowEditorStoreProvider>
    );

    const header = screen.getByTestId('node-detail-header');

    fireEvent.change(within(header).getByLabelText('节点别名'), {
      target: { value: '入口节点' }
    });
    fireEvent.change(within(header).getByLabelText('节点简介'), {
      target: { value: '收集首轮用户输入并启动工作流。' }
    });

    expect(within(header).getByLabelText('节点别名')).toHaveValue('入口节点');
    expect(within(header).getByLabelText('节点简介')).toHaveValue(
      '收集首轮用户输入并启动工作流。'
    );
    expect(latestDocument.graph.nodes).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          id: 'node-start',
          alias: '入口节点',
          description: '收集首轮用户输入并启动工作流。'
        })
      ])
    );
  });

  test('keeps issue-driven focus working after the inspector loses its header chrome', async () => {
    renderWithProviders(
      <AgentFlowEditorStoreProvider initialState={createInitialState()}>
        <FocusIssueSeed />
        <NodeConfigTab />
      </AgentFlowEditorStoreProvider>
    );

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /模型|model/ })).toHaveFocus();
    });
  });

  test('shows the effective context in the model summary trigger', async () => {
    const state = createInitialState();
    const llmNodeConfig = getLlmNodeConfig(state.draft.document);

    llmNodeConfig.model_provider = {
      provider_code: primaryProviderOption.provider_code,
      model_id: primaryProviderFirstModel.model_id,
      provider_label: primaryProviderOption.display_name,
      model_label: primaryProviderFirstModel.display_name
    };
    fetchModelProviderOptionsSpy.mockResolvedValue(
      modelProviderOptionsContract
    );

    renderWithProviders(
      <AgentFlowEditorStoreProvider initialState={state}>
        <SelectionSeed nodeId="node-llm" />
        <NodeConfigTab />
      </AgentFlowEditorStoreProvider>
    );

    const modelTrigger = await screen.findByRole('button', { name: /模型|model/ });

    await waitFor(() => {
      expect(
        within(modelTrigger).getByLabelText('上下文 128K')
      ).toBeInTheDocument();
    });
  });

  test('shows LLM generated outputs without exposing output contract editing', async () => {
    renderWithProviders(
      <AgentFlowEditorStoreProvider initialState={createInitialState()}>
        <SelectionSeed nodeId="node-llm" />
        <NodeConfigTab />
      </AgentFlowEditorStoreProvider>
    );

    expect(await screen.findByText('输出变量')).toBeInTheDocument();
    expect(screen.getByText('text')).toBeInTheDocument();
    expect(screen.getByText('usage')).toBeInTheDocument();
    expect(screen.queryByText('reasoning_content')).not.toBeInTheDocument();
    expect(screen.queryByText('节点产出的数据字段')).not.toBeInTheDocument();
    expect(
      screen.queryByRole('button', { name: '新增输出变量' })
    ).not.toBeInTheDocument();
    expect(screen.queryByLabelText('输出变量名 1')).not.toBeInTheDocument();
  });

  test('renders field validation errors under the owning inspector field', () => {
    const state = createInitialState();
    const answerNode = state.draft.document.graph.nodes.find(
      (node) => node.id === 'node-answer'
    );

    if (!answerNode) {
      throw new Error('expected default Answer node');
    }

    answerNode.bindings.answer_template = {
      kind: 'templated_text',
      value: '{{node-llm.text}}\n----\n{{node-llm-1.text}}'
    };

    const schema = nodeSchemaRegistry.resolveAgentFlowNodeSchema('answer');
    const adapter = nodeSchemaAdapterApi.createAgentFlowNodeSchemaAdapter({
      document: state.draft.document,
      nodeId: 'node-answer',
      issues: validateDocument(state.draft.document),
      setWorkingDocument: vi.fn(),
      dispatch: vi.fn()
    });

    renderWithProviders(
      <AgentFlowEditorStoreProvider initialState={state}>
        <NodeInspector schema={schema} adapter={adapter} />
      </AgentFlowEditorStoreProvider>
    );

    const field = screen.getByTestId(
      'inspector-field-bindings.answer_template'
    );

    expect(field).toHaveClass('agent-flow-editor__inspector-field--error');
    expect(
      within(field).getByText(
        '当前 binding 引用了已删除节点 node-llm-1 的输出。'
      )
    ).toBeInTheDocument();
  });

  test('keeps code output contract definition editable without rendering the shared output contract card', async () => {
    renderWithProviders(
      <AgentFlowEditorStoreProvider
        initialState={createInitialStateWithCodeNode()}
      >
        <SelectionSeed nodeId="node-code" />
        <NodeConfigTab />
      </AgentFlowEditorStoreProvider>
    );

    expect(
      await screen.findByLabelText(/JavaScript 代码|JavaScript code/)
    ).toBeInTheDocument();
    expect(screen.queryByText('JavaScript 代码')).not.toBeInTheDocument();
    expect(screen.queryByText('输出契约')).not.toBeInTheDocument();
    expect(
      screen.getByRole('button', { name: '新增变量' })
    ).toBeInTheDocument();
    expect(screen.queryByLabelText('代码结果')).not.toBeInTheDocument();
  });

  test('keeps Code boolean input selector values visible in the single value column', () => {
    renderWithProviders(
      <TemplatedNamedBindingsField
        ariaLabel="inputs"
        options={[
          {
            nodeId: 'node-start',
            nodeLabel: 'Start',
            outputKey: 'approved',
            outputLabel: 'approved',
            valueType: 'boolean',
            value: ['node-start', 'approved'],
            displayLabel: 'Start.approved'
          }
        ]}
        value={[
          {
            name: 'approved',
            valueType: 'boolean',
            value: {
              kind: 'selector',
              selector: ['node-start', 'approved']
            }
          }
        ]}
        onChange={vi.fn()}
      />
    );

    expect(screen.getByText('Start.approved')).toBeInTheDocument();
    expect(
      screen.queryByLabelText('inputs-0-value-mode')
    ).not.toBeInTheDocument();
  });

  test('adds Code input rows without preselecting a parameter type', () => {
    const handleChange = vi.fn();

    renderWithProviders(
      <TemplatedNamedBindingsField
        ariaLabel="inputs"
        options={[]}
        value={[]}
        onChange={handleChange}
      />
    );

    fireEvent.click(screen.getByRole('button', { name: '新增变量' }));

    expect(handleChange).toHaveBeenCalledWith([
      {
        name: '',
        value: { kind: 'constant', value: '' }
      }
    ]);
  });
});
