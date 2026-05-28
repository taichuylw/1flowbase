import { KeyOutlined, UserOutlined } from '@ant-design/icons';

import type { SectionNavItem } from '../../../shared/ui/section-page-layout/SectionPageLayout';
import { i18nText } from '../../../shared/i18n/text';

export type MeSectionKey = 'profile' | 'security';

const ME_SECTIONS: SectionNavItem[] = [
  { key: 'profile', label: i18nText("me", "profile.title"), to: '/me/profile', icon: <UserOutlined /> },
  { key: 'security', label: i18nText("me", "auto.k_8bf435e8a8"), to: '/me/security', icon: <KeyOutlined /> }
];

export function getMeSections(): SectionNavItem[] {
  return ME_SECTIONS;
}
