import {
  createFrontstageGroup,
  createFrontstagePage,
  deleteFrontstagePageNode,
  getDefaultApiBaseUrl,
  listFrontstagePages,
  moveFrontstagePageNode,
  updateFrontstagePageNodeTitle,
  type ApiBaseUrlLocation,
  type ConsoleFrontstagePageNode,
  type ConsoleFrontstagePageTreeNode
} from '@1flowbase/api-client';

export type FrontstagePageTreeNode = ConsoleFrontstagePageTreeNode;
export type FrontstagePageNode = ConsoleFrontstagePageNode;

export interface CreateFrontstageNodeInput {
  title: string | null;
  parentId: string | null;
  rank: string;
}

export interface RenameFrontstageNodeInput {
  title: string | null;
}

export interface MoveFrontstageNodeInput {
  parentId: string | null;
  rank: string;
}

export const frontstagePageTreeQueryKey = (workspaceId: string) =>
  ['frontstage', workspaceId, 'page-tree'] as const;

export function getFrontstageApiBaseUrl(
  locationLike: ApiBaseUrlLocation | undefined = typeof window !== 'undefined'
    ? window.location
    : undefined
): string {
  return (
    import.meta.env.VITE_API_BASE_URL ?? getDefaultApiBaseUrl(locationLike)
  );
}

export function fetchFrontstagePageTree(
  workspaceId: string
): Promise<FrontstagePageTreeNode[]> {
  return listFrontstagePages(workspaceId, getFrontstageApiBaseUrl());
}

export function createFrontstagePageGroupNode(
  workspaceId: string,
  input: CreateFrontstageNodeInput,
  csrfToken: string
): Promise<FrontstagePageNode> {
  return createFrontstageGroup(
    workspaceId,
    {
      title: input.title,
      parent_id: input.parentId,
      rank: input.rank
    },
    csrfToken,
    getFrontstageApiBaseUrl()
  );
}

export function createFrontstagePageNode(
  workspaceId: string,
  input: CreateFrontstageNodeInput,
  csrfToken: string
): Promise<FrontstagePageNode> {
  return createFrontstagePage(
    workspaceId,
    {
      title: input.title,
      parent_id: input.parentId,
      rank: input.rank
    },
    csrfToken,
    getFrontstageApiBaseUrl()
  );
}

export function renameFrontstagePageNode(
  workspaceId: string,
  pageNodeId: string,
  input: RenameFrontstageNodeInput,
  csrfToken: string
): Promise<FrontstagePageNode> {
  return updateFrontstagePageNodeTitle(
    workspaceId,
    pageNodeId,
    { title: input.title },
    csrfToken,
    getFrontstageApiBaseUrl()
  );
}

export function moveFrontstageNode(
  workspaceId: string,
  pageNodeId: string,
  input: MoveFrontstageNodeInput,
  csrfToken: string
): Promise<FrontstagePageNode> {
  return moveFrontstagePageNode(
    workspaceId,
    pageNodeId,
    {
      parent_id: input.parentId,
      rank: input.rank
    },
    csrfToken,
    getFrontstageApiBaseUrl()
  );
}

export function deleteFrontstageNode(
  workspaceId: string,
  pageNodeId: string,
  csrfToken: string
): Promise<void> {
  return deleteFrontstagePageNode(
    workspaceId,
    pageNodeId,
    csrfToken,
    getFrontstageApiBaseUrl()
  );
}
