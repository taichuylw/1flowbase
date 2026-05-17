import { render, screen, waitFor, within } from '@testing-library/react';
import { beforeEach, describe, expect, test, vi } from 'vitest';

import { AppProviders } from '../../../../app/AppProviders';
import { resetAuthStore, useAuthStore } from '../../../../state/auth-store';
import type { FrontstagePageContent } from '../../api/page-content';
import { FrontStagePage } from '../../pages/FrontStagePage';

const pageContentSaveHook = vi.hoisted(() => ({
  useFrontstagePageContentSave: vi.fn()
}));
const blockCatalogHook = vi.hoisted(() => ({
  useFrontstageBlockCatalog: vi.fn()
}));
const blockCodeHook = vi.hoisted(() => ({
  useFrontstageBlockCode: vi.fn()
}));
const blockCodeApi = vi.hoisted(() => ({
  fetchFrontstageBlockCode: vi.fn(),
  frontstageBlockCodeQueryKey: vi.fn(
    (workspaceId: string, pageId: string, codeRef: string) =>
      ['frontstage', workspaceId, 'pages', pageId, 'block-code', codeRef] as const
  ),
  saveFrontstageBlockCode: vi.fn()
}));

vi.mock('../../hooks/use-frontstage-page-content-save', () => pageContentSaveHook);
vi.mock('../../hooks/use-frontstage-block-catalog', () => blockCatalogHook);
vi.mock('../../hooks/use-frontstage-block-code', () => blockCodeHook);
vi.mock('../../api/block-code', () => blockCodeApi);

function authenticate() {
  useAuthStore.getState().setAuthenticated({
    csrfToken: 'csrf-123',
    actor: {
      id: 'actor-1',
      account: 'normal-user',
      effective_display_role: 'developer',
      current_workspace_id: 'workspace-1'
    },
    me: {
      id: 'user-1',
      account: 'normal-user',
      email: 'user@example.com',
      phone: null,
      nickname: 'Normal User',
      name: 'Normal User',
      avatar_url: null,
      introduction: '',
      effective_display_role: 'developer',
      permissions: ['route_page.view.all']
    }
  });
}

function createPageContent(): FrontstagePageContent {
  return {
    page: {
      id: 'page-1',
      title: 'Landing',
      kind: 'page',
      parentId: null,
      rank: '001000',
      schemaRootUid: 'root-1'
    },
    schema: {
      rootUid: 'root-1',
      payload: {}
    },
    root: {
      uid: 'root-1',
      payload: {
        blocks: [
          {
            id: 'hero',
            codeRef: 'hero-code',
            contributionCode: 'official.hero',
            runtime: { kind: 'iframe', entry: 'blocks/hero.js' },
            layout: { order: 0, region: 'main' }
          }
        ]
      }
    }
  };
}

describe('FrontStagePage PageCanvas runtime source UI', () => {
  beforeEach(() => {
    resetAuthStore();
    vi.clearAllMocks();
    authenticate();
    pageContentSaveHook.useFrontstagePageContentSave.mockReturnValue({
      save: vi.fn(),
      saving: false,
      isPending: false,
      error: null,
      reset: vi.fn(),
      clearError: vi.fn()
    });
    blockCatalogHook.useFrontstageBlockCatalog.mockReturnValue({
      items: [],
      diagnostics: [],
      loading: false,
      error: null
    });
    blockCodeHook.useFrontstageBlockCode.mockReturnValue({
      code: '',
      draft: '',
      dirty: false,
      loading: false,
      saving: false,
      error: null,
      setDraft: vi.fn(),
      reset: vi.fn(),
      save: vi.fn()
    });
    blockCodeApi.fetchFrontstageBlockCode.mockResolvedValue({
      pageId: 'page-1',
      codeRef: 'hero-code',
      code: 'export default { render() {} }'
    });
  });

  test('queries block code for the active page render plan and shows ready status in canvas slots', async () => {
    render(
      <AppProviders>
        <FrontStagePage
          workspaceId="workspace-1"
          pageId="page-1"
          initialPageTree={[{ id: 'page-1', title: 'Landing', kind: 'page' }]}
          pageContent={createPageContent()}
        />
      </AppProviders>
    );

    await waitFor(() => {
      expect(blockCodeApi.fetchFrontstageBlockCode).toHaveBeenCalledWith(
        'workspace-1',
        'page-1',
        'hero-code'
      );
    });

    const slots = within(screen.getByTestId('page-canvas-render-slots'))
      .getAllByRole('button');
    expect(slots).toHaveLength(1);
    expect(slots[0]).toHaveTextContent('hero');
    await waitFor(() => {
      expect(slots[0]).toHaveTextContent('代码已就绪');
    });
  });
});
