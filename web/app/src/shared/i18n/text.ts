import type { TOptions } from 'i18next';

import { appI18n } from './app-i18n';

export function i18nText(namespace: string, key: string, options?: TOptions): string {
  return appI18n.t(key, {
    ns: namespace,
    ...options
  });
}
