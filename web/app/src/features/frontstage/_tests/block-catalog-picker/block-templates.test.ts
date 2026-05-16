import { describe, expect, test } from 'vitest';

import { createBlankJsBlockTemplateCode } from '../../lib/block-templates';

describe('frontstage block templates', () => {
  test('creates a complete blank JS block skeleton with the selected block refs', () => {
    const code = createBlankJsBlockTemplateCode({
      blockId: 'frontstage-js-block-1',
      codeRef: 'frontstage-js-block-1-code',
      contributionCode: 'frontstage.js-ui-block'
    });

    expect(code).toContain("@1flowbase/block-sdk");
    expect(code).toContain("@1flowbase/antd-facade");
    expect(code).toContain('defineBlock');
    expect(code).toContain("blockId: 'frontstage-js-block-1'");
    expect(code).toContain("codeRef: 'frontstage-js-block-1-code'");
    expect(code).toContain("contributionCode: 'frontstage.js-ui-block'");
    expect(code).toContain('async setup(ctx)');
    expect(code).toContain('async render(ctx)');
    expect(code).toContain('ctx.data.query');
    expect(code).toContain('ctx.data.create');
    expect(code).toContain('ctx.data.update');
    expect(code).toContain('ctx.data.delete');
  });
});
