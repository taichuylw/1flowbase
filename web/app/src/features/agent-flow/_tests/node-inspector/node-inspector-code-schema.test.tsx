import { readFileSync } from 'node:fs';

import { fireEvent, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, test } from 'vitest';

import { createDefaultAgentFlowDocument } from '@1flowbase/flow-schema';
import { NodeConfigTab } from '../../components/detail/tabs/NodeConfigTab';
import { AgentFlowEditorStoreProvider } from '../../store/editor/AgentFlowEditorStoreProvider';
import {
  DocumentObserver,
  SelectionSeed,
  createInitialStateWithCustomCodeNode,
  createInitialStateWithStructuredCodeNode,
  getCodeNode,
  openSelect,
  renderWithProviders,
  selectOption,
  setupNodeInspectorTest
} from './support';

beforeEach(setupNodeInspectorTest);

describe('NodeInspector code schema', () => {
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
});
