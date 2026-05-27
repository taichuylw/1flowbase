export const SUPPORTED_APP_LOCALES = ['zh-CN', 'en-US'] as const;

export type AppLocale = (typeof SUPPORTED_APP_LOCALES)[number];

export const FALLBACK_APP_LOCALE: AppLocale = 'en-US';

export function toAppLocale(locale: string | null | undefined): AppLocale | null {
  if (!locale) {
    return null;
  }

  const normalized = locale.replace('_', '-').toLowerCase();

  if (
    normalized === 'zh' ||
    normalized === 'zh-cn' ||
    normalized === 'zh-hans' ||
    normalized === 'zh-hans-cn'
  ) {
    return 'zh-CN';
  }

  if (normalized === 'en' || normalized === 'en-us') {
    return 'en-US';
  }

  return null;
}

export function resolveAppLocale(
  preferredLocale: string | null | undefined,
  browserLocales: readonly string[] = []
): AppLocale {
  const preferredAppLocale = toAppLocale(preferredLocale);

  if (preferredAppLocale) {
    return preferredAppLocale;
  }

  for (const browserLocale of browserLocales) {
    const appLocale = toAppLocale(browserLocale);

    if (appLocale) {
      return appLocale;
    }
  }

  return FALLBACK_APP_LOCALE;
}
