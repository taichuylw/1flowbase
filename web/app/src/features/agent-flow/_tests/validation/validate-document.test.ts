import { describe, expect, test } from 'vitest';

import { ERROR_BRANCH_SOURCE_HANDLE } from '../../lib/node-error-policy';
import { listLlmProviderOptions } from '../../lib/model-options';
import { validateDocument } from '../../lib/validate-document';
import {
  addSecondLlmNode,
  createCodeDocumentWithOutputs,
  createDefaultAgentFlowDocument,
  createNodeDocument,
  modelProviderOptionsContract,
  primaryGroup,
  primaryModel,
  primaryProvider
} from './support';

describe('validateDocument', () => {
  test.each(['__trace'])('flags internal output selector key %s', (key) => {
    const document = createDefaultAgentFlowDocument({ flowId: 'flow-1' });
    const answerNode = document.graph.nodes.find(
      (node) => node.id === 'node-answer'
    );

    if (!answerNode) {
      throw new Error('expected default Answer node');
    }

    answerNode.outputs = [{ key, title: key, valueType: 'json' }];

    const issues = validateDocument(document);

    expect(issues).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          nodeId: 'node-answer',
          fieldKey: 'config.output_contract',
          title: '输出变量名保留'
        })
      ])
    );
  });

  test('flags unknown v1-only LLM outputs in text mode', () => {
    const document = createDefaultAgentFlowDocument({ flowId: 'flow-1' });
    const llmNode = document.graph.nodes.find((node) => node.id === 'node-llm');

    if (!llmNode) {
      throw new Error('expected default LLM node');
    }

    llmNode.outputs = [
      { key: 'text', title: '模型输出', valueType: 'string' },
      { key: 'reasoning_content', title: '推理内容', valueType: 'string' }
    ];

    const issues = validateDocument(document);

    expect(issues).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          nodeId: 'node-llm',
          fieldKey: 'config.output_contract',
          title: '输出变量名未知'
        })
      ])
    );
  });

  test('flags JSON Schema on non structured output types', () => {
    const document = createCodeDocumentWithOutputs([
      {
        key: 'summary',
        title: 'Summary',
        valueType: 'string',
        jsonSchema: { type: 'string' }
      }
    ]);

    const issues = validateDocument(document);

    expect(issues).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          nodeId: 'node-code',
          fieldKey: 'config.output_contract',
          title: 'JSON Schema 类型不匹配'
        })
      ])
    );
  });

  test('flags Code output names with unsupported characters', () => {
    const document = createCodeDocumentWithOutputs([
      {
        key: 'risk-score',
        title: 'risk-score',
        valueType: 'number'
      }
    ]);

    const issues = validateDocument(document);

    expect(issues).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          nodeId: 'node-code',
          fieldKey: 'config.output_contract',
          title: '输出变量名格式错误'
        })
      ])
    );
  });

  test('flags Code output display name drift', () => {
    const document = createCodeDocumentWithOutputs([
      {
        key: 'riskScore',
        title: 'Risk Score',
        valueType: 'number'
      }
    ]);

    const issues = validateDocument(document);

    expect(issues).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          nodeId: 'node-code',
          fieldKey: 'config.output_contract',
          title: '输出变量名与显示名不一致'
        })
      ])
    );
  });

  test('accepts runtime fields when the node contract declares them as output selectors', () => {
    const document = createDefaultAgentFlowDocument({ flowId: 'flow-1' });
    const llmNode = document.graph.nodes.find((node) => node.id === 'node-llm');

    if (!llmNode) {
      throw new Error('expected default LLM node');
    }

    llmNode.outputs = [
      { key: 'text', title: '模型输出', valueType: 'string' },
      { key: 'usage', title: '用量', valueType: 'json' }
    ];

    const issues = validateDocument(document);

    expect(
      issues.some(
        (issue) =>
          issue.nodeId === 'node-llm' &&
          issue.fieldKey === 'config.output_contract' &&
          issue.title === '输出变量名未知'
      )
    ).toBe(false);
    expect(
      issues.some(
        (issue) =>
          issue.nodeId === 'node-llm' &&
          issue.fieldKey === 'config.output_contract' &&
          issue.title === '输出变量名保留'
      )
    ).toBe(false);
  });

  test('accepts structured LLM output only when response format enables JSON', () => {
    const textDocument = createDefaultAgentFlowDocument({
      flowId: 'flow-text'
    });
    const textLlmNode = textDocument.graph.nodes.find(
      (node) => node.id === 'node-llm'
    );

    if (!textLlmNode) {
      throw new Error('expected default LLM node');
    }

    textLlmNode.outputs = [
      { key: 'text', title: '模型输出', valueType: 'string' },
      { key: 'structured_output', title: '结构化输出', valueType: 'json' }
    ];

    const textIssues = validateDocument(textDocument);

    expect(textIssues).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          nodeId: 'node-llm',
          fieldKey: 'config.output_contract',
          title: '输出变量名未知'
        })
      ])
    );

    const jsonDocument = createDefaultAgentFlowDocument({
      flowId: 'flow-json'
    });
    const jsonLlmNode = jsonDocument.graph.nodes.find(
      (node) => node.id === 'node-llm'
    );

    if (!jsonLlmNode) {
      throw new Error('expected default LLM node');
    }

    jsonLlmNode.config.response_format = { mode: 'json_object' };
    jsonLlmNode.outputs = [
      { key: 'text', title: '模型输出', valueType: 'string' },
      { key: 'structured_output', title: '结构化输出', valueType: 'json' }
    ];

    const jsonIssues = validateDocument(jsonDocument);

    expect(
      jsonIssues.some(
        (issue) =>
          issue.nodeId === 'node-llm' &&
          issue.fieldKey === 'config.output_contract' &&
          issue.title === '输出变量名未知'
      )
    ).toBe(false);
  });

  test('flags unknown outputs on contracted non-LLM nodes', () => {
    const document = createDefaultAgentFlowDocument({ flowId: 'flow-1' });
    const answerNode = document.graph.nodes.find(
      (node) => node.id === 'node-answer'
    );

    if (!answerNode) {
      throw new Error('expected default Answer node');
    }

    answerNode.outputs = [
      { key: 'answer', title: '对话输出', valueType: 'string' },
      { key: 'extra', title: 'Extra', valueType: 'string' }
    ];

    const issues = validateDocument(document);

    expect(issues).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          nodeId: 'node-answer',
          fieldKey: 'config.output_contract',
          title: '输出变量名未知'
        })
      ])
    );
  });

  test('validates plugin_node outputs against output_schema_snapshot', () => {
    const document = createDefaultAgentFlowDocument({ flowId: 'flow-1' });
    document.graph.nodes.push({
      ...createNodeDocument('plugin_node', 'node-plugin'),
      plugin_id: 'plugin-1',
      plugin_version: '1.0.0',
      contribution_code: 'custom_plugin_node',
      node_shell: 'generic',
      schema_version: '1flowbase.node-contribution/v2',
      plugin_unique_identifier: 'plugin-1:custom_plugin_node:v2',
      package_id: 'plugin-1',
      contribution_checksum: 'checksum',
      compiled_contribution_hash: 'compiled-hash',
      output_schema_snapshot: {
        outputs: [
          { key: 'custom_payload', title: 'Custom Payload', valueType: 'json' }
        ]
      },
      outputs: [
        { key: 'custom_payload', title: 'Custom Payload', valueType: 'json' }
      ]
    });

    const issues = validateDocument(document);

    expect(
      issues.some(
        (issue) =>
          issue.nodeId === 'node-plugin' &&
          issue.fieldKey === 'config.output_contract' &&
          issue.title === '输出变量名未知'
      )
    ).toBe(false);
  });

  test('flags plugin_node output drift from snapshot', () => {
    const document = createDefaultAgentFlowDocument({ flowId: 'flow-1' });
    document.graph.nodes.push({
      ...createNodeDocument('plugin_node', 'node-plugin'),
      plugin_id: 'plugin-1',
      plugin_version: '1.0.0',
      contribution_code: 'custom_plugin_node',
      node_shell: 'generic',
      schema_version: '1flowbase.node-contribution/v2',
      plugin_unique_identifier: 'plugin-1:custom_plugin_node:v2',
      package_id: 'plugin-1',
      contribution_checksum: 'checksum',
      compiled_contribution_hash: 'compiled-hash',
      output_schema_snapshot: {
        outputs: [
          { key: 'custom_payload', title: 'Custom Payload', valueType: 'json' }
        ]
      },
      outputs: [
        { key: 'custom_payload', title: 'Custom Payload', valueType: 'json' },
        { key: 'stale_output', title: 'Stale Output', valueType: 'json' }
      ]
    });

    const issues = validateDocument(document);

    expect(
      issues.some(
        (issue) =>
          issue.nodeId === 'node-plugin' &&
          issue.fieldKey === 'config.output_contract' &&
          issue.title === '输出变量名未知'
      )
    ).toBe(true);
  });

  test('flags legacy plugin_node contribution schema versions', () => {
    const document = createDefaultAgentFlowDocument({ flowId: 'flow-1' });
    document.graph.nodes.push({
      ...createNodeDocument('plugin_node', 'node-plugin'),
      plugin_id: 'plugin-1',
      plugin_version: '1.0.0',
      contribution_code: 'custom_plugin_node',
      node_shell: 'generic',
      schema_version: '1flowbase.node-contribution/v1',
      plugin_unique_identifier: 'plugin-1:custom_plugin_node:v1',
      package_id: 'plugin-1',
      contribution_checksum: 'checksum',
      compiled_contribution_hash: 'compiled-hash',
      output_schema_snapshot: {
        outputs: [
          { key: 'custom_payload', title: 'Custom Payload', valueType: 'json' }
        ]
      },
      outputs: [
        { key: 'custom_payload', title: 'Custom Payload', valueType: 'json' }
      ]
    });

    const issues = validateDocument(document);

    expect(issues).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          nodeId: 'node-plugin',
          title: '插件节点缺少贡献身份'
        })
      ])
    );
  });

  test('flags internal plugin_node output keys even when snapshot allows them', () => {
    const document = createDefaultAgentFlowDocument({ flowId: 'flow-1' });
    document.graph.nodes.push({
      ...createNodeDocument('plugin_node', 'node-plugin'),
      plugin_id: 'plugin-1',
      plugin_version: '1.0.0',
      contribution_code: 'custom_plugin_node',
      node_shell: 'generic',
      schema_version: '1flowbase.node-contribution/v2',
      plugin_unique_identifier: 'plugin-1:custom_plugin_node:v2',
      package_id: 'plugin-1',
      contribution_checksum: 'checksum',
      compiled_contribution_hash: 'compiled-hash',
      output_schema_snapshot: {
        outputs: [{ key: '__trace', title: 'Trace', valueType: 'json' }]
      },
      outputs: [{ key: '__trace', title: 'Trace', valueType: 'json' }]
    });

    const issues = validateDocument(document);

    expect(
      issues.some(
        (issue) =>
          issue.nodeId === 'node-plugin' &&
          issue.fieldKey === 'config.output_contract' &&
          issue.title === '输出变量名保留'
      )
    ).toBe(true);
  });

  test('keeps all backend-provided models selectable, including manual entries', () => {
    const options = {
      ...modelProviderOptionsContract,
      providers: [
        {
          ...primaryProvider,
          model_groups: [
            {
              ...primaryGroup,
              models: [
                {
                  ...primaryModel,
                  model_id: 'gpt-4o-mini',
                  display_name: 'GPT-4o Mini'
                },
                {
                  ...primaryModel,
                  model_id: 'gpt-4o',
                  display_name: 'GPT-4o'
                },
                {
                  ...primaryModel,
                  model_id: 'manual-enabled-model',
                  display_name: '手动启用模型',
                  source: 'manual'
                }
              ]
            }
          ]
        }
      ]
    };

    expect(
      listLlmProviderOptions(
        options as typeof modelProviderOptionsContract
      )[0]?.models.map((model) => model.value)
    ).toEqual(['gpt-4o-mini', 'gpt-4o', 'manual-enabled-model']);
  });

  test('returns field, node, and global issues', () => {
    const broken = createDefaultAgentFlowDocument({ flowId: 'flow-1' });
    broken.graph.nodes = broken.graph.nodes.filter(
      (node) => node.id !== 'node-answer'
    );

    const issues = validateDocument(broken);

    expect(issues.some((issue) => issue.scope === 'field')).toBe(true);
    expect(issues.some((issue) => issue.scope === 'node')).toBe(true);
    expect(issues.some((issue) => issue.scope === 'global')).toBe(true);
  });

  test('returns a field issue when a templated binding points to an unreachable output', () => {
    const broken = createDefaultAgentFlowDocument({ flowId: 'flow-1' });
    const llmNode = broken.graph.nodes.find((node) => node.id === 'node-llm');

    if (!llmNode) {
      throw new Error('expected default LLM node');
    }

    llmNode.bindings.user_prompt = {
      kind: 'templated_text',
      value: '请基于 {{node-answer.answer}} 回复用户'
    };

    const issues = validateDocument(broken);

    expect(
      issues.some(
        (issue) =>
          issue.scope === 'field' &&
          issue.nodeId === 'node-llm' &&
          issue.fieldKey === 'bindings.user_prompt' &&
          issue.message === '当前 binding 引用了未接入上游链路的输出。'
      )
    ).toBe(true);
  });

  test('returns a field error when a binding references a deleted source node', () => {
    const broken = createDefaultAgentFlowDocument({ flowId: 'flow-1' });
    const answerNode = broken.graph.nodes.find(
      (node) => node.id === 'node-answer'
    );

    if (!answerNode) {
      throw new Error('expected default Answer node');
    }

    answerNode.bindings.answer_template = {
      kind: 'templated_text',
      value: '{{node-llm.text}}\n----\n{{node-llm-1.text}}'
    };

    const issues = validateDocument(broken);

    expect(issues).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          scope: 'field',
          level: 'error',
          nodeId: 'node-answer',
          fieldKey: 'bindings.answer_template',
          title: '绑定引用节点不存在',
          message: '当前 binding 引用了已删除节点 node-llm-1 的输出。'
        })
      ])
    );
  });

  test('rejects duplicate Answer presentation output references', () => {
    const broken = createDefaultAgentFlowDocument({ flowId: 'flow-1' });
    const answerNode = broken.graph.nodes.find(
      (node) => node.id === 'node-answer'
    );

    if (!answerNode) {
      throw new Error('expected default Answer node');
    }

    answerNode.bindings.answer_template = {
      kind: 'templated_text',
      value: '{{node-llm.text}}\n----\n{{node-llm.text}}'
    };

    const issues = validateDocument(broken);

    expect(issues).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          scope: 'field',
          level: 'error',
          nodeId: 'node-answer',
          fieldKey: 'bindings.answer_template',
          title: 'Answer 输出变量重复引用'
        })
      ])
    );
  });

  test('rejects Answer presentation order that reverses a real dependency', () => {
    const broken = createDefaultAgentFlowDocument({ flowId: 'flow-1' });
    addSecondLlmNode(broken, true);
    const answerNode = broken.graph.nodes.find(
      (node) => node.id === 'node-answer'
    );

    if (!answerNode) {
      throw new Error('expected default Answer node');
    }

    answerNode.bindings.answer_template = {
      kind: 'templated_text',
      value: '{{node-llm-2.text}}\n----\n{{node-llm.text}}'
    };

    const issues = validateDocument(broken);

    expect(issues).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          scope: 'field',
          level: 'error',
          nodeId: 'node-answer',
          fieldKey: 'bindings.answer_template',
          title: 'Answer 展示顺序违反执行依赖'
        })
      ])
    );
  });

  test('allows parallel Answer presentation references in template order', () => {
    const document = createDefaultAgentFlowDocument({ flowId: 'flow-1' });
    addSecondLlmNode(document, false);
    const answerNode = document.graph.nodes.find(
      (node) => node.id === 'node-answer'
    );

    if (!answerNode) {
      throw new Error('expected default Answer node');
    }

    answerNode.bindings.answer_template = {
      kind: 'templated_text',
      value: '{{node-llm-2.text}}\n----\n{{node-llm.text}}'
    };

    const issues = validateDocument(document);

    expect(
      issues.some(
        (issue) =>
          issue.nodeId === 'node-answer' &&
          issue.fieldKey === 'bindings.answer_template' &&
          (issue.title === 'Answer 展示顺序违反执行依赖' ||
            issue.title === 'Answer 输出变量重复引用')
      )
    ).toBe(false);
  });

  test('accepts templated bindings that reference application environment variables', () => {
    const document = createDefaultAgentFlowDocument({ flowId: 'flow-1' });
    const llmNode = document.graph.nodes.find((node) => node.id === 'node-llm');

    if (!llmNode) {
      throw new Error('expected default LLM node');
    }

    llmNode.bindings.user_prompt = {
      kind: 'templated_text',
      value: '请调用 {{env.ApiBaseUrl}} 处理请求'
    };

    const issues = validateDocument(document, null, [
      {
        name: 'ApiBaseUrl',
        value_type: 'string',
        value: 'https://api.example.com',
        description: '当前应用 API 地址'
      }
    ]);

    expect(
      issues.some(
        (issue) =>
          issue.nodeId === 'node-llm' &&
          issue.fieldKey === 'bindings.user_prompt' &&
          issue.message === '当前 binding 引用了未接入上游链路的输出。'
      )
    ).toBe(false);
  });

  test('flags If / Else branches whose non-else conditions are empty', () => {
    const document = createDefaultAgentFlowDocument({ flowId: 'flow-1' });
    document.graph.nodes.push(createNodeDocument('if_else', 'node-if-else'));

    const issues = validateDocument(document);

    expect(issues).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          nodeId: 'node-if-else',
          fieldKey: 'bindings.branches'
        })
      ])
    );
  });

  test('flags If / Else branch rules whose left selector is empty', () => {
    const document = createDefaultAgentFlowDocument({ flowId: 'flow-1' });
    const ifElseNode = createNodeDocument('if_else', 'node-if-else');

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
              conditions: [{ kind: 'rule', left: [], comparator: 'exists' }]
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
    document.graph.nodes.push(ifElseNode);
    document.graph.edges.push({
      id: 'edge-start-if-else',
      source: 'node-start',
      target: 'node-if-else',
      sourceHandle: null,
      targetHandle: null,
      containerId: null,
      points: []
    });

    const issues = validateDocument(document);

    expect(issues).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          nodeId: 'node-if-else',
          fieldKey: 'bindings.branches'
        })
      ])
    );
  });

  test('flags If / Else branch rules whose right selector is empty', () => {
    const document = createDefaultAgentFlowDocument({ flowId: 'flow-1' });
    const ifElseNode = createNodeDocument('if_else', 'node-if-else');

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
                  right: { kind: 'selector', selector: [] }
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
    document.graph.nodes.push(ifElseNode);
    document.graph.edges.push({
      id: 'edge-start-if-else',
      source: 'node-start',
      target: 'node-if-else',
      sourceHandle: null,
      targetHandle: null,
      containerId: null,
      points: []
    });

    const issues = validateDocument(document);

    expect(issues).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          nodeId: 'node-if-else',
          fieldKey: 'bindings.branches'
        })
      ])
    );
  });

  test('flags If / Else branch groups that mix complete and incomplete rules', () => {
    const document = createDefaultAgentFlowDocument({ flowId: 'flow-1' });
    const ifElseNode = createNodeDocument('if_else', 'node-if-else');

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
                  comparator: 'exists'
                },
                { kind: 'rule', left: [], comparator: 'exists' }
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
    document.graph.nodes.push(ifElseNode);
    document.graph.edges.push({
      id: 'edge-start-if-else',
      source: 'node-start',
      target: 'node-if-else',
      sourceHandle: null,
      targetHandle: null,
      containerId: null,
      points: []
    });

    const issues = validateDocument(document);

    expect(issues).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          nodeId: 'node-if-else',
          fieldKey: 'bindings.branches'
        })
      ])
    );
  });

  test('allows the fixed exception source handle only when the source node uses exception branch policy', () => {
    const document = createDefaultAgentFlowDocument({ flowId: 'flow-1' });
    const llmNode = document.graph.nodes.find((node) => node.id === 'node-llm');
    const answerNode = document.graph.nodes.find(
      (node) => node.id === 'node-answer'
    );

    if (!llmNode || !answerNode) {
      throw new Error('expected default llm and answer nodes');
    }

    llmNode.config.error_policy = 'error_branch';
    document.graph.edges.push({
      id: 'edge-llm-error-answer',
      source: llmNode.id,
      target: answerNode.id,
      sourceHandle: ERROR_BRANCH_SOURCE_HANDLE,
      targetHandle: null,
      containerId: null,
      points: []
    });

    expect(validateDocument(document)).toEqual(
      expect.not.arrayContaining([
        expect.objectContaining({
          id: 'edge-llm-error-answer-invalid-source-handle'
        })
      ])
    );

    llmNode.config.error_policy = 'none';

    expect(validateDocument(document)).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          id: 'edge-llm-error-answer-invalid-source-handle',
          nodeId: 'node-llm',
          fieldKey: 'config.error_policy'
        })
      ])
    );
  });

  test('flags mounted LLM tool branches without a Tool Result node', () => {
    const document = createDefaultAgentFlowDocument({ flowId: 'flow-1' });
    const llmNode = document.graph.nodes.find((node) => node.id === 'node-llm');

    if (!llmNode) {
      throw new Error('expected default llm node');
    }

    llmNode.config.visible_internal_llm_tools_enabled = true;
    llmNode.config.visible_internal_llm_tools = [
      {
        type: 'visible_internal_llm_tool',
        tool_name: 'inspect_context',
        connector_id: 'inspect_context',
        target_node_id: 'node-tool-transform'
      }
    ];
    document.graph.nodes.push(
      createNodeDocument(
        'template_transform',
        'node-tool-transform',
        llmNode.position.x + 240,
        llmNode.position.y + 160
      )
    );
    document.graph.edges.push({
      id: 'edge-llm-mounted-tool',
      source: llmNode.id,
      target: 'node-tool-transform',
      sourceHandle: 'visible_internal_llm_tool:inspect_context',
      targetHandle: null,
      containerId: null,
      points: []
    });

    expect(validateDocument(document)).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          nodeId: 'node-llm',
          fieldKey: 'config.visible_internal_llm_tools_enabled',
          title: '工具分支缺少 Tool Result'
        })
      ])
    );
  });

  test('requires an allowed policy before mounted tool branches can contain LLM nodes', () => {
    const document = createDefaultAgentFlowDocument({ flowId: 'flow-1' });
    const llmNode = document.graph.nodes.find((node) => node.id === 'node-llm');

    if (!llmNode) {
      throw new Error('expected default llm node');
    }

    const mountedLlm = {
      ...createNodeDocument(
        'llm',
        'node-mounted-llm',
        llmNode.position.x + 240,
        llmNode.position.y + 160
      ),
      config: llmNode.config,
      bindings: llmNode.bindings,
      outputs: llmNode.outputs
    };
    const toolResult = createNodeDocument(
      'tool_result',
      'node-tool-result',
      llmNode.position.x + 520,
      llmNode.position.y + 160
    );

    llmNode.config.visible_internal_llm_tools_enabled = true;
    llmNode.config.visible_internal_llm_tools = [
      {
        type: 'visible_internal_llm_tool',
        tool_name: 'inspect_context',
        connector_id: 'inspect_context',
        target_node_id: mountedLlm.id
      }
    ];
    document.graph.nodes.push(mountedLlm, toolResult);
    document.graph.edges.push(
      {
        id: 'edge-llm-mounted-tool',
        source: llmNode.id,
        target: mountedLlm.id,
        sourceHandle: 'visible_internal_llm_tool:inspect_context',
        targetHandle: null,
        containerId: null,
        points: []
      },
      {
        id: 'edge-mounted-llm-tool-result',
        source: mountedLlm.id,
        target: toolResult.id,
        sourceHandle: null,
        targetHandle: null,
        containerId: null,
        points: []
      }
    );

    expect(validateDocument(document)).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          nodeId: 'node-llm',
          fieldKey: 'config.internal_llm_node_policy'
        })
      ])
    );

    llmNode.config.internal_llm_node_policy = 'allowed';

    expect(validateDocument(document)).toEqual(
      expect.not.arrayContaining([
        expect.objectContaining({
          nodeId: 'node-llm',
          fieldKey: 'config.internal_llm_node_policy'
        })
      ])
    );
  });

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

  test('flags a missing llm model provider selection on the unified field', () => {
    const document = createDefaultAgentFlowDocument({ flowId: 'flow-1' });

    const issues = validateDocument(document);

    expect(issues).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          nodeId: 'node-llm',
          fieldKey: 'config.model_provider',
          title: 'LLM 缺少模型供应商'
        })
      ])
    );
  });

  test('flags unavailable provider code and missing model in provider catalog', () => {
    const document = createDefaultAgentFlowDocument({ flowId: 'flow-1' });
    const llmNode = document.graph.nodes.find((node) => node.id === 'node-llm');

    if (!llmNode) {
      throw new Error('expected default LLM node');
    }

    llmNode.config.model_provider = {
      provider_code: 'provider-stale',
      model_id: 'gpt-4.1'
    };

    const issues = validateDocument(document, modelProviderOptionsContract);

    expect(issues).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          nodeId: 'node-llm',
          fieldKey: 'config.model_provider',
          title: 'LLM 模型供应商不可用'
        })
      ])
    );
  });

  test('flags a model that is not in the backend-provided model list', () => {
    const document = createDefaultAgentFlowDocument({ flowId: 'flow-1' });
    const llmNode = document.graph.nodes.find((node) => node.id === 'node-llm');

    if (!llmNode) {
      throw new Error('expected default LLM node');
    }

    llmNode.config.model_provider = {
      provider_code: 'openai_compatible',
      model_id: 'missing-model'
    };

    const issues = validateDocument(document, modelProviderOptionsContract);

    expect(issues).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          nodeId: 'node-llm',
          fieldKey: 'config.model_provider',
          title: 'LLM 模型不可用'
        })
      ])
    );
  });

  test('accepts stable llm provider and model selection without source instance', () => {
    const document = createDefaultAgentFlowDocument({ flowId: 'flow-1' });
    const llmNode = document.graph.nodes.find((node) => node.id === 'node-llm');

    if (!llmNode) {
      throw new Error('expected default LLM node');
    }

    llmNode.config.model_provider = {
      provider_code: primaryProvider.provider_code,
      model_id: primaryModel.model_id
    };

    const issues = validateDocument(document, modelProviderOptionsContract);

    expect(issues).not.toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          nodeId: 'node-llm',
          fieldKey: 'config.model_provider'
        })
      ])
    );
  });

  test('flags an ambiguous stable model that is exposed by multiple included instances', () => {
    const document = createDefaultAgentFlowDocument({ flowId: 'flow-1' });
    const duplicatedContract = JSON.parse(
      JSON.stringify(modelProviderOptionsContract)
    ) as typeof modelProviderOptionsContract;
    const duplicatedProvider = duplicatedContract.providers[0];
    const llmNode = document.graph.nodes.find((node) => node.id === 'node-llm');

    if (!llmNode) {
      throw new Error('expected default LLM node');
    }

    duplicatedProvider.model_groups = [
      {
        source_instance_id: 'provider-openai-prod',
        source_instance_display_name: 'OpenAI Production',
        models: [{ ...primaryModel }]
      },
      {
        source_instance_id: 'provider-openai-backup',
        source_instance_display_name: 'OpenAI Backup',
        models: [{ ...primaryModel }]
      }
    ];
    llmNode.config.model_provider = {
      provider_code: primaryProvider.provider_code,
      model_id: primaryModel.model_id
    };

    const issues = validateDocument(document, duplicatedContract);

    expect(issues).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          nodeId: 'node-llm',
          fieldKey: 'config.model_provider',
          title: 'LLM 模型解析不唯一'
        })
      ])
    );
  });

  test('keeps the node populated but flags a model that does not exist under the selected provider', () => {
    const document = createDefaultAgentFlowDocument({ flowId: 'flow-1' });
    const llmNode = document.graph.nodes.find((node) => node.id === 'node-llm');

    if (!llmNode) {
      throw new Error('expected default LLM node');
    }

    llmNode.config.model_provider = {
      provider_code: primaryProvider.provider_code,
      model_id: 'missing-model'
    };

    const issues = validateDocument(document, modelProviderOptionsContract);

    expect(llmNode.config.model_provider).toEqual(
      expect.objectContaining({
        provider_code: primaryProvider.provider_code,
        model_id: 'missing-model'
      })
    );
    expect(issues).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          nodeId: 'node-llm',
          fieldKey: 'config.model_provider',
          title: 'LLM 模型不可用'
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
