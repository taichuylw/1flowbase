import type { SectionNavItem } from '../../../shared/ui/section-page-layout/SectionPageLayout';
import { i18nText } from '../../../shared/i18n/text';

export type SettingsSectionKey =
  | 'docs'
  | 'system-runtime'
  | 'host-infrastructure'
  | 'memory-observation'
  | 'files'
  | 'data-models'
  | 'model-providers'
  | 'members'
  | 'roles';

export interface SettingsSectionNavItem extends SectionNavItem {
  key: SettingsSectionKey;
}

export interface SettingsSectionDefinition extends SettingsSectionNavItem {
  requiredPermissions: string[];
}

export const settingsSectionDefinitions: SettingsSectionDefinition[] = [
  {
    key: 'docs',
    label: i18nText("settings", "auto.k_ddd798b421"),
    to: '/settings/docs',
    requiredPermissions: ['api_reference.view.all']
  },
  {
    key: 'system-runtime',
    label: i18nText("settings", "auto.k_5027fd1718"),
    to: '/settings/system-runtime',
    requiredPermissions: ['system_runtime.view.all']
  },
  {
    key: 'host-infrastructure',
    label: i18nText("settings", "auto.k_add2c7fd5b"),
    to: '/settings/host-infrastructure',
    requiredPermissions: ['plugin_config.view.all']
  },
  {
    key: 'memory-observation',
    label: i18nText("settings", "auto.k_5d461a917d"),
    to: '/settings/memory-observation',
    requiredPermissions: ['plugin_config.view.all']
  },
  {
    key: 'files',
    label: i18nText("settings", "auto.k_3f2244c98f"),
    to: '/settings/files',
    requiredPermissions: [
      'file_table.view.all',
      'file_table.view.own',
      'file_table.create.all'
    ]
  },
  {
    key: 'data-models',
    label: i18nText("settings", "auto.k_a3ccf702c5"),
    to: '/settings/data-models',
    requiredPermissions: [
      'state_model.view.all',
      'state_model.view.own',
      'state_model.manage.all',
      'state_model.manage.own'
    ]
  },
  {
    key: 'model-providers',
    label: i18nText("settings", "auto.k_77d78db072"),
    to: '/settings/model-providers',
    requiredPermissions: [
      'state_model.view.all',
      'state_model.view.own',
      'state_model.manage.all',
      'state_model.manage.own'
    ]
  },
  {
    key: 'members',
    label: i18nText("settings", "auto.k_baf84751a2"),
    to: '/settings/members',
    requiredPermissions: ['user.view.all']
  },
  {
    key: 'roles',
    label: i18nText("settings", "auto.k_e47b7f25dd"),
    to: '/settings/roles',
    requiredPermissions: ['role_permission.view.all']
  }
];
