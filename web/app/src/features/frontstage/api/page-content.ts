import {
  getFrontstagePageDetail,
  saveFrontstagePageContent as saveConsoleFrontstagePageContent,
  type ConsoleFrontstagePageDetail,
  type ConsoleFrontstagePageNode
} from '@1flowbase/api-client';

import { getFrontstageApiBaseUrl } from './page-tree';

export interface FrontstagePageContentNode {
  id: string;
  title: string | null;
  icon?: string | null;
  tooltip?: string | null;
  kind: 'group' | 'page';
  parentId: string | null;
  rank: string;
  schemaRootUid: string | null;
}

export interface FrontstagePageSchema {
  rootUid: string;
  payload: unknown;
}

export interface FrontstagePageRoot {
  uid: string;
  payload: unknown;
}

export interface FrontstagePageContent {
  page: FrontstagePageContentNode;
  schema: FrontstagePageSchema;
  root: FrontstagePageRoot;
}

export interface SaveFrontstagePageContentPayloadInput {
  payload: unknown;
}

export interface SaveFrontstagePageContentInput {
  schema: SaveFrontstagePageContentPayloadInput;
  root: SaveFrontstagePageContentPayloadInput;
}

export const frontstagePageContentQueryKey = (
  workspaceId: string,
  pageId: string
) => ['frontstage', workspaceId, 'pages', pageId, 'content'] as const;

function mapFrontstagePageNode(
  page: ConsoleFrontstagePageNode
): FrontstagePageContentNode {
  return {
    id: page.id,
    title: page.title,
    icon: page.icon,
    tooltip: page.tooltip,
    kind: page.kind,
    parentId: page.parent_id,
    rank: page.rank,
    schemaRootUid: page.schema_root_uid
  };
}

function mapFrontstagePageContent(
  detail: ConsoleFrontstagePageDetail
): FrontstagePageContent {
  return {
    page: mapFrontstagePageNode(detail.page),
    schema: {
      rootUid: detail.schema.root_uid,
      payload: detail.schema.payload
    },
    root: {
      uid: detail.root.uid,
      payload: detail.root.payload
    }
  };
}

export async function fetchFrontstagePageContent(
  workspaceId: string,
  pageId: string
): Promise<FrontstagePageContent> {
  const detail = await getFrontstagePageDetail(
    workspaceId,
    pageId,
    getFrontstageApiBaseUrl()
  );

  return mapFrontstagePageContent(detail);
}

export async function saveFrontstagePageContent(
  workspaceId: string,
  pageId: string,
  input: SaveFrontstagePageContentInput,
  csrfToken: string
): Promise<FrontstagePageContent> {
  const detail = await saveConsoleFrontstagePageContent(
    workspaceId,
    pageId,
    input,
    csrfToken,
    getFrontstageApiBaseUrl()
  );

  return mapFrontstagePageContent(detail);
}
