import { basicFields } from '../base';
import type { NodeDefinition } from '../types';
import { i18nText } from '../../../../../shared/i18n/text';

export const startNodeDefinition: NodeDefinition = {
  label: 'Start',
  sections: [
    {
      key: 'basics',
      title: 'Basics',
      fields: basicFields
    },
    {
      key: 'inputs',
      title: i18nText("agentFlow", "auto.k_f4cbd6bd13"),
      fields: [
        {
          key: 'config.input_fields',
          label: i18nText("agentFlow", "auto.k_f4cbd6bd13"),
          editor: 'start_input_fields'
        }
      ]
    },
    {
      key: 'advanced',
      title: i18nText("agentFlow", "auto.k_c271d29118"),
      fields: [
        {
          key: 'config.model_list',
          label: i18nText("agentFlow", "auto.k_c271d29118"),
          editor: 'start_model_list'
        }
      ]
    }
  ]
};
