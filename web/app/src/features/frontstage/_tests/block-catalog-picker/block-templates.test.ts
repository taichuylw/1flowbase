import { describe, expect, test } from 'vitest';

import { createDefaultJsBlockWorkerExecutor } from '@1flowbase/page-runtime';

import { createBlankJsBlockTemplateCode } from '../../lib/block-templates';

describe('frontstage block templates', () => {
  test('creates a complete blank JS block skeleton with the selected block refs', () => {
    const code = createBlankJsBlockTemplateCode({
      blockId: 'frontstage-js-block-1',
      codeRef: 'frontstage-js-block-1-code',
      contributionCode: 'frontstage.js-ui-block'
    });

    expect(code).toContain("@1flowbase/block-sdk");
    expect(code).toContain("@1flowbase/block-renderer/antd-facade");
    expect(code).toContain('defineBlock');
    expect(code).toContain("blockId: 'frontstage-js-block-1'");
    expect(code).toContain("codeRef: 'frontstage-js-block-1-code'");
    expect(code).toContain("contributionCode: 'frontstage.js-ui-block'");
    expect(code).toContain("id: 'frontstage-js-block-1'");
    expect(code).toContain("title: 'Blank JS Block'");
    expect(code).toContain('initialState');
    expect(code).toContain('async render(ctx)');
    expect(code).toContain('ctx.data.query');
    expect(code).toContain('ctx.data.create');
    expect(code).toContain('ctx.data.update');
    expect(code).toContain('ctx.data.delete');
  });

  test('runs the generated blank JS block through the default worker executor', async () => {
    const source = createBlankJsBlockTemplateCode({
      blockId: 'frontstage-js-block-1',
      codeRef: 'frontstage-js-block-1-code',
      contributionCode: 'frontstage.js-ui-block'
    });

    const executor = createDefaultJsBlockWorkerExecutor();

    const messages = await executor.handleMessage({
      direction: 'host_to_worker',
      type: 'run',
      request: {
        requestId: 'request-1',
        blockId: 'frontstage-js-block-1',
        source,
        props: {},
        state: { error: 'Template error' },
        contextSnapshot: {
          workspace: { id: 'workspace-1' },
          application: { id: 'application-1' },
          page: { id: 'page-1', route: '/frontstage/page-1' }
        },
        limits: {
          timeoutMs: 1000,
          maxRenderDepth: 8,
          maxRenderNodes: 250
        }
      }
    });

    expect(messages).toEqual([
      {
        direction: 'worker_to_host',
        type: 'rendered',
        requestId: 'request-1',
        schema: {
          primitive: 'Stack',
          children: [
            {
              primitive: 'Title',
              props: { children: 'Blank JS Block' }
            },
            {
              primitive: 'Text',
              props: {
                children: 'Start from this built-in blank JS Block skeleton.'
              }
            },
            {
              primitive: 'Alert',
              props: { type: 'error', message: 'Template error' }
            }
          ]
        }
      }
    ]);

    expect(source).not.toContain('Card');
    expect(source).not.toContain('Space');
    expect(source).not.toContain('Typography');
    expect(source).not.toContain('meta:');
    expect(source).not.toContain('ctx.state(');
  });
});
