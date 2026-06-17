import { useMemo, useState } from 'react';
import { DownOutlined, RightOutlined } from '@ant-design/icons';
import { Typography } from 'antd';

import type { AgentFlowTraceItem } from '../../../api/runtime';
import { DebugWorkflowNodeItem, StatusIcon } from './DebugWorkflowNodeRow';
import { DebugWorkflowNodeDetailContent } from './LlmToolTraceTree';
import { groupTraceItemsForDisplay } from './debug-workflow-trace-utils';
import { i18nText } from '../../../../../shared/i18n/text';

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
  const traceGroups = useMemo(() => groupTraceItemsForDisplay(items), [items]);

  if (items.length === 0) {
    return null;
  }

  const status = workflowStatus(items);

  return (
    <section
      aria-label={i18nText('agentFlow', 'auto.workflow')}
      className="agent-flow-editor__debug-workflow-process"
    >
      <button
        aria-expanded={expanded}
        className="agent-flow-editor__debug-workflow-header"
        onClick={() => setExpanded((current) => !current)}
        type="button"
      >
        <span className="agent-flow-editor__debug-workflow-title">
          <StatusIcon status={status} />
          <Typography.Text>
            {i18nText('agentFlow', 'auto.workflow')}
          </Typography.Text>
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
                  <DebugWorkflowNodeDetailContent
                    item={item}
                    onLoadArtifact={onLoadArtifact}
                  />
                </div>
              </DebugWorkflowNodeItem>
            );
          })}
        </div>
      ) : null}
    </section>
  );
}
