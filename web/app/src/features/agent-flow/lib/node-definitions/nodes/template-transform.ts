import { basicFields } from '../base';
import type { NodeDefinition } from '../types';
import { i18nText } from '../../../../../shared/i18n/text';

export const templateTransformNodeDefinition: NodeDefinition = {
  label: 'Template Transform',
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
          key: 'bindings.template',
          label: i18nText("agentFlow", "auto.key_agnapdinnc"),
          editor: 'templated_text',
          required: true
        }
      ]
    },
    {
      key: 'outputs',
      title: 'Outputs',
      fields: [{ key: 'outputs.text', label: i18nText("agentFlow", "auto.key_nkbhnkfien"), editor: 'text', required: true }]
    }
  ]
};
