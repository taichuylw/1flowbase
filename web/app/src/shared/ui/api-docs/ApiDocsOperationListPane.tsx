import type { UIEvent } from 'react';

import { Empty, Input, Result, Spin, Typography } from 'antd';

import type {
  ApiDocsExplorerQueryState,
  ApiDocsOperationWithCategory
} from './ApiDocsExplorer';

export interface ApiDocsOperationListPaneProps {
  categoriesLength: number;
  selectedCategoryId: string | null;
  selectedCategoryLabel: string | null;
  showAllOperationsWhenNoCategory: boolean;
  loading: boolean;
  error: boolean;
  operations: ApiDocsOperationWithCategory[];
  filteredOperations: ApiDocsOperationWithCategory[];
  selectedOperationId: string | null;
  operationSearch: string;
  selectedCategoryOperationTotal: number;
  fetchingNextPage: boolean;
  onOperationSearchChange: (value: string) => void;
  onOperationScroll: (event: UIEvent<HTMLDivElement>) => void;
  onQueryStateChange: (
    nextState: ApiDocsExplorerQueryState,
    mode?: 'push' | 'replace'
  ) => void;
}

export function ApiDocsOperationListPane({
  categoriesLength,
  selectedCategoryId,
  selectedCategoryLabel,
  showAllOperationsWhenNoCategory,
  loading,
  error,
  operations,
  filteredOperations,
  selectedOperationId,
  operationSearch,
  selectedCategoryOperationTotal,
  fetchingNextPage,
  onOperationSearchChange,
  onOperationScroll,
  onQueryStateChange
}: ApiDocsOperationListPaneProps) {
  if (categoriesLength === 0) {
    return (
      <section className="api-docs-panel__pane" aria-label="接口列表">
        <div className="api-docs-panel__pane-header">
          <div className="api-docs-panel__pane-copy">
            <Typography.Text strong>接口列表</Typography.Text>
            <Typography.Text type="secondary">
              当前暂无可访问分类
            </Typography.Text>
          </div>
        </div>
        <div className="api-docs-panel__pane-body">
          <Empty
            description="暂无接口分类"
            image={Empty.PRESENTED_IMAGE_SIMPLE}
          />
        </div>
      </section>
    );
  }

  if (!selectedCategoryId && !showAllOperationsWhenNoCategory) {
    return (
      <section className="api-docs-panel__pane" aria-label="接口列表">
        <div className="api-docs-panel__pane-header">
          <div className="api-docs-panel__pane-copy">
            <Typography.Text strong>接口列表</Typography.Text>
            <Typography.Text type="secondary">
              在上方先选分类后展示接口
            </Typography.Text>
          </div>
        </div>
        <div className="api-docs-panel__pane-body">
          <Result
            status="info"
            title="选择一个分类后查看接口列表"
            subTitle="分类选择放在头部，下方列表只负责当前分类下的接口浏览。"
          />
        </div>
      </section>
    );
  }

  if (loading) {
    return (
      <section className="api-docs-panel__pane" aria-label="接口列表">
        <div className="api-docs-panel__pane-header">
          <div className="api-docs-panel__pane-copy">
            <Typography.Text strong>接口列表</Typography.Text>
            <Typography.Text type="secondary">
              正在加载 {selectedCategoryLabel ?? '全部分类'} 的接口
            </Typography.Text>
          </div>
        </div>
        <div className="api-docs-panel__pane-state">
          <Spin size="large" />
        </div>
      </section>
    );
  }

  if (error) {
    return (
      <section className="api-docs-panel__pane" aria-label="接口列表">
        <div className="api-docs-panel__pane-header">
          <div className="api-docs-panel__pane-copy">
            <Typography.Text strong>接口列表</Typography.Text>
            <Typography.Text type="secondary">
              {selectedCategoryId ? '当前分类' : '全部分类'}接口加载失败
            </Typography.Text>
          </div>
        </div>
        <div className="api-docs-panel__pane-body">
          <Result
            status="error"
            title="接口列表加载失败"
            subTitle="请刷新后重试，或切换到其他分类。"
          />
        </div>
      </section>
    );
  }

  return (
    <section className="api-docs-panel__pane" aria-label="接口列表">
      <div className="api-docs-panel__pane-header">
        <div className="api-docs-panel__pane-copy">
          <Typography.Text strong>接口列表</Typography.Text>
          <Typography.Text type="secondary">
            {selectedCategoryLabel ?? '全部分类'} 共{' '}
            {selectedCategoryId
              ? selectedCategoryOperationTotal
              : operations.length}{' '}
            个接口
          </Typography.Text>
        </div>
      </div>
      <div className="api-docs-panel__pane-toolbar">
        <Input
          aria-label="搜索接口"
          allowClear
          placeholder="搜索接口"
          value={operationSearch}
          onChange={(event) => onOperationSearchChange(event.target.value)}
        />
      </div>
      <div className="api-docs-panel__pane-body" onScroll={onOperationScroll}>
        {!operations.length ? (
          <Empty
            description={
              operationSearch.trim()
                ? '未找到匹配接口'
                : selectedCategoryId
                  ? '当前分类暂无接口'
                  : '暂无接口'
            }
            image={Empty.PRESENTED_IMAGE_SIMPLE}
          />
        ) : filteredOperations.length === 0 ? (
          <Empty
            description="未找到匹配接口"
            image={Empty.PRESENTED_IMAGE_SIMPLE}
          />
        ) : (
          <div className="api-docs-panel__list">
            {filteredOperations.map((operation) => (
              <button
                key={operation.id}
                type="button"
                className="api-docs-panel__list-button api-docs-panel__list-button--operation"
                aria-pressed={selectedOperationId === operation.id}
                onClick={() =>
                  onQueryStateChange({
                    categoryId: selectedCategoryId,
                    operationId: operation.id
                  })
                }
              >
                <span className="api-docs-panel__list-button-main">
                  <span className="api-docs-panel__operation-heading">
                    <span
                      className={`api-docs-panel__operation-method api-docs-panel__operation-method--${operation.method.toLowerCase()}`}
                    >
                      {operation.method}
                    </span>
                    <span className="api-docs-panel__operation-path">
                      {operation.path}
                    </span>
                  </span>
                  <span className="api-docs-panel__list-button-subtitle">
                    {operation.summary ?? operation.description ?? operation.id}
                  </span>
                </span>
              </button>
            ))}
            {selectedCategoryId && fetchingNextPage ? (
              <div className="api-docs-panel__list-loading">
                <Spin size="small" />
              </div>
            ) : null}
          </div>
        )}
      </div>
    </section>
  );
}
