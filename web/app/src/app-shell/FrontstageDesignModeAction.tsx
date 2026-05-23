import { Menu, Tooltip } from 'antd';
import { useEffect } from 'react';

import { useAuthStore } from '../state/auth-store';
import { useFrontstageDesignModeStore } from '../state/frontstage-design-mode-store';

const DESIGN_MODE_PERMISSION = 'frontstage.page.design';

function isFrontstageRoute(pathname: string) {
  return pathname === '/frontstage' || pathname.startsWith('/frontstage/');
}

interface FrontstageDesignModeActionBaseProps {
  pathname: string;
}

function FrontstageDesignModeActionBase({
  pathname
}: FrontstageDesignModeActionBaseProps) {
  const sessionStatus = useAuthStore((state) => state.sessionStatus);
  const actor = useAuthStore((state) => state.actor);
  const me = useAuthStore((state) => state.me);
  const isDesignMode = useFrontstageDesignModeStore(
    (state) => state.isDesignMode
  );
  const setDesignMode = useFrontstageDesignModeStore(
    (state) => state.setDesignMode
  );
  const toggleDesignMode = useFrontstageDesignModeStore(
    (state) => state.toggleDesignMode
  );

  const isAllowedRoute = isFrontstageRoute(pathname);
  const canUseDesignMode =
    actor?.effective_display_role === 'root' ||
    Boolean(me?.permissions.includes(DESIGN_MODE_PERMISSION));
  const hasResolvedDesignModePermission = sessionStatus !== 'unknown';

  // Exit design mode only when permission is lost; route changes should not
  // overwrite the browser-persisted preference.
  useEffect(() => {
    if (hasResolvedDesignModePermission && !canUseDesignMode && isDesignMode) {
      setDesignMode(false);
    }
  }, [
    canUseDesignMode,
    hasResolvedDesignModePermission,
    isDesignMode,
    setDesignMode
  ]);

  // Support reading design mode from URL query parameters (for non-SPA transition/initial page load)
  useEffect(() => {
    if (isAllowedRoute && canUseDesignMode) {
      const params = new URLSearchParams(window.location.search);
      if (params.get('design') === 'true') {
        setDesignMode(true);
        // Clean up the URL search params so it doesn't stay there on subsequent actions
        const newUrl = window.location.pathname;
        window.history.replaceState({}, '', newUrl);
      }
    }
  }, [isAllowedRoute, canUseDesignMode, setDesignMode]);

  if (!canUseDesignMode) {
    return null;
  }

  const label = isDesignMode ? '退出设计模式' : '进入设计模式';

  const handleClick = () => {
    toggleDesignMode();
  };

  const selectedKeys = isDesignMode ? ['design-mode'] : [];

  return (
    <Tooltip title={label}>
      <Menu
        className="app-shell-design-menu"
        mode="horizontal"
        selectable={false}
        selectedKeys={selectedKeys}
        items={[
          {
            key: 'design-mode',
            className: isDesignMode
              ? 'app-shell-design-mode-button ant-menu-item-selected'
              : 'app-shell-design-mode-button',
            label: (
              <span
                className="app-shell-design-block"
                aria-label={label}
                role="button"
                aria-pressed={isDesignMode}
              >
                UI
              </span>
            )
          }
        ]}
        onClick={handleClick}
        disabledOverflow
      />
    </Tooltip>
  );
}

export function FrontstageDesignModeAction({
  pathname
}: {
  pathname: string;
  useRouterNavigation?: boolean;
}) {
  return <FrontstageDesignModeActionBase pathname={pathname} />;
}
