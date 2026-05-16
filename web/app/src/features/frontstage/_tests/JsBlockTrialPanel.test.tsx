import { fireEvent, render, screen, within } from '@testing-library/react';
import { useState } from 'react';
import { describe, expect, test, vi } from 'vitest';

import { JsBlockTrialPanel } from '../components/JsBlockTrialPanel';
import type { NormalizedFrontstageBlockCatalogEntry } from '../lib/block-catalog';
import type { FrontstageBlockInstance } from '../lib/page-document';
import type { RestrictedBlockLoaderLimits } from '../lib/restricted-block-loader';

function createBlock(
  overrides: Partial<FrontstageBlockInstance> = {}
): FrontstageBlockInstance {
  return {
    id: 'hero-block',
    sourceId: 'hero-block',
    codeRef: 'hero-code',
    sourceCodeRef: 'hero-code',
    catalog: {
      providerCode: 'official',
      installationId: 'installation-1'
    },
    contribution: {
      pluginId: 'official.blocks',
      pluginVersion: '1.0.0',
      code: 'hero.banner'
    },
    props: { title: 'Hello' },
    layout: { order: 1 },
    order: 1,
    runtime: {
      kind: 'iframe',
      entry: 'blocks/hero/index.js',
      hint: 'iframe'
    },
    ...overrides
  };
}

function createCatalogEntry(
  overrides: Partial<NormalizedFrontstageBlockCatalogEntry> = {}
): NormalizedFrontstageBlockCatalogEntry {
  return {
    id: 'official:hero.banner',
    runtimeKind: 'iframe',
    installationId: 'installation-1',
    providerCode: 'official',
    pluginId: 'official.blocks',
    pluginVersion: '1.0.0',
    contributionCode: 'hero.banner',
    title: 'Hero Banner',
    entry: 'blocks/hero/index.js',
    permissions: {
      network: 'none',
      storage: 'none',
      secrets: 'none'
    },
    contextContract: {
      primitives: ['text', 'button', 'data_record'],
      inputSchema: { type: 'object' }
    },
    uiCapabilities: ['responsive', 'data_binding'],
    raw: {} as NormalizedFrontstageBlockCatalogEntry['raw'],
    ...overrides
  };
}

function createLimits(
  overrides: Partial<RestrictedBlockLoaderLimits> = {}
): RestrictedBlockLoaderLimits {
  return {
    timeoutMs: 1000,
    maxRenderDepth: 8,
    maxRenderNodes: 250,
    allowedActions: ['record.save'],
    allowedEvents: ['record.saved'],
    allowedDataModels: ['records'],
    allowedDataOperations: ['query'],
    maxEventChainDepth: 4,
    ...overrides
  };
}

describe('JsBlockTrialPanel', () => {
  test('shows clear empty states when the selected block or catalog entry is missing', () => {
    const { rerender } = render(
      <JsBlockTrialPanel
        block={null}
        catalogEntry={createCatalogEntry()}
        code="export default {}"
        contextSnapshot={{ pageId: 'page-1' }}
        limits={createLimits()}
      />
    );

    expect(screen.getByText('请选择一个区块')).toBeInTheDocument();

    rerender(
      <JsBlockTrialPanel
        block={createBlock()}
        catalogEntry={null}
        code="export default {}"
        contextSnapshot={{ pageId: 'page-1' }}
        limits={createLimits()}
      />
    );

    expect(screen.getByText('缺少区块目录条目')).toBeInTheDocument();
  });

  test('renders a valid run plan summary without executing JavaScript', () => {
    render(
      <JsBlockTrialPanel
        block={createBlock()}
        catalogEntry={createCatalogEntry()}
        code="export default { render() {} }"
        contextSnapshot={{ pageId: 'page-1', locale: 'zh-CN' }}
        limits={createLimits()}
      />
    );

    expect(screen.getByText('Run plan 已生成')).toBeInTheDocument();
    expect(
      screen.getByText('restricted-block:hero-block:hero-code')
    ).toBeInTheDocument();
    expect(screen.getByText('hero-block')).toBeInTheDocument();
    expect(screen.getByText('1000ms')).toBeInTheDocument();
    expect(screen.getByText('pageId, locale')).toBeInTheDocument();

    const schemaOptions = screen.getByTestId('js-block-trial-schema-options');
    expect(within(schemaOptions).getByText('8')).toBeInTheDocument();
    expect(within(schemaOptions).getByText('250')).toBeInTheDocument();
    expect(within(schemaOptions).getByText('query')).toBeInTheDocument();
    expect(within(schemaOptions).getByText('record.save')).toBeInTheDocument();

    const mediatorPolicy = screen.getByTestId('js-block-trial-mediator-policy');
    expect(within(mediatorPolicy).getByText('records')).toBeInTheDocument();
    expect(within(mediatorPolicy).getByText('4')).toBeInTheDocument();
  });

  test('renders structured rejection details from the run plan builder', () => {
    render(
      <JsBlockTrialPanel
        block={createBlock({ codeRef: '' })}
        catalogEntry={createCatalogEntry()}
        code="export default {}"
        contextSnapshot={{ pageId: 'page-1' }}
        limits={createLimits()}
      />
    );

    expect(screen.getByText('Run plan 被拒绝')).toBeInTheDocument();
    expect(screen.getByText('missing_code_ref')).toBeInTheDocument();
    expect(screen.getByText('block.codeRef')).toBeInTheDocument();
    expect(
      screen.getByText('Restricted block codeRef is required.')
    ).toBeInTheDocument();
    expect(screen.getByText('hero-block')).toBeInTheDocument();
    expect(screen.getByText('official:hero.banner')).toBeInTheDocument();
  });

  test('offers controlled editors for code, context snapshot, and limits', () => {
    const onCodeChange = vi.fn();
    const onContextSnapshotChange = vi.fn();
    const onLimitsChange = vi.fn();

    function Harness() {
      const [code, setCode] = useState('export default {}');
      const [contextSnapshot, setContextSnapshot] = useState<
        Record<string, unknown>
      >({
        pageId: 'page-1'
      });
      const [limits, setLimits] = useState<RestrictedBlockLoaderLimits>(
        createLimits()
      );

      return (
        <JsBlockTrialPanel
          block={createBlock()}
          catalogEntry={createCatalogEntry()}
          code={code}
          contextSnapshot={contextSnapshot}
          limits={limits}
          onCodeChange={(nextCode) => {
            onCodeChange(nextCode);
            setCode(nextCode);
          }}
          onContextSnapshotChange={(nextContextSnapshot) => {
            onContextSnapshotChange(nextContextSnapshot);
            setContextSnapshot(nextContextSnapshot);
          }}
          onLimitsChange={(nextLimits) => {
            onLimitsChange(nextLimits);
            setLimits(nextLimits);
          }}
        />
      );
    }

    render(<Harness />);

    fireEvent.change(screen.getByLabelText('JS 代码'), {
      target: { value: 'export default { render() { return null } }' }
    });
    expect(onCodeChange).toHaveBeenCalledWith(
      'export default { render() { return null } }'
    );

    fireEvent.change(screen.getByLabelText('Context snapshot'), {
      target: { value: '{ "pageId": "page-2", "recordId": "record-1" }' }
    });
    fireEvent.click(screen.getByRole('button', { name: '更新 context' }));
    expect(onContextSnapshotChange).toHaveBeenCalledWith({
      pageId: 'page-2',
      recordId: 'record-1'
    });
    expect(screen.getByText('pageId, recordId')).toBeInTheDocument();

    fireEvent.change(screen.getByLabelText('Runtime limits'), {
      target: {
        value:
          '{ "timeoutMs": 2000, "maxRenderDepth": 4, "maxRenderNodes": 120, "allowedActions": ["record.archive"], "allowedEvents": [], "allowedDataModels": ["cases"], "allowedDataOperations": ["query"], "maxEventChainDepth": 2 }'
      }
    });
    fireEvent.click(screen.getByRole('button', { name: '更新 limits' }));
    expect(onLimitsChange).toHaveBeenCalledWith({
      timeoutMs: 2000,
      maxRenderDepth: 4,
      maxRenderNodes: 120,
      allowedActions: ['record.archive'],
      allowedEvents: [],
      allowedDataModels: ['cases'],
      allowedDataOperations: ['query'],
      maxEventChainDepth: 2
    });
    expect(screen.getByText('2000ms')).toBeInTheDocument();
  });
});
