import { basicFields } from '../base';
import type { NodeDefinition } from '../types';

export const codeNodeDefinition: NodeDefinition = {
  label: 'Code',
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
          key: 'bindings.named_bindings',
          label: '输入变量',
          editor: 'named_bindings'
        }
      ]
    },
    {
      key: 'advanced',
      title: 'JavaScript',
      fields: [
        {
          key: 'config.source',
          label: 'JavaScript 代码',
          editor: 'code_source',
          required: true
        }
      ]
    },
    {
      key: 'outputs',
      title: '输出变量',
      fields: [
        {
          key: 'config.output_contract',
          label: '输出变量',
          editor: 'output_contract_definition'
        }
      ]
    }
  ]
};
