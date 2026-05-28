import { basicFields } from '../base';
import type { NodeDefinition } from '../types';
import { i18nText } from '../../../../../shared/i18n/text';

export const toolNodeDefinition: NodeDefinition = {
  label: 'Tool',
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
        { key: 'config.tool_name', label: i18nText("agentFlow", "auto.tool_name"), editor: 'text', required: true },
        { key: 'bindings.parameters', label: i18nText("agentFlow", "auto.tool_input_parameters"), editor: 'named_bindings' }
      ]
    },
    {
      key: 'outputs',
      title: 'Outputs',
      fields: [{ key: 'outputs.result', label: i18nText("agentFlow", "auto.tool_output"), editor: 'text', required: true }]
    }
  ]
};
