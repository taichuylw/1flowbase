import type { MouseEvent as ReactMouseEvent } from 'react';

interface StartCanvasFrameResizeOptions {
  cursor: 'col-resize' | 'row-resize';
  onMove: (event: MouseEvent) => void;
  onStart: () => void;
  onStop: () => void;
}

export function startCanvasFrameResize(
  event: ReactMouseEvent<HTMLDivElement>,
  { cursor, onMove, onStart, onStop }: StartCanvasFrameResizeOptions
): () => void {
  event.preventDefault();

  const previousCursor = document.body.style.cursor;
  const previousUserSelect = document.body.style.userSelect;

  onStart();
  document.body.style.cursor = cursor;
  document.body.style.userSelect = 'none';

  const cleanup = () => {
    window.removeEventListener('mousemove', onMove);
    window.removeEventListener('mouseup', cleanup);
    document.body.style.cursor = previousCursor;
    document.body.style.userSelect = previousUserSelect;
    onStop();
  };

  window.addEventListener('mousemove', onMove);
  window.addEventListener('mouseup', cleanup);

  return cleanup;
}
