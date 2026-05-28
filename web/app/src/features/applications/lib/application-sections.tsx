import type { ReactNode } from 'react';

import {
  ApiOutlined,
  DeploymentUnitOutlined,
  FundOutlined,
  UnorderedListOutlined
} from '@ant-design/icons';

import type { SectionNavItem } from '../../../shared/ui/section-page-layout/SectionPageLayout';
import { i18nText } from '../../../shared/i18n/text';

export type ApplicationSectionKey = 'orchestration' | 'api' | 'logs' | 'monitoring';

const SECTION_DEFINITIONS: Array<{
  key: ApplicationSectionKey;
  label: string;
  icon: ReactNode;
}> = [
  {
    key: 'orchestration',
    label: i18nText("applications", "auto.k_63881557e3"),
    icon: <DeploymentUnitOutlined />
  },
  {
    key: 'api',
    label: 'API',
    icon: <ApiOutlined />
  },
  {
    key: 'logs',
    label: i18nText("applications", "auto.k_4de50894b8"),
    icon: <UnorderedListOutlined />
  },
  {
    key: 'monitoring',
    label: i18nText("applications", "auto.k_c87cbd5fc8"),
    icon: <FundOutlined />
  }
];

export function getApplicationSections(applicationId: string): SectionNavItem[] {
  return SECTION_DEFINITIONS.map((section) => ({
    key: section.key,
    label: section.label,
    icon: section.icon,
    to: `/applications/${applicationId}/${section.key}`
  }));
}
