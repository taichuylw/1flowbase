import { Link } from '@tanstack/react-router';
import { Menu } from 'antd';
import type { MenuProps } from 'antd';

import { getPrimaryNavigationRoutes } from '../routes/route-helpers';
import { getSelectedRouteId } from '../routes/route-config';
import { useAuthStore } from '../state/auth-store';

function renderNavigationLink(
  pathname: string,
  label: string,
  useRouterLinks: boolean,
  isCurrent: boolean
) {
  if (useRouterLinks) {
    return (
      <Link
        to={pathname}
        className="app-shell-menu-link"
        aria-current={isCurrent ? 'page' : undefined}
      >
        {label}
      </Link>
    );
  }

  return (
    <a
      href={pathname}
      className="app-shell-menu-link"
      aria-current={isCurrent ? 'page' : undefined}
    >
      {label}
    </a>
  );
}

export function Navigation({
  pathname,
  useRouterLinks
}: {
  pathname: string;
  useRouterLinks: boolean;
}) {
  const workspaceId = useAuthStore((state) => state.actor?.current_workspace_id);
  const selectedKey = getSelectedRouteId(pathname);
  const items: MenuProps['items'] = getPrimaryNavigationRoutes().map((route) => {
    const path =
      route.id === 'frontstage' && workspaceId
        ? `/frontstage/${workspaceId}`
        : route.path;

    return {
      key: route.id,
      label: renderNavigationLink(path, route.navLabel!, useRouterLinks, route.id === selectedKey)
    };
  });

  return (
    <nav className="app-shell-navigation" aria-label="Primary">
      <Menu
        className="app-shell-menu"
        mode="horizontal"
        selectedKeys={[selectedKey]}
        items={items}
        disabledOverflow
      />
    </nav>
  );
}
