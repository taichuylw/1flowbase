import { basicFields } from '../base';
import type { NodeDefinition } from '../types';
import { i18nText } from '../../../../../shared/i18n/text';

export const toolResultNodeDefinition: NodeDefinition = {
  label: 'Tool Result',
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
          key: 'bindings.result_template',
          label: i18nText('agentFlow', 'auto.tool_result_content'),
          editor: 'templated_text',
          required: true
        }
      ]
    },
    {
      key: 'outputs',
      title: 'Outputs',
      fields: [
        {
          key: 'outputs.result',
          label: i18nText('agentFlow', 'auto.tool_result_output'),
          editor: 'text',
          required: true
        }
      ]
    }
  ]
};
