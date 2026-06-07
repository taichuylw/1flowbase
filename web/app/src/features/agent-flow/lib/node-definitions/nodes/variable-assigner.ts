import { basicFields } from '../base';
import type { NodeDefinition } from '../types';
import { i18nText } from '../../../../../shared/i18n/text';

export const variableAssignerNodeDefinition: NodeDefinition = {
  label: '变量赋值',
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
          label: i18nText("agentFlow", "auto.variable_alt"),
          editor: 'variable_assignment',
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
