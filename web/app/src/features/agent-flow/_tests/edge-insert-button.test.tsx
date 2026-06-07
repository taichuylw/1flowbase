import { render, screen, within } from '@testing-library/react';
import { describe, expect, test, vi } from 'vitest';

import { AppProviders } from '../../../app/AppProviders';
import { EdgeInsertButton } from '../components/canvas/EdgeInsertButton';

describe('EdgeInsertButton', () => {
  test('uses the shared connector add icon slot', () => {
    render(
      <AppProviders>
        <EdgeInsertButton
          open={false}
          options={[]}
          onOpenChange={vi.fn()}
          onPickNode={vi.fn()}
        />
      </AppProviders>
    );

    const button = screen.getByRole('button', {
      name: '在此连线上新增节点'
    });

    expect(
      button.querySelector('.agent-flow-connector-add-icon')
    ).toBeInTheDocument();
    expect(within(button).queryByText('+')).not.toBeInTheDocument();
  });
});
