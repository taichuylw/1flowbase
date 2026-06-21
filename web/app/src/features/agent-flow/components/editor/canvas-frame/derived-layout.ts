import {
  clampNodeDetailWidth,
  NODE_DETAIL_DEFAULT_WIDTH,
  NODE_DETAIL_MIN_CANVAS_WIDTH,
  getNodeDetailLayout
} from '../../../lib/detail-panel-width';
import {
  CONVERSATION_LOG_MIN_WIDTH,
  DEBUG_CONSOLE_GAP,
  DEBUG_CONSOLE_MIN_WIDTH,
  HISTORY_DOCK_MIN_WIDTH,
  VARIABLE_CACHE_BOTTOM_GAP,
  VARIABLE_CACHE_DEFAULT_HEIGHT,
  VARIABLE_CACHE_MAX_TOP_GAP,
  VARIABLE_CACHE_MIN_DETAIL_WIDTH,
  VARIABLE_CACHE_MIN_HEIGHT,
  VARIABLE_CACHE_MIN_SIDEBAR_WIDTH,
  VARIABLES_DOCK_MIN_WIDTH
} from './layout';

interface CanvasFrameLayoutInput {
  bodyHeight: number;
  bodyWidth: number;
  conversationLogMessageOpen: boolean;
  conversationLogWidth: number;
  conversationVariablesDockWidth: number;
  conversationVariablesOpen: boolean;
  debugConsoleOpen: boolean;
  debugConsoleWidth: number;
  environmentVariablesDockWidth: number;
  environmentVariablesOpen: boolean;
  historyDockWidth: number;
  historyOpen: boolean;
  nodeDetailWidth: number;
  selectedNodeId: string | null;
  systemVariablesDockWidth: number;
  systemVariablesOpen: boolean;
  variableCacheHeight: number;
  variableCacheSidebarWidth: number;
}

export function deriveCanvasFrameLayout(input: CanvasFrameLayoutInput) {
  const canvasFrameWidth =
    input.bodyWidth || NODE_DETAIL_DEFAULT_WIDTH + NODE_DETAIL_MIN_CANVAS_WIDTH;
  const maxDebugConsoleWidth = Math.max(
    canvasFrameWidth -
      (input.selectedNodeId ? input.nodeDetailWidth : 0) -
      NODE_DETAIL_MIN_CANVAS_WIDTH,
    DEBUG_CONSOLE_MIN_WIDTH
  );
  const boundedDebugConsoleWidth = Math.min(
    Math.max(input.debugConsoleWidth, DEBUG_CONSOLE_MIN_WIDTH),
    maxDebugConsoleWidth
  );
  const conversationLogOpen =
    input.debugConsoleOpen && input.conversationLogMessageOpen;
  const maxConversationLogWidth = Math.max(
    canvasFrameWidth -
      boundedDebugConsoleWidth -
      DEBUG_CONSOLE_GAP -
      (input.selectedNodeId ? input.nodeDetailWidth + DEBUG_CONSOLE_GAP : 0) -
      NODE_DETAIL_MIN_CANVAS_WIDTH,
    CONVERSATION_LOG_MIN_WIDTH
  );
  const boundedConversationLogWidth = Math.min(
    Math.max(input.conversationLogWidth, CONVERSATION_LOG_MIN_WIDTH),
    maxConversationLogWidth
  );
  const variablesDockOpen =
    input.systemVariablesOpen ||
    input.environmentVariablesOpen ||
    input.conversationVariablesOpen;
  const maxVariablesDockWidth = Math.max(
    canvasFrameWidth -
      (input.selectedNodeId ? input.nodeDetailWidth : 0) -
      NODE_DETAIL_MIN_CANVAS_WIDTH,
    VARIABLES_DOCK_MIN_WIDTH
  );
  const rawVariablesDockWidth = input.conversationVariablesOpen
    ? input.conversationVariablesDockWidth
    : input.environmentVariablesOpen
      ? input.environmentVariablesDockWidth
      : input.systemVariablesDockWidth;
  const boundedVariablesDockWidth = Math.min(
    Math.max(rawVariablesDockWidth, VARIABLES_DOCK_MIN_WIDTH),
    maxVariablesDockWidth
  );
  const maxHistoryDockWidth = Math.max(
    canvasFrameWidth -
      (input.selectedNodeId ? input.nodeDetailWidth : 0) -
      NODE_DETAIL_MIN_CANVAS_WIDTH,
    HISTORY_DOCK_MIN_WIDTH
  );
  const boundedHistoryDockWidth = Math.min(
    Math.max(input.historyDockWidth, HISTORY_DOCK_MIN_WIDTH),
    maxHistoryDockWidth
  );
  const sideDockOccupiedWidth = input.debugConsoleOpen
    ? boundedDebugConsoleWidth +
      DEBUG_CONSOLE_GAP +
      (conversationLogOpen
        ? boundedConversationLogWidth + DEBUG_CONSOLE_GAP
        : 0)
    : variablesDockOpen
      ? boundedVariablesDockWidth + DEBUG_CONSOLE_GAP
      : input.historyOpen
        ? boundedHistoryDockWidth + DEBUG_CONSOLE_GAP
        : 0;
  const detailContainerWidth = canvasFrameWidth - sideDockOccupiedWidth;
  const boundedNodeDetailWidth = clampNodeDetailWidth(
    input.nodeDetailWidth,
    detailContainerWidth
  );
  const nodeDetailLayout = getNodeDetailLayout(boundedNodeDetailWidth);
  const nodeDetailOccupiedWidth = input.selectedNodeId
    ? boundedNodeDetailWidth + DEBUG_CONSOLE_GAP
    : 0;
  const variableCacheRightOffset =
    16 + nodeDetailOccupiedWidth + sideDockOccupiedWidth;
  const variableCacheCenterLeft =
    (canvasFrameWidth - nodeDetailOccupiedWidth - sideDockOccupiedWidth) / 2;
  const variableCacheMaxHeight = Math.max(
    VARIABLE_CACHE_MIN_HEIGHT,
    (input.bodyHeight ||
      VARIABLE_CACHE_DEFAULT_HEIGHT + VARIABLE_CACHE_MAX_TOP_GAP) -
      VARIABLE_CACHE_MAX_TOP_GAP -
      VARIABLE_CACHE_BOTTOM_GAP
  );
  const boundedVariableCacheHeight = Math.min(
    Math.max(input.variableCacheHeight, VARIABLE_CACHE_MIN_HEIGHT),
    variableCacheMaxHeight
  );
  const variableCachePanelInnerWidth = Math.max(
    canvasFrameWidth - variableCacheRightOffset - 32,
    VARIABLE_CACHE_MIN_DETAIL_WIDTH + VARIABLE_CACHE_MIN_SIDEBAR_WIDTH
  );
  const variableCacheSidebarMaxWidth = Math.max(
    variableCachePanelInnerWidth - VARIABLE_CACHE_MIN_DETAIL_WIDTH,
    VARIABLE_CACHE_MIN_SIDEBAR_WIDTH
  );
  const boundedVariableCacheSidebarWidth = Math.max(
    VARIABLE_CACHE_MIN_SIDEBAR_WIDTH,
    Math.min(input.variableCacheSidebarWidth, variableCacheSidebarMaxWidth)
  );

  return {
    boundedConversationLogWidth,
    boundedDebugConsoleWidth,
    boundedHistoryDockWidth,
    boundedNodeDetailWidth,
    boundedVariableCacheHeight,
    boundedVariableCacheSidebarWidth,
    boundedVariablesDockWidth,
    canvasFrameWidth,
    conversationLogOpen,
    detailContainerWidth,
    nodeDetailLayout,
    sideDockOccupiedWidth,
    variableCacheCenterLeft,
    variableCacheMaxHeight,
    variableCacheRightOffset,
    variableCacheSidebarMaxWidth,
    variablesDockOpen
  };
}
