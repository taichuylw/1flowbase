import { basicFields } from '../base';
import type { NodeDefinition } from '../types';
import { i18nText } from '../../../../../shared/i18n/text';

export const questionClassifierNodeDefinition: NodeDefinition = {
  label: 'Question Classifier',
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
          key: 'bindings.question',
          label: i18nText("agentFlow", "auto.k_1c781a63d7"),
          editor: 'selector',
          required: true
        }
      ]
    },
    {
      key: 'outputs',
      title: 'Outputs',
      fields: [{ key: 'outputs.label', label: i18nText("agentFlow", "auto.k_0effee892c"), editor: 'text', required: true }]
    }
  ]
};
