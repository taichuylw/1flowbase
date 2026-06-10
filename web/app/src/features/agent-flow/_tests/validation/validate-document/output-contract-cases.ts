import { describe, expect, test } from 'vitest';
import { validateDocument } from '../../../lib/validate-document';
import {
  createCodeDocumentWithOutputs,
  createDefaultAgentFlowDocument,
  createNodeDocument
} from '../support';

describe('validateDocument output contracts', () => {
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
});
