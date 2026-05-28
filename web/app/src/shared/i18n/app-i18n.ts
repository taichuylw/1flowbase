import i18next from 'i18next';
import resourcesToBackend from 'i18next-resources-to-backend';
import { initReactI18next } from 'react-i18next';

import { FALLBACK_APP_LOCALE, SUPPORTED_APP_LOCALES } from './locales';

const appTranslationLoaders = {
  appShell: {
    zh_Hans: () => import('../../app-shell/i18n/zh_Hans.json'),
    en_US: () => import('../../app-shell/i18n/en_US.json')
  },
  auth: {
    zh_Hans: () => import('../../features/auth/i18n/zh_Hans.json'),
    en_US: () => import('../../features/auth/i18n/en_US.json')
  },
  me: {
    zh_Hans: () => import('../../features/me/i18n/zh_Hans.json'),
    en_US: () => import('../../features/me/i18n/en_US.json')
  }
} as const;

type AppTranslationNamespace = keyof typeof appTranslationLoaders;

function isAppTranslationNamespace(namespace: string): namespace is AppTranslationNamespace {
  return namespace in appTranslationLoaders;
}

export const appI18n = i18next.createInstance();

void appI18n
  .use(initReactI18next)
  .use(
    resourcesToBackend((language: string, namespace: string) => {
      if (!isAppTranslationNamespace(namespace)) {
        throw new Error(`Unknown translation namespace: ${namespace}`);
      }

      const localeLoaders = appTranslationLoaders[namespace];
      const loadTranslation = localeLoaders[language as keyof typeof localeLoaders];

      if (!loadTranslation) {
        throw new Error(`Unknown translation locale: ${language}`);
      }

      return loadTranslation();
    })
  )
  .init({
    fallbackLng: FALLBACK_APP_LOCALE,
    supportedLngs: [...SUPPORTED_APP_LOCALES],
    defaultNS: 'me',
    interpolation: {
      escapeValue: false
    },
    react: {
      useSuspense: true
    }
  });
