const APPLICATION_RUN_COMPATIBILITY_MODE_LABELS: Record<string, string> = {
  'native-v1': 'Native',
  'openai-chat-completions-v1': 'OpenAI Chat',
  'openai-responses-v1': 'OpenAI Responses',
  'anthropic-messages-v1': 'Anthropic'
};

export function formatApplicationRunCompatibilityMode(
  compatibilityMode: string | null | undefined
) {
  const normalized = compatibilityMode?.trim();

  if (!normalized) {
    return '-';
  }

  return APPLICATION_RUN_COMPATIBILITY_MODE_LABELS[normalized] ?? normalized;
}
