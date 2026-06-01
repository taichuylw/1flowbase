import { describe, expect, test } from 'vitest';
import { createDefaultAgentFlowDocument } from '@1flowbase/flow-schema';
import { shiftDownstreamNodesBFS } from '../layout';

describe('Topological BFS Downstream Shifting', () => {
  test('shifts downstream nodes when parent moves forward', () => {
    // 1. Initial document: start -> llm -> answer
    // Default positions might be arbitrary. Let's explicitly set positions for test predictability:
    // start (x: 100, y: 100) -> llm (x: 380, y: 100) -> answer (x: 660, y: 100)
    const document = createDefaultAgentFlowDocument({ flowId: 'flow-1' });
    document.graph.nodes = document.graph.nodes.map((node) => {
      if (node.id === 'node-start') {
        return { ...node, position: { x: 100, y: 100 } };
      }
      if (node.id === 'node-llm') {
        return { ...node, position: { x: 380, y: 100 } };
      }
      if (node.id === 'node-answer') {
        return { ...node, position: { x: 660, y: 100 } };
      }
      return node;
    });

    // Move start node forward to x: 200.
    // The gap threshold is 280.
    // llm's minimum x = 200 + 280 = 480.
    // Since llm's current x is 380 < 480, llm should shift to x: 480.
    // answer's minimum x = 480 + 280 = 760.
    // Since answer's current x is 660 < 760, answer should shift to x: 760.
    document.graph.nodes = document.graph.nodes.map((node) =>
      node.id === 'node-start'
        ? { ...node, position: { x: 200, y: 100 } }
        : node
    );

    const updatedDoc = shiftDownstreamNodesBFS(document, 'node-start', 280);

    const startNode = updatedDoc.graph.nodes.find((n) => n.id === 'node-start');
    const llmNode = updatedDoc.graph.nodes.find((n) => n.id === 'node-llm');
    const answerNode = updatedDoc.graph.nodes.find(
      (n) => n.id === 'node-answer'
    );

    expect(startNode?.position.x).toBe(200);
    expect(llmNode?.position.x).toBe(480);
    expect(answerNode?.position.x).toBe(760);
  });

  test('does not shift parent nodes when downstream node is shifted', () => {
    const document = createDefaultAgentFlowDocument({ flowId: 'flow-1' });
    document.graph.nodes = document.graph.nodes.map((node) => {
      if (node.id === 'node-start') {
        return { ...node, position: { x: 100, y: 100 } };
      }
      if (node.id === 'node-llm') {
        return { ...node, position: { x: 380, y: 100 } };
      }
      if (node.id === 'node-answer') {
        return { ...node, position: { x: 660, y: 100 } };
      }
      return node;
    });

    // Move llm node forward to x: 500.
    // answer's minimum x = 500 + 280 = 780.
    // Since answer's current x is 660 < 780, answer shifts to 780.
    // start is a parent of llm, it should NOT shift. It stays at 100.
    document.graph.nodes = document.graph.nodes.map((node) =>
      node.id === 'node-llm' ? { ...node, position: { x: 500, y: 100 } } : node
    );

    const updatedDoc = shiftDownstreamNodesBFS(document, 'node-llm', 280);

    const startNode = updatedDoc.graph.nodes.find((n) => n.id === 'node-start');
    const llmNode = updatedDoc.graph.nodes.find((n) => n.id === 'node-llm');
    const answerNode = updatedDoc.graph.nodes.find(
      (n) => n.id === 'node-answer'
    );

    expect(startNode?.position.x).toBe(100);
    expect(llmNode?.position.x).toBe(500);
    expect(answerNode?.position.x).toBe(780);
  });

  test('does not shift parallel branches that are not connected downstream', () => {
    const document = createDefaultAgentFlowDocument({ flowId: 'flow-1' });

    // Add a parallel node 'node-parallel' with position (x: 400, y: 300)
    // No edges connect 'node-llm' to 'node-parallel'
    document.graph.nodes.push({
      id: 'node-parallel',
      type: 'llm',
      alias: 'Parallel',
      containerId: null,
      position: { x: 400, y: 300 },
      configVersion: 1,
      config: {},
      bindings: {},
      outputs: []
    });

    // Set other nodes
    document.graph.nodes = document.graph.nodes.map((node) => {
      if (node.id === 'node-start') {
        return { ...node, position: { x: 100, y: 100 } };
      }
      if (node.id === 'node-llm') {
        return { ...node, position: { x: 380, y: 100 } };
      }
      if (node.id === 'node-answer') {
        return { ...node, position: { x: 660, y: 100 } };
      }
      return node;
    });

    // Shift start node to x: 200, which shifts llm to 480 and answer to 760.
    // 'node-parallel' is NOT a downstream outgoer of start or llm, so it should stay at 400.
    document.graph.nodes = document.graph.nodes.map((node) =>
      node.id === 'node-start'
        ? { ...node, position: { x: 200, y: 100 } }
        : node
    );

    const updatedDoc = shiftDownstreamNodesBFS(document, 'node-start', 280);

    const parallelNode = updatedDoc.graph.nodes.find(
      (n) => n.id === 'node-parallel'
    );
    expect(parallelNode?.position.x).toBe(400);
  });

  test('resolves vertical overlapping of sibling nodes using AABB MTV', () => {
    const document = createDefaultAgentFlowDocument({ flowId: 'flow-1' });

    // Connect node-start to node-llm and a new sibling node-sibling
    document.graph.nodes.push({
      id: 'node-sibling',
      type: 'llm',
      alias: 'Sibling',
      containerId: null,
      position: { x: 380, y: 110 }, // Overlaps in Y with node-llm (x: 380, y: 100)
      configVersion: 1,
      config: {},
      bindings: {},
      outputs: []
    });

    document.graph.edges.push({
      id: 'edge-start-sibling',
      source: 'node-start',
      target: 'node-sibling',
      sourceHandle: null,
      targetHandle: null,
      containerId: null,
      points: []
    });

    // Explicitly set positions
    document.graph.nodes = document.graph.nodes.map((node) => {
      if (node.id === 'node-start') {
        return { ...node, position: { x: 100, y: 100 } };
      }
      if (node.id === 'node-llm') {
        return { ...node, position: { x: 380, y: 100 } };
      }
      return node;
    });

    // Move start node forward to trigger shifting.
    // This will trigger BFS which shifts both node-llm and node-sibling.
    // They are both active, and they overlap vertically at x: 480.
    document.graph.nodes = document.graph.nodes.map((node) =>
      node.id === 'node-start'
        ? { ...node, position: { x: 200, y: 100 } }
        : node
    );

    const updatedDoc = shiftDownstreamNodesBFS(document, 'node-start', 280);

    const llmNode = updatedDoc.graph.nodes.find((n) => n.id === 'node-llm');
    const siblingNode = updatedDoc.graph.nodes.find(
      (n) => n.id === 'node-sibling'
    );

    if (!llmNode || !siblingNode) {
      throw new Error('expected shifted downstream nodes to exist');
    }

    // Verify they are separated vertically: difference in Y must be at least NODE_HEIGHT (96) + gapY (40) = 136px
    const yDiff = Math.abs(llmNode.position.y - siblingNode.position.y);
    expect(yDiff).toBeGreaterThanOrEqual(136);
  });
});
