import { useState } from 'react';

import { useQueryClient } from '@tanstack/react-query';
import { Alert, Layout, Modal, Typography } from 'antd';
import type { UploadFile } from 'antd/es/upload/interface';

import { useAuthStore } from '../../../../state/auth-store';
import { ModelProviderCatalogPanel } from '../../components/model-providers/ModelProviderCatalogPanel';
import { ModelProviderInstanceDrawer } from '../../components/model-providers/ModelProviderInstanceDrawer';
import { ModelProviderInstancesModal } from '../../components/model-providers/ModelProviderInstancesModal';
import { OfficialPluginInstallPanel } from '../../components/model-providers/OfficialPluginInstallPanel';
import { PluginUploadInstallModal } from '../../components/model-providers/PluginUploadInstallModal';
import {
  settingsModelProviderCatalogQueryKey,
  settingsModelProviderInstancesQueryKey,
  settingsModelProviderOptionsQueryKey
} from '../../api/model-providers';
import {
  settingsOfficialPluginsQueryKey,
  settingsPluginFamiliesQueryKey
} from '../../api/plugins';
import '../../components/model-providers/model-provider-panel.css';
import {
  getErrorMessage,
  MODEL_PROVIDER_MAIN_INSTANCE_QUERY_KEY_PREFIX,
  MODEL_PROVIDER_MODELS_QUERY_KEY_PREFIX,
  resetUploadState,
  type ModelProviderDrawerState,
  type ModelProviderInstanceModalState,
  type RecentVersionSwitchNotice,
  type UploadResultSummary
} from './model-providers/shared';
import { useModelProviderData } from './model-providers/use-model-provider-data';
import { useModelProviderMutations } from './model-providers/use-model-provider-mutations';
import { useOfficialPluginTask } from './model-providers/use-official-plugin-task';
import { SettingsSectionSurface } from '../../components/SettingsSectionSurface';
import { i18nText } from '../../../../shared/i18n/text';

export function SettingsModelProvidersSection({
  canManage
}: {
  canManage: boolean;
}) {
  const queryClient = useQueryClient();
  const csrfToken = useAuthStore((state) => state.csrfToken);
  const [modal, modalContextHolder] = Modal.useModal();
  const [drawerState, setDrawerState] =
    useState<ModelProviderDrawerState>(null);
  const [instanceModalState, setInstanceModalState] =
    useState<ModelProviderInstanceModalState>(null);
  const [uploadModalOpen, setUploadModalOpen] = useState(false);
  const [uploadFileList, setUploadFileList] = useState<UploadFile[]>([]);
  const [uploadValidationMessage, setUploadValidationMessage] = useState<
    string | null
  >(null);
  const [uploadResultSummary, setUploadResultSummary] =
    useState<UploadResultSummary>(null);
  const [recentVersionSwitchNotice, setRecentVersionSwitchNotice] =
    useState<RecentVersionSwitchNotice>(null);
  const [officialSearchQuery, setOfficialSearchQuery] = useState('');
  const clearUploadState = () => {
    resetUploadState(
      setUploadFileList,
      setUploadValidationMessage,
      setUploadResultSummary
    );
  };

  const handleOfficialInstallSettled = async (status: 'success' | 'failed') => {
    if (status !== 'success') {
      return;
    }

    await Promise.all([
      queryClient.invalidateQueries({
        queryKey: settingsModelProviderCatalogQueryKey
      }),
      queryClient.invalidateQueries({
        queryKey: settingsModelProviderInstancesQueryKey
      }),
      queryClient.invalidateQueries({
        queryKey: settingsPluginFamiliesQueryKey
      }),
      queryClient.invalidateQueries({
        queryKey: settingsModelProviderOptionsQueryKey
      }),
      queryClient.invalidateQueries({
        queryKey: MODEL_PROVIDER_MAIN_INSTANCE_QUERY_KEY_PREFIX
      }),
      queryClient.invalidateQueries({
        queryKey: MODEL_PROVIDER_MODELS_QUERY_KEY_PREFIX
      }),
      queryClient.invalidateQueries({
        queryKey: settingsOfficialPluginsQueryKey
      })
    ]);
  };
  const { officialInstallState, setOfficialInstallState, pluginTaskQuery } =
    useOfficialPluginTask({
      onSettled: handleOfficialInstallSettled
    });
  const {
    catalogQuery,
    familiesQuery,
    officialCatalogQuery,
    instancesQuery,
    optionsQuery,
    mainInstanceQuery,
    families,
    officialCatalogEntries,
    officialSourceMeta,
    currentCatalogEntriesByProviderCode,
    familiesByProviderCode,
    editingInstance,
    editingModelCatalog,
    drawerCatalogEntry,
    drawerDefaultIncludedInMain,
    modalInstances,
    modalCatalogEntry,
    modalProviderOption,
    overviewRows
  } = useModelProviderData({
    drawerState,
    instanceModalState,
    officialSearchQuery
  });
  const {
    createMutation,
    updateMutation,
    updateInstanceInclusionMutation,
    updateMainInstanceSettingsMutation,
    previewMutation,
    validateMutation,
    refreshMutation,
    revealSecretMutation,
    deleteMutation,
    familyDeleteMutation,
    officialInstallMutation,
    uploadMutation,
    refreshCurrentNodeArtifactMutation,
    installCurrentNodeArtifactMutation,
    versionMutation
  } = useModelProviderMutations({
    csrfToken,
    queryClient,
    setDrawerState,
    setInstanceModalState,
    setOfficialInstallState,
    setUploadValidationMessage,
    setUploadResultSummary,
    setRecentVersionSwitchNotice
  });

  const errorMessage =
    getErrorMessage(catalogQuery.error) ??
    getErrorMessage(familiesQuery.error) ??
    getErrorMessage(officialCatalogQuery.error) ??
    getErrorMessage(instancesQuery.error) ??
    getErrorMessage(optionsQuery.error) ??
    getErrorMessage(mainInstanceQuery.error) ??
    getErrorMessage(createMutation.error) ??
    getErrorMessage(updateMutation.error) ??
    getErrorMessage(updateInstanceInclusionMutation.error) ??
    getErrorMessage(updateMainInstanceSettingsMutation.error) ??
    getErrorMessage(previewMutation.error) ??
    getErrorMessage(revealSecretMutation.error) ??
    getErrorMessage(validateMutation.error) ??
    getErrorMessage(refreshMutation.error) ??
    getErrorMessage(deleteMutation.error) ??
    getErrorMessage(familyDeleteMutation.error) ??
    getErrorMessage(officialInstallMutation.error) ??
    getErrorMessage(versionMutation.error) ??
    getErrorMessage(refreshCurrentNodeArtifactMutation.error) ??
    getErrorMessage(installCurrentNodeArtifactMutation.error) ??
    getErrorMessage(pluginTaskQuery.error);
  const uploadErrorMessage =
    uploadValidationMessage ?? getErrorMessage(uploadMutation.error);

  return (
    <>
      {modalContextHolder}
      <SettingsSectionSurface
        title={i18nText('settings', 'auto.model_providers')}
        hideHeader
        heightMode="fill"
        status={
          errorMessage ? (
            <Alert type="error" showIcon message={errorMessage} />
          ) : null
        }
      >
        <div className="model-provider-panel">
          <Layout className="model-provider-panel__main">
            <Layout.Content className="model-provider-panel__left">
              <ModelProviderCatalogPanel
                overviewRows={overviewRows}
                entries={families}
                currentCatalogEntries={currentCatalogEntriesByProviderCode}
                loading={catalogQuery.isLoading || familiesQuery.isLoading}
                canManage={canManage}
                deletingProviderCode={
                  familyDeleteMutation.isPending
                    ? (familyDeleteMutation.variables ?? null)
                    : null
                }
                switchingProviderCode={
                  versionMutation.isPending &&
                  versionMutation.variables.mode === 'switch'
                    ? versionMutation.variables.providerCode
                    : null
                }
                upgradingProviderCode={
                  versionMutation.isPending &&
                  versionMutation.variables.mode === 'upgrade'
                    ? versionMutation.variables.providerCode
                    : null
                }
                refreshingArtifactInstallationId={
                  refreshCurrentNodeArtifactMutation.isPending
                    ? (refreshCurrentNodeArtifactMutation.variables ?? null)
                    : null
                }
                installingArtifactInstallationId={
                  installCurrentNodeArtifactMutation.isPending
                    ? (installCurrentNodeArtifactMutation.variables ?? null)
                    : null
                }
                onViewInstances={(entry) => {
                  setInstanceModalState({
                    providerCode: entry.provider_code,
                    displayName: entry.display_name
                  });
                }}
                onCreate={(entry) => {
                  setDrawerState({
                    mode: 'create',
                    providerCode: entry.provider_code
                  });
                }}
                onUpgradeLatest={(entry) => {
                  versionMutation.mutate({
                    mode: 'upgrade',
                    providerCode: entry.provider_code
                  });
                }}
                onSwitchVersion={(entry, installationId) => {
                  versionMutation.mutate({
                    mode: 'switch',
                    providerCode: entry.provider_code,
                    installationId
                  });
                }}
                onRefreshCurrentNodeArtifact={(entry) => {
                  refreshCurrentNodeArtifactMutation.mutate(
                    entry.current_installation_id
                  );
                }}
                onInstallCurrentNodeArtifact={(entry) => {
                  installCurrentNodeArtifactMutation.mutate(
                    entry.current_installation_id
                  );
                }}
                onDelete={(entry) => {
                  void modal.confirm({
                    title: i18nText('settings', 'auto.delete_supplier'),
                    icon: null,
                    centered: true,
                    okText: i18nText('settings', 'auto.delete'),
                    okType: 'danger',
                    cancelText: i18nText('settings', 'auto.cancel'),
                    okButtonProps: {
                      loading:
                        familyDeleteMutation.isPending &&
                        familyDeleteMutation.variables === entry.provider_code
                    },
                    content: (
                      <div className="model-provider-panel__install-confirm">
                        <div className="model-provider-panel__install-confirm-card">
                          <Typography.Title level={5}>
                            {entry.display_name}
                          </Typography.Title>
                          <Typography.Paragraph type="secondary">
                            {i18nText(
                              'settings',
                              'auto.deletion_all_instances_installation_records_local_plug_files_provider_cleaned'
                            )}
                          </Typography.Paragraph>
                          <Typography.Paragraph type="secondary">
                            {i18nText(
                              'settings',
                              'auto.existing_process_node_still_references_provider_subsequent_error_reports_normal'
                            )}
                          </Typography.Paragraph>
                        </div>
                      </div>
                    ),
                    onOk: async () => {
                      await familyDeleteMutation.mutateAsync(
                        entry.provider_code
                      );
                    }
                  });
                }}
              />
            </Layout.Content>

            <Layout.Sider
              width={360}
              theme="light"
              className="model-provider-panel__sidebar"
            >
              <OfficialPluginInstallPanel
                sourceMeta={officialSourceMeta}
                entries={officialCatalogEntries}
                familiesByProviderCode={familiesByProviderCode}
                searchQuery={officialSearchQuery}
                loading={officialCatalogQuery.isLoading}
                canManage={canManage}
                activePluginId={officialInstallState.pluginId}
                installState={officialInstallState.status}
                upgradingProviderCode={
                  versionMutation.isPending &&
                  versionMutation.variables?.mode === 'upgrade'
                    ? (versionMutation.variables.providerCode ?? null)
                    : null
                }
                onInstall={(entry) => {
                  officialInstallMutation.mutate(entry.plugin_id);
                }}
                onOpenUpload={() => {
                  setUploadModalOpen(true);
                  clearUploadState();
                }}
                onSearchQueryChange={setOfficialSearchQuery}
                onUpgradeLatest={(entry) => {
                  versionMutation.mutate({
                    mode: 'upgrade',
                    providerCode: entry.provider_code
                  });
                }}
              />
            </Layout.Sider>
          </Layout>
        </div>
      </SettingsSectionSurface>

      <ModelProviderInstanceDrawer
        open={drawerState !== null}
        mode={drawerState?.mode ?? 'create'}
        catalogEntry={drawerCatalogEntry}
        instance={editingInstance}
        cachedModelCatalog={editingModelCatalog}
        defaultIncludedInMain={drawerDefaultIncludedInMain}
        submitting={createMutation.isPending || updateMutation.isPending}
        onClose={() => setDrawerState(null)}
        onRevealSecret={async (fieldKey) => {
          if (!editingInstance) {
            throw new Error('missing provider instance');
          }

          const result = await revealSecretMutation.mutateAsync({
            instanceId: editingInstance.id,
            key: fieldKey
          });

          return typeof result.value === 'string'
            ? result.value
            : JSON.stringify(result.value ?? '');
        }}
        onSubmit={async (values) => {
          if (drawerState?.mode === 'edit' && editingInstance) {
            await updateMutation.mutateAsync({
              instanceId: editingInstance.id,
              display_name: values.display_name,
              included_in_main: values.included_in_main,
              configured_models: values.configured_models,
              preview_token: values.preview_token,
              config: values.config
            });
            return;
          }

          if (!drawerCatalogEntry) {
            throw new Error('missing provider catalog entry');
          }

          await createMutation.mutateAsync({
            installationId: drawerCatalogEntry.installation_id,
            display_name: values.display_name,
            included_in_main: values.included_in_main,
            configured_models: values.configured_models,
            preview_token: values.preview_token,
            config: values.config
          });
        }}
        onPreviewModels={async (config) => {
          if (drawerState?.mode === 'edit' && editingInstance) {
            return previewMutation.mutateAsync({
              instanceId: editingInstance.id,
              config
            });
          }

          if (!drawerCatalogEntry) {
            throw new Error('missing provider catalog entry');
          }

          return previewMutation.mutateAsync({
            installationId: drawerCatalogEntry.installation_id,
            config
          });
        }}
      />

      <ModelProviderInstancesModal
        open={instanceModalState !== null}
        catalogEntry={modalCatalogEntry}
        providerDisplayName={instanceModalState?.displayName ?? null}
        mainInstance={
          mainInstanceQuery.data ??
          (modalProviderOption
            ? {
                provider_code: modalProviderOption.provider_code,
                auto_include_new_instances:
                  modalProviderOption.main_instance.auto_include_new_instances
              }
            : null)
        }
        modelGroups={modalProviderOption?.model_groups ?? []}
        instances={modalInstances}
        updatingMainInstance={updateMainInstanceSettingsMutation.isPending}
        updatingInstanceId={
          updateInstanceInclusionMutation.isPending
            ? (updateInstanceInclusionMutation.variables?.instance.id ?? null)
            : null
        }
        refreshingCandidates={validateMutation.isPending}
        refreshing={refreshMutation.isPending}
        deleting={deleteMutation.isPending}
        canManage={canManage}
        versionSwitchNotice={
          instanceModalState &&
          recentVersionSwitchNotice?.providerCode ===
            instanceModalState.providerCode
            ? {
                targetVersion: recentVersionSwitchNotice.targetVersion,
                migratedInstanceCount:
                  recentVersionSwitchNotice.migratedInstanceCount
              }
            : null
        }
        onClose={() => {
          setInstanceModalState(null);
          setRecentVersionSwitchNotice((current) =>
            current && current.providerCode === instanceModalState?.providerCode
              ? null
              : current
          );
        }}
        onEdit={(instance) => {
          setDrawerState({
            mode: 'edit',
            instanceId: instance.id
          });
        }}
        onRefreshCandidates={(instance) => {
          validateMutation.mutate(instance.id);
        }}
        onRefreshModels={(instance) => {
          refreshMutation.mutate(instance.id);
        }}
        onDelete={(instance) => {
          deleteMutation.mutate(instance.id);
        }}
        onToggleAutoIncludeNewInstances={(checked) => {
          if (!instanceModalState) {
            return;
          }

          updateMainInstanceSettingsMutation.mutate({
            providerCode: instanceModalState.providerCode,
            auto_include_new_instances: checked
          });
        }}
        onToggleIncludedInMain={(instance, checked) => {
          updateInstanceInclusionMutation.mutate({
            instance,
            included_in_main: checked
          });
        }}
      />

      <PluginUploadInstallModal
        open={uploadModalOpen}
        submitting={uploadMutation.isPending}
        resultSummary={uploadResultSummary}
        errorMessage={uploadErrorMessage}
        fileList={uploadFileList}
        onClose={() => {
          setUploadModalOpen(false);
          clearUploadState();
        }}
        onChange={(nextFiles) => {
          clearUploadState();
          setUploadFileList(nextFiles.slice(-1));
        }}
        onSubmit={() => {
          const file = uploadFileList[0]?.originFileObj;
          if (!(file instanceof File)) {
            setUploadValidationMessage(
              i18nText('settings', 'auto.select_plug_package_first')
            );
            return;
          }

          uploadMutation.mutate(file);
        }}
      />
    </>
  );
}
