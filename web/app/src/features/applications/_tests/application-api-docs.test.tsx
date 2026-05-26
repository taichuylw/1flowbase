import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, test, vi } from 'vitest';

const explorerState = vi.hoisted(() => ({
  lastProps: null as null | {
    queryState: { categoryId: string | null; operationId: string | null };
    catalogQueryKey: readonly unknown[];
    categoryOperationsQueryKey: (categoryId: string) => readonly unknown[];
    fetchCategoryOperations: (
      categoryId: string,
      request?: { offset?: number; limit?: number; q?: string | null }
    ) => Promise<unknown>;
    operationSpecQueryKey: (operationId: string) => readonly unknown[];
    showAllOperationsWhenNoCategory?: boolean;
    selectFirstCategoryWhenEmpty?: boolean;
    toolbarPortalId?: string;
    onQueryStateChange: (next: {
      categoryId: string | null;
      operationId: string | null;
    }) => void;
  }
}));

const publicApi = vi.hoisted(() => ({
  applicationApiDocsCatalogQueryKey: vi.fn(
    (applicationId: string, locale?: string | null) =>
      [
        'applications',
        applicationId,
        'public-api',
        'docs',
        'catalog',
        locale ?? 'default'
      ] as const
  ),
  applicationApiDocsCategoryOperationsQueryKey: vi.fn(
    (applicationId: string, categoryId: string, locale?: string | null) =>
      [
        'applications',
        applicationId,
        'public-api',
        'docs',
        'category',
        categoryId,
        'operations',
        locale ?? 'default'
      ] as const
  ),
  applicationApiDocsOperationSpecQueryKey: vi.fn(
    (applicationId: string, operationId: string, locale?: string | null) =>
      [
        'applications',
        applicationId,
        'public-api',
        'docs',
        'operation',
        operationId,
        'openapi',
        locale ?? 'default'
      ] as const
  ),
  fetchApplicationApiDocsCatalog: vi.fn(),
  fetchApplicationApiDocsCategoryOperations: vi.fn(),
  fetchApplicationApiDocsOperationSpec: vi.fn(),
  getApplicationApiDocsLocale: vi.fn(() => 'zh_Hans')
}));

vi.mock('../api/public-api', () => publicApi);
vi.mock('../../../shared/ui/api-docs/ApiDocsExplorer', () => ({
  ApiDocsExplorer: (props: typeof explorerState.lastProps) => {
    explorerState.lastProps = props;
    return (
      <button
        type="button"
        onClick={() =>
          props?.onQueryStateChange({
            categoryId: 'openai-compatible-api',
            operationId: 'applicationOpenAiCreateChatCompletion'
          })
        }
      >
        docs explorer
      </button>
    );
  }
}));

import { AppProviders } from '../../../app/AppProviders';
import { ApplicationApiDocsPanel } from '../components/api/ApplicationApiDocsPanel';

describe('ApplicationApiDocsPanel', () => {
  test('uses app-local docs state without navigating to settings docs', () => {
    window.history.pushState({}, '', '/applications/app-1/api');
    Object.defineProperty(window.navigator, 'languages', {
      value: ['zh-CN', 'en-US'],
      configurable: true
    });

    render(
      <AppProviders>
        <ApplicationApiDocsPanel applicationId="app-1" />
      </AppProviders>
    );

    expect(
      screen.queryByText('Support Agent API 文档')
    ).not.toBeInTheDocument();
    expect(screen.queryByText('active publication v3')).not.toBeInTheDocument();
    expect(explorerState.lastProps?.queryState).toEqual({
      categoryId: null,
      operationId: null
    });
    expect(explorerState.lastProps?.showAllOperationsWhenNoCategory).toBeUndefined();
    expect(explorerState.lastProps?.selectFirstCategoryWhenEmpty).toBe(true);
    expect(explorerState.lastProps?.toolbarPortalId).toBeUndefined();
    expect(explorerState.lastProps?.catalogQueryKey).toEqual([
      'applications',
      'app-1',
      'public-api',
      'docs',
      'catalog',
      'zh_Hans'
    ]);
    expect(
      explorerState.lastProps?.categoryOperationsQueryKey(
        'openai-compatible-api'
      )
    ).toEqual([
      'applications',
      'app-1',
      'public-api',
      'docs',
      'category',
      'openai-compatible-api',
      'operations',
      'zh_Hans'
    ]);
    void explorerState.lastProps?.fetchCategoryOperations(
      'openai-compatible-api',
      { offset: 20, limit: 20, q: 'chat' }
    );
    expect(
      publicApi.fetchApplicationApiDocsCategoryOperations
    ).toHaveBeenCalledWith(
      'app-1',
      'openai-compatible-api',
      { offset: 20, limit: 20, q: 'chat' },
      'zh_Hans'
    );
    expect(
      explorerState.lastProps?.operationSpecQueryKey(
        'applicationOpenAiCreateChatCompletion'
      )
    ).toEqual([
      'applications',
      'app-1',
      'public-api',
      'docs',
      'operation',
      'applicationOpenAiCreateChatCompletion',
      'openapi',
      'zh_Hans'
    ]);

    fireEvent.click(screen.getByRole('button', { name: 'docs explorer' }));

    expect(window.location.pathname).toBe('/applications/app-1/api');
    expect(window.location.search).toBe('');
    expect(explorerState.lastProps?.queryState).toEqual({
      categoryId: 'openai-compatible-api',
      operationId: 'applicationOpenAiCreateChatCompletion'
    });
  });
});
