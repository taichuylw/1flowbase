import { readFileSync } from 'node:fs';

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
  createInitialStateWithCustomCodeNode,
  createInitialStateWithDataModelNode,
  createInitialStateWithIfElseNode,
  createInitialStateWithLoopNode,
  createInitialStateWithStructuredCodeNode,
  createAgentFlowNodeSchemaAdapterSpy,
  fetchDataModelOptionsSpy,
  fetchModelProviderOptionsSpy,
  getCodeNode,
  getDataModelNode,
  getLlmNodeConfig,
  openSelect,
  primaryProviderFirstModel,
  primaryProviderOption,
  renderWithProviders,
  resolveAgentFlowNodeSchemaSpy,
  selectDataModelOption,
  selectOption,
  setupNodeInspectorTest
} from './support';

beforeEach(setupNodeInspectorTest);

describe('NodeInspector', () => {
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

  test('renders Code as input variables, JavaScript editor, then output variables and persists edits', async () => {
    const inspectorStyles = readFileSync(
      'src/features/agent-flow/components/editor/styles/inspector.css',
      'utf8'
    );
    let latestDocument = createDefaultAgentFlowDocument({ flowId: 'flow-1' });

    expect(inspectorStyles).toContain(
      'grid-template-columns: minmax(88px, 0.7fr) minmax(96px, 0.65fr) minmax(168px, 1.5fr) 28px;'
    );

    renderWithProviders(
      <AgentFlowEditorStoreProvider
        initialState={createInitialStateWithCustomCodeNode()}
      >
        <SelectionSeed nodeId="node-code" />
        <DocumentObserver
          onChange={(document) => {
            latestDocument = document;
          }}
        />
        <NodeConfigTab />
      </AgentFlowEditorStoreProvider>
    );

    expect(screen.queryByText('Advanced')).not.toBeInTheDocument();
    expect(
      screen.queryByRole('combobox', { name: '运行语言' })
    ).not.toBeInTheDocument();

    const inputField = screen.getByTestId(
      'inspector-field-bindings.named_bindings'
    );
    const sourceField = await screen.findByTestId(
      'inspector-field-config.source'
    );
    const outputField = screen.getByTestId(
      'inspector-field-config.output_contract'
    );

    expect(inputField.compareDocumentPosition(sourceField)).toBe(
      Node.DOCUMENT_POSITION_FOLLOWING
    );
    expect(sourceField.compareDocumentPosition(outputField)).toBe(
      Node.DOCUMENT_POSITION_FOLLOWING
    );
    expect(screen.getByLabelText(/输入变量-0-name|input variables-0-name/)).toHaveValue('arg1');
    expect(screen.getAllByLabelText(/输入变量-0-type|input variables-0-type/).length).toBeGreaterThan(0);
    expect(
      screen.queryByLabelText(/输入变量-0-value-mode|input variables-0-value-mode/)
    ).not.toBeInTheDocument();
    expect(screen.getAllByLabelText(/输入变量-0-value|input variables-0-value/).length).toBeGreaterThan(0);
    expect(
      screen.queryByLabelText(/输入变量-0-selector|input variables-0-selector/)
    ).not.toBeInTheDocument();
    expect(
      screen.queryByRole('button', { name: '复制arg1' })
    ).not.toBeInTheDocument();
    expect(
      screen.queryByRole('button', { name: '放大编辑arg1' })
    ).not.toBeInTheDocument();
    expect(
      screen.getByRole('button', { name: '删除变量 arg1' })
    ).toBeInTheDocument();
    const codeEditor = await screen.findByLabelText(/JavaScript 代码|JavaScript code/);

    expect(codeEditor).toHaveValue('return { riskScore: 0.82 };');
    expect(screen.getByLabelText('输出变量名 1')).toHaveValue('riskScore');
    expect(screen.queryByLabelText('输出显示名 1')).not.toBeInTheDocument();

    fireEvent.change(screen.getByLabelText(/输入变量-0-name|input variables-0-name/), {
      target: { value: 'score_1' }
    });
    fireEvent.change(codeEditor, {
      target: { value: 'return { risk_score: inputs.score };' }
    });
    fireEvent.change(screen.getByLabelText('输出变量名 1'), {
      target: { value: 'risk_score' }
    });

    await waitFor(() => {
      expect(getCodeNode(latestDocument).config).toMatchObject({
        language: 'javascript',
        source: 'return { risk_score: inputs.score };'
      });
      expect(getCodeNode(latestDocument).bindings.named_bindings).toEqual({
        kind: 'named_bindings',
        value: [
          {
            name: 'score_1',
            valueType: 'string',
            value: {
              kind: 'templated_text',
              value: '{{sys.conversation_id}}'
            }
          }
        ]
      });
      expect(getCodeNode(latestDocument).outputs).toEqual([
        {
          key: 'risk_score',
          title: 'risk_score',
          valueType: 'number',
          selector: ['result', 'risk_score']
        }
      ]);
    });
  });

  test('renders JSON schema controls for structured code outputs', async () => {
    let latestDocument = createInitialStateWithStructuredCodeNode().draft
      .document;

    renderWithProviders(
      <AgentFlowEditorStoreProvider
        initialState={{
          flow_id: 'flow-1',
          draft: {
            id: 'draft-1',
            flow_id: 'flow-1',
            updated_at: '2026-04-16T10:00:00Z',
            document: latestDocument
          },
          autosave_interval_seconds: 30,
          versions: []
        }}
      >
        <SelectionSeed nodeId="node-code" />
        <DocumentObserver
          onChange={(document) => {
            latestDocument = document;
          }}
        />
        <NodeConfigTab />
      </AgentFlowEditorStoreProvider>
    );

    expect(await screen.findByLabelText('输出变量名 1')).toHaveValue(
      'chat_history'
    );
    fireEvent.click(screen.getByRole('button', { name: '编辑 JSON Schema' }));

    const dialog = await screen.findByRole('dialog', { name: 'JSON Schema' });
    expect(dialog).toHaveClass('agent-flow-model-settings__panel');
    expect(screen.getByRole('tab', { name: 'Schema 字段' })).toBeInTheDocument();
    expect(screen.getByRole('tab', { name: 'JSON 解析' })).toBeInTheDocument();
    expect(screen.getByLabelText('Schema 字段名 1')).toHaveValue('role');
    expect(screen.getByLabelText('Schema 字段名 2')).toHaveValue('content');

    fireEvent.click(screen.getByRole('button', { name: '添加 Schema 字段' }));
    fireEvent.change(screen.getByLabelText('Schema 字段名 3'), {
      target: { value: 'metadata' }
    });
    await openSelect('Schema 字段类型 3');
    await selectOption('Object');
    fireEvent.click(screen.getByRole('button', { name: '添加 metadata 子字段' }));
    fireEvent.change(screen.getByLabelText('Schema 字段名 3.1'), {
      target: { value: 'source' }
    });
    fireEvent.click(screen.getByRole('button', { name: '保存' }));

    await waitFor(() => {
      expect(getCodeNode(latestDocument).outputs[0].jsonSchema).toMatchObject({
        type: 'array',
        items: {
          type: 'object',
          required: ['role', 'content', 'metadata'],
          properties: {
            role: { type: 'string' },
            content: { type: 'string' },
            metadata: {
              type: 'object',
              required: ['source'],
              properties: {
                source: { type: 'string' }
              }
            }
          }
        }
      });
      expect(getCodeNode(latestDocument).outputs[0]).toMatchObject({
        key: 'chat_history',
        title: 'chat_history',
        selector: ['result', 'chat_history']
      });
    });
  });

  test('parses JSON Schema from highlighted code mode', async () => {
    let latestDocument = createInitialStateWithStructuredCodeNode().draft
      .document;

    renderWithProviders(
      <AgentFlowEditorStoreProvider
        initialState={{
          flow_id: 'flow-1',
          draft: {
            id: 'draft-1',
            flow_id: 'flow-1',
            updated_at: '2026-04-16T10:00:00Z',
            document: latestDocument
          },
          autosave_interval_seconds: 30,
          versions: []
        }}
      >
        <SelectionSeed nodeId="node-code" />
        <DocumentObserver
          onChange={(document) => {
            latestDocument = document;
          }}
        />
        <NodeConfigTab />
      </AgentFlowEditorStoreProvider>
    );

    expect(await screen.findByLabelText('输出变量名 1')).toHaveValue(
      'chat_history'
    );
    fireEvent.click(screen.getByRole('button', { name: '编辑 JSON Schema' }));
    fireEvent.click(await screen.findByRole('tab', { name: 'JSON 解析' }));
    fireEvent.change(await screen.findByLabelText('JSON Schema 内容'), {
      target: {
        value: JSON.stringify(
          {
            type: 'object',
            properties: {
              summary: { type: 'string' }
            },
            required: ['summary']
          },
          null,
          2
        )
      }
    });
    fireEvent.click(screen.getByRole('button', { name: '保存' }));

    await waitFor(() => {
      expect(getCodeNode(latestDocument).outputs[0]).toMatchObject({
        valueType: 'object',
        jsonSchema: {
          type: 'object',
          properties: {
            summary: { type: 'string' }
          },
          required: ['summary']
        }
      });
    });
  });

  test('keeps parsed JSON Schema root type when returning to field mode', async () => {
    const initialState = createInitialStateWithStructuredCodeNode();
    const codeNode = getCodeNode(initialState.draft.document);
    codeNode.outputs = [
      {
        key: 'chat_history',
        title: 'Chat History',
        valueType: 'object',
        jsonSchema: {
          type: 'object',
          required: [],
          properties: {}
        }
      }
    ];
    let latestDocument = initialState.draft.document;

    renderWithProviders(
      <AgentFlowEditorStoreProvider initialState={initialState}>
        <SelectionSeed nodeId="node-code" />
        <DocumentObserver
          onChange={(document) => {
            latestDocument = document;
          }}
        />
        <NodeConfigTab />
      </AgentFlowEditorStoreProvider>
    );

    fireEvent.click(
      await screen.findByRole('button', { name: '编辑 JSON Schema' })
    );
    fireEvent.click(await screen.findByRole('tab', { name: 'JSON 解析' }));
    fireEvent.change(await screen.findByLabelText('JSON Schema 内容'), {
      target: {
        value: JSON.stringify(
          {
            type: 'array',
            items: {
              type: 'object',
              properties: {
                role: { type: 'string' },
                content: { type: 'string' }
              },
              required: ['role', 'content']
            }
          },
          null,
          2
        )
      }
    });
    fireEvent.click(screen.getByRole('tab', { name: 'Schema 字段' }));

    await waitFor(() => {
      expect(screen.getByLabelText('Schema 字段名 1')).toHaveValue('role');
      expect(screen.getByLabelText('Schema 字段名 2')).toHaveValue('content');
    });
    fireEvent.click(screen.getByRole('button', { name: '保存' }));

    await waitFor(() => {
      expect(getCodeNode(latestDocument).outputs[0]).toMatchObject({
        valueType: 'array',
        jsonSchema: {
          type: 'array',
          items: {
            type: 'object',
            properties: {
              role: { type: 'string' },
              content: { type: 'string' }
            },
            required: ['role', 'content']
          }
        }
      });
    });
  });

  test('renders loop number fields in compact inline rows while keeping condition groups stacked', () => {
    renderWithProviders(
      <AgentFlowEditorStoreProvider
        initialState={createInitialStateWithLoopNode()}
      >
        <SelectionSeed nodeId="node-loop" />
        <NodeConfigTab />
      </AgentFlowEditorStoreProvider>
    );

    expect(screen.queryByText('Inputs')).not.toBeInTheDocument();
    expect(screen.getByText('Policy')).toBeInTheDocument();
    const toolbar = screen.getByTestId('condition-group-toolbar');

    expect(
      within(toolbar).getByRole('combobox', { name: /入口条件-operator|Entry conditions-operator/ })
    ).toBeInTheDocument();
    expect(
      within(toolbar).getByRole('button', { name: /新增条件$/ })
    ).toBeInTheDocument();
    expect(screen.getByTestId('inspector-field-config.max_rounds')).toHaveClass(
      'agent-flow-editor__inspector-field--inline'
    );
    expect(
      screen.getByTestId('inspector-field-bindings.entry_condition')
    ).not.toHaveClass('agent-flow-editor__inspector-field--inline');
  });

  test('keeps If / Else condition rule controls inside narrow inspector bounds', () => {
    const inspectorStyles = readFileSync(
      'src/features/agent-flow/components/editor/styles/inspector.css',
      'utf8'
    );

    expect(inspectorStyles).toContain(
      '.agent-flow-condition-group__rule {\n  display: flex;\n  flex-wrap: wrap;'
    );
    expect(inspectorStyles).toContain(
      '.agent-flow-condition-group__rule > :not(.agent-flow-binding-row__delete) {\n  flex: 1 1 136px;\n  min-width: 0;'
    );
    expect(inspectorStyles).toContain(
      '.agent-flow-condition-group__rule > .agent-flow-binding-row__delete {\n  flex: 0 0 28px;'
    );
  });

  test('edits If / Else branches as first-class branch handles', async () => {
    const initialState = createInitialStateWithIfElseNode();
    let latestDocument = initialState.draft.document;

    renderWithProviders(
      <AgentFlowEditorStoreProvider initialState={initialState}>
        <SelectionSeed nodeId="node-if-else" />
        <DocumentObserver
          onChange={(document) => {
            latestDocument = document;
          }}
        />
        <NodeConfigTab />
      </AgentFlowEditorStoreProvider>
    );

    const branchField = await screen.findByTestId(
      'inspector-field-bindings.branches'
    );

    expect(screen.getByTestId('if-else-branch-if')).toBeInTheDocument();
    expect(screen.getByTestId('if-else-branch-else')).toBeInTheDocument();
    expect(
      within(screen.getByTestId('if-else-branch-else')).getByLabelText(
        'Else 分支名称'
      )
    ).toBeDisabled();
    expect(
      screen.queryByRole('button', { name: '删除 Else 分支' })
    ).not.toBeInTheDocument();

    const addElseIfButton = within(branchField).getByTestId(
      'if-else-add-else-if'
    );
    expect(
      addElseIfButton.compareDocumentPosition(
        screen.getByTestId('if-else-branch-else')
      ) & Node.DOCUMENT_POSITION_FOLLOWING
    ).toBeTruthy();

    fireEvent.click(
      addElseIfButton
    );

    const elseIfBranch = await screen.findByTestId(
      'if-else-branch-else-if-1'
    );

    expect(
      within(elseIfBranch).getByLabelText('Else If 1 分支名称')
    ).toHaveValue('Else If 1');
    expect(
      within(elseIfBranch).getByRole('button', {
        name: '删除 Else If 1 分支'
      })
    ).toBeInTheDocument();

    fireEvent.click(
      within(screen.getByTestId('if-else-branch-if')).getByRole('button', {
        name: /新增条件组/
      })
    );

    expect(
      within(screen.getByTestId('if-else-branch-if')).getAllByTestId(
        'condition-group-toolbar'
      )
    ).toHaveLength(2);

    await waitFor(() => {
      const ifElseNode = latestDocument.graph.nodes.find(
        (node) => node.id === 'node-if-else'
      );
      const branchBinding = ifElseNode?.bindings.branches;

      if (!branchBinding || branchBinding.kind !== 'if_else_branches') {
        throw new Error('expected If / Else branch binding');
      }

      expect(branchBinding.value.branches).toEqual([
        expect.objectContaining({
          kind: 'if',
          sourceHandle: 'if',
          condition: {
            operator: 'and',
            conditions: [{ operator: 'and', conditions: [] }]
          }
        }),
        expect.objectContaining({
          id: 'else-if-1',
          kind: 'else_if',
          title: 'Else If 1',
          sourceHandle: 'else-if-1',
          condition: { operator: 'and', conditions: [] }
        }),
        expect.objectContaining({
          kind: 'else',
          sourceHandle: 'else'
        })
      ]);
    });
  });

  test('supports empty comparator in If / Else condition rules without right value', async () => {
    const initialState = createInitialStateWithIfElseNode();
    let latestDocument = initialState.draft.document;

    renderWithProviders(
      <AgentFlowEditorStoreProvider initialState={initialState}>
        <SelectionSeed nodeId="node-if-else" />
        <DocumentObserver
          onChange={(document) => {
            latestDocument = document;
          }}
        />
        <NodeConfigTab />
      </AgentFlowEditorStoreProvider>
    );

    const ifBranch = await screen.findByTestId('if-else-branch-if');

    fireEvent.click(
      within(ifBranch).getByRole('button', { name: /新增条件$/ })
    );

    await openSelect('分支-if-0-comparator');
    expect(await screen.findByTitle('为空')).toBeInTheDocument();
    await selectOption('为空');

    expect(
      screen.queryByRole('combobox', { name: '分支-if-0-right-kind' })
    ).not.toBeInTheDocument();
    expect(screen.queryByLabelText('分支-if-0-right')).not.toBeInTheDocument();

    await waitFor(() => {
      const ifElseNode = latestDocument.graph.nodes.find(
        (node) => node.id === 'node-if-else'
      );
      const branchBinding = ifElseNode?.bindings.branches;

      if (!branchBinding || branchBinding.kind !== 'if_else_branches') {
        throw new Error('expected If / Else branch binding');
      }

      const conditions = branchBinding.value.branches[0]?.condition?.conditions;

      expect(conditions).toEqual([
        expect.objectContaining({
          comparator: 'empty'
        })
      ]);
      expect(conditions?.[0]).not.toHaveProperty('right');
    });
  });

  test('stores typed fixed values in If / Else condition rules', async () => {
    const initialState = createInitialStateWithIfElseNode();
    const startNode = initialState.draft.document.graph.nodes.find(
      (node) => node.id === 'node-start'
    );

    if (!startNode) {
      throw new Error('expected Start node');
    }

    startNode.config.input_fields = [
      {
        key: 'amount',
        label: 'Amount',
        inputType: 'number',
        required: false
      }
    ];
    initialState.draft.document.graph.edges.push({
      id: 'edge-start-if-else',
      source: 'node-start',
      target: 'node-if-else',
      sourceHandle: null,
      targetHandle: null,
      containerId: null,
      points: []
    });
    let latestDocument = initialState.draft.document;

    renderWithProviders(
      <AgentFlowEditorStoreProvider initialState={initialState}>
        <SelectionSeed nodeId="node-if-else" />
        <DocumentObserver
          onChange={(document) => {
            latestDocument = document;
          }}
        />
        <NodeConfigTab />
      </AgentFlowEditorStoreProvider>
    );

    const ifBranch = await screen.findByTestId('if-else-branch-if');

    fireEvent.click(
      within(ifBranch).getByRole('button', { name: /新增条件$/ })
    );

    await openSelect('分支-if-0-left');
    await selectOption('Start');
    await selectOption('amount');
    await openSelect('分支-if-0-comparator');
    await selectOption('等于');
    const rightInput = await screen.findByLabelText('分支-if-0-right');

    rightInput.focus();
    fireEvent.change(rightInput, {
      target: { value: '5' }
    });

    await waitFor(() => {
      expect(screen.getByLabelText('分支-if-0-right')).toHaveFocus();
    });

    await waitFor(() => {
      const ifElseNode = latestDocument.graph.nodes.find(
        (node) => node.id === 'node-if-else'
      );
      const branchBinding = ifElseNode?.bindings.branches;

      if (!branchBinding || branchBinding.kind !== 'if_else_branches') {
        throw new Error('expected If / Else branch binding');
      }

      expect(
        branchBinding.value.branches[0]?.condition?.conditions[0]
      ).toMatchObject({
        left: ['node-start', 'amount'],
        comparator: 'equals',
        right: { kind: 'constant', value: 5 }
      });
    });
  });

  test('renders structured fixed values in If / Else condition rules as JSON text', async () => {
    const initialState = createInitialStateWithIfElseNode();
    const ifElseNode = initialState.draft.document.graph.nodes.find(
      (node) => node.id === 'node-if-else'
    );

    if (!ifElseNode) {
      throw new Error('expected If / Else node');
    }

    ifElseNode.bindings.branches = {
      kind: 'if_else_branches',
      value: {
        branches: [
          {
            id: 'if',
            kind: 'if',
            title: 'If',
            sourceHandle: 'if',
            condition: {
              operator: 'and',
              conditions: [
                {
                  kind: 'rule',
                  left: ['node-start', 'query'],
                  comparator: 'equals',
                  right: {
                    kind: 'constant',
                    value: { tier: 'gold' }
                  }
                }
              ]
            }
          },
          {
            id: 'else',
            kind: 'else',
            title: 'Else',
            sourceHandle: 'else'
          }
        ]
      }
    };

    renderWithProviders(
      <AgentFlowEditorStoreProvider initialState={initialState}>
        <SelectionSeed nodeId="node-if-else" />
        <NodeConfigTab />
      </AgentFlowEditorStoreProvider>
    );

    expect(await screen.findByLabelText('分支-if-0-right')).toHaveValue(
      '{"tier":"gold"}'
    );
  });

  test('loads Data Model options from the feature API and disables unavailable models', async () => {
    renderWithProviders(
      <AgentFlowEditorStoreProvider
        initialState={createInitialStateWithDataModelNode()}
      >
        <SelectionSeed nodeId="node-data-model" />
        <NodeConfigTab />
      </AgentFlowEditorStoreProvider>
    );

    await openSelect('Data Model');

    expect(fetchDataModelOptionsSpy).toHaveBeenCalledTimes(1);
    expect(
      await screen.findByTestId('data-model-option-orders')
    ).not.toHaveAttribute('aria-disabled', 'true');
    expect(
      screen.getByTestId('data-model-option-draft_orders')
    ).toHaveAttribute('aria-disabled', 'true');
    expect(
      screen.getByTestId('data-model-option-disabled_orders')
    ).toHaveAttribute('aria-disabled', 'true');
    expect(
      screen.getByTestId('data-model-option-broken_orders')
    ).toHaveAttribute('aria-disabled', 'true');
  });

  test('updates selected Data Model metadata when the selected model changes', async () => {
    let latestDocument = createDefaultAgentFlowDocument({ flowId: 'flow-1' });

    renderWithProviders(
      <AgentFlowEditorStoreProvider
        initialState={createInitialStateWithDataModelNode()}
      >
        <SelectionSeed nodeId="node-data-model" />
        <DocumentObserver
          onChange={(document) => {
            latestDocument = document;
          }}
        />
        <NodeConfigTab />
      </AgentFlowEditorStoreProvider>
    );

    await openSelect('Data Model');
    await selectDataModelOption('orders');

    await waitFor(() => {
      expect(getDataModelNode(latestDocument).config).toMatchObject({
        data_model_code: 'orders',
        data_model_id: 'model-orders',
        data_model_label: 'Orders',
        data_model_fields: [
          { code: 'name', title: 'Name', valueType: 'string', required: true },
          {
            code: 'amount',
            title: 'Amount',
            valueType: 'number',
            required: false
          },
          {
            code: 'status',
            title: 'Status',
            valueType: 'enum',
            required: false
          },
          {
            code: 'customer',
            title: 'Customer',
            valueType: 'many_to_one',
            required: false
          },
          {
            code: 'lines',
            title: 'Lines',
            valueType: 'one_to_many',
            required: false
          },
          {
            code: 'approved',
            title: 'Approved',
            valueType: 'boolean',
            required: false
          }
        ]
      });
    });
  });

  test('edits Data Model list query binding', async () => {
    let latestDocument = createDefaultAgentFlowDocument({ flowId: 'flow-1' });

    renderWithProviders(
      <AgentFlowEditorStoreProvider
        initialState={createInitialStateWithDataModelNode()}
      >
        <SelectionSeed nodeId="node-data-model" />
        <DocumentObserver
          onChange={(document) => {
            latestDocument = document;
          }}
        />
        <NodeConfigTab />
      </AgentFlowEditorStoreProvider>
    );

    expect(await screen.findByText('请先选择 Data Model')).toBeInTheDocument();

    await openSelect('Data Model');
    await selectDataModelOption('orders');
    fireEvent.click(
      await screen.findByRole('button', { name: '新增过滤条件' })
    );
    await openSelect('过滤字段 1');
    await selectOption('Status');
    await openSelect('过滤操作符 1');
    await selectOption('eq');
    await openSelect('过滤值来源 1');
    await selectOption('变量');
    await openSelect('过滤变量 1');
    await selectOption('Start');
    await selectOption('query');
    fireEvent.click(
      await screen.findByRole('button', { name: '新增过滤条件' })
    );
    await openSelect('过滤字段 2');
    await selectOption('Amount');
    await waitFor(() => {
      expect(getDataModelNode(latestDocument).bindings.query).toMatchObject({
        value: {
          filters: [
            expect.anything(),
            {
              field_code: 'amount',
              value: { kind: 'constant', value: 0 }
            }
          ]
        }
      });
    });
    fireEvent.change(screen.getByLabelText('过滤值 2'), {
      target: { value: '123' }
    });
    fireEvent.click(
      await screen.findByRole('button', { name: '新增过滤条件' })
    );
    await openSelect('过滤字段 3');
    await selectOption('Approved');
    await waitFor(() => {
      expect(getDataModelNode(latestDocument).bindings.query).toMatchObject({
        value: {
          filters: [
            expect.anything(),
            expect.anything(),
            {
              field_code: 'approved',
              value: { kind: 'constant', value: false }
            }
          ]
        }
      });
    });
    await openSelect('过滤值来源 3');
    await selectOption('变量');
    await openSelect('过滤值来源 3');
    await selectOption('常量');
    await waitFor(() => {
      expect(getDataModelNode(latestDocument).bindings.query).toMatchObject({
        value: {
          filters: [
            expect.anything(),
            expect.anything(),
            {
              field_code: 'approved',
              value: { kind: 'constant', value: false }
            }
          ]
        }
      });
    });
    await openSelect('过滤值 3');
    await selectOption('true');
    fireEvent.click(screen.getByRole('button', { name: '新增排序规则' }));
    await openSelect('排序字段 1');
    await selectOption('Amount');
    await openSelect('排序方向 1');
    await selectOption('desc');
    await openSelect('展开关联');
    await selectOption('Customer');
    fireEvent.change(screen.getByLabelText('页码'), { target: { value: '2' } });
    fireEvent.change(screen.getByLabelText('每页数量'), {
      target: { value: '50' }
    });

    await waitFor(() => {
      expect(getDataModelNode(latestDocument).bindings.query).toMatchObject({
        kind: 'data_model_query',
        value: {
          filters: [
            {
              field_code: 'status',
              operator: 'eq',
              value: {
                kind: 'selector',
                selector: ['node-start', 'query']
              }
            },
            {
              field_code: 'amount',
              operator: 'eq',
              value: { kind: 'constant', value: 123 }
            },
            {
              field_code: 'approved',
              operator: 'eq',
              value: { kind: 'constant', value: true }
            }
          ],
          sorts: [{ field_code: 'amount', direction: 'desc' }],
          expand_relations: ['customer'],
          page: { kind: 'constant', value: 2 },
          page_size: { kind: 'constant', value: 50 }
        }
      });
    });
  }, 30_000);

  test('keeps Data Model query pagination editable when selected model has no fields', async () => {
    let latestDocument = createDefaultAgentFlowDocument({ flowId: 'flow-1' });

    renderWithProviders(
      <AgentFlowEditorStoreProvider
        initialState={createInitialStateWithDataModelNode()}
      >
        <SelectionSeed nodeId="node-data-model" />
        <DocumentObserver
          onChange={(document) => {
            latestDocument = document;
          }}
        />
        <NodeConfigTab />
      </AgentFlowEditorStoreProvider>
    );

    expect(await screen.findByText('请先选择 Data Model')).toBeInTheDocument();

    await openSelect('Data Model');
    await selectDataModelOption('empty_orders');

    await waitFor(() => {
      expect(screen.queryByText('请先选择 Data Model')).not.toBeInTheDocument();
      expect(screen.getByLabelText('页码')).toBeInTheDocument();
      expect(screen.getByLabelText('每页数量')).toBeInTheDocument();
    });

    expect(screen.getByRole('button', { name: '新增过滤条件' })).toBeDisabled();
    expect(screen.getByRole('button', { name: '新增排序规则' })).toBeDisabled();

    fireEvent.change(screen.getByLabelText('页码'), { target: { value: '3' } });
    fireEvent.change(screen.getByLabelText('每页数量'), {
      target: { value: '10' }
    });

    await waitFor(() => {
      expect(getDataModelNode(latestDocument).bindings.query).toMatchObject({
        kind: 'data_model_query',
        value: {
          filters: [],
          sorts: [],
          page: { kind: 'constant', value: 3 },
          page_size: { kind: 'constant', value: 10 }
        }
      });
    });
  }, 10000);

  test('renders Data Model create, update, and delete editors from fixed node types', async () => {
    const { unmount: unmountCreate } = renderWithProviders(
      <AgentFlowEditorStoreProvider
        initialState={createInitialStateWithDataModelNode('data_model_create')}
      >
        <SelectionSeed nodeId="node-data-model" />
        <NodeConfigTab />
      </AgentFlowEditorStoreProvider>
    );

    expect(screen.queryByLabelText('Action')).not.toBeInTheDocument();
    expect(
      screen.queryByTestId('inspector-field-bindings.record_id')
    ).not.toBeInTheDocument();
    expect(
      screen.queryByTestId('inspector-field-bindings.query')
    ).not.toBeInTheDocument();
    expect(
      screen.getByTestId('inspector-field-bindings.payload')
    ).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: '新增字段赋值' }));
    expect(screen.getAllByLabelText('Payload-0-field').length).toBeGreaterThan(
      0
    );
    expect(
      screen.getAllByLabelText('Payload-0-variable').length
    ).toBeGreaterThan(0);
    unmountCreate();

    const { unmount: unmountUpdate } = renderWithProviders(
      <AgentFlowEditorStoreProvider
        initialState={createInitialStateWithDataModelNode('data_model_update')}
      >
        <SelectionSeed nodeId="node-data-model" />
        <NodeConfigTab />
      </AgentFlowEditorStoreProvider>
    );

    expect(
      screen.getByTestId('inspector-field-bindings.record_id')
    ).toBeInTheDocument();
    expect(
      screen.queryByTestId('inspector-field-bindings.query')
    ).not.toBeInTheDocument();
    expect(
      screen.getByTestId('inspector-field-bindings.payload')
    ).toBeInTheDocument();
    unmountUpdate();

    renderWithProviders(
      <AgentFlowEditorStoreProvider
        initialState={createInitialStateWithDataModelNode('data_model_delete')}
      >
        <SelectionSeed nodeId="node-data-model" />
        <NodeConfigTab />
      </AgentFlowEditorStoreProvider>
    );

    expect(
      screen.getByTestId('inspector-field-bindings.record_id')
    ).toBeInTheDocument();
    expect(
      screen.queryByTestId('inspector-field-bindings.query')
    ).not.toBeInTheDocument();
    expect(
      screen.queryByTestId('inspector-field-bindings.payload')
    ).not.toBeInTheDocument();
  }, 45000);

  test('keeps pagination unique to Data Model list query editors', async () => {
    const { unmount } = renderWithProviders(
      <AgentFlowEditorStoreProvider
        initialState={createInitialStateWithDataModelNode('data_model_get')}
      >
        <SelectionSeed nodeId="node-data-model" />
        <NodeConfigTab />
      </AgentFlowEditorStoreProvider>
    );

    await openSelect('Data Model');
    await selectDataModelOption('orders');

    await waitFor(() => {
      expect(
        screen.getByTestId('inspector-field-bindings.record_id')
      ).toBeInTheDocument();
      expect(screen.queryByLabelText('Query')).not.toBeInTheDocument();
      expect(screen.queryByLabelText('页码')).not.toBeInTheDocument();
      expect(screen.queryByLabelText('每页数量')).not.toBeInTheDocument();
      expect(
        screen.queryByRole('button', { name: '新增过滤条件' })
      ).not.toBeInTheDocument();
    });

    unmount();

    renderWithProviders(
      <AgentFlowEditorStoreProvider
        initialState={createInitialStateWithDataModelNode('data_model_list')}
      >
        <SelectionSeed nodeId="node-data-model" />
        <NodeConfigTab />
      </AgentFlowEditorStoreProvider>
    );

    await openSelect('Data Model');
    await selectDataModelOption('orders');

    await waitFor(() => {
      expect(screen.getByLabelText('页码')).toBeInTheDocument();
      expect(screen.getByLabelText('每页数量')).toBeInTheDocument();
    });
  });
});
