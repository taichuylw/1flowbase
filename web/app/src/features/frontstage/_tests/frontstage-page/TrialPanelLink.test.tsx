import {
  fireEvent,
  render,
  screen,
  waitFor,
  within
} from '@testing-library/react';
import { beforeEach, describe, expect, test, vi } from 'vitest';

import { AppProviders } from '../../../../app/AppProviders';
import { resetAuthStore, useAuthStore } from '../../../../state/auth-store';
import type { FrontstagePageContent } from '../../api/page-content';
import type { UseFrontstagePageCanvasRuntimeSessionsResult } from '../../hooks/use-frontstage-page-canvas-runtime-sessions';
import type { NormalizedFrontstageBlockCatalogEntry } from '../../lib/block-catalog';
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
const runtimeSessionsHook = vi.hoisted(() => ({
  useFrontstagePageCanvasRuntimeSessions: vi.fn()
}));
const blockCodeApi = vi.hoisted(() => ({
  fetchFrontstageBlockCode: vi.fn(
    (_workspaceId: string, pageId: string, codeRef: string) =>
      Promise.resolve({ pageId, codeRef, code: 'export default {}' })
  ),
  frontstageBlockCodeQueryKey: vi.fn(
    (workspaceId: string, pageId: string, codeRef: string) =>
      [
        'frontstage',
        workspaceId,
        'pages',
        pageId,
        'block-code',
        codeRef
      ] as const
  ),
  saveFrontstageBlockCode: vi.fn()
}));

vi.mock('../../hooks/use-frontstage-page-content-save', () =>
  pageContentSaveHook
);
vi.mock('../../hooks/use-frontstage-block-catalog', () => blockCatalogHook);
vi.mock('../../hooks/use-frontstage-block-code', () => blockCodeHook);
vi.mock(
  '../../hooks/use-frontstage-page-canvas-runtime-sessions',
  () => runtimeSessionsHook
);
vi.mock('../../api/block-code', () => blockCodeApi);

function authenticate(permissions: string[]) {
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
      permissions
    }
  });
}

function createPageContent(
  overrides: Partial<FrontstagePageContent> = {}
): FrontstagePageContent {
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
      payload: {}
    },
    ...overrides
  };
}

function createBlockPayload(blockId: string) {
  return {
    id: blockId,
    codeRef: `${blockId}-code`,
    catalog: {
      providerCode: 'official',
      installationId: 'installation-1'
    },
    contribution: {
      pluginId: 'official.blocks',
      pluginVersion: '1.0.0',
      code: 'hero.banner'
    },
    props: { title: 'Hello' },
    layout: {
      order: 0,
      region: 'main'
    },
    runtime: {
      kind: 'iframe',
      entry: 'blocks/hero/index.html',
      hint: 'iframe'
    }
  };
}

function createPageContentWithBlock(blockId: string): FrontstagePageContent {
  const block = createBlockPayload(blockId);

  return createPageContent({
    schema: {
      rootUid: 'root-1',
      payload: { blocks: [block] }
    },
    root: {
      uid: 'root-1',
      payload: { blocks: [block] }
    }
  });
}

function createCatalogEntry(
  overrides: Partial<NormalizedFrontstageBlockCatalogEntry> = {}
): NormalizedFrontstageBlockCatalogEntry {
  return {
    id: 'official:hero.banner',
    runtimeKind: 'iframe',
    installationId: 'installation-1',
    providerCode: 'official',
    pluginId: 'official.blocks',
    pluginVersion: '1.0.0',
    contributionCode: 'hero.banner',
    title: 'Hero Banner',
    entry: 'blocks/hero/index.html',
    permissions: {
      network: 'none',
      storage: 'none',
      secrets: 'none'
    },
    contextContract: {
      primitives: ['text'],
      inputSchema: { type: 'object' }
    },
    uiCapabilities: ['responsive'],
    raw: {} as NormalizedFrontstageBlockCatalogEntry['raw'],
    ...overrides
  };
}

function mockPageContentSaveState() {
  pageContentSaveHook.useFrontstagePageContentSave.mockReturnValue({
    save: vi.fn(),
    saving: false,
    isPending: false,
    error: null,
    reset: vi.fn(),
    clearError: vi.fn()
  });
}

function mockBlockCatalog(items: NormalizedFrontstageBlockCatalogEntry[] = []) {
  blockCatalogHook.useFrontstageBlockCatalog.mockReturnValue({
    items,
    diagnostics: [],
    loading: false,
    error: null
  });
}

function mockBlockCode(code = 'export default { render() {} }') {
  blockCodeHook.useFrontstageBlockCode.mockReturnValue({
    code,
    draft: code,
    dirty: false,
    loading: false,
    saving: false,
    error: null,
    setDraft: vi.fn(),
    reset: vi.fn(),
    save: vi.fn()
  });
}

function mockRuntimeSessions(
  overrides: Partial<UseFrontstagePageCanvasRuntimeSessionsResult> = {}
) {
  runtimeSessionsHook.useFrontstagePageCanvasRuntimeSessions.mockReturnValue({
    entries: [],
    snapshotsBySlot: {},
    running: false,
    hasError: false,
    ...overrides
  });
}

function renderFrontStagePage(pageContent: FrontstagePageContent) {
  return render(
    <AppProviders>
      <FrontStagePage
        workspaceId="workspace-1"
        pageId="page-1"
        initialPageTree={[
          {
            id: 'page-1',
            title: '页面 page-1',
            kind: 'page'
          }
        ]}
        pageContent={pageContent}
      />
    </AppProviders>
  );
}

function getBlockRow(blockId: string) {
  return screen.getByRole('button', {
    name: new RegExp(blockId)
  });
}

describe('FrontStagePage JS block trial panel link', () => {
  beforeEach(() => {
    resetAuthStore();
    vi.clearAllMocks();
    mockPageContentSaveState();
    mockBlockCatalog();
    mockBlockCode();
    mockRuntimeSessions();
  });

  test('opens the trial panel for the selected block with a matched catalog entry and run plan', () => {
    authenticate(['frontstage.page.design']);
    mockBlockCatalog([createCatalogEntry()]);
    mockBlockCode('export default { render() { return null } }');
    renderFrontStagePage(createPageContentWithBlock('hero'));

    fireEvent.click(screen.getByRole('button', { name: '进入设计模式' }));
    expect(
      screen.queryByRole('button', { name: 'JS Block 试运行' })
    ).not.toBeInTheDocument();

    fireEvent.click(getBlockRow('hero'));
    const actions = screen.getByTestId('frontstage-selected-block-actions');
    fireEvent.click(
      within(actions).getByRole('button', { name: 'JS Block 试运行' })
    );

    const panel = screen.getByRole('region', { name: 'JS Block 试运行面板' });
    expect(within(panel).getByText('Run plan 已生成')).toBeInTheDocument();
    expect(
      within(panel).getByText('restricted-block:hero:hero-code')
    ).toBeInTheDocument();
    expect(within(panel).getByText('1000ms')).toBeInTheDocument();
    expect(
      within(panel).getByText(
        'workspaceId, pageId, pageTitle, blockId, blockCodeRef, props'
      )
    ).toBeInTheDocument();
    expect(blockCodeHook.useFrontstageBlockCode).toHaveBeenLastCalledWith({
      workspaceId: 'workspace-1',
      pageId: 'page-1',
      codeRef: 'hero-code'
    });
  });

  test('passes null catalog entry so the existing missing catalog empty state is shown', () => {
    authenticate(['frontstage.page.design']);
    mockBlockCatalog([
      createCatalogEntry({
        pluginVersion: '9.9.9'
      })
    ]);
    renderFrontStagePage(createPageContentWithBlock('hero'));

    fireEvent.click(screen.getByRole('button', { name: '进入设计模式' }));
    fireEvent.click(getBlockRow('hero'));
    fireEvent.click(screen.getByRole('button', { name: 'JS Block 试运行' }));

    const panel = screen.getByRole('region', { name: 'JS Block 试运行面板' });
    expect(within(panel).getByText('缺少区块目录条目')).toBeInTheDocument();
    expect(
      within(panel).queryByText('Run plan 已生成')
    ).not.toBeInTheDocument();
  });

  test('does not expose the trial entry without design permission or before a block is selected', () => {
    authenticate(['route_page.view.all']);
    const view = renderFrontStagePage(createPageContentWithBlock('hero'));

    expect(
      screen.queryByRole('button', { name: '进入设计模式' })
    ).not.toBeInTheDocument();
    expect(
      screen.queryByRole('button', { name: 'JS Block 试运行' })
    ).not.toBeInTheDocument();

    resetAuthStore();
    authenticate(['frontstage.page.design']);
    view.rerender(
      <AppProviders>
        <FrontStagePage
          workspaceId="workspace-1"
          pageId="page-1"
          initialPageTree={[
            {
              id: 'page-1',
              title: '页面 page-1',
              kind: 'page'
            }
          ]}
          pageContent={createPageContentWithBlock('hero')}
        />
      </AppProviders>
    );

    fireEvent.click(screen.getByRole('button', { name: '进入设计模式' }));
    expect(
      screen.queryByRole('button', { name: 'JS Block 试运行' })
    ).not.toBeInTheDocument();
  });

  test('cleans up the trial panel when exiting design mode', async () => {
    authenticate(['frontstage.page.design']);
    mockBlockCatalog([createCatalogEntry()]);
    render(
      <AppProviders>
        <FrontStagePage
          workspaceId="workspace-1"
          pageId="page-1"
          initialPageTree={[
            {
              id: 'page-1',
              title: '页面 page-1',
              kind: 'page'
            }
          ]}
          pageContent={createPageContentWithBlock('hero')}
        />
      </AppProviders>
    );

    fireEvent.click(screen.getByRole('button', { name: '进入设计模式' }));
    fireEvent.click(getBlockRow('hero'));
    fireEvent.click(screen.getByRole('button', { name: 'JS Block 试运行' }));
    expect(
      screen.getByRole('region', { name: 'JS Block 试运行面板' })
    ).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: '退出设计模式' }));
    await waitFor(() => {
      expect(
        screen.queryByRole('region', { name: 'JS Block 试运行面板' })
      ).not.toBeInTheDocument();
    });
  });

  test('cleans up the trial panel when switching pages', async () => {
    authenticate(['frontstage.page.design']);
    mockBlockCatalog([createCatalogEntry()]);
    const pageTree = [
      {
        id: 'page-1',
        title: '页面 page-1',
        kind: 'page' as const
      },
      {
        id: 'page-2',
        title: '页面 page-2',
        kind: 'page' as const
      }
    ];
    const view = render(
      <AppProviders>
        <FrontStagePage
          workspaceId="workspace-1"
          pageId="page-1"
          initialPageTree={pageTree}
          pageContent={createPageContentWithBlock('hero')}
        />
      </AppProviders>
    );

    fireEvent.click(screen.getByRole('button', { name: '进入设计模式' }));
    fireEvent.click(getBlockRow('hero'));
    fireEvent.click(screen.getByRole('button', { name: 'JS Block 试运行' }));
    expect(
      screen.getByRole('region', { name: 'JS Block 试运行面板' })
    ).toBeInTheDocument();

    fireEvent.click(
      screen.getByRole('button', { name: /页面 page-2\s+页面节点/ })
    );
    view.rerender(
      <AppProviders>
        <FrontStagePage
          workspaceId="workspace-1"
          pageId="page-2"
          initialPageTree={pageTree}
          pageContent={createPageContent({
            page: {
              id: 'page-2',
              title: 'Second',
              kind: 'page',
              parentId: null,
              rank: '002000',
              schemaRootUid: 'root-2'
            },
            schema: {
              rootUid: 'root-2',
              payload: {}
            },
            root: {
              uid: 'root-2',
              payload: {}
            }
          })}
        />
      </AppProviders>
    );

    await waitFor(() => {
      expect(
        screen.queryByRole('region', { name: 'JS Block 试运行面板' })
      ).not.toBeInTheDocument();
    });
  });
});
