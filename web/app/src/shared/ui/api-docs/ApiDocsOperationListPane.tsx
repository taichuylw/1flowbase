import type { UIEvent } from 'react';

import { Empty, Input, Result, Spin, Typography } from 'antd';

import type {
  ApiDocsExplorerQueryState,
  ApiDocsOperationWithCategory
} from './ApiDocsExplorer';
import { i18nText } from '../../i18n/text';

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
      <section className="api-docs-panel__pane" aria-label={i18nText("sharedUi", "auto.k_14f70b1746")}>
        <div className="api-docs-panel__pane-header">
          <div className="api-docs-panel__pane-copy">
            <Typography.Text strong>{i18nText("sharedUi", "auto.k_14f70b1746")}</Typography.Text>
            <Typography.Text type="secondary">
              {i18nText("sharedUi", "auto.k_f9d7f46db7")}</Typography.Text>
          </div>
        </div>
        <div className="api-docs-panel__pane-body">
          <Empty
            description={i18nText("sharedUi", "auto.k_b457de4013")}
            image={Empty.PRESENTED_IMAGE_SIMPLE}
          />
        </div>
      </section>
    );
  }

  if (!selectedCategoryId && !showAllOperationsWhenNoCategory) {
    return (
      <section className="api-docs-panel__pane" aria-label={i18nText("sharedUi", "auto.k_14f70b1746")}>
        <div className="api-docs-panel__pane-header">
          <div className="api-docs-panel__pane-copy">
            <Typography.Text strong>{i18nText("sharedUi", "auto.k_14f70b1746")}</Typography.Text>
            <Typography.Text type="secondary">
              {i18nText("sharedUi", "auto.k_866cd947b2")}</Typography.Text>
          </div>
        </div>
        <div className="api-docs-panel__pane-body">
          <Result
            status="info"
            title={i18nText("sharedUi", "auto.k_df1e4ab2a0")}
            subTitle={i18nText("sharedUi", "auto.k_203014aed2")}
          />
        </div>
      </section>
    );
  }

  if (loading) {
    return (
      <section className="api-docs-panel__pane" aria-label={i18nText("sharedUi", "auto.k_14f70b1746")}>
        <div className="api-docs-panel__pane-header">
          <div className="api-docs-panel__pane-copy">
            <Typography.Text strong>{i18nText("sharedUi", "auto.k_14f70b1746")}</Typography.Text>
            <Typography.Text type="secondary">
              {i18nText("sharedUi", "auto.k_3667cb105a")}{selectedCategoryLabel ?? i18nText("sharedUi", "auto.k_a8e369c4b6")} {i18nText("sharedUi", "auto.k_d1e639b007")}</Typography.Text>
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
      <section className="api-docs-panel__pane" aria-label={i18nText("sharedUi", "auto.k_14f70b1746")}>
        <div className="api-docs-panel__pane-header">
          <div className="api-docs-panel__pane-copy">
            <Typography.Text strong>{i18nText("sharedUi", "auto.k_14f70b1746")}</Typography.Text>
            <Typography.Text type="secondary">
              {selectedCategoryId ? i18nText("sharedUi", "auto.k_162b9c0d9c") : i18nText("sharedUi", "auto.k_a8e369c4b6")}{i18nText("sharedUi", "auto.k_b24005bd49")}</Typography.Text>
          </div>
        </div>
        <div className="api-docs-panel__pane-body">
          <Result
            status="error"
            title={i18nText("sharedUi", "auto.k_63d7e0bd78")}
            subTitle={i18nText("sharedUi", "auto.k_2f6e1b8cc2")}
          />
        </div>
      </section>
    );
  }

  return (
    <section className="api-docs-panel__pane" aria-label={i18nText("sharedUi", "auto.k_14f70b1746")}>
      <div className="api-docs-panel__pane-header">
        <div className="api-docs-panel__pane-copy">
          <Typography.Text strong>{i18nText("sharedUi", "auto.k_14f70b1746")}</Typography.Text>
          <Typography.Text type="secondary">
            {selectedCategoryLabel ?? i18nText("sharedUi", "auto.k_a8e369c4b6")} {i18nText("sharedUi", "auto.k_3b6ef811b8")}{' '}
            {selectedCategoryId
              ? selectedCategoryOperationTotal
              : operations.length}{' '}
            {i18nText("sharedUi", "auto.k_b4eda10b96")}</Typography.Text>
        </div>
      </div>
      <div className="api-docs-panel__pane-toolbar">
        <Input
          aria-label={i18nText("sharedUi", "auto.k_fec0b01c0c")}
          allowClear
          placeholder={i18nText("sharedUi", "auto.k_fec0b01c0c")}
          value={operationSearch}
          onChange={(event) => onOperationSearchChange(event.target.value)}
        />
      </div>
      <div className="api-docs-panel__pane-body" onScroll={onOperationScroll}>
        {!operations.length ? (
          <Empty
            description={
              operationSearch.trim()
                ? i18nText("sharedUi", "auto.k_6bc6b4abd5")
                : selectedCategoryId
                  ? i18nText("sharedUi", "auto.k_bb7bfaac4e")
                  : i18nText("sharedUi", "auto.k_744891d58f")
            }
            image={Empty.PRESENTED_IMAGE_SIMPLE}
          />
        ) : filteredOperations.length === 0 ? (
          <Empty
            description={i18nText("sharedUi", "auto.k_6bc6b4abd5")}
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
