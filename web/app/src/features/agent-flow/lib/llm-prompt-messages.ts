import type {
  FlowBinding,
  LlmPromptMessage,
  LlmPromptMessageRole
} from '@1flowbase/flow-schema';

export const LLM_PROMPT_MESSAGE_ROLES: LlmPromptMessageRole[] = [
  'system',
  'user',
  'assistant'
];

const LLM_DYNAMIC_PROMPT_MESSAGE_ROLES: LlmPromptMessageRole[] = [
  'user',
  'assistant'
];

export function createPromptMessage(
  role: LlmPromptMessageRole,
  index: number,
  value = ''
): LlmPromptMessage {
  return {
    id: `${role}-${index + 1}`,
    role,
    content: {
      kind: 'templated_text',
      value
    }
  };
}

function isFlowBinding(value: unknown): value is FlowBinding {
  return Boolean(value && typeof value === 'object' && 'kind' in value);
}

export function normalizePromptMessagesBinding(
  promptMessages: unknown
): LlmPromptMessage[] {
  if (
    isFlowBinding(promptMessages) &&
    promptMessages.kind === 'prompt_messages' &&
    Array.isArray(promptMessages.value)
  ) {
    const normalizedMessages: LlmPromptMessage[] = promptMessages.value.map(
      (message, index) => ({
        id: message.id || `${message.role}-${index + 1}`,
        role: LLM_PROMPT_MESSAGE_ROLES.includes(message.role)
          ? message.role
          : 'user',
        content: {
          kind: 'templated_text',
          value:
            message.content?.kind === 'templated_text'
              ? message.content.value
              : ''
        }
      })
    );
    const systemMessageIndex = normalizedMessages.findIndex(
      (message) => message.role === 'system'
    );
    const systemMessage =
      systemMessageIndex >= 0
        ? normalizedMessages[systemMessageIndex]
        : createPromptMessage('system', 0);
    const dynamicMessages = normalizedMessages
      .filter((_, index) => index !== systemMessageIndex)
      .map((message, index) => ({
        ...message,
        id: message.id || `user-${index + 2}`,
        role: LLM_DYNAMIC_PROMPT_MESSAGE_ROLES.includes(message.role)
          ? message.role
          : 'user'
      }));

    return [systemMessage, ...dynamicMessages];
  }

  return [createPromptMessage('system', 0)];
}

export function toPromptMessagesBinding(
  messages: LlmPromptMessage[]
): FlowBinding {
  return {
    kind: 'prompt_messages',
    value: messages.map((message, index) => ({
      id: message.id || `${message.role}-${index + 1}`,
      role: message.role,
      content: {
        kind: 'templated_text',
        value: message.content.value
      }
    }))
  };
}
