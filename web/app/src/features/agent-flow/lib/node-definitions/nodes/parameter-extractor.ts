import { basicFields } from '../base';
import type { NodeDefinition } from '../types';
import { i18nText } from '../../../../../shared/i18n/text';

export const parameterExtractorNodeDefinition: NodeDefinition = {
  label: 'Parameter Extractor',
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
          key: 'bindings.source_text',
          label: i18nText("agentFlow", "auto.key_cfmjbhfpkj"),
          editor: 'selector',
          required: true
        }
      ]
    },
    {
      key: 'outputs',
      title: 'Outputs',
      fields: [
        {
          key: 'outputs.parameters',
          label: i18nText("agentFlow", "auto.key_lomlepapba"),
          editor: 'text',
          required: true
        }
      ]
    }
  ]
};
