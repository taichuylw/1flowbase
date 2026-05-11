import { useQuery } from '@tanstack/react-query';
import { Empty, Result } from 'antd';

import type { CanvasNodeSchema } from '../../../../../shared/schema-ui/contracts/canvas-node-schema';
import { SchemaRenderer } from '../../../../../shared/schema-ui/runtime/SchemaRenderer';
import type { SchemaAdapter } from '../../../../../shared/schema-ui/registry/create-renderer-registry';

import {
  applicationRunNodeLastRunQueryKey,
  fetchApplicationRunNodeLastRun,
  fetchNodeLastRun,
  nodeLastRunQueryKey
} from '../../../api/runtime';
import { agentFlowRendererRegistry } from '../../../schema/agent-flow-renderer-registry';
import { NodeRunIOCard } from '../last-run/NodeRunIOCard';
import { NodeRunMetadataCard } from '../last-run/NodeRunMetadataCard';
import { NodeRunSummaryCard } from '../last-run/NodeRunSummaryCard';

function isNodeLastRun(value: unknown): value is NonNullable<
  Awaited<ReturnType<typeof fetchNodeLastRun>>
> {
  if (!value || typeof value !== 'object') {
    return false;
  }

  const candidate = value as Record<string, unknown>;

  return Boolean(
    candidate.flow_run &&
      typeof candidate.flow_run === 'object' &&
      candidate.node_run &&
      typeof candidate.node_run === 'object' &&
      Array.isArray(candidate.events)
  );
}

export function NodeLastRunTab({
  activeRunId,
  applicationId,
  nodeId,
  onResolveRunScope,
  schema,
  adapter
}: {
  activeRunId?: string | null;
  applicationId?: string;
  nodeId?: string;
  onResolveRunScope?: ((runId: string | null) => void) | undefined;
  schema?: CanvasNodeSchema;
  adapter?: SchemaAdapter;
}) {
  const lastRunQuery = useQuery({
    queryKey: activeRunId
      ? applicationRunNodeLastRunQueryKey(
          applicationId ?? 'unknown',
          activeRunId,
          nodeId ?? 'unknown'
        )
      : nodeLastRunQueryKey(applicationId ?? 'unknown', nodeId ?? 'unknown'),
    queryFn: async () => {
      if (activeRunId) {
        return fetchApplicationRunNodeLastRun(
          applicationId!,
          activeRunId,
          nodeId!
        );
      }

      const lastRun = await fetchNodeLastRun(applicationId!, nodeId!);
      if (lastRun?.flow_run?.id) {
        onResolveRunScope?.(lastRun.flow_run.id);
      }
      return lastRun;
    },
    enabled: Boolean(applicationId && nodeId)
  });
  if (lastRunQuery.isPending) {
    return <Result status="info" title="正在加载上次运行" />;
  }

  if (lastRunQuery.isError) {
    return <Result status="error" title="上次运行加载失败" />;
  }

  if (!lastRunQuery.data) {
    return (
      <Empty
        description={activeRunId ? '当前运行没有该节点记录' : '当前节点还没有运行记录'}
        image={Empty.PRESENTED_IMAGE_SIMPLE}
      />
    );
  }

  if (!isNodeLastRun(lastRunQuery.data)) {
    return <Result status="warning" title="上次运行数据异常" />;
  }

  const lastRun = lastRunQuery.data;

  if (!schema || !adapter) {
    return (
      <div className="agent-flow-node-detail__last-run">
        <NodeRunSummaryCard lastRun={lastRun} />
        <NodeRunIOCard lastRun={lastRun} />
        <NodeRunMetadataCard lastRun={lastRun} />
      </div>
    );
  }

  const runtimeAdapter: SchemaAdapter = {
    ...adapter,
    getDerived(key: string) {
      if (key === 'lastRun') {
        return lastRun;
      }

      return adapter.getDerived(key);
    }
  };

  return (
    <div className="agent-flow-node-detail__last-run">
      <SchemaRenderer
        adapter={runtimeAdapter}
        blocks={schema.detail.tabs.lastRun.blocks}
        registry={agentFlowRendererRegistry}
      />
    </div>
  );
}
