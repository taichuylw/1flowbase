import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { Alert, Button, Result } from 'antd';

import { useAuthStore } from '../../../state/auth-store';
import {
  applicationDetailQueryKey,
  type ApplicationDetail
} from '../api/applications';
import {
  applicationApiMappingQueryKey,
  applicationApiPublicationQueryKey,
  fetchApplicationApiMapping,
  fetchApplicationApiPublication,
  publishApplicationApiVersion,
  setApplicationApiEnabled
} from '../api/public-api';
import { ApplicationApiDocsPanel } from '../components/api/ApplicationApiDocsPanel';
import { ApplicationApiKeysPanel } from '../components/api/ApplicationApiKeysPanel';
import { ApplicationApiStatusBar } from '../components/api/ApplicationApiStatusBar';
import './application-api-page.css';
import { i18nText } from '../../../shared/i18n/text';

export function ApplicationApiPage({
  application
}: {
  application: ApplicationDetail;
}) {
  const csrfToken = useAuthStore((state) => state.csrfToken) ?? '';
  const queryClient = useQueryClient();
  const docsToolbarId = `application-api-docs-toolbar-${application.id}`;
  const publicationQuery = useQuery({
    queryKey: applicationApiPublicationQueryKey(application.id),
    queryFn: () => fetchApplicationApiPublication(application.id),
    retry: false
  });
  const mappingQuery = useQuery({
    queryKey: applicationApiMappingQueryKey(application.id),
    queryFn: () => fetchApplicationApiMapping(application.id)
  });
  const publication = publicationQuery.data ?? null;
  const invalidatePublication = () => {
    void queryClient.invalidateQueries({
      queryKey: applicationApiPublicationQueryKey(application.id)
    });
    void queryClient.invalidateQueries({
      queryKey: applicationDetailQueryKey(application.id)
    });
  };
  const publishMutation = useMutation({
    mutationFn: async () => {
      const mapping =
        mappingQuery.data ?? (await fetchApplicationApiMapping(application.id));
      return publishApplicationApiVersion(application.id, mapping, csrfToken);
    },
    onSuccess: invalidatePublication
  });
  const statusMutation = useMutation({
    mutationFn: (enabled: boolean) =>
      setApplicationApiEnabled(application.id, enabled, csrfToken),
    onSuccess: invalidatePublication
  });

  if (!publication && publicationQuery.isLoading) {
    return <Result status="info" title={i18nText("applications", "auto.k_efe6d2cf3c")} />;
  }

  return (
    <div className="application-api-page">
      <ApplicationApiStatusBar
        publication={publication}
        loading={statusMutation.isPending}
        onToggleEnabled={(enabled) => statusMutation.mutate(enabled)}
        toolbar={
          <div
            id={docsToolbarId}
            className="application-api-status__docs-toolbar-target"
          />
        }
      >
        <ApplicationApiKeysPanel
          applicationId={application.id}
          csrfToken={csrfToken}
          onCreatedToken={() => undefined}
          variant="embedded"
        />
      </ApplicationApiStatusBar>
      {!publication ? (
        <Alert
          type="warning"
          showIcon
          message={i18nText("applications", "auto.k_7a5fb3c3d0")}
          description={i18nText("applications", "auto.k_dc3901e5a6")}
          action={
            <Button
              type="primary"
              loading={publishMutation.isPending || mappingQuery.isLoading}
              onClick={() => publishMutation.mutate()}
            >
              {i18nText("applications", "auto.k_b9f1ed2cf2")}</Button>
          }
        />
      ) : null}
      <ApplicationApiDocsPanel
        applicationId={application.id}
        toolbarPortalId={docsToolbarId}
      />
    </div>
  );
}
