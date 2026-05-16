import { apiFetch } from './transport';

export interface ConsoleFrontstagePageTreeNode {
  id: string;
  title: string | null;
  kind: 'group' | 'page';
  children: ConsoleFrontstagePageTreeNode[];
}

export interface ConsoleFrontstagePageNode {
  id: string;
  title: string | null;
  kind: 'group' | 'page';
  parent_id: string | null;
  rank: string;
  schema_root_uid: string | null;
}

export interface CreateFrontstagePageNodeInput {
  title?: string | null;
  parent_id?: string | null;
  rank?: string | null;
}

export interface UpdateFrontstagePageNodeTitleInput {
  title?: string | null;
}

export interface MoveFrontstagePageNodeInput {
  parent_id?: string | null;
  rank?: string | null;
}

export function listFrontstagePages(
  workspaceId: string,
  baseUrl?: string
): Promise<ConsoleFrontstagePageTreeNode[]> {
  return apiFetch<ConsoleFrontstagePageTreeNode[]>({
    path: `/api/console/frontstage/${workspaceId}/pages`,
    method: 'GET',
    baseUrl
  });
}

export function createFrontstageGroup(
  workspaceId: string,
  input: CreateFrontstagePageNodeInput,
  csrfToken: string,
  baseUrl?: string
): Promise<ConsoleFrontstagePageNode> {
  return apiFetch<ConsoleFrontstagePageNode>({
    path: `/api/console/frontstage/${workspaceId}/pages/groups`,
    method: 'POST',
    body: input,
    csrfToken,
    baseUrl
  });
}

export function createFrontstagePage(
  workspaceId: string,
  input: CreateFrontstagePageNodeInput,
  csrfToken: string,
  baseUrl?: string
): Promise<ConsoleFrontstagePageNode> {
  return apiFetch<ConsoleFrontstagePageNode>({
    path: `/api/console/frontstage/${workspaceId}/pages`,
    method: 'POST',
    body: input,
    csrfToken,
    baseUrl
  });
}

export function updateFrontstagePageNodeTitle(
  workspaceId: string,
  pageNodeId: string,
  input: UpdateFrontstagePageNodeTitleInput,
  csrfToken: string,
  baseUrl?: string
): Promise<ConsoleFrontstagePageNode> {
  return apiFetch<ConsoleFrontstagePageNode>({
    path: `/api/console/frontstage/${workspaceId}/pages/${pageNodeId}`,
    method: 'PATCH',
    body: input,
    csrfToken,
    baseUrl
  });
}

export function moveFrontstagePageNode(
  workspaceId: string,
  pageNodeId: string,
  input: MoveFrontstagePageNodeInput,
  csrfToken: string,
  baseUrl?: string
): Promise<ConsoleFrontstagePageNode> {
  return apiFetch<ConsoleFrontstagePageNode>({
    path: `/api/console/frontstage/${workspaceId}/pages/${pageNodeId}/move`,
    method: 'POST',
    body: input,
    csrfToken,
    baseUrl
  });
}

export function deleteFrontstagePageNode(
  workspaceId: string,
  pageNodeId: string,
  csrfToken: string,
  baseUrl?: string
): Promise<void> {
  return apiFetch<void>({
    path: `/api/console/frontstage/${workspaceId}/pages/${pageNodeId}`,
    method: 'DELETE',
    csrfToken,
    baseUrl
  });
}
