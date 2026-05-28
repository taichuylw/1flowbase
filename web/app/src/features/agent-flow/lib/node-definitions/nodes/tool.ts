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
        { key: 'config.tool_name', label: i18nText("agentFlow", "auto.k_e28d76530c"), editor: 'text', required: true },
        { key: 'bindings.parameters', label: i18nText("agentFlow", "auto.k_3ecd60a177"), editor: 'named_bindings' }
      ]
    },
    {
      key: 'outputs',
      title: 'Outputs',
      fields: [{ key: 'outputs.result', label: i18nText("agentFlow", "auto.k_fb7edd231f"), editor: 'text', required: true }]
    }
  ]
};
