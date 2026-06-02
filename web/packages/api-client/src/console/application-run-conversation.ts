import type {
  ConsoleApplicationConversationMessagesPage,
  GetConsoleApplicationConversationMessagesInput
} from './application-runtime';
import { apiFetch } from '../transport';

export function getConsoleApplicationRunConversationMessages(
  applicationId: string,
  runId: string,
  input: GetConsoleApplicationConversationMessagesInput = {},
  baseUrl?: string
) {
  const searchParams = new URLSearchParams();
  if (input.before !== undefined) {
    searchParams.set('before', input.before);
  }
  if (input.after !== undefined) {
    searchParams.set('after', input.after);
  }
  if (input.limit !== undefined) {
    searchParams.set('limit', String(input.limit));
  }

  const queryString = searchParams.toString();

  return apiFetch<ConsoleApplicationConversationMessagesPage>({
    path:
      `/api/console/applications/${applicationId}/logs/runs/${runId}/conversation/messages` +
      (queryString ? `?${queryString}` : ''),
    baseUrl
  });
}
