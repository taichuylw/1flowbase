export function createStyleBoundaryFrontstagePageContent() {
  return {
    page: {
      id: 'page-1',
      title: 'Landing',
      kind: 'page' as const,
      parentId: null,
      rank: '001000',
      schemaRootUid: 'root-1'
    },
    schema: {
      rootUid: 'root-1',
      payload: { blocks: [] }
    },
    root: {
      uid: 'root-1',
      payload: { blocks: [] }
    }
  };
}
