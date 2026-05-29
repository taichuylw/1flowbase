import type { TOptions } from 'i18next';

import { appI18n } from './app-i18n';

export function i18nText(namespace: string, key: string, options?: TOptions): string {
  const fallback =
    typeof options?.defaultValue === 'string' &&
    options.defaultValue.trim().length > 0
      ? options.defaultValue
      : key;
  const translated = appI18n.t(key, {
    ...options,
    ns: namespace,
    defaultValue: fallback
  });

  return typeof translated === 'string' && translated.trim().length > 0
    ? translated
    : fallback;
}
