import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, test, vi } from 'vitest';

import { ApplicationEnvironmentVariablesPanel } from '../../components/editor/ApplicationEnvironmentVariablesPanel';

describe('ApplicationEnvironmentVariablesPanel', () => {
  test('switches the value editor when the variable type changes', async () => {
    render(
      <ApplicationEnvironmentVariablesPanel
        variables={[]}
        onClose={vi.fn()}
        onSave={vi.fn()}
      />
    );

    fireEvent.click(screen.getByRole('button', { name: /添加环境变量/ }));

    expect(screen.getByPlaceholderText('请输入变量值')).toBeInTheDocument();

    fireEvent.mouseDown(screen.getByRole('combobox', { name: '类型' }));
    fireEvent.click(await screen.findByTitle('number'));

    expect(screen.getByRole('spinbutton')).toBeInTheDocument();

    fireEvent.mouseDown(screen.getByRole('combobox', { name: '类型' }));
    fireEvent.click(await screen.findByTitle('boolean'));

    expect(screen.getByText('true')).toBeInTheDocument();
    expect(screen.getByText('false')).toBeInTheDocument();
  });
});
