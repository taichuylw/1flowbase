import {
  CloudDownloadOutlined,
  FileTextOutlined,
  GithubOutlined,
  QuestionCircleOutlined,
  TeamOutlined,
  WarningOutlined
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
  const releaseUrl = releaseStatus?.release_info?.html_url;

  return (
    <div
      className="app-shell-release-status"
      onClick={(event) => event.stopPropagation()}
    >
      <div className="app-shell-release-status__header">
        <span className="app-shell-release-status__title">
          <CloudDownloadOutlined />
          <span>{i18nText("appShell", "release_status.version")}</span>
          {releaseStatus?.has_update ? (
            <span className="app-shell-release-status__dot" aria-hidden="true" />
          ) : null}
        </span>
        {releaseStatusQuery.isFetching ? <Spin size="small" /> : null}
      </div>

      {releaseStatus ? (
        <>
          <dl className="app-shell-release-status__versions">
            <div>
              <dt>{i18nText("appShell", "release_status.current_version")}</dt>
              <dd>{releaseStatus.current_version}</dd>
            </div>
            <div>
              <dt>{i18nText("appShell", "release_status.latest_version")}</dt>
              <dd>{releaseStatus.latest_version}</dd>
            </div>
          </dl>

          <div
            className={
              releaseStatus.has_update
                ? 'app-shell-release-status__state app-shell-release-status__state--update'
                : 'app-shell-release-status__state'
            }
          >
            {releaseStatus.has_update
              ? i18nText("appShell", "release_status.update_available")
              : i18nText("appShell", "release_status.up_to_date")}
          </div>

          {releaseStatus.warning ? (
            <div className="app-shell-release-status__warning">
              <WarningOutlined />
              <span>{i18nText("appShell", "release_status.warning")}</span>
            </div>
          ) : null}

          <div className="app-shell-release-status__links">
            {releaseUrl ? (
              <a href={releaseUrl} target="_blank" rel="noreferrer">
                {i18nText("appShell", "release_status.view_release")}
              </a>
            ) : null}
            <a
              href={releaseStatus.contributors_url}
              target="_blank"
              rel="noreferrer"
            >
              <TeamOutlined />
              {i18nText("appShell", "release_status.contributors")}
            </a>
          </div>

          <details className="app-shell-release-status__upgrade">
            <summary>{i18nText("appShell", "release_status.docker_upgrade")}</summary>
            <code>{releaseStatus.upgrade_commands.shell}</code>
            <code>{releaseStatus.upgrade_commands.powershell}</code>
          </details>
        </>
      ) : (
        <div className="app-shell-release-status__loading">
          {i18nText("appShell", "release_status.loading")}
        </div>
      )}
    </div>
  );
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
