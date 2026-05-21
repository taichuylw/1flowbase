import { EditOutlined } from '@ant-design/icons';
import { Button, Tooltip } from 'antd';
import { useEffect } from 'react';

import { useAuthStore } from '../state/auth-store';
import { useFrontstageDesignModeStore } from '../state/frontstage-design-mode-store';

const DESIGN_MODE_PERMISSION = 'frontstage.page.design';

function isFrontstageRoute(pathname: string) {
  return pathname === '/frontstage' || pathname.startsWith('/frontstage/');
}

export function FrontstageDesignModeAction({
  pathname
}: {
  pathname: string;
}) {
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

  useEffect(() => {
    if ((!isAllowedRoute || !canUseDesignMode) && isDesignMode) {
      setDesignMode(false);
    }
  }, [canUseDesignMode, isAllowedRoute, isDesignMode, setDesignMode]);

  if (!isAllowedRoute || !canUseDesignMode) {
    return null;
  }

  const label = isDesignMode ? '退出设计模式' : '进入设计模式';

  return (
    <Tooltip title={label}>
      <Button
        aria-label={label}
        aria-pressed={isDesignMode}
        className={[
          'app-shell-design-mode-button',
          isDesignMode ? 'app-shell-design-mode-button--active' : null
        ]
          .filter(Boolean)
          .join(' ')}
        icon={<EditOutlined />}
        onClick={toggleDesignMode}
        type="text"
      />
    </Tooltip>
  );
}
