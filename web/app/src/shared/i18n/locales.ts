export const SUPPORTED_APP_LOCALES = ['zh_Hans', 'en_US'] as const;

export type AppLocale = (typeof SUPPORTED_APP_LOCALES)[number];

export const FALLBACK_APP_LOCALE: AppLocale = 'en_US';

export function toAppLocale(locale: string | null | undefined): AppLocale | null {
  if (!locale) {
    return null;
  }

  const normalized = locale.replace('-', '_').toLowerCase();

  if (
    normalized === 'zh' ||
    normalized === 'zh_cn' ||
    normalized === 'zh_hans' ||
    normalized === 'zh_hans_cn' ||
    normalized === 'zh_sg'
  ) {
    return 'zh_Hans';
  }

  if (normalized === 'en' || normalized === 'en_us' || normalized === 'en_gb') {
    return 'en_US';
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
