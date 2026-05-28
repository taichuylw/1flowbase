import type { ReactNode } from 'react';

import type { SectionNavItem } from '../../../../shared/ui/section-page-layout/SectionPageLayout';
import { SectionPageLayout } from '../../../../shared/ui/section-page-layout/SectionPageLayout';
import { i18nText } from '../../../../shared/i18n/text';

export const SETTINGS_PAGE_TITLE = i18nText("settings", "auto.key_hnolpjmlad");
export const SETTINGS_PAGE_DESCRIPTION =
  i18nText("settings", "auto.key_ncghbmihhg");

export function SettingsNavigation({
  activeKey,
  navItems,
  children,
  heightMode = 'natural'
}: {
  activeKey: string;
  navItems: SectionNavItem[];
  children: ReactNode;
  heightMode?: 'natural' | 'viewport';
}) {
  return (
    <SectionPageLayout
      pageDescription={SETTINGS_PAGE_DESCRIPTION}
      navItems={navItems}
      activeKey={activeKey}
      contentWidth="wide"
      heightMode={heightMode}
      hideCompactNav
    >
      {children}
    </SectionPageLayout>
  );
}
