import { Alert, Space, Tabs } from 'antd';
import { useQuery } from '@tanstack/react-query';

import {
  fetchSettingsHostInfrastructureProviders,
  settingsHostInfrastructureProvidersQueryKey
} from '../../api/host-infrastructure';
import { SettingsSectionSurface } from '../SettingsSectionSurface';
import { HostInfrastructureMemoryObservationPanel } from './HostInfrastructureMemoryObservationPanel';
import { HostInfrastructureProviderTable } from './HostInfrastructureProviderTable';
import './host-infrastructure-panel.css';

export function HostInfrastructurePanel({ canManage }: { canManage: boolean }) {
  const providersQuery = useQuery({
    queryKey: settingsHostInfrastructureProvidersQueryKey,
    queryFn: fetchSettingsHostInfrastructureProviders
  });

  return (
    <SettingsSectionSurface title="基础设施" hideHeader heightMode="fill">
      <Tabs
        className="host-infrastructure-panel"
        items={[
          {
            key: 'providers',
            label: 'Provider 配置',
            children: (
              <Space
                direction="vertical"
                size={16}
                className="host-infrastructure-panel__providers"
              >
                <Alert
                  type="info"
                  showIcon
                  message="安装、配置和启用会保存为待应用变更，重启 api-server 一次后生效。"
                />
                <HostInfrastructureProviderTable
                  providers={providersQuery.data ?? []}
                  loading={providersQuery.isLoading}
                  canManage={canManage}
                />
              </Space>
            )
          },
          {
            key: 'memory',
            label: '内存观察',
            children: (
              <HostInfrastructureMemoryObservationPanel canManage={canManage} />
            )
          }
        ]}
      />
    </SettingsSectionSurface>
  );
}
