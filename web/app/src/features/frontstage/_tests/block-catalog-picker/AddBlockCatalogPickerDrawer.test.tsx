import { fireEvent, render, screen, within } from '@testing-library/react';
import { describe, expect, test, vi } from 'vitest';

import { AddBlockCatalogPickerDrawer } from '../../components/AddBlockCatalogPickerDrawer';
import type { NormalizedFrontstageBlockCatalogEntry } from '../../lib/block-catalog';

function createCatalogEntry(
  overrides: Partial<NormalizedFrontstageBlockCatalogEntry> = {}
): NormalizedFrontstageBlockCatalogEntry {
  return {
    id: '1flowbase:frontstage.js-ui-block',
    runtimeKind: 'iframe',
    installationId: 'builtin-installation',
    providerCode: '1flowbase',
    pluginId: 'builtin-frontstage',
    pluginVersion: '1.0.0',
    contributionCode: 'frontstage.js-ui-block',
    title: '空白 JS Block',
    entry: 'index.js',
    permissions: {
      network: 'none',
      storage: 'none',
      secrets: 'none'
    },
    contextContract: {
      primitives: [],
      inputSchema: {}
    },
    uiCapabilities: [],
    raw: {} as NormalizedFrontstageBlockCatalogEntry['raw'],
    ...overrides
  };
}

describe('AddBlockCatalogPickerDrawer', () => {
  test('shows a clear empty state when no catalog entries are available', () => {
    render(
      <AddBlockCatalogPickerDrawer
        open
        items={[]}
        onSelect={vi.fn()}
        onClose={vi.fn()}
      />
    );

    expect(screen.getByRole('dialog', { name: '新增区块' })).toBeInTheDocument();
    expect(
      screen.getByText('当前没有可用区块目录项，暂时无法新增区块。')
    ).toBeInTheDocument();
  });

  test('renders catalog entries with built-in templates defaulting to blank', () => {
    const onSelect = vi.fn();
    const entry = createCatalogEntry();
    render(
      <AddBlockCatalogPickerDrawer
        open
        items={[entry]}
        onSelect={onSelect}
        onClose={vi.fn()}
      />
    );

    const row = screen.getByText('空白 JS Block').closest('.ant-list-item');
    expect(row).not.toBeNull();
    expect(within(row as HTMLElement).getByText('iframe')).toBeInTheDocument();
    expect(within(row as HTMLElement).getByText('1flowbase')).toBeInTheDocument();
    expect(
      within(row as HTMLElement).getByText('frontstage.js-ui-block')
    ).toBeInTheDocument();
    expect(screen.getByRole('radio', { name: 'Blank JS Block' })).toBeChecked();
    expect(screen.getByRole('radio', { name: 'Data Table' })).toBeInTheDocument();

    fireEvent.click(
      within(row as HTMLElement).getByRole('button', { name: '选择' })
    );

    expect(onSelect).toHaveBeenCalledWith(entry, 'blank');
  });

  test('emits the selected catalog entry and template id', () => {
    const onSelect = vi.fn();
    const entry = createCatalogEntry();
    render(
      <AddBlockCatalogPickerDrawer
        open
        items={[entry]}
        onSelect={onSelect}
        onClose={vi.fn()}
      />
    );

    fireEvent.click(screen.getByRole('radio', { name: 'Data Table' }));
    fireEvent.click(screen.getByRole('button', { name: '选择' }));

    expect(onSelect).toHaveBeenCalledWith(entry, 'data-table');
  });

  test('disables template selection and catalog selection while saving or loading', () => {
    const entry = createCatalogEntry();
    const { rerender } = render(
      <AddBlockCatalogPickerDrawer
        open
        items={[entry]}
        saving
        onSelect={vi.fn()}
        onClose={vi.fn()}
      />
    );

    expect(screen.getByRole('radio', { name: 'Blank JS Block' })).toBeDisabled();
    expect(screen.getByRole('button', { name: '选择' })).toBeDisabled();

    rerender(
      <AddBlockCatalogPickerDrawer
        open
        items={[entry]}
        loading
        onSelect={vi.fn()}
        onClose={vi.fn()}
      />
    );

    expect(screen.getByRole('radio', { name: 'Blank JS Block' })).toBeDisabled();
    expect(screen.getByRole('button', { name: '选择' })).toBeDisabled();
  });

  test('resets template selection to blank when reopened', () => {
    const entry = createCatalogEntry();
    const { rerender } = render(
      <AddBlockCatalogPickerDrawer
        open
        items={[entry]}
        onSelect={vi.fn()}
        onClose={vi.fn()}
      />
    );

    fireEvent.click(screen.getByRole('radio', { name: 'Create Form' }));
    expect(screen.getByRole('radio', { name: 'Create Form' })).toBeChecked();

    rerender(
      <AddBlockCatalogPickerDrawer
        open={false}
        items={[entry]}
        onSelect={vi.fn()}
        onClose={vi.fn()}
      />
    );
    rerender(
      <AddBlockCatalogPickerDrawer
        open
        items={[entry]}
        onSelect={vi.fn()}
        onClose={vi.fn()}
      />
    );

    expect(screen.getByRole('radio', { name: 'Blank JS Block' })).toBeChecked();
  });
});
