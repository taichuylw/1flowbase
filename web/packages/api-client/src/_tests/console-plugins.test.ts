import { describe, expect, test, vi } from 'vitest';
import * as transport from '../transport';

import {
  clearConsoleHostInfrastructureCacheDomain,
  clearConsoleHostInfrastructureCacheEntry,
  getConsoleHostInfrastructureCacheOverview,
  listConsoleHostInfrastructureCacheEntries,
  revealConsoleHostInfrastructureCacheEntry
} from '../console-plugins';

describe('console-plugins host infrastructure cache client', () => {
  const apiFetchSpy = vi
    .spyOn(transport, 'apiFetch')
    .mockImplementation(async (input) => input as never);

  test('transport spy is active', () => {
    expect(apiFetchSpy).toHaveBeenCalledTimes(0);
  });

  test('points cache overview at the host infrastructure cache route', async () => {
    await expect(
      getConsoleHostInfrastructureCacheOverview()
    ).resolves.toMatchObject({
      path: '/api/console/settings/host-infrastructure/cache'
    });
  });

  test('points cache entries at the selected domain route', async () => {
    await expect(
      listConsoleHostInfrastructureCacheEntries('application-logs')
    ).resolves.toMatchObject({
      path: '/api/console/settings/host-infrastructure/cache/domains/application-logs/entries'
    });
  });

  test('reveals cache entry value with csrf', async () => {
    await expect(
      revealConsoleHostInfrastructureCacheEntry(
        'application-logs',
        'application-logs:run:1',
        'csrf-123'
      )
    ).resolves.toMatchObject({
      path: '/api/console/settings/host-infrastructure/cache/domains/application-logs/entries/reveal',
      method: 'POST',
      body: { key: 'application-logs:run:1' },
      csrfToken: 'csrf-123'
    });
  });

  test('clears cache entry with csrf', async () => {
    await expect(
      clearConsoleHostInfrastructureCacheEntry(
        'application-logs',
        'application-logs:run:1',
        'csrf-123'
      )
    ).resolves.toMatchObject({
      path: '/api/console/settings/host-infrastructure/cache/domains/application-logs/entries/clear',
      method: 'POST',
      body: { key: 'application-logs:run:1' },
      csrfToken: 'csrf-123'
    });
  });

  test('clears cache domain with csrf', async () => {
    await expect(
      clearConsoleHostInfrastructureCacheDomain('application-logs', 'csrf-123')
    ).resolves.toMatchObject({
      path: '/api/console/settings/host-infrastructure/cache/domains/application-logs/clear',
      method: 'POST',
      csrfToken: 'csrf-123'
    });
  });
});
