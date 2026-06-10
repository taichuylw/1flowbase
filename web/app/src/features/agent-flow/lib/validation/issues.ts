import type { FlowNodeDocument } from '@1flowbase/flow-schema';

import type { InspectorSectionKey } from '../node-definitions';
import { findInspectorSectionKey } from '../node-definitions';

export interface AgentFlowIssue {
  id: string;
  scope: 'field' | 'node' | 'global';
  level: 'error' | 'warning';
  nodeId: string | null;
  sectionKey: InspectorSectionKey | null;
  fieldKey?: string | null;
  title: string;
  message: string;
}

export function pushFieldIssue(
  issues: AgentFlowIssue[],
  node: FlowNodeDocument,
  fieldKey: string,
  title: string,
  message: string,
  sectionKey?: InspectorSectionKey | null
) {
  issues.push({
    id: `${node.id}-${fieldKey}-${issues.length}`,
    scope: 'field',
    level: 'error',
    nodeId: node.id,
    sectionKey: sectionKey ?? findInspectorSectionKey(node.type, fieldKey),
    fieldKey,
    title,
    message
  });
}
