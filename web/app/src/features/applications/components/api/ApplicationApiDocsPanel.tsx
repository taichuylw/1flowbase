import { useCallback, useState } from 'react';

import { Space, Typography } from 'antd';

import { ApiDocsExplorer } from '../../../../shared/ui/api-docs/ApiDocsExplorer';
import { getApplicationsApiBaseUrl } from '../../api/applications';
import {
  applicationApiDocsCatalogQueryKey,
  applicationApiDocsCategoryOperationsQueryKey,
  applicationApiDocsOperationSpecQueryKey,
  fetchApplicationApiDocsCatalog,
  fetchApplicationApiDocsCategoryOperations,
  fetchApplicationApiDocsOperationSpec
} from '../../api/public-api';

export function ApplicationApiDocsPanel({
  applicationId,
  applicationName,
  defaultCategoryId
}: {
  applicationId: string;
  applicationName: string;
  defaultCategoryId: string;
}) {
  const [queryState, setQueryState] = useState<{
    categoryId: string | null;
    operationId: string | null;
  }>({ categoryId: defaultCategoryId, operationId: null });
  const handleQueryStateChange = useCallback(
    (nextState: { categoryId: string | null; operationId: string | null }) =>
      setQueryState(nextState),
    []
  );

  return (
    <section className="application-api-panel">
      <Space direction="vertical" size={12} className="application-api-docs-head">
        <Typography.Title level={4}>{applicationName} API 文档</Typography.Title>
      </Space>
      <ApiDocsExplorer
        queryState={queryState}
        onQueryStateChange={handleQueryStateChange}
        catalogQueryKey={applicationApiDocsCatalogQueryKey(applicationId)}
        fetchCatalog={() => fetchApplicationApiDocsCatalog(applicationId)}
        categoryOperationsQueryKey={(categoryId) =>
          applicationApiDocsCategoryOperationsQueryKey(applicationId, categoryId)
        }
        fetchCategoryOperations={(categoryId) =>
          fetchApplicationApiDocsCategoryOperations(applicationId, categoryId)
        }
        operationSpecQueryKey={(operationId) =>
          applicationApiDocsOperationSpecQueryKey(applicationId, operationId)
        }
        fetchOperationSpec={(operationId) =>
          fetchApplicationApiDocsOperationSpec(applicationId, operationId)
        }
        baseServerUrl={getApplicationsApiBaseUrl}
      />
    </section>
  );
}
