import { basicFields } from '../base';
import type { NodeDefinition } from '../types';
import { i18nText } from '../../../../../shared/i18n/text';

export const iterationNodeDefinition: NodeDefinition = {
  label: 'Iteration',
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
        { key: 'bindings.items', label: i18nText("agentFlow", "auto.k_21155a3cf5"), editor: 'selector', required: true }
      ]
    },
    {
      key: 'outputs',
      title: 'Outputs',
      fields: [{ key: 'outputs.result', label: i18nText("agentFlow", "auto.k_bad648c3c2"), editor: 'text', required: true }]
    }
  ]
};
