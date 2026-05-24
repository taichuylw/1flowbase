import {
  clearConsoleHostInfrastructureCacheDomain,
  clearConsoleHostInfrastructureCacheEntry,
  getConsoleHostInfrastructureCacheOverview,
  listConsoleHostInfrastructureCacheEntries,
  listConsoleHostInfrastructureProviders,
  revealConsoleHostInfrastructureCacheEntry,
  saveConsoleHostInfrastructureProviderConfig,
  type ConsoleCacheEntryMetadata,
  type ConsoleCacheEntryValue,
  type ConsoleCacheDomain,
  type ConsoleHostInfrastructureProviderConfig,
  type ConsoleHostInfrastructureCacheEntries,
  type ConsoleHostInfrastructureCacheOverview,
  type SaveConsoleHostInfrastructureProviderConfigInput
} from '@1flowbase/api-client';

export type SettingsHostInfrastructureProviderConfig =
  ConsoleHostInfrastructureProviderConfig;
export type SettingsHostInfrastructureCacheOverview =
  ConsoleHostInfrastructureCacheOverview;
export type SettingsHostInfrastructureCacheEntries =
  ConsoleHostInfrastructureCacheEntries;
export type SettingsHostInfrastructureCacheDomain = ConsoleCacheDomain;
export type SettingsHostInfrastructureCacheEntry = ConsoleCacheEntryMetadata;
export type SettingsHostInfrastructureCacheEntryValue = ConsoleCacheEntryValue;

export type SaveSettingsHostInfrastructureProviderConfigInput =
  SaveConsoleHostInfrastructureProviderConfigInput;

export const settingsHostInfrastructureProvidersQueryKey = [
  'settings',
  'host-infrastructure',
  'providers'
] as const;

export const settingsHostInfrastructureCacheOverviewQueryKey = [
  'settings',
  'host-infrastructure',
  'cache'
] as const;

export function settingsHostInfrastructureCacheEntriesQueryKey(
  domainCode: string | null
) {
  return [
    'settings',
    'host-infrastructure',
    'cache',
    'domains',
    domainCode,
    'entries'
  ] as const;
}

export function fetchSettingsHostInfrastructureProviders() {
  return listConsoleHostInfrastructureProviders();
}

export function fetchSettingsHostInfrastructureCacheOverview() {
  return getConsoleHostInfrastructureCacheOverview();
}

export function fetchSettingsHostInfrastructureCacheEntries(
  domainCode: string
) {
  return listConsoleHostInfrastructureCacheEntries(domainCode);
}

export function revealSettingsHostInfrastructureCacheEntry(
  domainCode: string,
  key: string,
  csrfToken: string
) {
  return revealConsoleHostInfrastructureCacheEntry(domainCode, key, csrfToken);
}

export function clearSettingsHostInfrastructureCacheEntry(
  domainCode: string,
  key: string,
  csrfToken: string
) {
  return clearConsoleHostInfrastructureCacheEntry(domainCode, key, csrfToken);
}

export function clearSettingsHostInfrastructureCacheDomain(
  domainCode: string,
  csrfToken: string
) {
  return clearConsoleHostInfrastructureCacheDomain(domainCode, csrfToken);
}

export function saveSettingsHostInfrastructureProviderConfig(
  installationId: string,
  providerCode: string,
  input: SaveSettingsHostInfrastructureProviderConfigInput,
  csrfToken: string
) {
  return saveConsoleHostInfrastructureProviderConfig(
    installationId,
    providerCode,
    input,
    csrfToken
  );
}
