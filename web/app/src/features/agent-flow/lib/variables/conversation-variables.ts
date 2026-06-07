import type {
  FlowAuthoringDocument,
  FlowConversationVariableDocument
} from '@1flowbase/flow-schema';

export type AgentFlowConversationVariable = FlowConversationVariableDocument;

export const conversationVariableNodeId = 'conversation';

export const conversationVariableValueTypeOptions = [
  'string',
  'number',
  'boolean',
  'object',
  'array[string]',
  'array[number]',
  'array[boolean]',
  'array[object]'
].map((value) => ({ label: value, value }));

export function formatConversationVariableTitle(name: string) {
  return `${conversationVariableNodeId}.${name}`;
}

export function listConversationVariables(
  document: FlowAuthoringDocument
): AgentFlowConversationVariable[] {
  return document.variables?.conversation ?? [];
}

export function replaceConversationVariables(
  document: FlowAuthoringDocument,
  conversationVariables: AgentFlowConversationVariable[]
): FlowAuthoringDocument {
  return {
    ...document,
    variables: {
      ...(document.variables ?? {}),
      conversation: conversationVariables.map((variable) => ({
        name: variable.name,
        valueType: variable.valueType,
        description: variable.description?.trim() ?? ''
      }))
    }
  };
}
