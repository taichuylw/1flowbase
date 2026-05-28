import { basicFields } from '../base';
import type { NodeDefinition } from '../types';
import { i18nText } from '../../../../../shared/i18n/text';

export const codeNodeDefinition: NodeDefinition = {
  label: 'Code',
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
          key: 'bindings.named_bindings',
          label: i18nText("agentFlow", "auto.k_9a45fe515b"),
          editor: 'named_bindings'
        }
      ]
    },
    {
      key: 'advanced',
      title: 'JavaScript',
      fields: [
        {
          key: 'config.source',
          label: i18nText("agentFlow", "auto.k_934ea42891"),
          editor: 'code_source',
          required: true
        }
      ]
    },
    {
      key: 'outputs',
      title: i18nText("agentFlow", "auto.k_1860add605"),
      fields: [
        {
          key: 'config.output_contract',
          label: i18nText("agentFlow", "auto.k_1860add605"),
          editor: 'output_contract_definition'
        }
      ]
    }
  ]
};
