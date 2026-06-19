export interface LazyTraceChildrenState<TNode, TPageInfo, TProjectionStatus> {
  additionalNodes: TNode[];
  pageInfo: TPageInfo | null;
  projectionStatus?: TProjectionStatus;
  loadingMore: boolean;
  loadMoreFailed: boolean;
}

export type LazyTraceChildrenAction<TNode, TPageInfo, TProjectionStatus> =
  | { type: 'load_more_started' }
  | {
      type: 'load_more_succeeded';
      page: {
        items: TNode[];
        page_info: TPageInfo;
        projection_status?: TProjectionStatus;
      };
    }
  | { type: 'load_more_failed' };

export function createInitialLazyTraceChildrenState<
  TNode,
  TPageInfo,
  TProjectionStatus
>(): LazyTraceChildrenState<TNode, TPageInfo, TProjectionStatus> {
  return {
    additionalNodes: [],
    pageInfo: null,
    projectionStatus: undefined,
    loadingMore: false,
    loadMoreFailed: false
  };
}

export function appendTraceChildrenPage<
  TNode extends { trace_node_id: string }
>(current: TNode[], nextPageItems: TNode[]) {
  if (current.length === 0) {
    return nextPageItems;
  }

  const seenTraceNodeIds = new Set(
    current.map((childNode) => childNode.trace_node_id)
  );
  const next = [...current];

  for (const childNode of nextPageItems) {
    if (!seenTraceNodeIds.has(childNode.trace_node_id)) {
      seenTraceNodeIds.add(childNode.trace_node_id);
      next.push(childNode);
    }
  }

  return next;
}

export function lazyTraceChildrenReducer<
  TNode extends { trace_node_id: string },
  TPageInfo,
  TProjectionStatus
>(
  state: LazyTraceChildrenState<TNode, TPageInfo, TProjectionStatus>,
  action: LazyTraceChildrenAction<TNode, TPageInfo, TProjectionStatus>
): LazyTraceChildrenState<TNode, TPageInfo, TProjectionStatus> {
  switch (action.type) {
    case 'load_more_started':
      return {
        ...state,
        loadingMore: true,
        loadMoreFailed: false
      };
    case 'load_more_succeeded':
      return {
        additionalNodes: appendTraceChildrenPage(
          state.additionalNodes,
          action.page.items
        ),
        pageInfo: action.page.page_info,
        projectionStatus: action.page.projection_status,
        loadingMore: false,
        loadMoreFailed: false
      };
    case 'load_more_failed':
      return {
        ...state,
        loadingMore: false,
        loadMoreFailed: true
      };
  }
}
