const MIN_PICKER_HEIGHT = 120;
const CANVAS_BOTTOM_GAP = 10;

export function calculateNodePickerMaxHeight(
  canvasBottom: number,
  anchorY: number
) {
  return Math.max(
    MIN_PICKER_HEIGHT,
    Math.floor(canvasBottom - anchorY - CANVAS_BOTTOM_GAP)
  );
}
