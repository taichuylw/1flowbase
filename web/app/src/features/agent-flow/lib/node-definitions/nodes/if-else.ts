import { basicFields } from '../base';
import type { NodeDefinition } from '../types';
import { i18nText } from '../../../../../shared/i18n/text';

export const ifElseNodeDefinition: NodeDefinition = {
  label: 'IfElse',
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
          key: 'bindings.condition_group',
          label: i18nText("agentFlow", "auto.key_fdgnfhocml"),
          editor: 'condition_group',
          required: true
        }
      ]
    },
    {
      key: 'outputs',
      title: 'Outputs',
      fields: []
    }
  ]
};
