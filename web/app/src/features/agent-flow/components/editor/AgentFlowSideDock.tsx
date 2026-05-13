import type {
  CSSProperties,
  ReactNode,
  MouseEvent as ReactMouseEvent
} from 'react';

interface AgentFlowSideDockProps {
  children: ReactNode;
  className?: string;
  'data-testid'?: string;
  isResizing?: boolean;
  resizeLabel: string;
  style?: CSSProperties;
  width: number;
  onResizeStart: (event: ReactMouseEvent<HTMLDivElement>) => void;
}

export function AgentFlowSideDock({
  children,
  className,
  'data-testid': dataTestId,
  isResizing = false,
  resizeLabel,
  style,
  width,
  onResizeStart
}: AgentFlowSideDockProps) {
  return (
    <div
      className={className ?? 'agent-flow-editor__side-dock'}
      data-resizing={isResizing ? 'true' : 'false'}
      data-testid={dataTestId}
      style={{ ...style, width: `${width}px` }}
    >
      <div
        aria-label={resizeLabel}
        aria-orientation="vertical"
        className="agent-flow-editor__side-dock-resize-handle"
        onMouseDown={onResizeStart}
        role="separator"
      />
      {children}
    </div>
  );
}
