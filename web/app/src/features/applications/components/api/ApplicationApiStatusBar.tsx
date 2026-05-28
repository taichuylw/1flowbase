import type { ReactNode } from 'react';

import { Alert, Space, Switch, Typography } from 'antd';

import type { ApplicationApiPublication } from '../../api/public-api';
import { i18nText } from '../../../../shared/i18n/text';

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
  if (!publication) {
    return (
      <section aria-label={i18nText("applications", "auto.k_0b262b3fc7")} className="application-api-status">
        <div className="application-api-status__header">
          <Alert
            type="warning"
            showIcon
            message={i18nText("applications", "auto.k_fb86a8f40c")}
            description={i18nText("applications", "auto.k_af2691a1d8")}
          />
          <div className="application-api-status__docs-toolbar">{toolbar}</div>
          <div className="application-api-status__actions">{children}</div>
        </div>
      </section>
    );
  }

  return (
    <section aria-label={i18nText("applications", "auto.k_0b262b3fc7")} className="application-api-status">
      <div className="application-api-status__header">
        <Space className="application-api-status__summary" align="center" wrap>
          <Typography.Text strong>{i18nText("applications", "auto.k_570e4c1e63")}</Typography.Text>
          <Switch
            checked={publication.api_enabled}
            loading={loading}
            checkedChildren={i18nText("applications", "auto.k_d4e9ca3dd4")}
            unCheckedChildren={i18nText("applications", "auto.k_d989e55188")}
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
