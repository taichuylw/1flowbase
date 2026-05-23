import { useState } from 'react';
import { DownOutlined, RightOutlined } from '@ant-design/icons';
import { Typography } from 'antd';

import type { AgentFlowTraceItem } from '../../../api/runtime';
import { NodeRunPayloadSections } from '../../detail/last-run/NodeRunIOCard';
import { DebugWorkflowNodeItem, StatusIcon } from './DebugWorkflowNodeRow';
import { LlmToolTraceTree } from './LlmToolTraceTree';
import { groupTraceItemsForDisplay } from './debug-workflow-trace-utils';
import { stripLlmRoundsFromDebugPayload } from './llm-tool-callbacks';

function workflowStatus(items: AgentFlowTraceItem[]) {
  if (items.some((item) => item.status === 'failed')) {
    return 'failed';
  }

  if (items.some((item) => item.status === 'waiting_human')) {
    return 'waiting_human';
  }

  if (items.some((item) => item.status === 'waiting_callback')) {
    return 'waiting_callback';
  }

  if (items.some((item) => item.status === 'running')) {
    return 'running';
  }

  if (items.every((item) => item.status === 'succeeded')) {
    return 'succeeded';
  }

  return 'running';
}

export function DebugWorkflowProcess({
  items,
  onLoadArtifact
}: {
  items: AgentFlowTraceItem[];
  onLoadArtifact?: (artifactRef: string) => Promise<unknown>;
}) {
  const [expanded, setExpanded] = useState(true);
  const [expandedNodeKeys, setExpandedNodeKeys] = useState<Set<string>>(
    () => new Set()
  );

  if (items.length === 0) {
    return null;
  }

  const status = workflowStatus(items);
  const traceGroups = groupTraceItemsForDisplay(items);

  return (
    <div
      aria-label="工作流"
      className="agent-flow-editor__debug-workflow-process"
      role="group"
    >
      <button
        aria-expanded={expanded}
        className="agent-flow-editor__debug-workflow-header"
        onClick={() => setExpanded((current) => !current)}
        type="button"
      >
        <span className="agent-flow-editor__debug-workflow-title">
          <StatusIcon status={status} />
          <Typography.Text>工作流</Typography.Text>
        </span>
        {expanded ? (
          <DownOutlined className="agent-flow-editor__debug-workflow-collapse" />
        ) : (
          <RightOutlined className="agent-flow-editor__debug-workflow-collapse" />
        )}
      </button>
      {expanded ? (
        <div className="agent-flow-editor__debug-workflow-collapse-list">
          {traceGroups.map((group) => {
            const nodeExpanded = expandedNodeKeys.has(group.key);
            const item = group.item;

            return (
              <DebugWorkflowNodeItem
                key={group.key}
                expanded={nodeExpanded}
                item={item}
                onToggle={() => {
                  setExpandedNodeKeys((current) => {
                    const next = new Set(current);

                    if (next.has(group.key)) {
                      next.delete(group.key);
                    } else {
                      next.add(group.key);
                    }

                    return next;
                  });
                }}
              >
                <div className="agent-flow-editor__debug-workflow-node-detail">
                  <LlmToolTraceTree
                    debugPayload={item.debugPayload}
                    debugPayloads={group.items.map(
                      (traceItem) => traceItem.debugPayload
                    )}
                    onLoadArtifact={onLoadArtifact}
                  />
                  <NodeRunPayloadSections
                    inputPayload={item.inputPayload}
                    debugPayload={stripLlmRoundsFromDebugPayload(
                      item.debugPayload
                    )}
                    outputPayload={item.outputPayload}
                    onLoadArtifact={onLoadArtifact}
                  />
                </div>
              </DebugWorkflowNodeItem>
            );
          })}
        </div>
      ) : null}
    </div>
  );
}
