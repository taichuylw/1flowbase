import { basicFields } from '../base';
import type { NodeDefinition } from '../types';
import { i18nText } from '../../../../../shared/i18n/text';

export const variableAssignerNodeDefinition: NodeDefinition = {
  label: 'Environment Variable Update',
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
          label: i18nText("agentFlow", "auto.environment_variable_update"),
          editor: 'environment_variable_update',
          required: true
        }
      ]
    },
    {
      key: 'outputs',
      title: 'Outputs',
      fields: [{ key: 'outputs.env', label: i18nText("agentFlow", "auto.environment_variables"), editor: 'text', required: true }]
    }
  ]
};
