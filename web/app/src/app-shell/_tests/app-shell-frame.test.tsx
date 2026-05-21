import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import fs from 'node:fs';
import path from 'node:path';
import { beforeEach, describe, expect, test, vi } from 'vitest';

import { resetAuthStore, useAuthStore } from '../../state/auth-store';
import {
  resetFrontstageDesignModeStore,
  useFrontstageDesignModeStore
} from '../../state/frontstage-design-mode-store';
import { AppShellFrame } from '../AppShellFrame';

describe('AppShellFrame', () => {
  beforeEach(() => {
    resetAuthStore();
    resetFrontstageDesignModeStore();
    useAuthStore.getState().setAuthenticated({
      csrfToken: 'csrf-token',
      actor: {
        id: 'user-1',
        account: 'root',
        effective_display_role: 'root',
        current_workspace_id: 'workspace-1'
      },
      me: {
        id: 'user-1',
        account: 'root',
        name: 'Root',
        nickname: 'Root',
        email: 'root@example.com',
        phone: null,
        avatar_url: null,
        introduction: '',
        effective_display_role: 'root',
        permissions: []
      }
    });
  });

  test('places the account menu before the settings menu in the top actions', async () => {
    render(
      <AppShellFrame pathname="/settings/data-models">
        <main>Content</main>
      </AppShellFrame>
    );

    await waitFor(() => {
      const accountLabel = screen.getByText('Root');
      const settingsTrigger = screen.getByLabelText('设置');

      expect(
        accountLabel.compareDocumentPosition(settingsTrigger) &
          Node.DOCUMENT_POSITION_FOLLOWING
      ).toBeTruthy();
    });
  });

  test('places frontstage design mode icon after settings and toggles shared state', async () => {
    render(
      <AppShellFrame pathname="/frontstage">
        <main>Content</main>
      </AppShellFrame>
    );

    await waitFor(() => {
      const settingsTrigger = screen.getByLabelText('设置');
      const designButton = screen.getByLabelText('进入设计模式');

      expect(
        settingsTrigger.compareDocumentPosition(designButton) &
          Node.DOCUMENT_POSITION_FOLLOWING
      ).toBeTruthy();
      expect(designButton).toHaveAttribute('aria-pressed', 'false');
    });

    fireEvent.click(screen.getByLabelText('进入设计模式'));

    expect(useFrontstageDesignModeStore.getState().isDesignMode).toBe(true);
    expect(screen.getByLabelText('退出设计模式')).toHaveAttribute(
      'aria-pressed',
      'true'
    );
  });

  test('renders frontstage design mode button globally on non-frontstage pages and navigates', async () => {
    const locationSpy = vi.fn();
    const originalLocation = window.location;
    
    // Mock window.location
    delete (window as any).location;
    window.location = {
      ...originalLocation,
      assign: vi.fn(),
      replace: vi.fn(),
      get href() {
        return 'http://localhost/';
      },
      set href(val: string) {
        locationSpy(val);
      },
      search: ''
    } as any;

    render(
      <AppShellFrame pathname="/">
        <main>Content</main>
      </AppShellFrame>
    );

    await waitFor(() => {
      const designButton = screen.getByLabelText('进入设计模式');
      expect(designButton).toBeInTheDocument();
    });

    fireEvent.click(screen.getByLabelText('进入设计模式'));

    expect(locationSpy).toHaveBeenCalledWith('/frontstage?design=true');

    // restore
    window.location = originalLocation;
  });

  test('keeps the top header to a single horizontally scrollable row', () => {
    const appShellCss = fs.readFileSync(
      path.resolve(import.meta.dirname, '../app-shell.css'),
      'utf8'
    );
    const headerRule = appShellCss.match(
      /\.app-shell-header\.ant-layout-header \{([\s\S]*?)\n\}/
    )?.[1];
    const actionRowRule = appShellCss.match(
      /\.app-shell-action-row\.ant-space \{([\s\S]*?)\n\}/
    )?.[1];
    const mobileActionsRule = appShellCss.match(
      /@media \(max-width: 767px\) \{[\s\S]*?\.app-shell-actions \{([\s\S]*?)\n {2}\}/
    )?.[1];

    expect(headerRule).toContain('flex-wrap: nowrap;');
    expect(headerRule).toContain('overflow-x: auto;');
    expect(headerRule).toContain('overflow-y: hidden;');
    expect(headerRule).toContain('white-space: nowrap;');
    expect(actionRowRule).toContain('flex-wrap: nowrap;');
    expect(appShellCss).not.toContain('flex-direction: column;');
    expect(appShellCss).not.toContain('height: auto;');
    expect(mobileActionsRule).toContain('align-self: center;');
    expect(mobileActionsRule).toContain('width: auto;');
    expect(mobileActionsRule).toContain('max-width: none;');
  });
});
