import {
  FileTextOutlined,
  GithubOutlined,
  QuestionCircleOutlined
} from '@ant-design/icons';
import type { ReactNode } from 'react';
import { Menu } from 'antd';
import { i18nText } from '../shared/i18n/text';

const HELP_LINKS = [
  {
    key: 'github',
    label: 'github',
    icon: <GithubOutlined />,
    href: 'https://github.com/taichuy/1flowbase'
  },
  {
    key: 'docs',
    label: i18nText("appShell", "auto.k_1069127253"),
    icon: <FileTextOutlined />,
    href: 'https://docs.taichuy.com/'
  }
] satisfies Array<{
  key: string;
  label: string;
  icon: ReactNode;
  href: string;
}>;

export function HelpChromeMenu() {
  return (
    <Menu
      className="app-shell-help-menu"
      mode="horizontal"
      selectable={false}
      items={[
        {
          key: 'help',
          label: (
            <span className="app-shell-help-block" aria-label={i18nText("appShell", "auto.k_adf465ebf0")}>
              <QuestionCircleOutlined />
            </span>
          ),
          popupClassName: 'app-shell-help-popup',
          children: HELP_LINKS.map((item) => ({
            key: item.key,
            label: (
              <a
                className="app-shell-help-popup__link"
                href={item.href}
                target="_blank"
                rel="noreferrer"
              >
                <span className="app-shell-help-popup__link-icon">
                  {item.icon}
                </span>
                <span>{item.label}</span>
              </a>
            )
          }))
        }
      ]}
      disabledOverflow
    />
  );
}
