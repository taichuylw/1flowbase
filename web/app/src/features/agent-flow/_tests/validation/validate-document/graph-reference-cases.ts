import { describe, expect, test } from 'vitest';
import { validateDocument } from '../../../lib/validate-document';
import { addSecondLlmNode, createDefaultAgentFlowDocument } from '../support';

describe('validateDocument graph references', () => {
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

  test('returns a node error for unresolved placeholder nodes', () => {
    const broken = createDefaultAgentFlowDocument({ flowId: 'flow-1' });
    const llmNode = broken.graph.nodes.find((node) => node.id === 'node-llm');

    if (!llmNode) {
      throw new Error('expected default LLM node');
    }

    broken.graph.nodes = broken.graph.nodes.map((node) =>
      node.id === 'node-llm'
        ? {
            ...node,
            type: 'unresolved_node',
            config: {
              unresolved: {
                dependency_status: 'missing_dependency',
                reason: 'missing_model_provider',
                original_type: 'llm',
                original_node: llmNode
              }
            },
            bindings: {},
            outputs: []
          }
        : node
    );

    const issues = validateDocument(broken);

    expect(issues).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          id: 'node-llm-unresolved-node',
          scope: 'node',
          level: 'error',
          nodeId: 'node-llm',
          title: '未知节点'
        })
      ])
    );
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
});
