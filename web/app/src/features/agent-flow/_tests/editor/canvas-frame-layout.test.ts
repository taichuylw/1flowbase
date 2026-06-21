import { describe, expect, test } from 'vitest';

import { deriveCanvasFrameLayout } from '../../components/editor/canvas-frame/derived-layout';

const baseLayoutInput = {
  bodyHeight: 720,
  bodyWidth: 1200,
  conversationLogMessageOpen: false,
  conversationLogWidth: 560,
  conversationVariablesDockWidth: 520,
  conversationVariablesOpen: false,
  debugConsoleOpen: false,
  debugConsoleWidth: 420,
  environmentVariablesDockWidth: 520,
  environmentVariablesOpen: false,
  historyDockWidth: 460,
  historyOpen: false,
  nodeDetailWidth: 360,
  selectedNodeId: null,
  systemVariablesDockWidth: 420,
  systemVariablesOpen: false,
  variableCacheHeight: 330,
  variableCacheSidebarWidth: 270
};

describe('deriveCanvasFrameLayout', () => {
  test('centers the variable cache trigger in the visible canvas area', () => {
    const layout = deriveCanvasFrameLayout({
      ...baseLayoutInput,
      debugConsoleOpen: true,
      selectedNodeId: 'node-1'
    });

    expect(layout.variableCacheCenterLeft).toBe(198);
  });
});
