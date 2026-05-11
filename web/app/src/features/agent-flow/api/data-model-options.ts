import {
  fetchConsoleAgentFlowDataModelOptions,
  type ConsoleAgentFlowDataModelFieldOption,
  type ConsoleAgentFlowDataModelOption,
  type ConsoleAgentFlowDataModelOptionState
} from '@1flowbase/api-client';

import { getApplicationsApiBaseUrl } from '../../applications/api/applications';

export type AgentFlowDataModelOptionState =
  ConsoleAgentFlowDataModelOptionState;
export type AgentFlowDataModelFieldOption = ConsoleAgentFlowDataModelFieldOption;
export type AgentFlowDataModelOption = ConsoleAgentFlowDataModelOption;

export const dataModelOptionsQueryKey = ['agent-flow', 'data-model-options'] as const;

export async function fetchDataModelOptions() {
  return fetchConsoleAgentFlowDataModelOptions(getApplicationsApiBaseUrl());
}
