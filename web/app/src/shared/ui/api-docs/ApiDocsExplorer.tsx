import { useDeferredValue, useEffect, useMemo, useState } from 'react';
import type { UIEvent } from 'react';
import { createPortal } from 'react-dom';

import { ApiReferenceReact } from '@scalar/api-reference-react';
import '@scalar/api-reference-react/style.css';
import { useInfiniteQuery, useQueries, useQuery } from '@tanstack/react-query';
import { Result, Select, Spin, Typography } from 'antd';

import './api-docs-explorer.css';
import { ApiDocsOperationListPane } from './ApiDocsOperationListPane';
import { i18nText } from '../../i18n/text';

type ScalarReferenceConfiguration = Exclude<
  Parameters<typeof ApiReferenceReact>[0]['configuration'],
  unknown[]
>;
type ScalarAuthenticationConfiguration =
  ScalarReferenceConfiguration['authentication'];
type ScalarDocumentContent = ScalarReferenceConfiguration['content'];

const emptyCategories: ApiDocsCatalogCategory[] = [];
const apiDocsOperationsPageSize = 20;

export interface ApiDocsCatalogOperation {
  id: string;
  method: string;
  path: string;
  summary: string | null;
  description: string | null;
  tags: string[];
  group: string;
  deprecated: boolean;
}

export interface ApiDocsCatalogCategory {
  id: string;
  label: string;
  operation_count: number;
}

export interface ApiDocsCatalog {
  title: string;
  version: string;
  categories: ApiDocsCatalogCategory[];
}

export interface ApiDocsCategoryOperations {
  id: string;
  label: string;
  operations: ApiDocsCatalogOperation[];
  total?: number;
  offset?: number;
  limit?: number;
  has_more?: boolean;
  next_offset?: number | null;
}

export interface ApiDocsCategoryOperationsRequest {
  offset?: number;
  limit?: number;
  q?: string | null;
}

export interface ApiDocsExplorerQueryState {
  categoryId: string | null;
  operationId: string | null;
}

type QueryKey = readonly unknown[];

type CategorySelectOption = {
  value: string;
  label: string;
  categoryId: string;
  operationCount: number;
  searchText: string;
};

export type ApiDocsOperationWithCategory = ApiDocsCatalogOperation & {
  categoryId: string | null;
  categoryLabel: string | null;
};

export interface ApiDocsExplorerProps<TAuthenticationSnapshot = unknown> {
  queryState: ApiDocsExplorerQueryState;
  onQueryStateChange: (
    nextState: ApiDocsExplorerQueryState,
    mode?: 'push' | 'replace'
  ) => void;
  catalogQueryKey: QueryKey;
  fetchCatalog: () => Promise<ApiDocsCatalog>;
  categoryOperationsQueryKey: (categoryId: string) => QueryKey;
  fetchCategoryOperations: (
    categoryId: string,
    request?: ApiDocsCategoryOperationsRequest
  ) => Promise<ApiDocsCategoryOperations>;
  operationSpecQueryKey: (operationId: string) => QueryKey;
  fetchOperationSpec: (
    operationId: string
  ) => Promise<ScalarDocumentContent>;
  baseServerUrl: string | (() => string);
  showAllOperationsWhenNoCategory?: boolean;
  selectFirstCategoryWhenEmpty?: boolean;
  toolbarPortalId?: string;
  authentication?: {
    queryKey: QueryKey;
    queryFn: () => Promise<TAuthenticationSnapshot>;
    buildConfig: (
      operationSpec: ScalarDocumentContent | undefined,
      snapshot: TAuthenticationSnapshot | undefined
    ) => ScalarAuthenticationConfiguration;
  };
}

function normalizeSearchText(input: string): string {
  return input.toLowerCase().replace(/[\s\-/:_]+/g, '');
}

function buildCategorySearchText(category: ApiDocsCatalogCategory): string {
  return normalizeSearchText(
    `${category.id} ${category.label} ${category.operation_count}`
  );
}

function buildOperationSearchText(operation: ApiDocsCatalogOperation): string {
  return normalizeSearchText(
    `${operation.method} ${operation.path} ${operation.summary ?? ''} ${operation.description ?? ''} ${operation.group} ${operation.id}`
  );
}

export function ApiDocsExplorer<TAuthenticationSnapshot = unknown>({
  queryState,
  onQueryStateChange,
  catalogQueryKey,
  fetchCatalog,
  categoryOperationsQueryKey,
  fetchCategoryOperations,
  operationSpecQueryKey,
  fetchOperationSpec,
  baseServerUrl,
  showAllOperationsWhenNoCategory = false,
  selectFirstCategoryWhenEmpty = false,
  toolbarPortalId,
  authentication
}: ApiDocsExplorerProps<TAuthenticationSnapshot>) {
  const [operationSearch, setOperationSearch] = useState('');
  const [toolbarPortalElement, setToolbarPortalElement] =
    useState<HTMLElement | null>(null);
  const deferredOperationSearch = useDeferredValue(operationSearch);
  const normalizedOperationSearch = useMemo(
    () => normalizeSearchText(deferredOperationSearch),
    [deferredOperationSearch]
  );
  const catalogQuery = useQuery({
    queryKey: catalogQueryKey,
    queryFn: fetchCatalog
  });
  const categories = catalogQuery.data?.categories ?? emptyCategories;
  const selectedCategoryId =
    categories.find((category) => category.id === queryState.categoryId)?.id ??
    null;
  const selectedCategory =
    categories.find((category) => category.id === selectedCategoryId) ?? null;
  const totalOperations = categories.reduce(
    (total, category) => total + category.operation_count,
    0
  );
  const categoryOptions: CategorySelectOption[] = categories.map(
    (category) => ({
      value: category.id,
      label: category.label,
      categoryId: category.id,
      operationCount: category.operation_count,
      searchText: buildCategorySearchText(category)
    })
  );

  useEffect(() => {
    if (catalogQuery.isLoading || !queryState.categoryId || selectedCategoryId) {
      return;
    }

    onQueryStateChange({ categoryId: null, operationId: null }, 'replace');
  }, [
    catalogQuery.isLoading,
    onQueryStateChange,
    queryState.categoryId,
    selectedCategoryId
  ]);

  useEffect(() => {
    if (
      catalogQuery.isLoading ||
      !selectFirstCategoryWhenEmpty ||
      showAllOperationsWhenNoCategory ||
      queryState.categoryId ||
      categories.length === 0
    ) {
      return;
    }

    onQueryStateChange(
      { categoryId: categories[0].id, operationId: null },
      'replace'
    );
  }, [
    catalogQuery.isLoading,
    categories,
    onQueryStateChange,
    queryState.categoryId,
    selectFirstCategoryWhenEmpty,
    showAllOperationsWhenNoCategory
  ]);

  const categoryOperationsQuery = useInfiniteQuery({
    queryKey: [
      ...categoryOperationsQueryKey(selectedCategoryId ?? ''),
      'search',
      normalizedOperationSearch,
      'page-size',
      apiDocsOperationsPageSize
    ],
    initialPageParam: 0,
    queryFn: ({ pageParam }) =>
      fetchCategoryOperations(selectedCategoryId!, {
        offset: Number(pageParam),
        limit: apiDocsOperationsPageSize,
        q: deferredOperationSearch.trim() || null
      }),
    getNextPageParam: (lastPage) => {
      if (!lastPage.has_more) {
        return undefined;
      }

      return (
        lastPage.next_offset ??
        (lastPage.offset ?? 0) + lastPage.operations.length
      );
    },
    enabled: Boolean(selectedCategoryId)
  });

  const allCategoryOperationsQueries = useQueries({
    queries: categories.map((category) => ({
      queryKey: [
        ...categoryOperationsQueryKey(category.id),
        'page-size',
        apiDocsOperationsPageSize
      ],
      queryFn: () =>
        fetchCategoryOperations(category.id, {
          offset: 0,
          limit: apiDocsOperationsPageSize
        }),
      enabled: showAllOperationsWhenNoCategory && !selectedCategoryId
    }))
  });
  const allCategoryOperationsLoading =
    showAllOperationsWhenNoCategory &&
    !selectedCategoryId &&
    allCategoryOperationsQueries.some((query) => query.isLoading);
  const allCategoryOperationsError =
    showAllOperationsWhenNoCategory &&
    !selectedCategoryId &&
    allCategoryOperationsQueries.some((query) => query.isError);
  const allCategoryOperations = useMemo(
    () =>
      allCategoryOperationsQueries.flatMap((query, index) => {
        const category = categories[index];

        return (query.data?.operations ?? []).map((operation) => ({
          ...operation,
          categoryId: category?.id ?? null,
          categoryLabel: category?.label ?? null
        }));
      }),
    [allCategoryOperationsQueries, categories]
  );
  const selectedCategoryOperations = useMemo(
    () =>
      (categoryOperationsQuery.data?.pages.flatMap((page) => page.operations) ?? []).map(
        (operation) => ({
          ...operation,
          categoryId: selectedCategoryId,
          categoryLabel: selectedCategory?.label ?? null
        })
      ),
    [categoryOperationsQuery.data?.pages, selectedCategory?.label, selectedCategoryId]
  );

  useEffect(() => {
    if (!toolbarPortalId) {
      setToolbarPortalElement(null);
      return;
    }

    setToolbarPortalElement(document.getElementById(toolbarPortalId));
  }, [toolbarPortalId]);
  const operations: ApiDocsOperationWithCategory[] =
    showAllOperationsWhenNoCategory && !selectedCategoryId
      ? allCategoryOperations
      : selectedCategoryOperations;
  const selectedOperationId =
    operations.find((operation) => operation.id === queryState.operationId)
      ?.id ?? null;
  const selectedOperation =
    operations.find((operation) => operation.id === selectedOperationId) ??
    null;
  const selectedCategoryOperationTotal =
    categoryOperationsQuery.data?.pages[0]?.total ??
    selectedCategory?.operation_count ??
    selectedCategoryOperations.length;

  const filteredOperations = useMemo(() => {
    const normalizedQuery = normalizeSearchText(operationSearch);

    if (!normalizedQuery) {
      return operations;
    }

    return operations.filter((operation) =>
      buildOperationSearchText(operation).includes(normalizedQuery)
    );
  }, [operationSearch, operations]);

  useEffect(() => {
    if (
      !selectedCategoryId ||
      categoryOperationsQuery.isLoading ||
      categoryOperationsQuery.isFetchingNextPage ||
      !queryState.operationId ||
      selectedOperationId
    ) {
      return;
    }

    if (categoryOperationsQuery.hasNextPage) {
      void categoryOperationsQuery.fetchNextPage();
      return;
    }

    onQueryStateChange(
      { categoryId: selectedCategoryId, operationId: null },
      'replace'
    );
  }, [
    categoryOperationsQuery.isLoading,
    categoryOperationsQuery.isFetchingNextPage,
    categoryOperationsQuery.hasNextPage,
    categoryOperationsQuery.fetchNextPage,
    onQueryStateChange,
    queryState.operationId,
    selectedCategoryId,
    selectedOperationId
  ]);

  useEffect(() => {
    if (
      !showAllOperationsWhenNoCategory ||
      selectedCategoryId ||
      allCategoryOperationsLoading ||
      !queryState.operationId ||
      selectedOperationId
    ) {
      return;
    }

    onQueryStateChange({ categoryId: null, operationId: null }, 'replace');
  }, [
    allCategoryOperationsLoading,
    onQueryStateChange,
    queryState.operationId,
    selectedCategoryId,
    selectedOperationId,
    showAllOperationsWhenNoCategory
  ]);

  const operationSpecQuery = useQuery({
    queryKey: operationSpecQueryKey(selectedOperationId ?? ''),
    queryFn: () => fetchOperationSpec(selectedOperationId!),
    enabled: Boolean(selectedOperationId)
  });
  const authenticationQuery = useQuery({
    queryKey: authentication?.queryKey ?? ['api-docs', 'authentication-disabled'],
    queryFn: () => authentication!.queryFn(),
    enabled: Boolean(authentication && selectedOperationId)
  });

  function handleOperationPaneScroll(event: UIEvent<HTMLDivElement>) {
    if (
      !selectedCategoryId ||
      !categoryOperationsQuery.hasNextPage ||
      categoryOperationsQuery.isFetchingNextPage
    ) {
      return;
    }

    const target = event.currentTarget;
    const distanceToBottom =
      target.scrollHeight - target.scrollTop - target.clientHeight;

    if (distanceToBottom <= 160) {
      void categoryOperationsQuery.fetchNextPage();
    }
  }

  function renderCategorySelector() {
    return (
      <section className="api-docs-panel__toolbar" aria-label={i18nText("sharedUi", "auto.k_5bdcfd64f7")}>
        <div className="api-docs-panel__header-control">
          <Select
            aria-label={i18nText("sharedUi", "auto.k_6328c9d25d")}
            className="api-docs-panel__category-select"
            showSearch
            allowClear
            disabled={categories.length === 0}
            value={selectedCategoryId ?? undefined}
            options={categoryOptions}
            placeholder={
              categories.length === 0
                ? i18nText("sharedUi", "auto.k_b457de4013")
                : showAllOperationsWhenNoCategory
                  ? i18nText("sharedUi", "auto.k_54eb6f4c5d")
                  : i18nText("sharedUi", "auto.k_2fe7420637")
            }
            optionRender={(option) => {
              const category = option.data as CategorySelectOption;

              return (
                <div className="api-docs-panel__category-option">
                  <div className="api-docs-panel__category-option-copy">
                    <span className="api-docs-panel__category-option-label">
                      {category.label}
                    </span>
                    <span
                      className="api-docs-panel__category-option-id"
                      aria-hidden="true"
                    >
                      {category.categoryId}
                    </span>
                  </div>
                  <span
                    className="api-docs-panel__category-option-count"
                    aria-hidden="true"
                  >
                    {category.operationCount} {i18nText("sharedUi", "auto.k_b4eda10b96")}</span>
                </div>
              );
            }}
            filterOption={(input, option) =>
              String(
                (option as CategorySelectOption | undefined)?.searchText ?? ''
              ).includes(normalizeSearchText(input))
            }
            onChange={(nextCategoryId) =>
              onQueryStateChange({
                categoryId: nextCategoryId ?? null,
                operationId: null
              })
            }
            notFoundContent={i18nText("sharedUi", "auto.k_5d81a88762")}
          />
        </div>
        <Typography.Text className="api-docs-panel__count">
          {i18nText("sharedUi", "auto.k_3b6ef811b8")}{totalOperations} {i18nText("sharedUi", "auto.k_b4eda10b96")}</Typography.Text>
      </section>
    );
  }

  function renderOperationPane() {
    return (
      <ApiDocsOperationListPane
        categoriesLength={categories.length}
        selectedCategoryId={selectedCategoryId}
        selectedCategoryLabel={selectedCategory?.label ?? null}
        showAllOperationsWhenNoCategory={showAllOperationsWhenNoCategory}
        loading={
          selectedCategoryId
            ? categoryOperationsQuery.isLoading
            : allCategoryOperationsLoading
        }
        error={
          selectedCategoryId
            ? categoryOperationsQuery.isError
            : allCategoryOperationsError
        }
        operations={operations}
        filteredOperations={filteredOperations}
        selectedOperationId={selectedOperationId}
        operationSearch={operationSearch}
        selectedCategoryOperationTotal={selectedCategoryOperationTotal}
        fetchingNextPage={categoryOperationsQuery.isFetchingNextPage}
        onOperationSearchChange={setOperationSearch}
        onOperationScroll={handleOperationPaneScroll}
        onQueryStateChange={onQueryStateChange}
      />
    );
  }

  function renderDetailPane() {
    if (
      (!selectedCategoryId && !showAllOperationsWhenNoCategory) ||
      !selectedOperationId
    ) {
      return (
        <div className="api-docs-panel__detail-state">
          <Result
            status="info"
            title={i18nText("sharedUi", "auto.k_9802607ef1")}
            subTitle={
              showAllOperationsWhenNoCategory
                ? i18nText("sharedUi", "auto.k_8d2ecb0ce3")
                : i18nText("sharedUi", "auto.k_42c68783d0")
            }
          />
        </div>
      );
    }

    if (operationSpecQuery.isLoading) {
      return (
        <div className="api-docs-panel__detail-state">
          <Spin size="large" />
          <Typography.Text type="secondary">
            {i18nText("sharedUi", "auto.k_3667cb105a")}{selectedOperation?.path ?? i18nText("sharedUi", "auto.k_e556eefd90")} {i18nText("sharedUi", "auto.k_6056170c73")}</Typography.Text>
        </div>
      );
    }

    if (operationSpecQuery.isError) {
      return (
        <div className="api-docs-panel__detail-state">
          <Result
            status="error"
            title={i18nText("sharedUi", "auto.k_ae9226014a")}
            subTitle={i18nText("sharedUi", "auto.k_ff8140d881")}
          />
        </div>
      );
    }

    return (
      <div className="api-docs-panel__detail-viewer">
        <ApiReferenceReact
          configuration={{
            authentication: authentication?.buildConfig(
              operationSpecQuery.data,
              authenticationQuery.data
            ),
            baseServerURL:
              typeof baseServerUrl === 'function'
                ? baseServerUrl()
                : baseServerUrl,
            content: operationSpecQuery.data,
            showSidebar: false
          }}
        />
      </div>
    );
  }

  let content = null;

  if (catalogQuery.isLoading) {
    content = (
      <div className="api-docs-panel__detail-state">
        <Spin size="large" />
        <Typography.Text type="secondary">{i18nText("sharedUi", "auto.k_ad11ccaa05")}</Typography.Text>
      </div>
    );
  } else if (catalogQuery.isError) {
    content = (
      <Result
        status="error"
        title={i18nText("sharedUi", "auto.k_c3b0850520")}
        subTitle={i18nText("sharedUi", "auto.k_7062198413")}
      />
    );
  } else {
    content = (
      <>
        {toolbarPortalElement ? null : renderCategorySelector()}
        <div className="api-docs-panel__workspace">
          {renderOperationPane()}
          <section className="api-docs-panel__detail" aria-label={i18nText("sharedUi", "auto.k_552c849b13")}>
            {renderDetailPane()}
          </section>
        </div>
      </>
    );
  }

  return (
    <div
      className={
        toolbarPortalElement
          ? 'api-docs-panel api-docs-panel--external-toolbar'
          : 'api-docs-panel'
      }
    >
      {toolbarPortalElement && !catalogQuery.isLoading && !catalogQuery.isError
        ? createPortal(renderCategorySelector(), toolbarPortalElement)
        : null}
      {content}
    </div>
  );
}
