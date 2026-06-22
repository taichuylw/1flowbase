import { apiFetch } from '../transport';

export type ConsolePersonalAccessTokenExpirationPolicy =
  | '30d'
  | '1y'
  | '3y'
  | 'never';

export interface ConsolePersonalAccessToken {
  id: string;
  name: string;
  token: string | null;
  token_prefix: string;
  key_kind: string;
  role_code: string | null;
  creator_user_id: string;
  tenant_id: string;
  scope_kind: string;
  scope_id: string;
  enabled: boolean;
  revoked: boolean;
  expires_at: string | null;
  last_used_at: string | null;
  created_at: string;
  updated_at: string;
}

export interface ConsolePersonalAccessTokenListResponse {
  items: ConsolePersonalAccessToken[];
}

export interface ConsolePersonalAccessTokenRoleOption {
  code: string;
  name: string;
  scope_kind: 'system' | 'workspace';
}

export interface ConsolePersonalAccessTokenRoleOptionsResponse {
  items: ConsolePersonalAccessTokenRoleOption[];
}

export interface CreateConsolePersonalAccessTokenInput {
  name: string;
  role_code: string;
  expiration_policy: ConsolePersonalAccessTokenExpirationPolicy;
}

export interface RevokeConsolePersonalAccessTokenResponse {
  id: string;
}

export type CreatedConsolePersonalAccessToken = ConsolePersonalAccessToken & {
  token: string;
};

export function listConsolePersonalAccessTokens(
  baseUrl?: string
): Promise<ConsolePersonalAccessTokenListResponse> {
  return apiFetch<ConsolePersonalAccessTokenListResponse>({
    path: '/api/console/user-api-keys',
    baseUrl
  });
}

export function createConsolePersonalAccessToken(
  input: CreateConsolePersonalAccessTokenInput,
  csrfToken: string,
  baseUrl?: string
): Promise<CreatedConsolePersonalAccessToken> {
  return apiFetch<CreatedConsolePersonalAccessToken>({
    path: '/api/console/user-api-keys',
    method: 'POST',
    body: input,
    csrfToken,
    baseUrl
  });
}

export function listConsolePersonalAccessTokenRoleOptions(
  baseUrl?: string
): Promise<ConsolePersonalAccessTokenRoleOptionsResponse> {
  return apiFetch<ConsolePersonalAccessTokenRoleOptionsResponse>({
    path: '/api/console/user-api-keys/role-options',
    baseUrl
  });
}

export function revokeConsolePersonalAccessToken(
  apiKeyId: string,
  csrfToken: string,
  baseUrl?: string
): Promise<RevokeConsolePersonalAccessTokenResponse> {
  return apiFetch<RevokeConsolePersonalAccessTokenResponse>({
    path: `/api/console/user-api-keys/${encodeURIComponent(apiKeyId)}/revoke`,
    method: 'POST',
    csrfToken,
    baseUrl
  });
}
