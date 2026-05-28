import { basicFields } from '../base';
import type { NodeDefinition } from '../types';
import { i18nText } from '../../../../../shared/i18n/text';

export const knowledgeRetrievalNodeDefinition: NodeDefinition = {
  label: 'Knowledge Retrieval',
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
        { key: 'bindings.query', label: i18nText("agentFlow", "auto.k_2a1d305a06"), editor: 'selector', required: true }
      ]
    },
    {
      key: 'outputs',
      title: 'Outputs',
      fields: [
        {
          key: 'outputs.documents',
          label: i18nText("agentFlow", "auto.k_b17d392ca1"),
          editor: 'text',
          required: true
        }
      ]
    },
    {
      key: 'policy',
      title: 'Policy',
      fields: [{ key: 'config.top_k', label: 'Top K', editor: 'number' }]
    }
  ]
};
