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
          label: i18nText("agentFlow", "auto.key_lalhmidajo"),
          editor: 'templated_text',
          required: true
        }
      ]
    },
    {
      key: 'outputs',
      title: 'Outputs',
      fields: [{ key: 'outputs.input', label: i18nText("agentFlow", "auto.key_geaooocjpb"), editor: 'text', required: true }]
    }
  ]
};
