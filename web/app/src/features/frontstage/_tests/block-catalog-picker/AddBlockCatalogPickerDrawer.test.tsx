import { fireEvent, render, screen, within } from '@testing-library/react';
import { describe, expect, test, vi } from 'vitest';

import { AddBlockCatalogPickerDrawer } from '../../components/AddBlockCatalogPickerDrawer';
import type { NormalizedFrontstageBlockCatalogEntry } from '../../lib/block-catalog';

function createCatalogEntry(
  overrides: Partial<NormalizedFrontstageBlockCatalogEntry> = {}
): NormalizedFrontstageBlockCatalogEntry {
  return {
    id: '1flowbase:frontstage.js-ui-block',
    runtimeKind: 'js-ui',
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

  test('renders catalog entries and emits the selected entry', () => {
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
    expect(within(row as HTMLElement).getByText('js-ui')).toBeInTheDocument();
    expect(within(row as HTMLElement).getByText('1flowbase')).toBeInTheDocument();
    expect(
      within(row as HTMLElement).getByText('frontstage.js-ui-block')
    ).toBeInTheDocument();

    fireEvent.click(
      within(row as HTMLElement).getByRole('button', { name: '选择' })
    );

    expect(onSelect).toHaveBeenCalledWith(entry);
  });

  test('disables selection while saving', () => {
    render(
      <AddBlockCatalogPickerDrawer
        open
        items={[createCatalogEntry()]}
        saving
        onSelect={vi.fn()}
        onClose={vi.fn()}
      />
    );

    expect(screen.getByRole('button', { name: '选择' })).toBeDisabled();
  });
});
