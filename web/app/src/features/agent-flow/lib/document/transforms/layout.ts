import type { FlowAuthoringDocument } from '@1flowbase/flow-schema';
import { getNodeById, getOutgoingEdges } from '../selectors';

const NODE_WIDTH = 196;
const NODE_HEIGHT = 96;

/**
 * 2D AABB 碰撞检测与 Y 轴最小平移向量 (MTV) 垂直消解算法
 */
function resolveAABBCollisions(
  positions: Record<string, { x: number; y: number }>,
  containerNodes: Array<{ id: string; containerId: string | null }>,
  nodeWidth = NODE_WIDTH,
  nodeHeight = NODE_HEIGHT,
  gapY = 40,
  maxIterations = 8
): Set<string> {
  const shiftedIds = new Set<string>();

  for (let iter = 0; iter < maxIterations; iter++) {
    let anyCollisionResolved = false;

    for (let i = 0; i < containerNodes.length; i++) {
      const idA = containerNodes[i].id;
      const posA = positions[idA];
      if (!posA) continue;

      for (let j = i + 1; j < containerNodes.length; j++) {
        const idB = containerNodes[j].id;
        const posB = positions[idB];
        if (!posB) continue;

        // 检测 2D 包围盒重叠量
        const dx = nodeWidth + 30 - Math.abs(posB.x - posA.x); // X 轴安全边距
        const dy = nodeHeight + gapY - Math.abs(posB.y - posA.y);

        if (dx > 0 && dy > 0) {
          // 发生空间碰撞，计算 Y 轴平移向量推开
          let shiftId = idB;
          let targetYSign = 1;

          if (posB.y < posA.y) {
            shiftId = idA;
            targetYSign = -1;
          } else if (posB.y === posA.y) {
            shiftId = idB;
            targetYSign = 1;
          }

          positions[shiftId] = {
            x: positions[shiftId].x,
            y: positions[shiftId].y + targetYSign * dy
          };
          shiftedIds.add(shiftId);
          anyCollisionResolved = true;
        }
      }
    }

    if (!anyCollisionResolved) {
      break;
    }
  }

  return shiftedIds;
}

/**
 * 局部受控力导向物理松弛算法 (Spring-Electrical Model)
 */
function applyLocalPhysicsRelaxation(
  positions: Record<string, { x: number; y: number }>,
  activeIds: Set<string>,
  containerNodes: Array<{ id: string }>,
  edges: FlowAuthoringDocument['graph']['edges'],
  iterations = 25
) {
  if (activeIds.size === 0) return;

  const kr = 30000;  // 库仑斥力系数
  const ka = 0.04;   // 弹簧引力系数
  const L0 = 280;    // 弹簧自然长度
  const damping = 0.7;

  // 仅对发生移动的活跃节点应用受控力学松弛
  const activeNodes = containerNodes.filter((n) => activeIds.has(n.id));

  const velocities: Record<string, { x: number; y: number }> = {};
  for (const node of activeNodes) {
    velocities[node.id] = { x: 0, y: 0 };
  }

  for (let iter = 0; iter < iterations; iter++) {
    const forces: Record<string, { x: number; y: number }> = {};
    for (const node of activeNodes) {
      forces[node.id] = { x: 0, y: 0 };
    }

    // 1. 计算库仑斥力 (来自容器内所有节点)
    for (let i = 0; i < activeNodes.length; i++) {
      const idA = activeNodes[i].id;
      const posA = positions[idA];
      if (!posA) continue;

      for (let j = 0; j < containerNodes.length; j++) {
        const idB = containerNodes[j].id;
        if (idA === idB) continue;

        const posB = positions[idB];
        if (!posB) continue;

        const dx = posA.x - posB.x;
        const dy = posA.y - posB.y;
        const distSq = dx * dx + dy * dy;
        const dist = Math.sqrt(distSq) || 1;

        if (dist < 380) {
          const forceMag = kr / (distSq + 100);
          forces[idA].x += (dx / dist) * forceMag;
          forces[idA].y += (dy / dist) * forceMag;
        }
      }
    }

    // 2. 计算弹簧引力 (沿边双向拉拽)
    for (const edge of edges) {
      const posSource = positions[edge.source];
      const posTarget = positions[edge.target];
      if (!posSource || !posTarget) continue;

      const dx = posTarget.x - posSource.x;
      const dy = posTarget.y - posSource.y;
      const dist = Math.sqrt(dx * dx + dy * dy) || 1;

      const forceMag = ka * (dist - L0);
      const fx = (dx / dist) * forceMag;
      const fy = (dy / dist) * forceMag;

      if (forces[edge.source]) {
        forces[edge.source].x += fx;
        forces[edge.source].y += fy;
      }
      if (forces[edge.target]) {
        forces[edge.target].x -= fx;
        forces[edge.target].y -= fy;
      }
    }

    // 3. 更新速度与位置
    for (const node of activeNodes) {
      const id = node.id;
      const vel = velocities[id];
      const force = forces[id];

      vel.x = (vel.x + force.x) * damping;
      vel.y = (vel.y + force.y) * damping;

      positions[id] = {
        x: positions[id].x + vel.x,
        y: positions[id].y + vel.y
      };
    }
  }
}

/**
 * 有向无环图 (DAG) 拓扑下流 BFS 推进与碰撞消解管道
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
  const shiftedNodeIds = new Set<string>();

  // 3. 执行阶段一：动态尺寸感知 BFS 拓扑推移
  const netGapX = Math.max(0, gapX - NODE_WIDTH); // 保证 280 默认步长完美兼容

  while (queue.length > 0) {
    const currentNodeId = queue.shift()!;
    visited.add(currentNodeId);

    const currentPos = positions[currentNodeId];
    if (!currentPos) continue;

    // 获取当前节点的所有出边 (下游)
    const outgoingEdges = getOutgoingEdges(document, currentNodeId);

    for (const edge of outgoingEdges) {
      const targetNodeId = edge.target;
      const targetNode = getNodeById(document, targetNodeId);
      if (!targetNode) continue;

      // 仅在同容器（containerId）内进行排布调整
      if (targetNode.containerId !== startNode.containerId) continue;

      const targetCurrentPos = positions[targetNodeId];
      if (!targetCurrentPos) continue;

      // 计算目标节点的最小 X 坐标 (父节点 X 坐标 + 节点宽度 + 净间隙)
      const targetMinX = currentPos.x + NODE_WIDTH + netGapX;

      if (targetCurrentPos.x < targetMinX) {
        positions[targetNodeId] = {
          x: targetMinX,
          y: targetCurrentPos.y
        };
        shiftedNodeIds.add(targetNodeId);

        if (!queue.includes(targetNodeId)) {
          queue.push(targetNodeId);
        }
      }
    }
  }

  // 获取当前容器内所有的节点
  const containerNodes = document.graph.nodes.filter(
    (node) => node.containerId === startNode.containerId
  );

  // 4. 执行阶段二：2D AABB 迭代消解垂直重叠
  let shiftedAABBNodeIds = new Set<string>();
  if (shiftedNodeIds.size > 0) {
    shiftedAABBNodeIds = resolveAABBCollisions(
      positions,
      containerNodes,
      NODE_WIDTH,
      NODE_HEIGHT,
      40,
      8
    );
  }

  // 5. 执行阶段三：局部活跃节点力导向物理松弛
  if (shiftedAABBNodeIds.size > 0) {
    const activeRelaxIds = new Set<string>([
      startNodeId,
      ...shiftedNodeIds,
      ...shiftedAABBNodeIds
    ]);
    const activeEdges = document.graph.edges.filter(
      (edge) => edge.containerId === startNode.containerId
    );

    applyLocalPhysicsRelaxation(
      positions,
      activeRelaxIds,
      containerNodes,
      activeEdges,
      25
    );
  }

  // 6. 对受影响节点的坐标进行舍入对齐，确保画布上没有极细微的小数偏移
  const finalPositions: Record<string, { x: number; y: number }> = {};
  for (const key of Object.keys(positions)) {
    finalPositions[key] = {
      x: Math.round(positions[key].x),
      y: Math.round(positions[key].y)
    };
  }

  // 7. 返回包含更新位置的新文档对象
  return {
    ...document,
    graph: {
      ...document.graph,
      nodes: document.graph.nodes.map((node) =>
        finalPositions[node.id]
          ? {
              ...node,
              position: finalPositions[node.id]
            }
          : node
      )
    }
  };
}
