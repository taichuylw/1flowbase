import { basicFields } from '../base';
import type { NodeDefinition } from '../types';
import { i18nText } from '../../../../../shared/i18n/text';

export const llmNodeDefinition: NodeDefinition = {
  label: 'LLM',
  sections: [
    {
      key: 'basics',
      title: 'Basics',
      fields: basicFields
    },
    {
      key: 'inputs',
      title: 'Inputs',
      fields: [
        {
          key: 'config.model_provider',
          label: i18nText("agentFlow", "auto.k_98fd0cbd9c"),
          editor: 'llm_model',
          required: true
        },
        {
          key: 'bindings.prompt_messages',
          label: i18nText("agentFlow", "auto.k_d9aa9fe0d6"),
          editor: 'llm_prompt_messages'
        }
      ]
    },
    {
      key: 'outputs',
      title: 'Outputs',
      fields: [{ key: 'outputs.text', label: i18nText("agentFlow", "auto.k_16de2aff0e"), editor: 'text', required: true }]
    },
    {
      key: 'advanced',
      title: 'Advanced',
      fields: [{ key: 'config.response_format', label: i18nText("agentFlow", "auto.k_dac673286f"), editor: 'llm_response_format' }]
    }
  ]
};
