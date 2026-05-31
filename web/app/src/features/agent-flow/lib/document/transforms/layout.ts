import type { FlowAuthoringDocument } from '@1flowbase/flow-schema';
import { getNodeById, getOutgoingEdges } from '../selectors';

/**
 * 有向无环图 (DAG) 拓扑下流 BFS 推进算法 (Topological BFS Shifting)
 * 仅沿有向图的下游依赖链（Outgoers）以 BFS 方式进行遍历，
 * 若发现子节点与父节点之间的横向距离小于设定的 gapX，则顺势向右推移并递归通知其下游子节点。
 */
export function shiftDownstreamNodesBFS(
  document: FlowAuthoringDocument,
  startNodeId: string,
  gapX = 280
): FlowAuthoringDocument {
  const startNode = getNodeById(document, startNodeId);
  if (!startNode) {
    return document;
  }

  // 1. 初始化更新后的坐标映射，预填充所有节点当前坐标
  const positions: Record<string, { x: number; y: number }> = {};
  for (const node of document.graph.nodes) {
    positions[node.id] = { ...node.position };
  }

  // 2. 初始化队列和已访问集合
  const queue: string[] = [startNodeId];
  const visited = new Set<string>();

  // 3. 执行 BFS 传播
  while (queue.length > 0) {
    const currentNodeId = queue.shift()!;
    // 如果已经处理过（为防止循环依赖，但有向无环图中本不会发生，保留作为防御性保护），可继续处理
    visited.add(currentNodeId);

    const currentPos = positions[currentNodeId];
    if (!currentPos) continue;

    // 获取该节点的所有出边 (即直接下游)
    const outgoingEdges = getOutgoingEdges(document, currentNodeId);

    for (const edge of outgoingEdges) {
      const targetNodeId = edge.target;
      const targetNode = getNodeById(document, targetNodeId);
      if (!targetNode) continue;

      // 仅在同容器（containerId）内进行排布调整
      if (targetNode.containerId !== startNode.containerId) continue;

      const targetCurrentPos = positions[targetNodeId];
      if (!targetCurrentPos) continue;

      // 计算目标节点的最小 X 坐标 (父节点 X 坐标 + 间隔)
      const targetMinX = currentPos.x + gapX;

      // 若当前目标节点的 X 坐标小于最小安全距离，则进行推开
      if (targetCurrentPos.x < targetMinX) {
        positions[targetNodeId] = {
          x: targetMinX,
          y: targetCurrentPos.y
        };

        // 由于目标节点发生了位置变化，重新入队，以便继续影响它的下游节点
        if (!queue.includes(targetNodeId)) {
          queue.push(targetNodeId);
        }
      }
    }
  }

  // 4. 返回包含更新位置的新文档对象
  return {
    ...document,
    graph: {
      ...document.graph,
      nodes: document.graph.nodes.map((node) =>
        positions[node.id]
          ? {
              ...node,
              position: positions[node.id]
            }
          : node
      )
    }
  };
}
