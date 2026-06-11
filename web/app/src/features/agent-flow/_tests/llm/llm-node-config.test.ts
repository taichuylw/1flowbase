import { describe, expect, test } from 'vitest';

import {
  DEFAULT_LLM_PARAMETERS,
  DEFAULT_LLM_CONTEXT_POLICY,
  DEFAULT_LLM_EXTERNAL_REASONING_POLICY,
  DEFAULT_LLM_VISIBLE_INTERNAL_TOOLS_ENABLED,
  DEFAULT_LLM_VISIBLE_INTERNAL_TOOLS,
  getLlmContextPolicy,
  getLlmExternalReasoningPolicy,
  getLlmParameterDefaultValue,
  getLlmModelProvider,
  getLlmParameters,
  getLlmVisibleInternalToolsEnabled,
  getLlmVisibleInternalTools,
  getLlmToolExternalToolPolicy,
  DEFAULT_LLM_EXTERNAL_TOOL_POLICY
} from '../../lib/llm-node-config';

describe('llm-node-config', () => {
  test('getLlmModelProvider only reads the current model_provider contract', () => {
    expect(
      getLlmModelProvider({
        provider_code: 'legacy_provider',
        model: 'legacy-model',
        protocol: 'legacy'
      })
    ).toEqual({
      provider_code: '',
      model_id: '',
      protocol: undefined,
      provider_label: undefined,
      model_label: undefined,
      schema_fetched_at: undefined
    });
  });

  test('getLlmModelProvider reads stable provider and model from the nested contract', () => {
    expect(
      getLlmModelProvider({
        model_provider: {
          provider_code: 'openai_compatible',
          model_id: 'gpt-4o-mini',
          protocol: 'openai_compatible',
          provider_label: 'OpenAI Compatible',
          model_label: 'gpt-4o-mini',
          schema_fetched_at: '2026-04-23T10:00:00Z'
        }
      })
    ).toEqual({
      provider_code: 'openai_compatible',
      model_id: 'gpt-4o-mini',
      protocol: 'openai_compatible',
      provider_label: 'OpenAI Compatible',
      model_label: 'gpt-4o-mini',
      schema_fetched_at: '2026-04-23T10:00:00Z'
    });
  });

  test('getLlmParameters ignores legacy flat parameter fields', () => {
    expect(
      getLlmParameters({
        temperature: 0.7,
        top_p_enabled: true,
        top_p: 0.9,
        max_tokens_enabled: true,
        max_tokens: 1024
      })
    ).toEqual(DEFAULT_LLM_PARAMETERS);
  });

  test('getLlmContextPolicy defaults integration context to enabled', () => {
    expect(getLlmContextPolicy({})).toEqual(DEFAULT_LLM_CONTEXT_POLICY);
    expect(
      getLlmContextPolicy({
        context_policy: {
          integration_context: 'disabled',
          context_selector: ['node-code', 'result', 'chat_history']
        }
      })
    ).toEqual({
      integration_context: 'disabled',
      context_selector: ['node-code', 'result', 'chat_history']
    });
  });

  test('getLlmExternalReasoningPolicy defaults follow external reasoning to false', () => {
    expect(getLlmExternalReasoningPolicy({})).toEqual(
      DEFAULT_LLM_EXTERNAL_REASONING_POLICY
    );
    expect(
      getLlmExternalReasoningPolicy({
        external_reasoning_policy: {
          follow_external_reasoning: true
        }
      })
    ).toEqual({
      follow_external_reasoning: true
    });
  });

  test('getLlmVisibleInternalToolsEnabled defaults mount tools to disabled', () => {
    expect(getLlmVisibleInternalToolsEnabled({})).toBe(
      DEFAULT_LLM_VISIBLE_INTERNAL_TOOLS_ENABLED
    );
    expect(
      getLlmVisibleInternalToolsEnabled({
        visible_internal_llm_tools_enabled: true
      })
    ).toBe(true);
  });

  test('getLlmVisibleInternalTools keeps only stable tool registrations', () => {
    expect(getLlmVisibleInternalTools({})).toEqual(
      DEFAULT_LLM_VISIBLE_INTERNAL_TOOLS
    );
    expect(
      getLlmVisibleInternalTools({
        visible_internal_llm_tools: [
          {
            type: 'visible_internal_llm_tool',
            tool_name: ' inspect_visible_context ',
            connector_id: ' inspect_visible_context ',
            target_node_id: ' node-mounted-llm ',
            description: ' Read visible context ',
            input_schema: { type: 'object' }
          },
          {
            type: 'visible_internal_llm_tool',
            tool_name: 'inspect_image',
            connector_id: 'inspect_image',
            target_node_id: ' ',
            input_schema: { type: 'object' }
          },
          {
            type: 'external_tool',
            tool_name: 'leak_external',
            target_node_id: 'node-other'
          },
          {
            type: 'visible_internal_llm_tool',
            tool_name: '',
            target_node_id: 'node-empty'
          }
        ]
      })
    ).toEqual([
      {
        type: 'visible_internal_llm_tool',
        tool_name: 'inspect_visible_context',
        connector_id: 'inspect_visible_context',
        target_node_id: 'node-mounted-llm',
        description: 'Read visible context',
        input_schema: { type: 'object' }
      },
      {
        type: 'visible_internal_llm_tool',
        tool_name: 'inspect_image',
        connector_id: 'inspect_image',
        target_node_id: '',
        input_schema: { type: 'object' }
      }
    ]);
  });

  test('getLlmVisibleInternalTools normalizes external_tool_policy to explicit values', () => {
    const [inheritedTool, defaultTool, invalidTool] =
      getLlmVisibleInternalTools({
        visible_internal_llm_tools: [
          {
            type: 'visible_internal_llm_tool',
            tool_name: 'frontend_llm',
            connector_id: 'frontend_llm',
            target_node_id: 'node-mounted-llm',
            external_tool_policy: 'inherited'
          },
          {
            type: 'visible_internal_llm_tool',
            tool_name: 'image_llm',
            connector_id: 'image_llm',
            target_node_id: 'node-mounted-llm'
          },
          {
            type: 'visible_internal_llm_tool',
            tool_name: 'broken_llm',
            connector_id: 'broken_llm',
            target_node_id: 'node-mounted-llm',
            external_tool_policy: 'open'
          }
        ]
      });

    expect(inheritedTool.external_tool_policy).toBe('inherited');
    expect(defaultTool.external_tool_policy).toBeUndefined();
    expect(invalidTool.external_tool_policy).toBeUndefined();
    expect(getLlmToolExternalToolPolicy(inheritedTool)).toBe('inherited');
    expect(getLlmToolExternalToolPolicy(defaultTool)).toBe(
      DEFAULT_LLM_EXTERNAL_TOOL_POLICY
    );
    expect(getLlmToolExternalToolPolicy(invalidTool)).toBe('forbidden');
  });

  test('getLlmParameterDefaultValue derives stable defaults when schema omits them', () => {
    expect(
      getLlmParameterDefaultValue({
        key: 'temperature',
        label: 'Temperature',
        type: 'number',
        required: false,
        advanced: false,
        options: [],
        visible_when: [],
        disabled_when: []
      })
    ).toBe(0);
    expect(
      getLlmParameterDefaultValue({
        key: 'stop',
        label: 'Stop',
        type: 'string_list',
        required: false,
        advanced: false,
        options: [],
        visible_when: [],
        disabled_when: []
      })
    ).toEqual([]);
  });
});
