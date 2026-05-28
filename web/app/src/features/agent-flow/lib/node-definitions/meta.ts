import type { NodeDefinitionMetaMap } from './types';
import { dataModelNodeMeta } from './nodes/data-model';
import { i18nText } from '../../../../shared/i18n/text';

export const nodeDefinitionMeta: NodeDefinitionMetaMap = {
  start: {
    summary: i18nText("agentFlow", "auto.key_nphhcmoaab"),
    helpHref: '/docs/agentflow/nodes/start'
  },
  answer: {
    summary: i18nText("agentFlow", "auto.key_chlehcafba"),
    helpHref: '/docs/agentflow/nodes/answer'
  },
  llm: {
    summary: i18nText("agentFlow", "auto.key_fhdmhpkhno"),
    helpHref: '/docs/agentflow/nodes/llm'
  },
  knowledge_retrieval: {
    summary: i18nText("agentFlow", "auto.key_mebamdkebn"),
    helpHref: '/docs/agentflow/nodes/knowledge-retrieval'
  },
  question_classifier: {
    summary: i18nText("agentFlow", "auto.key_coacfmohmn"),
    helpHref: '/docs/agentflow/nodes/question-classifier'
  },
  if_else: {
    summary: i18nText("agentFlow", "auto.key_cgljhioeag"),
    helpHref: '/docs/agentflow/nodes/if-else'
  },
  code: {
    summary: i18nText("agentFlow", "auto.key_kbmmnecplj"),
    helpHref: '/docs/agentflow/nodes/code'
  },
  template_transform: {
    summary: i18nText("agentFlow", "auto.key_bcicnkjflk"),
    helpHref: '/docs/agentflow/nodes/template-transform'
  },
  http_request: {
    summary: i18nText("agentFlow", "auto.key_mcfokcdnli"),
    helpHref: '/docs/agentflow/nodes/http-request'
  },
  tool: {
    summary: i18nText("agentFlow", "auto.key_chffgcfkai"),
    helpHref: '/docs/agentflow/nodes/tool'
  },
  ...dataModelNodeMeta,
  variable_assigner: {
    summary: i18nText("agentFlow", "auto.key_ikhofgojcb"),
    helpHref: '/docs/agentflow/nodes/variable-assigner'
  },
  parameter_extractor: {
    summary: i18nText("agentFlow", "auto.key_clmnekmoin"),
    helpHref: '/docs/agentflow/nodes/parameter-extractor'
  },
  iteration: {
    summary: i18nText("agentFlow", "auto.key_pnmhinikpg"),
    helpHref: '/docs/agentflow/nodes/iteration',
    canEnterContainer: true
  },
  loop: {
    summary: i18nText("agentFlow", "auto.key_fhalkoibpd"),
    helpHref: '/docs/agentflow/nodes/loop',
    canEnterContainer: true
  },
  human_input: {
    summary: i18nText("agentFlow", "auto.key_pibdlijfam"),
    helpHref: '/docs/agentflow/nodes/human-input'
  },
  plugin_node: {
    summary: i18nText("agentFlow", "auto.plugin_node_definition_summary"),
    helpHref: null
  }
};
