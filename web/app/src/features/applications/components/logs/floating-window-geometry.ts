export type FloatingWindowRect = {
  left: number;
  top: number;
  width: number;
  height: number;
};

export const FLOATING_WINDOW_MARGIN = 8;
export const DEFAULT_MIN_WIDTH = 360;
export const DEFAULT_MIN_HEIGHT = 320;

const FLOATING_WINDOW_WIDTH_STORAGE_PREFIX =
  'applicationLogsFloatingWindowWidth';

export function clamp(value: number, min: number, max: number) {
  return Math.min(max, Math.max(min, value));
}

export function getViewportSize() {
  if (typeof window === 'undefined') {
    return { width: 1280, height: 720 };
  }

  return {
    width: window.innerWidth,
    height: window.innerHeight
  };
}

export function clampRect(
  rect: FloatingWindowRect,
  minWidth: number,
  minHeight: number
): FloatingWindowRect {
  const viewport = getViewportSize();
  const maxWidth = Math.max(
    minWidth,
    viewport.width - FLOATING_WINDOW_MARGIN * 2
  );
  const maxHeight = Math.max(
    minHeight,
    viewport.height - FLOATING_WINDOW_MARGIN * 2
  );
  const width = clamp(rect.width, minWidth, maxWidth);
  const height = clamp(rect.height, minHeight, maxHeight);
  const maxLeft = Math.max(
    FLOATING_WINDOW_MARGIN,
    viewport.width - width - FLOATING_WINDOW_MARGIN
  );
  const maxTop = Math.max(
    FLOATING_WINDOW_MARGIN,
    viewport.height - height - FLOATING_WINDOW_MARGIN
  );

  return {
    left: clamp(rect.left, FLOATING_WINDOW_MARGIN, maxLeft),
    top: clamp(rect.top, FLOATING_WINDOW_MARGIN, maxTop),
    width,
    height
  };
}

function getWidthStorageKey(testId: string) {
  return `${FLOATING_WINDOW_WIDTH_STORAGE_PREFIX}:${testId}`;
}

function readStoredWidth(testId: string) {
  if (typeof window === 'undefined') {
    return null;
  }

  const rawWidth = window.localStorage.getItem(getWidthStorageKey(testId));
  const width = rawWidth ? Number(rawWidth) : Number.NaN;

  return Number.isFinite(width) && width > 0 ? width : null;
}

export function writeStoredWidth(testId: string, width: number) {
  if (typeof window === 'undefined') {
    return;
  }

  window.localStorage.setItem(
    getWidthStorageKey(testId),
    String(Math.round(width))
  );
}

export function applyStoredWidth(
  rect: FloatingWindowRect,
  testId: string
): FloatingWindowRect {
  const storedWidth = readStoredWidth(testId);

  if (!storedWidth) {
    return rect;
  }

  const right = rect.left + rect.width;

  return {
    ...rect,
    left: right - storedWidth,
    width: storedWidth
  };
}
