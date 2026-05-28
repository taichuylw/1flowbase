import i18next from 'i18next';
import resourcesToBackend from 'i18next-resources-to-backend';
import { initReactI18next } from 'react-i18next';

import { FALLBACK_APP_LOCALE, SUPPORTED_APP_LOCALES } from './locales';

const appTranslationLoaders = {
  agentFlow: {
    zh_Hans: () => import('../../features/agent-flow/i18n/zh_Hans.json'),
    en_US: () => import('../../features/agent-flow/i18n/en_US.json')
  },
  app: {
    zh_Hans: () => import('../../app/i18n/zh_Hans.json'),
    en_US: () => import('../../app/i18n/en_US.json')
  },
  appShell: {
    zh_Hans: () => import('../../app-shell/i18n/zh_Hans.json'),
    en_US: () => import('../../app-shell/i18n/en_US.json')
  },
  applications: {
    zh_Hans: () => import('../../features/applications/i18n/zh_Hans.json'),
    en_US: () => import('../../features/applications/i18n/en_US.json')
  },
  auth: {
    zh_Hans: () => import('../../features/auth/i18n/zh_Hans.json'),
    en_US: () => import('../../features/auth/i18n/en_US.json')
  },
  embeddedApps: {
    zh_Hans: () => import('../../features/embedded-apps/i18n/zh_Hans.json'),
    en_US: () => import('../../features/embedded-apps/i18n/en_US.json')
  },
  frontstage: {
    zh_Hans: () => import('../../features/frontstage/i18n/zh_Hans.json'),
    en_US: () => import('../../features/frontstage/i18n/en_US.json')
  },
  me: {
    zh_Hans: () => import('../../features/me/i18n/zh_Hans.json'),
    en_US: () => import('../../features/me/i18n/en_US.json')
  },
  schemaUi: {
    zh_Hans: () => import('../../shared/schema-ui/i18n/zh_Hans.json'),
    en_US: () => import('../../shared/schema-ui/i18n/en_US.json')
  },
  settings: {
    zh_Hans: () => import('../../features/settings/i18n/zh_Hans.json'),
    en_US: () => import('../../features/settings/i18n/en_US.json')
  },
  shared: {
    zh_Hans: () => import('./resources/zh_Hans.json'),
    en_US: () => import('./resources/en_US.json')
  },
  sharedUi: {
    zh_Hans: () => import('../../shared/ui/i18n/zh_Hans.json'),
    en_US: () => import('../../shared/ui/i18n/en_US.json')
  },
  tools: {
    zh_Hans: () => import('../../features/tools/i18n/zh_Hans.json'),
    en_US: () => import('../../features/tools/i18n/en_US.json')
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
    ns: Object.keys(appTranslationLoaders),
    supportedLngs: [...SUPPORTED_APP_LOCALES],
    defaultNS: 'me',
    interpolation: {
      escapeValue: false
    },
    react: {
      useSuspense: true
    }
  });
