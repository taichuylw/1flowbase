import { describe, expect, it } from 'vitest';

import { appI18n } from '../app-i18n';
import { i18nText } from '../text';

void appI18n.changeLanguage('en_US');

const moduleScopeSettingsDraftLabel = i18nText('settings', 'auto.draft_alt');

describe('i18nText', () => {
  it('returns namespace text during module initialization', () => {
    expect(moduleScopeSettingsDraftLabel).toBe('Draft');
  });

  it('does not return blank labels from empty translation values', () => {
    appI18n.addResource('en_US', 'shared', 'auto.empty_test_label', '');

    expect(i18nText('shared', 'auto.empty_test_label')).toBe(
      'auto.empty_test_label'
    );
  });

  it('does not suspend React while translation resources settle', () => {
    expect(appI18n.options.react?.useSuspense).toBe(false);
  });
});
