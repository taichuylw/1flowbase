import { describe, expect, test } from 'vitest';
import { validateDocument } from '../../../lib/validate-document';
import {
  createCodeDocumentWithOutputs,
  createDefaultAgentFlowDocument,
  createNodeDocument
} from '../support';

describe('validateDocument code, data model, and variables', () => {
  test('flags duplicate code output keys in the editable output contract', () => {
    const document = createCodeDocumentWithOutputs([
      { key: 'result', title: '结果', valueType: 'string' },
      { key: 'result', title: '重复结果', valueType: 'string' }
    ]);

    const issues = validateDocument(document);

    expect(issues).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          nodeId: 'node-code',
          message: '输出契约中的变量名必须唯一'
        })
      ])
    );
  });

  test('flags empty code output contract', () => {
    const document = createCodeDocumentWithOutputs([]);

    const issues = validateDocument(document);

    expect(issues).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          nodeId: 'node-code',
          title: '代码输出契约不能为空',
          message: 'Code 节点至少需要保留 1 个输出变量用于下游引用。'
        })
      ])
    );
  });

  test('flags unsupported code runtime language', () => {
    const document = createCodeDocumentWithOutputs([
      { key: 'result', title: '结果', valueType: 'unknown' }
    ]);
    const codeNode = document.graph.nodes.find(
      (node) => node.id === 'node-code'
    );

    if (!codeNode) {
      throw new Error('expected code node');
    }

    codeNode.config = {
      ...codeNode.config,
      language: 'python'
    };

    const issues = validateDocument(document);

    expect(issues).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          nodeId: 'node-code',
          fieldKey: 'config.language',
          title: '不支持的运行语言',
          message: '当前版本仅支持 JavaScript。'
        })
      ])
    );
  });

  test('validates only active Data Model node type bindings', () => {
    const document = createDefaultAgentFlowDocument({ flowId: 'flow-1' });
    document.graph.nodes.push({
      ...createNodeDocument('data_model_create', 'node-data-model'),
      config: { data_model_code: 'orders' },
      bindings: {
        query: {
          kind: 'data_model_query',
          value: {
            filters: [
              {
                field_code: 'status',
                operator: 'eq',
                value: { kind: 'selector', selector: ['node-answer', 'answer'] }
              }
            ],
            sorts: [],
            expand_relations: [],
            page: { kind: 'constant', value: 1 },
            page_size: { kind: 'constant', value: 20 }
          }
        },
        payload: {
          kind: 'named_bindings',
          value: [{ name: 'title', selector: ['node-start', 'query'] }]
        }
      }
    });

    const issues = validateDocument(document);

    expect(
      issues.some(
        (issue) =>
          issue.nodeId === 'node-data-model' &&
          issue.fieldKey === 'bindings.query'
      )
    ).toBe(false);
    expect(
      issues.some(
        (issue) =>
          issue.nodeId === 'node-data-model' &&
          issue.fieldKey === 'bindings.record_id'
      )
    ).toBe(false);
  });

  test('validates Code input parameter names and constant value types', () => {
    const document = createCodeDocumentWithOutputs([
      { key: 'result', title: 'result', valueType: 'string' }
    ]);
    const codeNode = document.graph.nodes.find(
      (node) => node.id === 'node-code'
    );

    if (!codeNode) {
      throw new Error('expected code node');
    }

    codeNode.bindings.named_bindings = {
      kind: 'named_bindings',
      value: [
        {
          name: 'bad-name',
          valueType: 'string',
          value: { kind: 'constant', value: 'ok' }
        },
        {
          name: 'items',
          valueType: 'array',
          value: { kind: 'constant', value: { not: 'array' } }
        }
      ]
    };

    expect(validateDocument(document)).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          nodeId: 'node-code',
          fieldKey: 'bindings.named_bindings',
          title: '输入变量名格式错误'
        }),
        expect.objectContaining({
          nodeId: 'node-code',
          fieldKey: 'bindings.named_bindings',
          title: '变量值与类型不匹配'
        })
      ])
    );
  });

  test('allows Code numeric input formulas with numeric selector tokens', () => {
    const document = createCodeDocumentWithOutputs([
      { key: 'result', title: 'result', valueType: 'string' }
    ]);
    const codeNode = document.graph.nodes.find(
      (node) => node.id === 'node-code'
    );

    if (!codeNode) {
      throw new Error('expected code node');
    }

    codeNode.bindings.named_bindings = {
      kind: 'named_bindings',
      value: [
        {
          name: 'score',
          valueType: 'number',
          value: {
            kind: 'templated_text',
            value: '{{sys.dialog_count}} + 1'
          }
        }
      ]
    };

    expect(validateDocument(document)).toEqual(
      expect.not.arrayContaining([
        expect.objectContaining({
          nodeId: 'node-code',
          fieldKey: 'bindings.named_bindings',
          title: '变量值与类型不匹配'
        })
      ])
    );
  });

  test('flags Code numeric input formulas with non numeric selector tokens', () => {
    const document = createCodeDocumentWithOutputs([
      { key: 'result', title: 'result', valueType: 'string' }
    ]);
    const codeNode = document.graph.nodes.find(
      (node) => node.id === 'node-code'
    );

    if (!codeNode) {
      throw new Error('expected code node');
    }

    codeNode.bindings.named_bindings = {
      kind: 'named_bindings',
      value: [
        {
          name: 'score',
          valueType: 'number',
          value: {
            kind: 'templated_text',
            value: '{{sys.conversation_id}} + 1'
          }
        }
      ]
    };

    expect(validateDocument(document)).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          nodeId: 'node-code',
          fieldKey: 'bindings.named_bindings',
          title: '变量值与类型不匹配'
        })
      ])
    );
  });

  test('requires variable assignment targets to be defined conversation variables', () => {
    const document = createDefaultAgentFlowDocument({ flowId: 'flow-1' });
    const variableNode = createNodeDocument(
      'variable_assigner',
      'node-variable-assigner'
    );

    variableNode.bindings.operations = {
      kind: 'state_write',
      value: [
        {
          path: ['env', 'ApiBaseUrl'],
          operator: 'set',
          source: null,
          value: { kind: 'constant', value: 'https://api.example.com' }
        }
      ]
    };
    document.graph.nodes.splice(1, 0, variableNode);

    expect(validateDocument(document)).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          nodeId: 'node-variable-assigner',
          fieldKey: 'bindings.operations',
          title: '变量赋值目标无效'
        })
      ])
    );

    document.variables = {
      conversation: [
        {
          name: 'ApiBaseUrl',
          valueType: 'string',
          description: ''
        }
      ]
    };
    variableNode.bindings.operations.value[0].path = [
      'conversation',
      'ApiBaseUrl'
    ];

    expect(validateDocument(document)).toEqual(
      expect.not.arrayContaining([
        expect.objectContaining({
          nodeId: 'node-variable-assigner',
          fieldKey: 'bindings.operations',
          title: '变量赋值目标无效'
        })
      ])
    );
  });

  test('does not crash on malformed saved Data Model query binding', () => {
    const document = createDefaultAgentFlowDocument({ flowId: 'flow-1' });
    document.graph.nodes.push({
      ...createNodeDocument('data_model_list', 'node-data-model'),
      config: { data_model_code: 'orders' },
      bindings: {
        query: {
          kind: 'data_model_query',
          value: {
            filters: [
              {},
              {
                field_code: 'status',
                operator: 'eq',
                value: {
                  kind: 'selector',
                  selector: ['node-start', 'query', false]
                }
              }
            ],
            page: { kind: 'selector', selector: ['node-start', 'query', null] }
          }
        } as never
      }
    });

    const issues = validateDocument(document);
    const queryIssues = issues.filter(
      (issue) =>
        issue.nodeId === 'node-data-model' &&
        issue.fieldKey === 'bindings.query'
    );

    expect(queryIssues).toHaveLength(2);
    expect(queryIssues).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          level: 'error',
          title: '绑定引用不可见'
        })
      ])
    );
    expect(issues).toContainEqual(
      expect.objectContaining({
        nodeId: 'node-data-model',
        id: 'node-data-model-orphan',
        level: 'warning'
      })
    );
  });
});
