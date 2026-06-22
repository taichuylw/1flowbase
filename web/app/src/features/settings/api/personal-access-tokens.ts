import {
  createConsolePersonalAccessToken,
  listConsolePersonalAccessTokens,
  revokeConsolePersonalAccessToken,
  type ConsolePersonalAccessToken,
  type CreateConsolePersonalAccessTokenInput
} from '@1flowbase/api-client';

export type SettingsPersonalAccessToken = ConsolePersonalAccessToken;
export type CreateSettingsPersonalAccessTokenInput =
  CreateConsolePersonalAccessTokenInput;

export const settingsPersonalAccessTokensQueryKey = [
  'settings',
  'personal-access-tokens'
] as const;

export async function fetchSettingsPersonalAccessTokens(): Promise<
  SettingsPersonalAccessToken[]
> {
  const response = await listConsolePersonalAccessTokens();

  return response.items;
}

export function createSettingsPersonalAccessToken(
  input: CreateSettingsPersonalAccessTokenInput,
  csrfToken: string
): Promise<SettingsPersonalAccessToken> {
  return createConsolePersonalAccessToken(input, csrfToken);
}

export async function revokeSettingsPersonalAccessToken(
  apiKeyId: string,
  csrfToken: string
): Promise<void> {
  await revokeConsolePersonalAccessToken(apiKeyId, csrfToken);
}
