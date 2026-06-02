import { apiFetch } from '../transport';

export interface ConsoleApiDocsCatalogOperation {
  id: string;
  method: string;
  path: string;
  summary: string | null;
  description: string | null;
  tags: string[];
  group: string;
  deprecated: boolean;
}

export interface ConsoleApiDocsCatalog {
  title: string;
  version: string;
  categories: ConsoleApiDocsCatalogCategory[];
}

export interface ConsoleApiDocsCatalogCategory {
  id: string;
  label: string;
  operation_count: number;
}

export interface ConsoleApiDocsCategoryOperations {
  id: string;
  label: string;
  operations: ConsoleApiDocsCatalogOperation[];
  total?: number;
  offset?: number;
  limit?: number;
  has_more?: boolean;
  next_offset?: number | null;
}

export interface ConsoleApiDocsCategoryOperationsRequest {
  offset?: number;
  limit?: number;
  q?: string | null;
}

export function fetchConsoleApiDocsCatalog(
  baseUrl?: string
): Promise<ConsoleApiDocsCatalog> {
  return apiFetch<ConsoleApiDocsCatalog>({
    path: '/api/console/docs/catalog',
    baseUrl
  });
}

export function fetchConsoleApiDocsCategoryOperations(
  categoryId: string,
  request: ConsoleApiDocsCategoryOperationsRequest = {},
  baseUrl?: string
): Promise<ConsoleApiDocsCategoryOperations> {
  const params = new URLSearchParams();

  if (request.offset !== undefined) {
    params.set('offset', String(request.offset));
  }

  if (request.limit !== undefined) {
    params.set('limit', String(request.limit));
  }

  if (request.q) {
    params.set('q', request.q);
  }

  const query = params.size > 0 ? `?${params.toString()}` : '';

  return apiFetch<ConsoleApiDocsCategoryOperations>({
    path: `/api/console/docs/categories/${encodeURIComponent(categoryId)}/operations${query}`,
    baseUrl
  });
}

export function fetchConsoleApiDocsCategorySpec(
  categoryId: string,
  baseUrl?: string
): Promise<Record<string, unknown>> {
  return apiFetch<Record<string, unknown>>({
    path: `/api/console/docs/categories/${encodeURIComponent(categoryId)}/openapi.json`,
    baseUrl,
    unwrapSuccess: false
  });
}

export function fetchConsoleApiOperationSpec(
  operationId: string,
  baseUrl?: string
): Promise<Record<string, unknown>> {
  return apiFetch<Record<string, unknown>>({
    path: `/api/console/docs/operations/${operationId}/openapi.json`,
    baseUrl,
    unwrapSuccess: false
  });
}
