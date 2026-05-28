import { LogoutOutlined, UserOutlined } from '@ant-design/icons';
import type { MenuProps } from 'antd';
import { i18nText } from '../shared/i18n/text';

export function createAccountMenuItems(accountLabel = i18nText("appShell", "auto.user")): MenuProps['items'] {
  return [
    {
      key: 'account',
      label: (
        <span className="app-shell-account-block">
          <span className="app-shell-account-label">{accountLabel}</span>
        </span>
      ),
      popupClassName: 'app-shell-account-popup',
      children: [
        {
          key: 'profile',
          label: i18nText("appShell", "auto.profile"),
          icon: <UserOutlined />
        },
        {
          key: 'sign-out',
          label: i18nText("appShell", "auto.logout"),
          icon: <LogoutOutlined />
        }
      ]
    }
  ];
}
