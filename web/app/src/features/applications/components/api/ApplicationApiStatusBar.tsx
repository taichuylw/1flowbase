import type { ReactNode } from 'react';
import { useTranslation } from 'react-i18next';

import { Space, Switch, Typography } from 'antd';

import type { ApplicationApiPublication } from '../../api/public-api';

export function ApplicationApiStatusBar({
  publication,
  loading,
  onToggleEnabled,
  toolbar,
  children
}: {
  publication: ApplicationApiPublication | null;
  loading?: boolean;
  onToggleEnabled?: (enabled: boolean) => void;
  toolbar?: ReactNode;
  children?: ReactNode;
}) {
  const { t } = useTranslation('applications');

  if (!publication) {
    return (
      <section aria-label={t('auto.public_api_status')} className="application-api-status">
        <div className="application-api-status__header">
          <Space className="application-api-status__summary" align="center" wrap>
            <Typography.Text strong>{t('auto.public_api')}</Typography.Text>
            <Switch
              checked={false}
              disabled
              checkedChildren={t('auto.enable')}
              unCheckedChildren={t('auto.disable')}
            />
          </Space>
          <div className="application-api-status__docs-toolbar">{toolbar}</div>
          <div className="application-api-status__actions">{children}</div>
        </div>
      </section>
    );
  }

  return (
    <section aria-label={t('auto.public_api_status')} className="application-api-status">
      <div className="application-api-status__header">
        <Space className="application-api-status__summary" align="center" wrap>
          <Typography.Text strong>{t('auto.public_api')}</Typography.Text>
          <Switch
            checked={publication.api_enabled}
            loading={loading}
            checkedChildren={t('auto.enable')}
            unCheckedChildren={t('auto.disable')}
            onChange={onToggleEnabled}
          />
          <Typography.Text type="secondary">
            active publication v{publication.version_sequence}
          </Typography.Text>
        </Space>
        <div className="application-api-status__docs-toolbar">{toolbar}</div>
        <div className="application-api-status__actions">{children}</div>
      </div>
    </section>
  );
}
