import { basicFields } from '../base';
import type { NodeDefinition } from '../types';
import { i18nText } from '../../../../../shared/i18n/text';

export const loopNodeDefinition: NodeDefinition = {
  label: 'Loop',
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
          key: 'bindings.entry_condition',
          label: i18nText("agentFlow", "auto.key_onfhklmfjc"),
          editor: 'condition_group',
          required: true
        }
      ]
    },
    {
      key: 'policy',
      title: 'Policy',
      fields: [{ key: 'config.max_rounds', label: i18nText("agentFlow", "auto.key_hlgcbncgli"), editor: 'number' }]
    },
    {
      key: 'outputs',
      title: 'Outputs',
      fields: [{ key: 'outputs.result', label: i18nText("agentFlow", "auto.key_lkngeimdmc"), editor: 'text', required: true }]
    }
  ]
};
