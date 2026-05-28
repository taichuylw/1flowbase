import { basicFields } from '../base';
import type { NodeDefinition } from '../types';
import { i18nText } from '../../../../../shared/i18n/text';

export const httpRequestNodeDefinition: NodeDefinition = {
  label: 'HTTP Request',
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
        { key: 'config.url', label: 'URL', editor: 'templated_text', required: true },
        { key: 'bindings.body', label: i18nText("agentFlow", "auto.request_body"), editor: 'templated_text' }
      ]
    },
    {
      key: 'outputs',
      title: 'Outputs',
      fields: [{ key: 'outputs.body', label: i18nText("agentFlow", "auto.response_body"), editor: 'text', required: true }]
    },
    {
      key: 'policy',
      title: 'Policy',
      fields: [{ key: 'config.method', label: 'Method', editor: 'text' }]
    }
  ]
};
