import { describe, expect, it } from 'vitest';

import { FALLBACK_APP_LOCALE, resolveAppLocale, SUPPORTED_APP_LOCALES, toAppLocale } from '../locales';

describe('app locale canonicalization', () => {
  it('uses backend canonical locale codes for app runtime', () => {
    expect(SUPPORTED_APP_LOCALES).toEqual(['zh_Hans', 'en_US']);
    expect(FALLBACK_APP_LOCALE).toBe('en_US');
  });

  it('normalizes browser and URL locale aliases to backend canonical locale codes', () => {
    expect(toAppLocale('zh-CN')).toBe('zh_Hans');
    expect(toAppLocale('zh_Hans')).toBe('zh_Hans');
    expect(toAppLocale('en-US')).toBe('en_US');
    expect(toAppLocale('en_US')).toBe('en_US');
  });

  it('resolves preferred locale before browser locale and falls back to en_US', () => {
    expect(resolveAppLocale('zh-CN', ['en-US'])).toBe('zh_Hans');
    expect(resolveAppLocale(null, ['en-US'])).toBe('en_US');
    expect(resolveAppLocale('fr-FR', ['de-DE'])).toBe('en_US');
  });
});
