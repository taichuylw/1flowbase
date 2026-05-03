import { render, screen } from '@testing-library/react';
import { describe, expect, test } from 'vitest';

import { LoadingState } from '../LoadingState';

describe('LoadingState', () => {
  test('uses the shared 1flowbase loading label', () => {
    render(<LoadingState />);

    expect(screen.getByRole('status', { name: '1flowbase' })).toBeInTheDocument();
    expect(screen.getByText('1flowbase')).toBeInTheDocument();
  });

  test('supports fullscreen and compact layout variants', () => {
    render(<LoadingState fullscreen compact />);

    expect(screen.getByRole('status', { name: '1flowbase' })).toHaveClass(
      'loading-state',
      'loading-state--fullscreen',
      'loading-state--compact'
    );
  });
});
