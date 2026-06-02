import type { FlowAuthoringDocument } from '@1flowbase/flow-schema';

import type { AgentFlowIssue } from '../../lib/validate-document';

export function getDocumentWithLatestViewport(
  currentDocument: FlowAuthoringDocument,
  viewport: FlowAuthoringDocument['editor']['viewport']
): FlowAuthoringDocument {
  const currentViewport = currentDocument.editor.viewport;

  if (
    currentViewport.x === viewport.x &&
    currentViewport.y === viewport.y &&
    currentViewport.zoom === viewport.zoom
  ) {
    return currentDocument;
  }

  return {
    ...currentDocument,
    editor: {
      ...currentDocument.editor,
      viewport
    }
  };
}

export function countIssuesByNodeId(
  issues: AgentFlowIssue[]
): Record<string, number> {
  const counts: Record<string, number> = {};

  for (const issue of issues) {
    if (!issue.nodeId) {
      continue;
    }

    counts[issue.nodeId] = (counts[issue.nodeId] ?? 0) + 1;
  }

  return counts;
}
