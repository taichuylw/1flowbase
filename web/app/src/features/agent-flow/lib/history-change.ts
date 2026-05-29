import type { FlowAuthoringDocument } from '@1flowbase/flow-schema';

import { classifyDocumentChange } from './document/change-kind';
import { i18nText } from '../../../shared/i18n/text';

export function buildVersionSummary(
  before: FlowAuthoringDocument,
  after: FlowAuthoringDocument
): string {
  const beforeIds = new Set(before.graph.nodes.map((node) => node.id));
  const afterIds = new Set(after.graph.nodes.map((node) => node.id));
  const added = after.graph.nodes.filter((node) => !beforeIds.has(node.id));
  const removed = before.graph.nodes.filter((node) => !afterIds.has(node.id));

  if (added.length > 0) {
    return i18nText("agentFlow", "auto.history_add_nodes", { value1: added.map((node) => node.alias).join('、') });
  }

  if (removed.length > 0) {
    return i18nText("agentFlow", "auto.delete_item", { value1: removed.map((node) => node.alias).join('、') });
  }

  return classifyDocumentChange(before, after) === 'logical'
    ? i18nText("agentFlow", "auto.history_update_node_configuration")
    : i18nText("agentFlow", "auto.history_update_canvas_layout");
}
