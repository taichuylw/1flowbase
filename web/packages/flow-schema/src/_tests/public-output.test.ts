import { describe, expect, it } from 'vitest';

import {
  DEFAULT_LLM_NODE_OUTPUTS,
  FLOW_SCHEMA_VERSION,
  NODE_CONTRIBUTION_SCHEMA_VERSION,
  createDefaultAgentFlowDocument,
  getLlmNodeOutputs,
  validatePublicOutputKey,
  type NodeRuntimeUiContract
} from '../index';

describe('schema v2 constants', () => {
  it('exports the flow and node contribution schema versions', () => {
    expect(FLOW_SCHEMA_VERSION).toBe('1flowbase.flow/v2');
    expect(NODE_CONTRIBUTION_SCHEMA_VERSION).toBe(
      '1flowbase.node-contribution/v2'
    );
  });
});

describe('public output key validation', () => {
  it.each([
    'metadata',
    'usage',
    'debug',
    'error',
    'route',
    'attempts',
    'finish_reason',
    'provider_instance_id',
    'provider_code',
    'protocol',
    'model',
    'event_count',
    'queue_snapshot_id',
    'provider_metadata',
    'provider_events',
    'tool_calls',
    'mcp_calls',
    'raw_response_ref',
    'raw_response_refs',
    'raw_ref',
    'raw_refs',
    'context_projection_ref',
    'context_projection_refs',
    'attempt_ref',
    'attempt_refs',
    '__raw'
  ])(
    'rejects reserved public output key %s',
    (key) => {
      expect(validatePublicOutputKey(key)).toEqual({
        ok: false,
        reason: 'reserved_public_output_key'
      });
    }
  );

  it.each(['text', 'structured_output', 'answer', 'record_id'])(
    'accepts public output key %s',
    (key) => {
      expect(validatePublicOutputKey(key)).toEqual({ ok: true });
    }
  );
});

describe('LLM authoring outputs', () => {
  it('defaults LLM public outputs to text only', () => {
    expect(DEFAULT_LLM_NODE_OUTPUTS).toEqual([
      { key: 'text', title: '模型输出', valueType: 'string' }
    ]);
  });

  it('seeds the default LLM node without runtime trace outputs', () => {
    const document = createDefaultAgentFlowDocument({ flowId: 'flow-1' });
    const llmNode = document.graph.nodes.find((node) => node.id === 'node-llm');

    expect(llmNode?.outputs).toEqual([
      { key: 'text', title: '模型输出', valueType: 'string' }
    ]);
  });

  it('adds structured_output only for explicitly structured response formats', () => {
    expect(getLlmNodeOutputs()).toEqual([
      { key: 'text', title: '模型输出', valueType: 'string' }
    ]);
    expect(getLlmNodeOutputs({ response_format: { mode: 'text' } })).toEqual([
      { key: 'text', title: '模型输出', valueType: 'string' }
    ]);
    expect(
      getLlmNodeOutputs({ response_format: { mode: 'json_schema' } })
    ).toEqual([
      { key: 'text', title: '模型输出', valueType: 'string' },
      { key: 'structured_output', title: '结构化输出', valueType: 'json' }
    ]);
    expect(
      getLlmNodeOutputs({ response_format: { mode: 'json_object' } })
    ).toEqual([
      { key: 'text', title: '模型输出', valueType: 'string' },
      { key: 'structured_output', title: '结构化输出', valueType: 'json' }
    ]);
  });
});

describe('node runtime UI contract type', () => {
  it('supports the required first-class contract sections', () => {
    const contract = {
      meta: {
        type: 'llm',
        title: 'LLM'
      },
      defaults: {
        alias: 'LLM',
        description: '',
        configVersion: 1,
        config: {},
        bindings: {},
        outputs: [{ key: 'text', title: '模型输出', valueType: 'string' }]
      },
      ports: {
        inputs: [{ key: 'input', title: 'Input' }],
        outputs: [{ key: 'output', title: 'Output' }]
      },
      card: {
        title: 'LLM',
        description: 'Generate text'
      },
      panel: {
        sections: []
      },
      runtime: {
        outputs: [{ key: 'text', title: '模型输出', valueType: 'string' }]
      },
      policies: {
        sideEffect: 'external_read'
      }
    } satisfies NodeRuntimeUiContract;

    expect(contract.defaults.outputs[0]?.key).toBe('text');
  });
});
