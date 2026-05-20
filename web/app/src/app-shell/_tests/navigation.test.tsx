import { render, screen, within } from '@testing-library/react';
import { describe, expect, test } from 'vitest';

import { Navigation } from '../Navigation';
import { resetAuthStore, useAuthStore } from '../../state/auth-store';

describe('Navigation', () => {
  test('links 前台 to base frontstage path when workspace is available', async () => {
    resetAuthStore();
    useAuthStore.getState().setAuthenticated({
      csrfToken: 'csrf-123',
      actor: {
        id: 'actor-1',
        account: 'normal-user',
        effective_display_role: 'developer',
        current_workspace_id: 'workspace-123'
      },
      me: {
        id: 'user-1',
        account: 'normal-user',
        email: 'normal-user@example.com',
        phone: null,
        nickname: 'Normal User',
        name: 'Normal User',
        avatar_url: null,
        introduction: '',
        effective_display_role: 'developer',
        permissions: ['route_page.view.all']
      }
    });

    render(<Navigation pathname="/embedded-apps" useRouterLinks={false} />);

    expect(await screen.findByRole('link', { name: '前台' })).toHaveAttribute('href', '/frontstage');
  });

  test('links 前台 to base frontstage path when workspace is not available', async () => {
    resetAuthStore();

    render(<Navigation pathname="/embedded-apps" useRouterLinks={false} />);

    expect(await screen.findByRole('link', { name: '前台' })).toHaveAttribute('href', '/frontstage');
  });

  test('renders primary console navigation and keeps settings out of the primary rail', async () => {
    resetAuthStore();

    render(<Navigation pathname="/embedded-apps" useRouterLinks={false} />);

    const nav = await screen.findByRole('navigation', { name: 'Primary' });

    expect(within(nav).getByRole('link', { name: '工作台' })).toBeInTheDocument();
    expect(within(nav).getByRole('link', { name: '前台' })).toBeInTheDocument();
    expect(within(nav).getByRole('link', { name: '子系统' })).toBeInTheDocument();
    expect(within(nav).getByRole('link', { name: '工具' })).toBeInTheDocument();
    expect(within(nav).queryByRole('link', { name: '设置' })).not.toBeInTheDocument();
    expect(await screen.findByRole('link', { name: '子系统', current: 'page' })).toBeInTheDocument();
  });
});
