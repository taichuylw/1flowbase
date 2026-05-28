import { basicFields } from '../base';
import type { NodeDefinition } from '../types';
import { i18nText } from '../../../../../shared/i18n/text';

export const variableAssignerNodeDefinition: NodeDefinition = {
  label: 'Variable Assigner',
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
          key: 'bindings.operations',
          label: i18nText("agentFlow", "auto.k_41663264f1"),
          editor: 'state_write',
          required: true
        }
      ]
    },
    {
      key: 'outputs',
      title: 'Outputs',
      fields: [{ key: 'outputs.state', label: i18nText("agentFlow", "auto.k_407e98ca1f"), editor: 'text', required: true }]
    }
  ]
};
