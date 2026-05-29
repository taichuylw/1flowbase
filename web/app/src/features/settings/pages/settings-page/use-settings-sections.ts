import { useMemo } from 'react';
import { useTranslation } from 'react-i18next';

import {
  settingsSectionDefinitions,
  type SettingsSectionKey,
  type SettingsSectionNavItem
} from '../../lib/settings-sections';

export function useSettingsSections({
  requestedSectionKey,
  isRoot,
  permissions
}: {
  requestedSectionKey?: SettingsSectionKey;
  isRoot: boolean;
  permissions: string[];
}) {
  const { t } = useTranslation('settings');
  const visibleSections = useMemo<SettingsSectionNavItem[]>(
    () =>
      settingsSectionDefinitions
        .filter(
          (section) =>
            isRoot ||
            section.requiredPermissions.some((permission) =>
              permissions.includes(permission)
            )
        )
        .map(({ key, labelKey, to }) => ({ key, label: t(labelKey), to })),
    [isRoot, permissions, t]
  );
  const fallbackSection = visibleSections[0] ?? null;
  const activeSection = requestedSectionKey
    ? (visibleSections.find((section) => section.key === requestedSectionKey) ??
      null)
    : null;
  const redirectSection =
    !requestedSectionKey || !activeSection ? fallbackSection : null;

  return {
    activeSection,
    fallbackSection,
    redirectSection,
    visibleSections
  };
}
