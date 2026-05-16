import { describe, expect, test } from 'vitest';

import {
  JsBlockWorkerAdapterError,
  type JsBlockRunRequest,
  type JsBlockWorkerLike
} from '@1flowbase/page-runtime';

import type { RestrictedBlockRunPlan } from '../../lib/restricted-block-loader';
import {
  createFrontstageRestrictedBlockRuntimeHost,
  type FrontstageRestrictedBlockRuntimeHostOptions
} from '../../lib/frontstage-restricted-block-runtime-host';
import { getFrontstageRestrictedBlockWorkerUrl } from '../../lib/restricted-block-worker-factory';

const validSource = `
import { defineBlock } from '@1flowbase/block-sdk';
import { Text } from '@1flowbase/block-renderer/antd-facade';

export default defineBlock({
  render() {
    return Text({ children: 'Ready' });
  }
});
`;

class FakeWorker implements JsBlockWorkerLike {
  onmessage: ((event: { data: unknown }) => void) | null = null;
  onerror: ((event: { message?: string }) => void) | null = null;
  onmessageerror: ((event: { message?: string }) => void) | null = null;
  readonly messages: unknown[] = [];
  terminateCount = 0;

  postMessage(message: unknown): void {
    this.messages.push(message);
  }

  terminate(): void {
    this.terminateCount += 1;
  }
}

class FakeNativeWorker {
  static readonly instances: FakeNativeWorker[] = [];

  onmessage: ((event: MessageEvent) => void) | null = null;
  onerror: ((event: ErrorEvent) => void) | null = null;
  onmessageerror: ((event: MessageEvent) => void) | null = null;
  readonly scriptUrl: string | URL;
  readonly options?: WorkerOptions;
  readonly messages: unknown[] = [];
  terminateCount = 0;

  constructor(scriptUrl: string | URL, options?: WorkerOptions) {
    this.scriptUrl = scriptUrl;
    this.options = options;
    FakeNativeWorker.instances.push(this);
  }

  postMessage(message: unknown): void {
    this.messages.push(message);
  }

  terminate(): void {
    this.terminateCount += 1;
  }
}

class ThrowingNativeWorker extends FakeNativeWorker {
  constructor(scriptUrl: string | URL, options?: WorkerOptions) {
    super(scriptUrl, options);
    throw new Error('native worker blocked');
  }
}

function createRunRequest(
  overrides: Partial<JsBlockRunRequest> = {}
): JsBlockRunRequest {
  return {
    requestId: 'restricted-block:block-1:code-1',
    blockId: 'block-1',
    source: validSource,
    props: { title: 'Hello' },
    state: { selected: false },
    contextSnapshot: { pageId: 'page-1' },
    limits: { timeoutMs: 1000, maxRenderDepth: 8, maxRenderNodes: 250 },
    ...overrides
  };
}

function createRunPlan(
  overrides: Partial<RestrictedBlockRunPlan> = {}
): RestrictedBlockRunPlan {
  return {
    ok: true,
    request: createRunRequest(),
    schemaValidationOptions: {
      maxDepth: 8,
      maxNodes: 250,
      allowedDataPermissions: ['query'],
      allowedActions: ['record.save'],
      allowedEvents: ['record.saved']
    },
    mediatorPolicy: {
      allowedEvents: ['record.saved'],
      allowedActions: ['record.save'],
      allowedDataModels: ['records'],
      allowedDataOperations: ['query'],
      maxEventChainDepth: 4
    },
    ...overrides
  };
}

function createSubject(
  options: Partial<FrontstageRestrictedBlockRuntimeHostOptions> = {}
) {
  const worker = new FakeWorker();
  const host = createFrontstageRestrictedBlockRuntimeHost({
    runPlan: createRunPlan(),
    workerFactory: () => worker,
    ...options
  });

  return { host, worker };
}

describe('FrontStage restricted block runtime host factory', () => {
  test('uses the FrontStage browser Worker factory by default', () => {
    FakeNativeWorker.instances.length = 0;

    createFrontstageRestrictedBlockRuntimeHost({
      runPlan: createRunPlan(),
      browserWorkerFactoryOptions: {
        workerConstructor: FakeNativeWorker
      }
    });

    expect(FakeNativeWorker.instances).toHaveLength(1);
    expect(String(FakeNativeWorker.instances[0]?.scriptUrl)).toBe(
      String(getFrontstageRestrictedBlockWorkerUrl())
    );
    expect(FakeNativeWorker.instances[0]?.options).toEqual({
      type: 'module',
      name: 'frontstage-restricted-block-runtime'
    });
  });

  test('uses an injected workerFactory instead of browser Worker options', () => {
    FakeNativeWorker.instances.length = 0;
    const worker = new FakeWorker();

    const host = createFrontstageRestrictedBlockRuntimeHost({
      runPlan: createRunPlan(),
      workerFactory: () => worker,
      browserWorkerFactoryOptions: {
        workerConstructor: ThrowingNativeWorker
      }
    });

    host.run();

    expect(FakeNativeWorker.instances).toEqual([]);
    expect(worker.messages).toEqual([
      {
        direction: 'host_to_worker',
        type: 'run',
        request: createRunRequest()
      }
    ]);
  });

  test('keeps run and dispose snapshots aligned with the restricted runtime host', () => {
    const { host, worker } = createSubject();

    expect(host.run()).toMatchObject({
      status: 'running',
      requestId: 'restricted-block:block-1:code-1',
      blockId: 'block-1',
      schemaValidationOptions: {
        maxDepth: 8,
        maxNodes: 250,
        allowedDataPermissions: ['query'],
        allowedActions: ['record.save'],
        allowedEvents: ['record.saved']
      },
      logs: [],
      effects: [],
      rejections: []
    });
    expect(host.dispose()).toMatchObject({
      status: 'disposed',
      requestId: 'restricted-block:block-1:code-1',
      blockId: 'block-1'
    });
    expect(worker.terminateCount).toBe(1);
  });

  test('passes through browser Worker construction errors with page-runtime attribution', () => {
    try {
      createFrontstageRestrictedBlockRuntimeHost({
        runPlan: createRunPlan(),
        browserWorkerFactoryOptions: {
          workerConstructor: ThrowingNativeWorker
        }
      });
      expect.unreachable('expected worker construction to fail');
    } catch (error) {
      expect(error).toBeInstanceOf(JsBlockWorkerAdapterError);
      expect(error).toMatchObject({
        code: 'worker_construct_failed',
        message:
          'Failed to construct JS block worker: native worker blocked'
      });
    }
  });
});
