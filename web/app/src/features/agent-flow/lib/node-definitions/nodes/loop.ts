import { basicFields } from '../base';
import type { NodeDefinition } from '../types';
import { i18nText } from '../../../../../shared/i18n/text';

export const loopNodeDefinition: NodeDefinition = {
  label: 'Loop',
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
          key: 'bindings.entry_condition',
          label: i18nText("agentFlow", "auto.k_ed57abc592"),
          editor: 'condition_group',
          required: true
        }
      ]
    },
    {
      key: 'policy',
      title: 'Policy',
      fields: [{ key: 'config.max_rounds', label: i18nText("agentFlow", "auto.k_7b621d26b8"), editor: 'number' }]
    },
    {
      key: 'outputs',
      title: 'Outputs',
      fields: [{ key: 'outputs.result', label: i18nText("agentFlow", "auto.k_bad648c3c2"), editor: 'text', required: true }]
    }
  ]
};
