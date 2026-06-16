import {
  downloadConsoleOfficialAgentFlowTemplate,
  getDefaultApiBaseUrl,
  listConsoleOfficialAgentFlowTemplateCatalog,
  type ApiBaseUrlLocation,
  type ConsoleAgentFlowTemplatePackage,
  type ConsoleOfficialAgentFlowTemplateCatalog,
  type ConsoleOfficialAgentFlowTemplateCatalogEntry
} from '@1flowbase/api-client';

export type OfficialAgentFlowTemplateCatalog =
  ConsoleOfficialAgentFlowTemplateCatalog;
export type OfficialAgentFlowTemplateCatalogEntry =
  ConsoleOfficialAgentFlowTemplateCatalogEntry;
export type OfficialAgentFlowTemplatePackage = ConsoleAgentFlowTemplatePackage;

export const officialAgentFlowTemplateCatalogQueryKey = [
  'templates',
  'official-agent-flow',
  'catalog'
] as const;

export const officialAgentFlowTemplateCatalogStaleTimeMs = 2 * 60 * 60 * 1000;

export function getTemplatesApiBaseUrl(
  locationLike: ApiBaseUrlLocation | undefined = typeof window !== 'undefined'
    ? window.location
    : undefined
): string {
  return (
    import.meta.env.VITE_API_BASE_URL ?? getDefaultApiBaseUrl(locationLike)
  );
}

export function fetchOfficialAgentFlowTemplateCatalog(
  cursor?: string | null
): Promise<OfficialAgentFlowTemplateCatalog> {
  return listConsoleOfficialAgentFlowTemplateCatalog(
    { cursor },
    getTemplatesApiBaseUrl()
  );
}

export function downloadOfficialAgentFlowTemplate(
  workflowId: string
): Promise<OfficialAgentFlowTemplatePackage> {
  return downloadConsoleOfficialAgentFlowTemplate(
    workflowId,
    getTemplatesApiBaseUrl()
  );
}
