import { describe, expect, test } from 'vitest';

import { transformJsBlockSource } from '../index';

const blockSkeleton = `
import { defineBlock } from '@1flowbase/block-sdk';
import { Card, Space, Typography } from '@1flowbase/block-renderer/antd-facade';

export default defineBlock({
  render() {
    return Card({
      children: Space({
        children: Typography({ children: 'Ready' })
      })
    });
  }
});
`;

describe('JS block source transform', () => {
  test('transforms the blank JS block skeleton into injected module bindings and an executable body', () => {
    const result = transformJsBlockSource(blockSkeleton);

    expect(result.ok).toBe(true);
    if (!result.ok) {
      return;
    }

    expect(result.injectedModules).toEqual([
      {
        source: '@1flowbase/block-sdk',
        bindings: [
          {
            kind: 'named',
            imported: 'defineBlock',
            local: 'defineBlock',
            source: '@1flowbase/block-sdk'
          }
        ]
      },
      {
        source: '@1flowbase/block-renderer/antd-facade',
        bindings: [
          {
            kind: 'named',
            imported: 'Card',
            local: 'Card',
            source: '@1flowbase/block-renderer/antd-facade'
          },
          {
            kind: 'named',
            imported: 'Space',
            local: 'Space',
            source: '@1flowbase/block-renderer/antd-facade'
          },
          {
            kind: 'named',
            imported: 'Typography',
            local: 'Typography',
            source: '@1flowbase/block-renderer/antd-facade'
          }
        ]
      }
    ]);
    expect(result.importBindings).toHaveLength(4);
    expect(result.moduleMapIdentifier).toBe('__flowbaseJsBlockModules');
    expect(result.defaultExportIdentifier).toBe(
      '__flowbaseJsBlockDefaultExport'
    );
    expect(result.executableBody).toContain(
      'const { defineBlock } = __flowbaseJsBlockModules["@1flowbase/block-sdk"];'
    );
    expect(result.executableBody).toContain(
      'const { Card, Space, Typography } = __flowbaseJsBlockModules["@1flowbase/block-renderer/antd-facade"];'
    );
    expect(result.executableBody).toContain(
      'const __flowbaseJsBlockDefaultExport = defineBlock({'
    );
    expect(result.executableBody).toContain(
      'return __flowbaseJsBlockDefaultExport;'
    );
    expect(result.executableBody).not.toContain("import { defineBlock }");
    expect(result.executableBody).not.toContain('export default');
  });

  test('supports alias imports from first-party modules', () => {
    const source = `
import { defineBlock as createBlock } from '@1flowbase/block-sdk';
import { Text as Copy, Button } from '@1flowbase/block-renderer/antd-facade';

export default createBlock({
  render() {
    return Copy({ children: 'Ready' });
  }
});
`;

    const result = transformJsBlockSource(source);

    expect(result.ok).toBe(true);
    if (!result.ok) {
      return;
    }

    expect(result.importBindings).toEqual([
      {
        kind: 'named',
        imported: 'defineBlock',
        local: 'createBlock',
        source: '@1flowbase/block-sdk'
      },
      {
        kind: 'named',
        imported: 'Text',
        local: 'Copy',
        source: '@1flowbase/block-renderer/antd-facade'
      },
      {
        kind: 'named',
        imported: 'Button',
        local: 'Button',
        source: '@1flowbase/block-renderer/antd-facade'
      }
    ]);
    expect(result.executableBody).toContain(
      'const { defineBlock: createBlock } = __flowbaseJsBlockModules["@1flowbase/block-sdk"];'
    );
    expect(result.executableBody).toContain(
      'const { Text: Copy, Button } = __flowbaseJsBlockModules["@1flowbase/block-renderer/antd-facade"];'
    );
  });

  test('supports namespace imports for the block SDK defineBlock contract', () => {
    const source = `
import * as BlockSdk from '@1flowbase/block-sdk';
import { Text } from '@1flowbase/block-renderer/antd-facade';

export default BlockSdk.defineBlock({
  render() {
    return Text({ children: 'Ready' });
  }
});
`;

    const result = transformJsBlockSource(source);

    expect(result.ok).toBe(true);
    if (!result.ok) {
      return;
    }

    expect(result.importBindings).toEqual([
      {
        kind: 'namespace',
        local: 'BlockSdk',
        source: '@1flowbase/block-sdk'
      },
      {
        kind: 'named',
        imported: 'Text',
        local: 'Text',
        source: '@1flowbase/block-renderer/antd-facade'
      }
    ]);
    expect(result.executableBody).toContain(
      'const BlockSdk = __flowbaseJsBlockModules["@1flowbase/block-sdk"];'
    );
    expect(result.executableBody).toContain(
      'const __flowbaseJsBlockDefaultExport = BlockSdk.defineBlock({'
    );
  });

  test('supports a direct default export defineBlock expression without a semicolon', () => {
    const source = `
import { defineBlock } from '@1flowbase/block-sdk';

export default defineBlock({
  render() {
    return { primitive: 'Text', props: { children: 'Ready' } };
  }
})
`;

    const result = transformJsBlockSource(source);

    expect(result.ok).toBe(true);
    if (!result.ok) {
      return;
    }

    expect(result.executableBody).toContain(
      "const __flowbaseJsBlockDefaultExport = defineBlock({"
    );
    expect(result.executableBody.trim().endsWith('return __flowbaseJsBlockDefaultExport;')).toBe(
      true
    );
  });

  test.each([
    [
      'illegal import',
      "import React from 'react';\nexport default {};",
      'import_denied'
    ],
    [
      'dynamic import',
      "await import('@1flowbase/block-sdk');\nexport default {};",
      'import_denied'
    ],
    [
      'require',
      "const sdk = require('@1flowbase/block-sdk');\nexport default {};",
      'import_denied'
    ],
    ['eval', "eval('2 + 2');\nexport default {};", 'transform_failed'],
    [
      'Function',
      "const fn = new Function('return 1');\nexport default {};",
      'transform_failed'
    ],
    ['fetch', "await fetch('/api/private');\nexport default {};", 'transform_failed'],
    ['DOM', 'document.querySelector("#root");\nexport default {};', 'transform_failed'],
    [
      'storage',
      "localStorage.getItem('token');\nexport default {};",
      'transform_failed'
    ],
    [
      'syntax',
      'const value = "unterminated\nexport default {};',
      'syntax_invalid'
    ]
  ] as const)('keeps source policy error code for %s', (_label, source, code) => {
    const result = transformJsBlockSource(source);

    expect(result.ok).toBe(false);
    expect(result.errors[0]).toMatchObject({ code });
  });

  test('returns transform_failed when the default export is missing', () => {
    const result = transformJsBlockSource(`
import { defineBlock } from '@1flowbase/block-sdk';

const block = defineBlock({
  render() {
    return { primitive: 'Text' };
  }
});
`);

    expect(result.ok).toBe(false);
    expect(result.errors[0]).toMatchObject({
      code: 'transform_failed',
      path: 'source.defaultExport'
    });
  });

  test('returns transform_failed when the default export is not a safe expression transform', () => {
    const result = transformJsBlockSource(`
export default function Block() {
  return { primitive: 'Text' };
}
`);

    expect(result.ok).toBe(false);
    expect(result.errors[0]).toMatchObject({
      code: 'transform_failed',
      path: 'source.defaultExport'
    });
  });

  test('returns transform_failed when the default export does not call the injected defineBlock', () => {
    const result = transformJsBlockSource(`
import { Text } from '@1flowbase/block-renderer/antd-facade';

export default {
  render() {
    return Text({ children: 'Ready' });
  }
};
`);

    expect(result.ok).toBe(false);
    expect(result.errors[0]).toMatchObject({
      code: 'transform_failed',
      path: 'source.defaultExport'
    });
  });

  test('returns transform_failed when the default export is repeated', () => {
    const result = transformJsBlockSource(`
import { defineBlock } from '@1flowbase/block-sdk';

export default defineBlock({
  render() {
    return { primitive: 'Text' };
  }
});

export default defineBlock({
  render() {
    return { primitive: 'Text' };
  }
});
`);

    expect(result.ok).toBe(false);
    expect(result.errors[0]).toMatchObject({
      code: 'transform_failed',
      path: 'source.defaultExport'
    });
  });
});
