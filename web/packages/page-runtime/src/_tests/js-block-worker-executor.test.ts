import { describe, expect, test, vi } from 'vitest';

import {
  attachJsBlockWorkerRuntime,
  createJsBlockRuntimeSession,
  createJsBlockWorkerExecutor,
  reduceJsBlockRuntimeSession
} from '../index';
import type {
  JsBlockInjectedModuleMap,
  JsBlockRunRequest,
  JsBlockWorkerRuntimeScope,
  JsBlockWorkerToHostMessage
} from '../index';

const validSource = `
import { defineBlock } from '@1flowbase/block-sdk';
import { Text } from '@1flowbase/antd-facade';

export default defineBlock({
  render(ctx) {
    ctx.events.emit('block.rendered', { title: ctx.props.title });
    return Text({ children: ctx.props.title });
  }
});
`;

function createModules(
  overrides: JsBlockInjectedModuleMap = {}
): JsBlockInjectedModuleMap {
  return {
    '@1flowbase/block-sdk': {
      defineBlock(definition: unknown) {
        return definition;
      }
    },
    '@1flowbase/antd-facade': {
      Text(input: { children?: unknown; props?: { children?: unknown } }) {
        return {
          primitive: 'Text',
          props: { children: input.props?.children ?? input.children }
        };
      }
    },
    ...overrides
  };
}

function createRunRequest(
  overrides: Partial<JsBlockRunRequest> = {}
): JsBlockRunRequest {
  return {
    requestId: 'request-1',
    blockId: 'block-1',
    source: validSource,
    props: { title: 'Ready' },
    state: { count: 1 },
    contextSnapshot: {
      currentUser: { id: 'user-1', displayName: 'Ada' },
      workspace: { id: 'workspace-1', name: 'Workspace' },
      application: { id: 'app-1', name: 'Application' },
      page: { id: 'page-1', route: '/frontstage/page-1', title: 'Frontstage' },
      params: { recordId: 'record-1' },
      theme: { mode: 'light', tokens: { colorPrimary: '#1677ff' } },
      ui: { locale: 'zh-CN', density: 'comfortable' }
    },
    limits: {
      timeoutMs: 1000,
      maxRenderDepth: 8,
      maxRenderNodes: 250
    },
    ...overrides
  };
}

describe('JS block worker executor', () => {
  test('posts ready on init and rendered schema on run', async () => {
    const messages: JsBlockWorkerToHostMessage[] = [];
    const executor = createJsBlockWorkerExecutor({
      modules: createModules(),
      postMessage: (message) => messages.push(message)
    });

    await executor.handleMessage({ direction: 'host_to_worker', type: 'init' });
    await executor.handleMessage({
      direction: 'host_to_worker',
      type: 'run',
      request: createRunRequest()
    });

    expect(messages).toEqual([
      { direction: 'worker_to_host', type: 'ready' },
      {
        direction: 'worker_to_host',
        type: 'event',
        requestId: 'request-1',
        name: 'block.rendered',
        payload: { title: 'Ready' }
      },
      {
        direction: 'worker_to_host',
        type: 'rendered',
        requestId: 'request-1',
        schema: { primitive: 'Text', props: { children: 'Ready' } }
      }
    ]);
  });

  test('leaves schema validation to the host reducer', async () => {
    const messages: JsBlockWorkerToHostMessage[] = [];
    const executor = createJsBlockWorkerExecutor({
      modules: createModules({
        '@1flowbase/antd-facade': {
          Text() {
            return { primitive: 'Unknown' };
          }
        }
      }),
      postMessage: (message) => messages.push(message)
    });

    await executor.handleMessage({
      direction: 'host_to_worker',
      type: 'run',
      request: createRunRequest()
    });

    const rendered = messages.find((message) => message.type === 'rendered');
    expect(rendered).toMatchObject({
      direction: 'worker_to_host',
      type: 'rendered',
      requestId: 'request-1',
      schema: { primitive: 'Unknown' }
    });

    const hostState = reduceJsBlockRuntimeSession(
      reduceJsBlockRuntimeSession(createJsBlockRuntimeSession(), {
        direction: 'host_to_worker',
        type: 'run',
        request: createRunRequest()
      }),
      rendered
    );

    expect(hostState.requests['request-1']).toMatchObject({
      status: 'failed',
      result: {
        ok: false,
        error: { kind: 'schema_invalid' }
      }
    });
  });

  test('maps render failures into runtime error messages', async () => {
    const messages: JsBlockWorkerToHostMessage[] = [];
    const executor = createJsBlockWorkerExecutor({
      modules: createModules(),
      postMessage: (message) => messages.push(message)
    });

    await executor.handleMessage({
      direction: 'host_to_worker',
      type: 'run',
      request: createRunRequest({
        source: `
import { defineBlock } from '@1flowbase/block-sdk';

export default defineBlock({
  render() {
    throw new Error('render exploded');
  }
});
`
      })
    });

    expect(messages).toEqual([
      {
        direction: 'worker_to_host',
        type: 'error',
        requestId: 'request-1',
        kind: 'runtime_error',
        message: 'JS block render failed: render exploded',
        errors: [
          {
            code: 'runtime_error',
            path: 'runtime.render',
            message: 'JS block render failed: render exploded'
          }
        ]
      }
    ]);
  });

  test('does not execute source that fails source policy', async () => {
    const messages: JsBlockWorkerToHostMessage[] = [];
    const defineBlock = vi.fn((definition: unknown) => definition);
    const executor = createJsBlockWorkerExecutor({
      modules: createModules({
        '@1flowbase/block-sdk': { defineBlock }
      }),
      postMessage: (message) => messages.push(message)
    });

    await executor.handleMessage({
      direction: 'host_to_worker',
      type: 'run',
      request: createRunRequest({
        source: "window.location.href = 'https://example.test';"
      })
    });

    expect(defineBlock).not.toHaveBeenCalled();
    expect(messages).toEqual([
      {
        direction: 'worker_to_host',
        type: 'error',
        requestId: 'request-1',
        kind: 'source_policy_failed',
        message: 'JS block source transform failed.',
        errors: [
          {
            code: 'transform_failed',
            path: 'source.identifiers.window',
            message: "Identifier 'window' is not allowed in JS block source."
          }
        ]
      }
    ]);
  });

  test('posts controlled action and data effects without real host responses', async () => {
    const messages: JsBlockWorkerToHostMessage[] = [];
    const executor = createJsBlockWorkerExecutor({
      modules: createModules(),
      postMessage: (message) => messages.push(message)
    });

    await executor.handleMessage({
      direction: 'host_to_worker',
      type: 'run',
      request: createRunRequest({
        source: `
import { defineBlock } from '@1flowbase/block-sdk';
import { Text } from '@1flowbase/antd-facade';

export default defineBlock({
  async render(ctx) {
    await ctx.actions.invoke('record.refresh', { id: ctx.params.recordId });
    await ctx.data.query('records', {
      model: 'private_records',
      where: { id: ctx.params.recordId }
    });
    await ctx.data.create('records', { title: 'New' });
    await ctx.data.update('records', 'record-1', { title: 'Updated' });
    await ctx.data.delete('records', 'record-1');
    return Text({ children: 'Done' });
  }
});
`
      })
    });

    expect(messages).toEqual([
      {
        direction: 'worker_to_host',
        type: 'action',
        requestId: 'request-1',
        actionId: 'record.refresh',
        payload: { id: 'record-1' }
      },
      {
        direction: 'worker_to_host',
        type: 'data',
        requestId: 'request-1',
        operation: 'query',
        payload: { model: 'records', where: { id: 'record-1' } }
      },
      {
        direction: 'worker_to_host',
        type: 'data',
        requestId: 'request-1',
        operation: 'create',
        payload: { model: 'records', input: { title: 'New' } }
      },
      {
        direction: 'worker_to_host',
        type: 'data',
        requestId: 'request-1',
        operation: 'update',
        payload: {
          model: 'records',
          id: 'record-1',
          input: { title: 'Updated' }
        }
      },
      {
        direction: 'worker_to_host',
        type: 'data',
        requestId: 'request-1',
        operation: 'delete',
        payload: { model: 'records', id: 'record-1' }
      },
      {
        direction: 'worker_to_host',
        type: 'rendered',
        requestId: 'request-1',
        schema: { primitive: 'Text', props: { children: 'Done' } }
      }
    ]);
  });

  test('applies ctx.patch only to the current run state snapshot', async () => {
    const messages: JsBlockWorkerToHostMessage[] = [];
    const executor = createJsBlockWorkerExecutor({
      modules: createModules(),
      postMessage: (message) => messages.push(message)
    });
    const request = createRunRequest({
      state: { count: 1 },
      source: `
import { defineBlock } from '@1flowbase/block-sdk';
import { Text } from '@1flowbase/antd-facade';

export default defineBlock({
  render(ctx) {
    ctx.patch({ count: ctx.state.count + 1 });
    return Text({ children: ctx.state.count });
  }
});
`
    });

    await executor.handleMessage({
      direction: 'host_to_worker',
      type: 'run',
      request
    });

    expect(request.state).toEqual({ count: 1 });
    expect(messages).toEqual([
      {
        direction: 'worker_to_host',
        type: 'rendered',
        requestId: 'request-1',
        schema: { primitive: 'Text', props: { children: 2 } }
      }
    ]);
  });

  test('ignores run messages after dispose', async () => {
    const messages: JsBlockWorkerToHostMessage[] = [];
    const executor = createJsBlockWorkerExecutor({
      modules: createModules(),
      postMessage: (message) => messages.push(message)
    });

    await executor.handleMessage({
      direction: 'host_to_worker',
      type: 'dispose'
    });
    await executor.handleMessage({
      direction: 'host_to_worker',
      type: 'run',
      request: createRunRequest()
    });

    expect(messages).toEqual([]);
  });

  test('attaches to a worker-like scope and detaches on dispose', async () => {
    const messages: JsBlockWorkerToHostMessage[] = [];
    let listener: ((event: { data: unknown }) => void) | null = null;
    const scope: JsBlockWorkerRuntimeScope = {
      postMessage: (message) => messages.push(message),
      addEventListener: (_type, nextListener) => {
        listener = nextListener;
      },
      removeEventListener: (_type, nextListener) => {
        if (listener === nextListener) {
          listener = null;
        }
      }
    };

    const attached = attachJsBlockWorkerRuntime(scope, {
      modules: createModules()
    });
    listener?.({ data: { direction: 'host_to_worker', type: 'init' } });
    await attached.flush();

    attached.dispose();
    listener?.({ data: { direction: 'host_to_worker', type: 'init' } });
    await attached.flush();

    expect(messages).toEqual([{ direction: 'worker_to_host', type: 'ready' }]);
    expect(listener).toBeNull();
  });
});
