import {
  getDefaultApiBaseUrl,
  patchConsoleMeMeta,
  type ApiBaseUrlLocation,
  type ConsoleMe,
  type ConsoleUserMeta
} from '@1flowbase/api-client';

export type UserPreferencePatch = ConsoleUserMeta;

export function getUserPreferencesApiBaseUrl(
  locationLike: ApiBaseUrlLocation | undefined =
    typeof window !== 'undefined' ? window.location : undefined
): string {
  return import.meta.env.VITE_API_BASE_URL ?? getDefaultApiBaseUrl(locationLike);
}

export function patchUserPreferences(
  patch: UserPreferencePatch,
  csrfToken: string,
  baseUrl = getUserPreferencesApiBaseUrl()
): Promise<ConsoleMe> {
  return patchConsoleMeMeta(patch, csrfToken, baseUrl);
}
