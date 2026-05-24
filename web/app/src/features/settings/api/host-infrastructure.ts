import {
  clearConsoleHostInfrastructureCacheDomain,
  clearConsoleHostInfrastructureCacheEntry,
  getConsoleHostInfrastructureMemoryOverview,
  getConsoleHostInfrastructureCacheOverview,
  listConsoleHostInfrastructureCacheEntries,
  listConsoleHostInfrastructureMemoryEntries,
  listConsoleHostInfrastructureProviders,
  revealConsoleHostInfrastructureCacheEntry,
  revealConsoleHostInfrastructureMemoryEntry,
  saveConsoleHostInfrastructureProviderConfig,
  type ConsoleCacheEntryMetadata,
  type ConsoleCacheEntryValue,
  type ConsoleCacheDomain,
  type ConsoleHostInfrastructureMemoryEntries,
  type ConsoleHostInfrastructureMemoryOverview,
  type ConsoleHostInfrastructureProviderConfig,
  type ConsoleHostInfrastructureCacheEntries,
  type ConsoleHostInfrastructureCacheOverview,
  type ConsoleMemoryContractSummary,
  type ConsoleMemoryEntryMetadata,
  type ConsoleMemoryEntryValue,
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
export type SettingsHostInfrastructureMemoryOverview =
  ConsoleHostInfrastructureMemoryOverview;
export type SettingsHostInfrastructureMemoryEntries =
  ConsoleHostInfrastructureMemoryEntries;
export type SettingsHostInfrastructureMemoryContract =
  ConsoleMemoryContractSummary;
export type SettingsHostInfrastructureMemoryEntry = ConsoleMemoryEntryMetadata;
export type SettingsHostInfrastructureMemoryEntryValue =
  ConsoleMemoryEntryValue;

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

export const settingsHostInfrastructureMemoryOverviewQueryKey = [
  'settings',
  'host-infrastructure',
  'memory'
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

export function settingsHostInfrastructureMemoryEntriesQueryKey(
  contractCode: string | null
) {
  return [
    'settings',
    'host-infrastructure',
    'memory',
    'contracts',
    contractCode,
    'entries'
  ] as const;
}

export function fetchSettingsHostInfrastructureProviders() {
  return listConsoleHostInfrastructureProviders();
}

export function fetchSettingsHostInfrastructureCacheOverview() {
  return getConsoleHostInfrastructureCacheOverview();
}

export function fetchSettingsHostInfrastructureMemoryOverview() {
  return getConsoleHostInfrastructureMemoryOverview();
}

export function fetchSettingsHostInfrastructureCacheEntries(
  domainCode: string
) {
  return listConsoleHostInfrastructureCacheEntries(domainCode);
}

export function fetchSettingsHostInfrastructureMemoryEntries(
  contractCode: string
) {
  return listConsoleHostInfrastructureMemoryEntries(contractCode);
}

export function revealSettingsHostInfrastructureCacheEntry(
  domainCode: string,
  key: string,
  csrfToken: string
) {
  return revealConsoleHostInfrastructureCacheEntry(domainCode, key, csrfToken);
}

export function revealSettingsHostInfrastructureMemoryEntry(
  contractCode: string,
  key: string,
  csrfToken: string
) {
  return revealConsoleHostInfrastructureMemoryEntry(
    contractCode,
    key,
    csrfToken
  );
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
