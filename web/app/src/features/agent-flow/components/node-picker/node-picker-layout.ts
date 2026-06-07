const MIN_PICKER_HEIGHT = 120;
const CANVAS_BOTTOM_GAP = 10;

export interface NodePickerMaxHeightInput {
  canvasBottom: number;
  anchorY: number;
  bottomBoundary?: number;
}

export function calculateNodePickerMaxHeight({
  canvasBottom,
  anchorY,
  bottomBoundary
}: NodePickerMaxHeightInput) {
  const effectiveBottom = Math.min(bottomBoundary ?? canvasBottom, canvasBottom);

  return Math.max(
    MIN_PICKER_HEIGHT,
    Math.floor(effectiveBottom - anchorY - CANVAS_BOTTOM_GAP)
  );
}
