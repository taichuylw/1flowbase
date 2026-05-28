import type {
  FlowBinding,
  FlowNodeOutputDocument,
  FlowNodeType,
  NodeRuntimePanelFieldDocument,
  NodeRuntimePanelSectionDocument,
  NodeRuntimeUiContract
} from '@1flowbase/flow-schema';
import {
  NODE_CONTRIBUTION_SCHEMA_VERSION,
  getLlmNodeOutputs
} from '@1flowbase/flow-schema';

import {
  getDataModelNodeDefaultConfig,
  getDataModelNodeOutputs,
  getDataModelActionForNodeType
} from './nodes/data-model';
import { i18nText } from '../../../../shared/i18n/text';

type BuiltinNodeRuntimeContractType = FlowNodeType;
type ContractCategory = 'io' | 'generation' | 'control' | 'data' | 'external';

const DEFAULT_RUNTIME_POLICY = {
  sideEffect: 'none' as const
};

const COMMON_RUNTIME_INPUTS = [
  { key: 'input_payload', title: 'Input Payload', valueType: 'json' }
];

const DEFAULT_CODE_SOURCE = `function main({arg1, arg2}) {
   const param=arg1 + arg2
    console.log(param)

    return {
        result: param
    }
}`;

function cloneJsonValue<T>(value: T): T {
  return structuredClone(value);
}

function duplicateOutput(
  output: FlowNodeOutputDocument
): FlowNodeOutputDocument {
  return { ...output };
}

function duplicateOutputs(outputs: FlowNodeOutputDocument[]) {
  return outputs.map(duplicateOutput);
}

function duplicateContractDefaults(
  contract: NodeRuntimeUiContract['defaults']
) {
  return {
    ...contract,
    config: cloneJsonValue(contract.config),
    bindings: cloneJsonValue(contract.bindings),
    outputs: duplicateOutputs(contract.outputs)
  };
}

function duplicateContract(
  contract: NodeRuntimeUiContract
): NodeRuntimeUiContract {
  return {
    ...contract,
    meta: { ...contract.meta },
    defaults: duplicateContractDefaults(contract.defaults),
    ports: {
      inputs: contract.ports.inputs.map((port) => ({ ...port })),
      outputs: contract.ports.outputs.map((port) => ({ ...port }))
    },
    card: { ...contract.card },
    panel: {
      sections: cloneJsonValue(contract.panel.sections)
    },
    runtime: {
      inputs: contract.runtime.inputs?.map((item) => ({ ...item })),
      processData: contract.runtime.processData?.map((item) => ({ ...item })),
      outputs: duplicateOutputs(contract.runtime.outputs)
    },
    policies: cloneJsonValue(contract.policies)
  };
}

function createContractDefaults({
  alias,
  description,
  config,
  bindings,
  outputs
}: {
  alias: string;
  description: string;
  config: Record<string, unknown>;
  bindings: Record<string, FlowBinding>;
  outputs: FlowNodeOutputDocument[];
}) {
  return {
    alias,
    description,
    configVersion: 1,
    config,
    bindings,
    outputs
  };
}

function createContractPorts(outputs: FlowNodeOutputDocument[]) {
  return {
    inputs: [],
    outputs: outputs.map((output) => ({
      key: output.key,
      title: output.title,
      valueType: output.valueType
    }))
  };
}

function panelField({
  key,
  title,
  renderer,
  valueType = 'string',
  required,
  description,
  options
}: Omit<NodeRuntimePanelFieldDocument, 'valueType'> & {
  valueType?: string;
}) {
  return {
    key,
    title,
    renderer,
    valueType,
    required,
    description,
    options
  } satisfies NodeRuntimePanelFieldDocument;
}

function panelSection(
  key: string,
  title: string,
  fields: NodeRuntimePanelFieldDocument[]
): NodeRuntimePanelSectionDocument {
  return {
    key,
    title,
    fields
  };
}

const basicsPanelSection = panelSection('basics', 'Basics', [
  panelField({
    key: 'alias',
    title: i18nText("agentFlow", "auto.key_pmgaljeipl"),
    renderer: 'text',
    required: true
  }),
  panelField({ key: 'description', title: i18nText("agentFlow", "auto.key_kbegdcdjie"), renderer: 'text' })
]);

function outputsPanelSection(outputs: FlowNodeOutputDocument[]) {
  return panelSection(
    'outputs',
    'Outputs',
    outputs.map((output) =>
      panelField({
        key: `outputs.${output.key}`,
        title: output.title,
        renderer: 'text',
        valueType: output.valueType,
        required: true
      })
    )
  );
}

function createNodeRuntimeContract({
  type,
  title,
  description,
  category,
  config,
  bindings = {},
  outputs,
  panelSections,
  runtimeOutputs
}: {
  type: BuiltinNodeRuntimeContractType;
  title: string;
  description: string;
  category: ContractCategory;
  config: Record<string, unknown>;
  bindings?: Record<string, FlowBinding>;
  outputs: FlowNodeOutputDocument[];
  panelSections: NodeRuntimePanelSectionDocument[];
  runtimeOutputs?: FlowNodeOutputDocument[];
}): NodeRuntimeUiContract {
  return {
    meta: {
      type,
      title,
      schemaVersion: NODE_CONTRIBUTION_SCHEMA_VERSION
    },
    defaults: createContractDefaults({
      alias: title,
      description,
      config,
      bindings,
      outputs: duplicateOutputs(outputs)
    }),
    ports: createContractPorts(outputs),
    card: {
      title,
      description,
      category
    },
    panel: {
      sections: cloneJsonValue(panelSections)
    },
    runtime: {
      inputs: COMMON_RUNTIME_INPUTS.map((item) => ({ ...item })),
      outputs: duplicateOutputs(runtimeOutputs ?? outputs)
    },
    policies: DEFAULT_RUNTIME_POLICY
  };
}

function createStartContract(): NodeRuntimeUiContract {
  return createNodeRuntimeContract({
    type: 'start',
    title: 'Start',
    description: i18nText("agentFlow", "auto.key_acnhljoecd"),
    category: 'io',
    config: { input_fields: [], model_list: [] },
    outputs: [],
    panelSections: [
      basicsPanelSection,
      panelSection('inputs', i18nText("agentFlow", "auto.key_pemlnglnbd"), [
        panelField({
          key: 'config.input_fields',
          title: i18nText("agentFlow", "auto.key_pemlnglnbd"),
          renderer: 'start_input_fields',
          valueType: 'array'
        })
      ]),
      panelSection('advanced', i18nText("agentFlow", "auto.key_mchbncjbbi"), [
        panelField({
          key: 'config.model_list',
          title: i18nText("agentFlow", "auto.key_mchbncjbbi"),
          renderer: 'start_model_list',
          valueType: 'array'
        })
      ])
    ]
  });
}

function createLlmContract(): NodeRuntimeUiContract {
  const outputs = getLlmNodeOutputs({ response_format: { mode: 'text' } });

  return createNodeRuntimeContract({
    type: 'llm',
    title: 'LLM',
    description: i18nText("agentFlow", "auto.key_ehoglgigeb"),
    category: 'generation',
    config: {
      model_provider: {
        provider_code: '',
        model_id: ''
      },
      llm_parameters: {
        schema_version: '1.0.0',
        items: {}
      },
      response_format: {
        mode: 'text'
      }
    },
    bindings: {
      prompt_messages: {
        kind: 'prompt_messages',
        value: [
          {
            id: 'system-1',
            role: 'system',
            content: { kind: 'templated_text', value: '' }
          }
        ]
      }
    },
    outputs,
    runtimeOutputs: outputs,
    panelSections: [
      basicsPanelSection,
      panelSection('inputs', 'Inputs', [
        panelField({
          key: 'config.model_provider',
          title: i18nText("agentFlow", "auto.key_jipnamlnjm"),
          renderer: 'llm_model',
          valueType: 'json',
          required: true
        }),
        panelField({
          key: 'bindings.prompt_messages',
          title: i18nText("agentFlow", "auto.key_njkkjpoang"),
          renderer: 'llm_prompt_messages',
          valueType: 'array'
        })
      ]),
      outputsPanelSection(outputs),
      panelSection('advanced', 'Advanced', [
        panelField({
          key: 'config.response_format',
          title: i18nText("agentFlow", "auto.key_nkmghdcigp"),
          renderer: 'llm_response_format',
          valueType: 'json'
        })
      ])
    ]
  });
}

function createAnswerContract(): NodeRuntimeUiContract {
  const outputs = [{ key: 'answer', title: i18nText("agentFlow", "auto.key_gohhkaedfc"), valueType: 'string' }];

  return createNodeRuntimeContract({
    type: 'answer',
    title: 'Answer',
    description: i18nText("agentFlow", "auto.key_ikpcmgegod"),
    category: 'io',
    config: {},
    outputs,
    panelSections: [
      basicsPanelSection,
      panelSection('inputs', 'Inputs', [
        panelField({
          key: 'bindings.answer_template',
          title: i18nText("agentFlow", "auto.key_kdgmhihndf"),
          renderer: 'templated_text',
          required: true
        })
      ]),
      outputsPanelSection(outputs)
    ]
  });
}

function createKnowledgeRetrievalContract(): NodeRuntimeUiContract {
  const outputs = [{ key: 'documents', title: i18nText("agentFlow", "auto.key_lbhndjcmkb"), valueType: 'array' }];

  return createNodeRuntimeContract({
    type: 'knowledge_retrieval',
    title: 'Knowledge Retrieval',
    description: i18nText("agentFlow", "auto.key_mebamdkebn"),
    category: 'generation',
    config: { top_k: 4 },
    outputs,
    panelSections: [
      basicsPanelSection,
      panelSection('inputs', 'Inputs', [
        panelField({
          key: 'bindings.query',
          title: i18nText("agentFlow", "auto.key_ckbndafkag"),
          renderer: 'selector',
          required: true
        })
      ]),
      outputsPanelSection(outputs),
      panelSection('policy', 'Policy', [
        panelField({
          key: 'config.top_k',
          title: 'Top K',
          renderer: 'number',
          valueType: 'number'
        })
      ])
    ]
  });
}

function createQuestionClassifierContract(): NodeRuntimeUiContract {
  const outputs = [{ key: 'label', title: i18nText("agentFlow", "auto.key_aoppooijcm"), valueType: 'string' }];

  return createNodeRuntimeContract({
    type: 'question_classifier',
    title: 'Question Classifier',
    description: i18nText("agentFlow", "auto.key_coacfmohmn"),
    category: 'control',
    config: { classes: [] },
    outputs,
    panelSections: [
      basicsPanelSection,
      panelSection('inputs', 'Inputs', [
        panelField({
          key: 'bindings.question',
          title: i18nText("agentFlow", "auto.key_bmhibkgdnh"),
          renderer: 'selector',
          required: true
        })
      ]),
      outputsPanelSection(outputs)
    ]
  });
}

function createIfElseContract(): NodeRuntimeUiContract {
  return createNodeRuntimeContract({
    type: 'if_else',
    title: 'If / Else',
    description: i18nText("agentFlow", "auto.key_jdjhlbjnlo"),
    category: 'control',
    config: { mode: 'all' },
    outputs: [],
    panelSections: [
      basicsPanelSection,
      panelSection('inputs', 'Inputs', [
        panelField({
          key: 'bindings.condition_group',
          title: i18nText("agentFlow", "auto.key_fdgnfhocml"),
          renderer: 'condition_group',
          valueType: 'json',
          required: true
        })
      ]),
      outputsPanelSection([])
    ]
  });
}

function createCodeContract(): NodeRuntimeUiContract {
  const outputs = [{ key: 'result', title: 'result', valueType: 'string' }];

  return createNodeRuntimeContract({
    type: 'code',
    title: 'Code',
    description: i18nText("agentFlow", "auto.key_kbmmnecplj"),
    category: 'data',
    config: { language: 'javascript', source: DEFAULT_CODE_SOURCE },
    bindings: {
      named_bindings: {
        kind: 'named_bindings',
        value: [
          {
            name: 'arg1',
            content: { kind: 'templated_text', value: '' }
          },
          {
            name: 'arg2',
            content: { kind: 'templated_text', value: '' }
          }
        ]
      }
    },
    outputs,
    panelSections: [
      basicsPanelSection,
      panelSection('inputs', 'Inputs', [
        panelField({
          key: 'bindings.named_bindings',
          title: i18nText("agentFlow", "auto.key_jkefpofbfl"),
          renderer: 'templated_named_bindings',
          valueType: 'json'
        })
      ]),
      panelSection('code', 'JavaScript', [
        panelField({
          key: 'config.source',
          title: i18nText("agentFlow", "auto.key_jdeokecijb"),
          renderer: 'code_source',
          valueType: 'string',
          required: true
        })
      ]),
      panelSection('outputs', i18nText("agentFlow", "auto.key_bigaknngaf"), [
        panelField({
          key: 'config.output_contract',
          title: i18nText("agentFlow", "auto.key_bigaknngaf"),
          renderer: 'output_contract_definition',
          valueType: 'array'
        })
      ])
    ]
  });
}

function createTemplateTransformContract(): NodeRuntimeUiContract {
  const outputs = [{ key: 'text', title: i18nText("agentFlow", "auto.key_nkbhnkfien"), valueType: 'string' }];

  return createNodeRuntimeContract({
    type: 'template_transform',
    title: 'Template Transform',
    description: i18nText("agentFlow", "auto.key_lfefknnpgl"),
    category: 'generation',
    config: { template: '' },
    outputs,
    panelSections: [
      basicsPanelSection,
      panelSection('inputs', 'Inputs', [
        panelField({
          key: 'bindings.template',
          title: i18nText("agentFlow", "auto.key_agnapdinnc"),
          renderer: 'templated_text',
          required: true
        })
      ]),
      outputsPanelSection(outputs)
    ]
  });
}

function createHttpRequestContract(): NodeRuntimeUiContract {
  const outputs = [{ key: 'body', title: i18nText("agentFlow", "auto.key_kmfcgapicj"), valueType: 'json' }];

  return createNodeRuntimeContract({
    type: 'http_request',
    title: 'HTTP Request',
    description: i18nText("agentFlow", "auto.key_dkifpengag"),
    category: 'external',
    config: { method: 'GET', url: '' },
    outputs,
    panelSections: [
      basicsPanelSection,
      panelSection('inputs', 'Inputs', [
        panelField({
          key: 'config.url',
          title: 'URL',
          renderer: 'templated_text',
          required: true
        }),
        panelField({
          key: 'bindings.body',
          title: i18nText("agentFlow", "auto.key_kfcgnldjpl"),
          renderer: 'templated_text'
        })
      ]),
      outputsPanelSection(outputs),
      panelSection('policy', 'Policy', [
        panelField({
          key: 'config.method',
          title: 'Method',
          renderer: 'text'
        })
      ])
    ]
  });
}

function createToolContract(): NodeRuntimeUiContract {
  const outputs = [{ key: 'result', title: i18nText("agentFlow", "auto.key_plhonncdbp"), valueType: 'unknown' }];

  return createNodeRuntimeContract({
    type: 'tool',
    title: 'Tool',
    description: i18nText("agentFlow", "auto.key_ijbjliocdo"),
    category: 'external',
    config: { tool_name: '' },
    outputs,
    panelSections: [
      basicsPanelSection,
      panelSection('inputs', 'Inputs', [
        panelField({
          key: 'config.tool_name',
          title: i18nText("agentFlow", "auto.key_ocinhgfdam"),
          renderer: 'text',
          required: true
        }),
        panelField({
          key: 'bindings.parameters',
          title: i18nText("agentFlow", "auto.key_domngakbhh"),
          renderer: 'named_bindings',
          valueType: 'json'
        })
      ]),
      outputsPanelSection(outputs)
    ]
  });
}

function createPluginNodeContract(): NodeRuntimeUiContract {
  const outputs = [{ key: 'result', title: i18nText("agentFlow", "auto.key_jndmnolnfc"), valueType: 'json' }];

  return createNodeRuntimeContract({
    type: 'plugin_node',
    title: 'Plugin Node',
    description: i18nText("agentFlow", "auto.key_jnldloelll"),
    category: 'external',
    config: {},
    outputs,
    panelSections: [basicsPanelSection, outputsPanelSection(outputs)]
  });
}

function createVariableAssignerContract(): NodeRuntimeUiContract {
  const outputs = [{ key: 'state', title: i18nText("agentFlow", "auto.key_eahojimkbp"), valueType: 'json' }];

  return createNodeRuntimeContract({
    type: 'variable_assigner',
    title: 'Variable Assigner',
    description: i18nText("agentFlow", "auto.key_fdgagfkfdf"),
    category: 'data',
    config: { writes: [] },
    outputs,
    panelSections: [
      basicsPanelSection,
      panelSection('inputs', 'Inputs', [
        panelField({
          key: 'bindings.operations',
          title: i18nText("agentFlow", "auto.key_ebggdcgepb"),
          renderer: 'state_write',
          valueType: 'array',
          required: true
        })
      ]),
      outputsPanelSection(outputs)
    ]
  });
}

function createParameterExtractorContract(): NodeRuntimeUiContract {
  const outputs = [{ key: 'parameters', title: i18nText("agentFlow", "auto.key_lomlepapba"), valueType: 'json' }];

  return createNodeRuntimeContract({
    type: 'parameter_extractor',
    title: 'Parameter Extractor',
    description: i18nText("agentFlow", "auto.key_clmnekmoin"),
    category: 'data',
    config: { schema: [] },
    outputs,
    panelSections: [
      basicsPanelSection,
      panelSection('inputs', 'Inputs', [
        panelField({
          key: 'bindings.source_text',
          title: i18nText("agentFlow", "auto.key_cfmjbhfpkj"),
          renderer: 'selector',
          required: true
        })
      ]),
      outputsPanelSection(outputs)
    ]
  });
}

function createIterationContract(): NodeRuntimeUiContract {
  const outputs = [{ key: 'result', title: i18nText("agentFlow", "auto.key_lkngeimdmc"), valueType: 'array' }];

  return createNodeRuntimeContract({
    type: 'iteration',
    title: 'Iteration',
    description: i18nText("agentFlow", "auto.key_eiaelgfdbg"),
    category: 'control',
    config: { max_steps: 10 },
    outputs,
    panelSections: [
      basicsPanelSection,
      panelSection('inputs', 'Inputs', [
        panelField({
          key: 'bindings.items',
          title: i18nText("agentFlow", "auto.key_cbbffkdmpf"),
          renderer: 'selector',
          required: true
        })
      ]),
      outputsPanelSection(outputs)
    ]
  });
}

function createLoopContract(): NodeRuntimeUiContract {
  const outputs = [{ key: 'result', title: i18nText("agentFlow", "auto.key_lkngeimdmc"), valueType: 'array' }];

  return createNodeRuntimeContract({
    type: 'loop',
    title: 'Loop',
    description: i18nText("agentFlow", "auto.key_gfleiobaoe"),
    category: 'control',
    config: { max_rounds: 10 },
    outputs,
    panelSections: [
      basicsPanelSection,
      panelSection('inputs', 'Inputs', [
        panelField({
          key: 'bindings.entry_condition',
          title: i18nText("agentFlow", "auto.key_onfhklmfjc"),
          renderer: 'condition_group',
          valueType: 'json',
          required: true
        })
      ]),
      panelSection('policy', 'Policy', [
        panelField({
          key: 'config.max_rounds',
          title: i18nText("agentFlow", "auto.key_hlgcbncgli"),
          renderer: 'number',
          valueType: 'number'
        })
      ]),
      outputsPanelSection(outputs)
    ]
  });
}

function createHumanInputContract(): NodeRuntimeUiContract {
  const outputs = [{ key: 'input', title: i18nText("agentFlow", "auto.key_geaooocjpb"), valueType: 'string' }];

  return createNodeRuntimeContract({
    type: 'human_input',
    title: 'Human Input',
    description: i18nText("agentFlow", "auto.key_nlgenmcpcj"),
    category: 'io',
    config: {},
    outputs,
    panelSections: [
      basicsPanelSection,
      panelSection('inputs', 'Inputs', [
        panelField({
          key: 'config.prompt',
          title: i18nText("agentFlow", "auto.key_lalhmidajo"),
          renderer: 'templated_text',
          required: true
        })
      ]),
      outputsPanelSection(outputs)
    ]
  });
}

type BuiltinDataModelRuntimeContractType =
  | 'data_model_list'
  | 'data_model_get'
  | 'data_model_create'
  | 'data_model_update'
  | 'data_model_delete';

const DATA_MODEL_NODE_TITLES = {
  data_model_list: 'Data Model List',
  data_model_get: 'Data Model Get',
  data_model_create: 'Data Model Create',
  data_model_update: 'Data Model Update',
  data_model_delete: 'Data Model Delete'
} satisfies Record<BuiltinDataModelRuntimeContractType, string>;

function createDataModelContract(
  nodeType: BuiltinDataModelRuntimeContractType
): NodeRuntimeUiContract {
  const action = getDataModelActionForNodeType(nodeType);

  if (!action) {
    throw new Error(`Unsupported Data Model node type: ${nodeType}`);
  }

  const outputs = getDataModelNodeOutputs(action);
  const inputFields: NodeRuntimePanelFieldDocument[] = [
    panelField({
      key: 'config.data_model_code',
      title: 'Data Model',
      renderer: 'data_model',
      required: true
    })
  ];

  if (action === 'list') {
    inputFields.push(
      panelField({
        key: 'bindings.query',
        title: 'Query',
        renderer: 'data_model_query',
        valueType: 'json'
      })
    );
  }

  if (action === 'get' || action === 'update' || action === 'delete') {
    inputFields.push(
      panelField({
        key: 'bindings.record_id',
        title: 'Record ID',
        renderer: 'selector',
        required: true
      })
    );
  }

  if (action === 'create' || action === 'update') {
    inputFields.push(
      panelField({
        key: 'bindings.payload',
        title: 'Payload',
        renderer: 'named_bindings',
        valueType: 'json',
        required: true
      })
    );
  }

  if (action === 'create' || action === 'update' || action === 'delete') {
    inputFields.push(
      panelField({
        key: 'config.side_effect_policy',
        title: 'Side Effect Policy',
        renderer: 'static_select',
        required: true,
        options: [
          { label: 'Disabled', value: 'disabled' },
          { label: 'Confirm Each Run', value: 'confirm_each_run' },
          { label: 'Allow With Idempotency', value: 'allow_with_idempotency' }
        ]
      })
    );
  }

  return createNodeRuntimeContract({
    type: nodeType,
    title: DATA_MODEL_NODE_TITLES[nodeType],
    description: i18nText("agentFlow", "auto.key_ommempekmn"),
    category: 'data',
    config: getDataModelNodeDefaultConfig(nodeType),
    outputs,
    panelSections: [
      basicsPanelSection,
      panelSection('inputs', 'Inputs', inputFields),
      outputsPanelSection(outputs)
    ]
  });
}

export const builtinNodeRuntimeContractTypes = [
  'start',
  'answer',
  'llm',
  'knowledge_retrieval',
  'question_classifier',
  'if_else',
  'code',
  'template_transform',
  'http_request',
  'tool',
  'data_model_list',
  'data_model_get',
  'data_model_create',
  'data_model_update',
  'data_model_delete',
  'variable_assigner',
  'parameter_extractor',
  'iteration',
  'loop',
  'human_input',
  'plugin_node'
] as const;

export const BUILTIN_NODE_RUNTIME_CONTRACTS: Record<
  BuiltinNodeRuntimeContractType,
  NodeRuntimeUiContract
> = {
  start: createStartContract(),
  answer: createAnswerContract(),
  llm: createLlmContract(),
  knowledge_retrieval: createKnowledgeRetrievalContract(),
  question_classifier: createQuestionClassifierContract(),
  if_else: createIfElseContract(),
  code: createCodeContract(),
  template_transform: createTemplateTransformContract(),
  http_request: createHttpRequestContract(),
  tool: createToolContract(),
  data_model_list: createDataModelContract('data_model_list'),
  data_model_get: createDataModelContract('data_model_get'),
  data_model_create: createDataModelContract('data_model_create'),
  data_model_update: createDataModelContract('data_model_update'),
  data_model_delete: createDataModelContract('data_model_delete'),
  variable_assigner: createVariableAssignerContract(),
  parameter_extractor: createParameterExtractorContract(),
  iteration: createIterationContract(),
  loop: createLoopContract(),
  human_input: createHumanInputContract(),
  plugin_node: createPluginNodeContract()
};

export function getBuiltinNodeRuntimeContract(
  nodeType: FlowNodeType
): NodeRuntimeUiContract | null {
  const contract =
    BUILTIN_NODE_RUNTIME_CONTRACTS[nodeType as BuiltinNodeRuntimeContractType];

  return contract ? duplicateContract(contract) : null;
}

export type { BuiltinNodeRuntimeContractType };
