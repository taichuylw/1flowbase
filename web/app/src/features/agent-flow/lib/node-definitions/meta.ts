import type { NodeDefinitionMetaMap } from './types';
import { dataModelNodeMeta } from './nodes/data-model';
import { i18nText } from '../../../../shared/i18n/text';

export const nodeDefinitionMeta: NodeDefinitionMetaMap = {
  start: {
    summary: i18nText("agentFlow", "auto.k_df772ce001"),
    helpHref: '/docs/agentflow/nodes/start'
  },
  answer: {
    summary: i18nText("agentFlow", "auto.k_27b4720510"),
    helpHref: '/docs/agentflow/nodes/answer'
  },
  llm: {
    summary: i18nText("agentFlow", "auto.k_573c7fa7de"),
    helpHref: '/docs/agentflow/nodes/llm'
  },
  knowledge_retrieval: {
    summary: i18nText("agentFlow", "auto.k_c410c3a41d"),
    helpHref: '/docs/agentflow/nodes/knowledge-retrieval'
  },
  question_classifier: {
    summary: i18nText("agentFlow", "auto.k_2e025ce7cd"),
    helpHref: '/docs/agentflow/nodes/question-classifier'
  },
  if_else: {
    summary: i18nText("agentFlow", "auto.k_26b978e406"),
    helpHref: '/docs/agentflow/nodes/if-else'
  },
  code: {
    summary: i18nText("agentFlow", "auto.k_a1ccd42fb9"),
    helpHref: '/docs/agentflow/nodes/code'
  },
  template_transform: {
    summary: i18nText("agentFlow", "auto.k_1282da95ba"),
    helpHref: '/docs/agentflow/nodes/template-transform'
  },
  http_request: {
    summary: i18nText("agentFlow", "auto.k_c25ea23db8"),
    helpHref: '/docs/agentflow/nodes/http-request'
  },
  tool: {
    summary: i18nText("agentFlow", "auto.k_2755625a08"),
    helpHref: '/docs/agentflow/nodes/tool'
  },
  ...dataModelNodeMeta,
  variable_assigner: {
    summary: i18nText("agentFlow", "auto.k_8a7e56e921"),
    helpHref: '/docs/agentflow/nodes/variable-assigner'
  },
  parameter_extractor: {
    summary: i18nText("agentFlow", "auto.k_2bcd4ace8d"),
    helpHref: '/docs/agentflow/nodes/parameter-extractor'
  },
  iteration: {
    summary: i18nText("agentFlow", "auto.k_fdc78d8af6"),
    helpHref: '/docs/agentflow/nodes/iteration',
    canEnterContainer: true
  },
  loop: {
    summary: i18nText("agentFlow", "auto.k_570bae81f3"),
    helpHref: '/docs/agentflow/nodes/loop',
    canEnterContainer: true
  },
  human_input: {
    summary: i18nText("agentFlow", "auto.k_f813b8950c"),
    helpHref: '/docs/agentflow/nodes/human-input'
  },
  plugin_node: {
    summary: i18nText("agentFlow", "auto.k_860eca1805"),
    helpHref: null
  }
};
