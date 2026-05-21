const APPLICATION_RUN_PROTOCOL_LABELS: Record<string, string> = {
  'native-v1': 'Native',
  'openai-chat-completions-v1': 'OpenAI Chat',
  'openai-responses-v1': 'OpenAI Responses',
  'anthropic-messages-v1': 'Anthropic'
};

export function formatApplicationRunProtocol(
  protocol: string | null | undefined
) {
  const normalized = protocol?.trim();

  if (!normalized) {
    return '-';
  }

  return APPLICATION_RUN_PROTOCOL_LABELS[normalized] ?? normalized;
}
