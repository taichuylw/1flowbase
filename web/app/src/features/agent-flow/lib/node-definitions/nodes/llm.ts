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
          label: i18nText('agentFlow', 'auto.model'),
          editor: 'llm_model',
          required: true
        },
        {
          key: 'config.context_policy',
          label: '集成上下文',
          editor: 'llm_context_policy'
        },
        {
          key: 'config.external_reasoning_policy',
          label: i18nText('agentFlow', 'auto.follow_external_reasoning'),
          editor: 'llm_external_reasoning_policy'
        },
        {
          key: 'bindings.prompt_messages',
          label: i18nText('agentFlow', 'auto.context_alt'),
          editor: 'llm_prompt_messages'
        }
      ]
    },
    {
      key: 'outputs',
      title: 'Outputs',
      fields: [
        {
          key: 'outputs.text',
          label: i18nText('agentFlow', 'auto.model_output'),
          editor: 'text',
          required: true
        }
      ]
    },
    {
      key: 'advanced',
      title: 'Advanced',
      fields: [
        {
          key: 'config.response_format',
          label: i18nText('agentFlow', 'auto.return_format'),
          editor: 'llm_response_format'
        }
      ]
    }
  ]
};
