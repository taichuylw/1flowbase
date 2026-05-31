import {
  useEffect,
  useRef,
  useState,
  type CSSProperties,
  type MouseEvent as ReactMouseEvent,
  type ReactNode
} from 'react';
import { useTranslation } from 'react-i18next';

export type FloatingWindowRect = {
  left: number;
  top: number;
  width: number;
  height: number;
};

export type ApplicationLogsFloatingWindowProps = {
  active: boolean;
  children: ReactNode;
  className?: string;
  minHeight?: number;
  minWidth?: number;
  testId: string;
  title: string;
  initialRect: () => FloatingWindowRect;
  onActivate: () => void;
  rect?: FloatingWindowRect;
  onRectChange?: (rect: FloatingWindowRect) => void;
};

export const FLOATING_WINDOW_MARGIN = 8;
export const DEFAULT_MIN_WIDTH = 360;
export const DEFAULT_MIN_HEIGHT = 320;
const FLOATING_WINDOW_VISIBLE_DRAG_HANDLE_HEIGHT = 48;
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

function clampDraggedRect(
  rect: FloatingWindowRect,
  minWidth: number,
  minHeight: number
): FloatingWindowRect {
  const clampedRect = clampRect(rect, minWidth, minHeight);
  const viewport = getViewportSize();
  const maxTop = Math.max(
    FLOATING_WINDOW_MARGIN,
    viewport.height - FLOATING_WINDOW_VISIBLE_DRAG_HANDLE_HEIGHT
  );

  return {
    ...clampedRect,
    top: clamp(rect.top, FLOATING_WINDOW_MARGIN, maxTop)
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

function writeStoredWidth(testId: string, width: number) {
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

function isInteractiveElement(target: EventTarget | null) {
  if (!(target instanceof HTMLElement)) {
    return false;
  }

  return Boolean(
    target.closest(
      'button, a, input, textarea, select, [role="button"], [role="tab"], [data-no-window-drag="true"]'
    )
  );
}

function getDragHeader(target: EventTarget | null, container: HTMLElement) {
  if (!(target instanceof HTMLElement)) {
    return null;
  }

  const header = target.closest<HTMLElement>(
    '.agent-flow-editor__dock-panel-header, .application-run-detail__header'
  );

  if (!header || !container.contains(header)) {
    return null;
  }

  return header;
}

export function ApplicationLogsFloatingWindow({
  active,
  children,
  className,
  minHeight = DEFAULT_MIN_HEIGHT,
  minWidth = DEFAULT_MIN_WIDTH,
  testId,
  title,
  initialRect,
  onActivate,
  rect,
  onRectChange
}: ApplicationLogsFloatingWindowProps) {
  const { t } = useTranslation('applications');
  const [localRect, setLocalRect] = useState(() =>
    clampRect(applyStoredWidth(initialRect(), testId), minWidth, minHeight)
  );

  const currentRect = rect ?? localRect;
  const setRect = (
    newRect: FloatingWindowRect | ((curr: FloatingWindowRect) => FloatingWindowRect)
  ) => {
    if (typeof newRect === 'function') {
      const next = newRect(currentRect);
      if (onRectChange) {
        onRectChange(next);
      } else {
        setLocalRect(next);
      }
    } else {
      if (onRectChange) {
        onRectChange(newRect);
      } else {
        setLocalRect(newRect);
      }
    }
  };

  const cleanupInteractionRef = useRef<(() => void) | null>(null);

  useEffect(() => {
    function handleViewportResize() {
      setRect((current) => clampRect(current, minWidth, minHeight));
    }

    window.addEventListener('resize', handleViewportResize);
    return () => window.removeEventListener('resize', handleViewportResize);
  }, [minHeight, minWidth]);

  useEffect(() => {
    return () => {
      cleanupInteractionRef.current?.();
    };
  }, []);

  function startDrag(event: ReactMouseEvent<HTMLDivElement>) {
    onActivate();

    if (
      event.button !== 0 ||
      isInteractiveElement(event.target) ||
      !getDragHeader(event.target, event.currentTarget)
    ) {
      return;
    }

    event.preventDefault();

    const startX = event.clientX;
    const startY = event.clientY;
    const startRect = currentRect;
    const previousCursor = document.body.style.cursor;
    const previousUserSelect = document.body.style.userSelect;

    cleanupInteractionRef.current?.();
    document.body.style.cursor = 'move';
    document.body.style.userSelect = 'none';

    const handleMouseMove = (moveEvent: MouseEvent) => {
      setRect((current) =>
        clampDraggedRect(
          {
            ...current,
            left: startRect.left + moveEvent.clientX - startX,
            top: startRect.top + moveEvent.clientY - startY
          },
          minWidth,
          minHeight
        )
      );
    };

    const cleanup = () => {
      window.removeEventListener('mousemove', handleMouseMove);
      window.removeEventListener('mouseup', cleanup);
      document.body.style.cursor = previousCursor;
      document.body.style.userSelect = previousUserSelect;
      cleanupInteractionRef.current = null;
    };

    cleanupInteractionRef.current = cleanup;
    window.addEventListener('mousemove', handleMouseMove);
    window.addEventListener('mouseup', cleanup);
  }

  function startWidthResize(
    edge: 'left' | 'right',
    event: ReactMouseEvent<HTMLDivElement>
  ) {
    if (event.button !== 0) {
      return;
    }

    event.preventDefault();
    onActivate();

    const startX = event.clientX;
    const startLeft = currentRect.left;
    const startWidth = currentRect.width;
    const startRight = startLeft + startWidth;
    const previousCursor = document.body.style.cursor;
    const previousUserSelect = document.body.style.userSelect;
    let latestWidth = startWidth;

    cleanupInteractionRef.current?.();
    document.body.style.cursor = 'ew-resize';
    document.body.style.userSelect = 'none';

    const handleMouseMove = (moveEvent: MouseEvent) => {
      const deltaX = moveEvent.clientX - startX;
      const nextLeft =
        edge === 'left'
          ? clamp(
              startLeft + deltaX,
              FLOATING_WINDOW_MARGIN,
              startRight - minWidth
            )
          : startLeft;

      setRect((current) => {
        const nextRect = clampRect(
          edge === 'left'
            ? {
                ...current,
                left: nextLeft,
                width: startRight - nextLeft
              }
            : {
                ...current,
                width: startWidth + deltaX
              },
          minWidth,
          minHeight
        );

        latestWidth = nextRect.width;

        return nextRect;
      });
    };

    const cleanup = () => {
      window.removeEventListener('mousemove', handleMouseMove);
      window.removeEventListener('mouseup', cleanup);
      writeStoredWidth(testId, latestWidth);
      document.body.style.cursor = previousCursor;
      document.body.style.userSelect = previousUserSelect;
      cleanupInteractionRef.current = null;
    };

    cleanupInteractionRef.current = cleanup;
    window.addEventListener('mousemove', handleMouseMove);
    window.addEventListener('mouseup', cleanup);
  }

  function startHeightResize(event: ReactMouseEvent<HTMLDivElement>) {
    if (event.button !== 0) {
      return;
    }

    event.preventDefault();
    onActivate();

    const startY = event.clientY;
    const startHeight = currentRect.height;
    const previousCursor = document.body.style.cursor;
    const previousUserSelect = document.body.style.userSelect;

    cleanupInteractionRef.current?.();
    document.body.style.cursor = 'ns-resize';
    document.body.style.userSelect = 'none';

    const handleMouseMove = (moveEvent: MouseEvent) => {
      setRect((current) =>
        clampRect(
          {
            ...current,
            height: startHeight + moveEvent.clientY - startY
          },
          minWidth,
          minHeight
        )
      );
    };

    const cleanup = () => {
      window.removeEventListener('mousemove', handleMouseMove);
      window.removeEventListener('mouseup', cleanup);
      document.body.style.cursor = previousCursor;
      document.body.style.userSelect = previousUserSelect;
      cleanupInteractionRef.current = null;
    };

    cleanupInteractionRef.current = cleanup;
    window.addEventListener('mousemove', handleMouseMove);
    window.addEventListener('mouseup', cleanup);
  }

  const style: CSSProperties = {
    left: currentRect.left,
    top: currentRect.top,
    width: currentRect.width,
    height: currentRect.height,
    zIndex: active ? 1051 : 1050
  };

  return (
    <div
      aria-label={title}
      aria-modal="false"
      className={['application-logs-floating-window', className]
        .filter(Boolean)
        .join(' ')}
      data-testid={testId}
      role="dialog"
      style={style}
      onMouseDownCapture={startDrag}
    >
      <div className="application-logs-floating-window__body">{children}</div>
      <div
        aria-label={t('auto.adjust_width_from_right', { value1: title })}
        aria-orientation="vertical"
        className="application-logs-floating-window__resize application-logs-floating-window__resize--right"
        role="separator"
        onMouseDown={(event) => startWidthResize('right', event)}
      />
      <div
        aria-label={t('auto.adjust_width_from_left', { value1: title })}
        aria-orientation="vertical"
        className="application-logs-floating-window__resize application-logs-floating-window__resize--left"
        role="separator"
        onMouseDown={(event) => startWidthResize('left', event)}
      />
      <div
        aria-label={t('auto.adjust_height_downward', { value1: title })}
        aria-orientation="horizontal"
        className="application-logs-floating-window__resize application-logs-floating-window__resize--bottom"
        role="separator"
        onMouseDown={startHeightResize}
      />
    </div>
  );
}
