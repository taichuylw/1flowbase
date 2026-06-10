import { describe, expect, test } from 'vitest';
import { ERROR_BRANCH_SOURCE_HANDLE } from '../../../lib/policy/node-error-policy';
import { validateDocument } from '../../../lib/validate-document';
import { createDefaultAgentFlowDocument, createNodeDocument } from '../support';

describe('validateDocument branches and tools', () => {
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

  test('requires an allowed tool policy before mounted tool branches can contain LLM nodes', () => {
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
          fieldKey:
            'config.visible_internal_llm_tools.0.internal_llm_node_policy'
        })
      ])
    );

    const visibleInternalTools = llmNode.config
      .visible_internal_llm_tools as Array<Record<string, unknown>>;
    visibleInternalTools[0].internal_llm_node_policy = 'allowed';

    expect(validateDocument(document)).toEqual(
      expect.not.arrayContaining([
        expect.objectContaining({
          nodeId: 'node-llm',
          fieldKey:
            'config.visible_internal_llm_tools.0.internal_llm_node_policy'
        })
      ])
    );
  });
});
