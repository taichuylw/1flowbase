import {
  fetchConsoleApiDocsCatalog,
  fetchConsoleApiDocsCategoryOperations,
  fetchConsoleApiOperationSpec,
  type ConsoleApiDocsCatalog,
  type ConsoleApiDocsCategoryOperations,
  type ConsoleApiDocsCategoryOperationsRequest
} from '@1flowbase/api-client';

export type SettingsApiDocsCatalog = ConsoleApiDocsCatalog;
export type SettingsApiDocsCategoryOperations = ConsoleApiDocsCategoryOperations;
export type SettingsApiDocsCategoryOperationsRequest =
  ConsoleApiDocsCategoryOperationsRequest;

export const settingsApiDocsCatalogQueryKey = ['settings', 'docs', 'catalog'] as const;
export const settingsApiDocsCategoryOperationsQueryKey = (categoryId: string) =>
  ['settings', 'docs', 'category', categoryId, 'operations'] as const;
export const settingsApiDocsOperationSpecQueryKey = (operationId: string) =>
  ['settings', 'docs', 'operation', operationId, 'openapi'] as const;

export function fetchSettingsApiDocsCatalog(): Promise<SettingsApiDocsCatalog> {
  return fetchConsoleApiDocsCatalog();
}

export function fetchSettingsApiDocsCategoryOperations(
  categoryId: string,
  request?: SettingsApiDocsCategoryOperationsRequest
): Promise<SettingsApiDocsCategoryOperations> {
  return fetchConsoleApiDocsCategoryOperations(categoryId, request);
}

export function fetchSettingsApiDocsOperationSpec(operationId: string) {
  return fetchConsoleApiOperationSpec(operationId);
}
