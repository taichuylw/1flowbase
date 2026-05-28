import type { FlowNodeOutputDocument } from '@1flowbase/flow-schema';
import { i18nText } from '../../../shared/i18n/text';

export interface AgentFlowSystemVariable extends FlowNodeOutputDocument {
  description: string;
}

export const systemVariableNodeId = 'sys';

export const agentFlowSystemVariables = [
  {
    key: 'conversation_id',
    title: 'sys.conversation_id',
    valueType: 'string',
    description: i18nText("agentFlow", "auto.k_9490434cd4")
  },
  {
    key: 'dialog_count',
    title: 'sys.dialog_count',
    valueType: 'number',
    description: i18nText("agentFlow", "auto.k_026dd41243")
  },
  {
    key: 'user_id',
    title: 'sys.user_id',
    valueType: 'string',
    description: i18nText("agentFlow", "auto.k_ee7720935b")
  },
  {
    key: 'app_id',
    title: 'sys.app_id',
    valueType: 'string',
    description: i18nText("agentFlow", "auto.k_642feaef05")
  },
  {
    key: 'workflow_id',
    title: 'sys.workflow_id',
    valueType: 'string',
    description: i18nText("agentFlow", "auto.k_494ba91036")
  },
  {
    key: 'workflow_run_id',
    title: 'sys.workflow_run_id',
    valueType: 'string',
    description: i18nText("agentFlow", "auto.k_f6d7ee5ee4")
  }
] satisfies AgentFlowSystemVariable[];
