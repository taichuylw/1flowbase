import type { HTMLAttributes, ReactNode } from 'react';

import './scrollable-surface.css';

export interface ScrollableSurfaceProps extends HTMLAttributes<HTMLElement> {
  children: ReactNode;
}

export function ScrollableSurface({
  children,
  className,
  ...surfaceProps
}: ScrollableSurfaceProps) {
  const surfaceClassName = ['scrollable-surface', className]
    .filter(Boolean)
    .join(' ');

  return (
    <section className={surfaceClassName} {...surfaceProps}>
      {children}
    </section>
  );
}
