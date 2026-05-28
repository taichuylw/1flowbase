import { Alert, Space } from 'antd';
import { useQuery } from '@tanstack/react-query';

import {
  fetchSettingsHostInfrastructureProviders,
  settingsHostInfrastructureProvidersQueryKey
} from '../../api/host-infrastructure';
import { SettingsSectionSurface } from '../SettingsSectionSurface';
import { HostInfrastructureProviderTable } from './HostInfrastructureProviderTable';
import './host-infrastructure-panel.css';
import { i18nText } from '../../../../shared/i18n/text';

export function HostInfrastructurePanel({ canManage }: { canManage: boolean }) {
  const providersQuery = useQuery({
    queryKey: settingsHostInfrastructureProvidersQueryKey,
    queryFn: fetchSettingsHostInfrastructureProviders
  });

  return (
    <SettingsSectionSurface title={i18nText("settings", "auto.infrastructure")} hideHeader heightMode="fill">
      <Space
        direction="vertical"
        size={16}
        className="host-infrastructure-panel"
      >
        <Alert
          type="info"
          showIcon
          message={i18nText("settings", "auto.key_hjpaloedpe")}
        />
        <HostInfrastructureProviderTable
          providers={providersQuery.data ?? []}
          loading={providersQuery.isLoading}
          canManage={canManage}
        />
      </Space>
    </SettingsSectionSurface>
  );
}
