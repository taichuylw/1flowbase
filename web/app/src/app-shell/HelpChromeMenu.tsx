import {
  CloudDownloadOutlined,
  FileTextOutlined,
  GithubOutlined,
  QuestionCircleOutlined
} from '@ant-design/icons';
import { fetchConsoleReleaseStatus } from '@1flowbase/api-client';
import { useQuery } from '@tanstack/react-query';
import type { ReactNode } from 'react';
import { Menu, Spin } from 'antd';
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
    label: i18nText("appShell", "auto.documentation"),
    icon: <FileTextOutlined />,
    href: 'https://docs.taichuy.com/'
  }
] satisfies Array<{
  key: string;
  label: string;
  icon: ReactNode;
  href: string;
}>;

function ReleaseStatusMenuItem() {
  const releaseStatusQuery = useQuery({
    queryKey: ['console-release-status'],
    queryFn: () => fetchConsoleReleaseStatus(),
    staleTime: 20 * 60 * 1000,
    retry: false
  });
  const releaseStatus = releaseStatusQuery.data;
  const currentVersion = formatReleaseVersion(releaseStatus?.current_version);
  const latestVersion =
    releaseStatus?.has_update && releaseStatus.latest_version
      ? formatReleaseVersion(releaseStatus.latest_version)
      : null;

  return (
    <div
      className="app-shell-release-status"
      onClick={(event) => event.stopPropagation()}
    >
      <div className="app-shell-release-status__line">
        <CloudDownloadOutlined />
        {currentVersion ? (
          <span>{currentVersion}</span>
        ) : (
          <Spin size="small" />
        )}
        {releaseStatusQuery.isFetching && currentVersion ? (
          <Spin size="small" />
        ) : null}
      </div>

      {latestVersion ? (
        <span className="app-shell-release-status__latest">
          {latestVersion}
          {i18nText("appShell", "release_status.latest_marker")}
        </span>
      ) : null}
    </div>
  );
}

function formatReleaseVersion(version: string | null | undefined) {
  const normalizedVersion = version?.trim();
  if (!normalizedVersion) {
    return null;
  }
  return normalizedVersion.startsWith('v')
    ? normalizedVersion
    : `v${normalizedVersion}`;
}

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
            <span className="app-shell-help-block" aria-label={i18nText("appShell", "auto.help")}>
              <QuestionCircleOutlined />
            </span>
          ),
          popupClassName: 'app-shell-help-popup',
          children: [
            ...HELP_LINKS.map((item) => ({
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
            })),
            {
              key: 'release-status',
              className: 'app-shell-help-popup__release-menu-item',
              label: <ReleaseStatusMenuItem />
            }
          ]
        }
      ]}
      disabledOverflow
    />
  );
}
