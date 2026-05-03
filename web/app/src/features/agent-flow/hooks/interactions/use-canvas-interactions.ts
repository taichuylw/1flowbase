import type { NodeChange } from '@xyflow/react';
import { useCallback, useMemo, useState } from 'react';

import { moveNodes } from '../../lib/document/transforms/node';
import { setViewport } from '../../lib/document/transforms/viewport';
import { useAgentFlowEditorStore } from '../../store/editor/provider';

type NodePositions = Record<string, { x: number; y: number }>;

function getPositionChanges(changes: NodeChange[]) {
  return changes.filter(
    (
      change
    ): change is NodeChange & {
      id: string;
      dragging?: boolean;
      position: { x: number; y: number };
    } =>
      change.type === 'position' &&
      'id' in change &&
      'position' in change &&
      Boolean(change.position)
  );
}

function toPositions(
  changes: Array<{
    id: string;
    position: { x: number; y: number };
  }>
) {
  return Object.fromEntries(
    changes.map((change) => [change.id, change.position])
  );
}

function removePositions(current: NodePositions, nodeIds: string[]) {
  if (nodeIds.length === 0) {
    return current;
  }

  let changed = false;
  const nextPositions = { ...current };

  for (const nodeId of nodeIds) {
    if (nodeId in nextPositions) {
      delete nextPositions[nodeId];
      changed = true;
    }
  }

  return changed ? nextPositions : current;
}

export function useCanvasInteractions() {
  const [transientNodePositions, setTransientNodePositions] =
    useState<NodePositions>({});
  const setWorkingDocument = useAgentFlowEditorStore(
    (state) => state.setWorkingDocument
  );

  const onNodesChange = useCallback(
    (changes: NodeChange[]) => {
      const positionChanges = getPositionChanges(changes);

      if (positionChanges.length === 0) {
        return;
      }

      const movingChanges = positionChanges.filter(
        (change) => change.dragging === true
      );
      const committedChanges = positionChanges.filter(
        (change) => change.dragging !== true
      );

      if (movingChanges.length > 0) {
        const movingPositions = toPositions(movingChanges);

        setTransientNodePositions((currentPositions) => ({
          ...currentPositions,
          ...movingPositions
        }));
      }

      if (committedChanges.length === 0) {
        return;
      }

      const committedPositions = toPositions(committedChanges);

      setTransientNodePositions((currentPositions) =>
        removePositions(currentPositions, Object.keys(committedPositions))
      );
      setWorkingDocument((currentDocument) =>
        moveNodes(currentDocument, committedPositions)
      );
    },
    [setWorkingDocument]
  );

  const commitViewportChange = useCallback(
    (viewport: { x: number; y: number; zoom: number }) => {
      setWorkingDocument((currentDocument) =>
        setViewport(currentDocument, {
          x: viewport.x,
          y: viewport.y,
          zoom: viewport.zoom
        })
      );
    },
    [setWorkingDocument]
  );

  return useMemo(
    () => ({
      transientNodePositions,
      onNodesChange,
      commitViewportChange
    }),
    [commitViewportChange, onNodesChange, transientNodePositions]
  );
}
