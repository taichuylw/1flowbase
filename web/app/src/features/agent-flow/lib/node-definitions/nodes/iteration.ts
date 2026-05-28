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
        { key: 'bindings.items', label: i18nText("agentFlow", "auto.circular_list"), editor: 'selector', required: true }
      ]
    },
    {
      key: 'outputs',
      title: 'Outputs',
      fields: [{ key: 'outputs.result', label: i18nText("agentFlow", "auto.aggregate_output"), editor: 'text', required: true }]
    }
  ]
};
