import { basicFields } from '../base';
import type { NodeDefinition } from '../types';
import { i18nText } from '../../../../../shared/i18n/text';

export const humanInputNodeDefinition: NodeDefinition = {
  label: 'Human Input',
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
          key: 'config.prompt',
          label: i18nText("agentFlow", "auto.k_b0b7c8309e"),
          editor: 'templated_text',
          required: true
        }
      ]
    },
    {
      key: 'outputs',
      title: 'Outputs',
      fields: [{ key: 'outputs.input', label: i18nText("agentFlow", "auto.k_640eee29f1"), editor: 'text', required: true }]
    }
  ]
};
