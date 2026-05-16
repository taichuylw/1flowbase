import type {
  BlockContext,
  BlockContextEntity,
  BlockContextIdentity,
  BlockContextPage,
  BlockContextRecord,
  BlockContextTheme,
  BlockContextUi
} from '@1flowbase/page-protocol';

import {
  evaluateJsBlockSource,
  type JsBlockInjectedModuleMap
} from './js-block-source-evaluator';
import type {
  JsBlockHostToWorkerMessage,
  JsBlockRunError,
  JsBlockRunRequest,
  JsBlockWorkerActionRequestMessage,
  JsBlockWorkerDataRequestMessage,
  JsBlockWorkerEventRequestMessage,
  JsBlockWorkerToHostMessage
} from './js-block-worker-runtime';

export interface JsBlockWorkerExecutorOptions {
  modules: JsBlockInjectedModuleMap;
  postMessage?: (message: JsBlockWorkerToHostMessage) => void;
}

export interface JsBlockWorkerExecutor {
  handleMessage(message: unknown): Promise<JsBlockWorkerToHostMessage[]>;
  dispose(): void;
}

export interface JsBlockWorkerRuntimeScope {
  postMessage(message: JsBlockWorkerToHostMessage): void;
  addEventListener?: (
    type: 'message',
    listener: (event: { data: unknown }) => void
  ) => void;
  removeEventListener?: (
    type: 'message',
    listener: (event: { data: unknown }) => void
  ) => void;
  onmessage?: ((event: { data: unknown }) => void) | null;
}

export interface AttachedJsBlockWorkerRuntime {
  executor: JsBlockWorkerExecutor;
  flush(): Promise<void>;
  dispose(): void;
}

type MutableBlockContext = BlockContext & {
  state: BlockContextRecord;
};

export function createJsBlockWorkerExecutor(
  options: JsBlockWorkerExecutorOptions
): JsBlockWorkerExecutor {
  let disposed = false;

  const dispatch = (
    output: JsBlockWorkerToHostMessage[],
    message: JsBlockWorkerToHostMessage
  ) => {
    output.push(message);
    options.postMessage?.(message);
  };

  return {
    async handleMessage(message) {
      const output: JsBlockWorkerToHostMessage[] = [];
      const hostMessage = normalizeHostMessage(message);
      if (!hostMessage || disposed) {
        return output;
      }

      if (hostMessage.type === 'init') {
        dispatch(output, {
          direction: 'worker_to_host',
          type: 'ready'
        });
        return output;
      }

      if (hostMessage.type === 'dispose') {
        disposed = true;
        return output;
      }

      if (hostMessage.type === 'timeout') {
        return output;
      }

      await runRequest(hostMessage.request, options.modules, (nextMessage) =>
        dispatch(output, nextMessage)
      );
      return output;
    },
    dispose() {
      disposed = true;
    }
  };
}

export function attachJsBlockWorkerRuntime(
  scope: JsBlockWorkerRuntimeScope,
  options: Omit<JsBlockWorkerExecutorOptions, 'postMessage'>
): AttachedJsBlockWorkerRuntime {
  const executor = createJsBlockWorkerExecutor({
    ...options,
    postMessage: (message) => scope.postMessage(message)
  });
  let pending = Promise.resolve();
  const listener = (event: { data: unknown }) => {
    pending = pending.then(() => executor.handleMessage(event.data)).then();
  };

  if (scope.addEventListener) {
    scope.addEventListener('message', listener);
  } else {
    scope.onmessage = listener;
  }

  return {
    executor,
    flush() {
      return pending;
    },
    dispose() {
      executor.dispose();
      if (scope.removeEventListener) {
        scope.removeEventListener('message', listener);
      } else if (scope.onmessage === listener) {
        scope.onmessage = null;
      }
    }
  };
}

async function runRequest(
  request: JsBlockRunRequest,
  modules: JsBlockInjectedModuleMap,
  postMessage: (message: JsBlockWorkerToHostMessage) => void
): Promise<void> {
  const evaluation = evaluateJsBlockSource({
    source: request.source,
    modules
  });

  if (!evaluation.ok) {
    postError(request, evaluation.error, postMessage);
    return;
  }

  const context = createBlockContext(request, postMessage);
  let schema: unknown;
  try {
    schema = await evaluation.block.render(context);
  } catch (error) {
    postError(
      request,
      runtimeError(
        'runtime.render',
        `JS block render failed: ${getErrorMessage(error)}`
      ),
      postMessage
    );
    return;
  }

  postMessage({
    direction: 'worker_to_host',
    type: 'rendered',
    requestId: request.requestId,
    schema
  });
}

function createBlockContext(
  request: JsBlockRunRequest,
  postMessage: (message: JsBlockWorkerToHostMessage) => void
): BlockContext {
  const snapshot = request.contextSnapshot;
  const state = { ...request.state };
  const postEvent = (message: JsBlockWorkerEventRequestMessage) =>
    postMessage(message);
  const postAction = (message: JsBlockWorkerActionRequestMessage) =>
    postMessage(message);
  const postData = (message: JsBlockWorkerDataRequestMessage) =>
    postMessage(message);

  const context: MutableBlockContext = {
    currentUser: readIdentity(snapshot.currentUser),
    workspace: readEntity(snapshot.workspace, 'workspace'),
    application: readEntity(snapshot.application, 'application'),
    page: readPage(snapshot.page),
    params: readRecord(snapshot.params),
    props: { ...request.props },
    state,
    patch(patch) {
      if (isRecord(patch)) {
        Object.assign(state, patch);
      }
    },
    data: {
      async query(model, params) {
        postData({
          direction: 'worker_to_host',
          type: 'data',
          requestId: request.requestId,
          operation: 'query',
          payload: {
            ...(isRecord(params) ? params : {}),
            model
          }
        });
        return undefined;
      },
      async create(model, input) {
        postData({
          direction: 'worker_to_host',
          type: 'data',
          requestId: request.requestId,
          operation: 'create',
          payload: { model, input }
        });
        return undefined;
      },
      async update(model, id, input) {
        postData({
          direction: 'worker_to_host',
          type: 'data',
          requestId: request.requestId,
          operation: 'update',
          payload: { model, id, input }
        });
        return undefined;
      },
      async delete(model, id) {
        postData({
          direction: 'worker_to_host',
          type: 'data',
          requestId: request.requestId,
          operation: 'delete',
          payload: { model, id }
        });
      }
    },
    actions: {
      async invoke(actionId, payload) {
        postAction({
          direction: 'worker_to_host',
          type: 'action',
          requestId: request.requestId,
          actionId,
          ...(isRecord(payload) ? { payload } : {})
        });
        return undefined;
      }
    },
    events: {
      emit(name, payload) {
        postEvent({
          direction: 'worker_to_host',
          type: 'event',
          requestId: request.requestId,
          name,
          ...(isRecord(payload) ? { payload } : {})
        });
      }
    },
    theme: readTheme(snapshot.theme),
    ui: readUi(snapshot.ui)
  };

  return context;
}

function postError(
  request: JsBlockRunRequest,
  error: JsBlockRunError,
  postMessage: (message: JsBlockWorkerToHostMessage) => void
): void {
  postMessage({
    direction: 'worker_to_host',
    type: 'error',
    requestId: request.requestId,
    kind: error.kind,
    message: error.message,
    errors: error.errors
  });
}

function runtimeError(path: string, message: string): JsBlockRunError {
  return {
    kind: 'runtime_error',
    message,
    errors: [{ code: 'runtime_error', path, message }]
  };
}

function normalizeHostMessage(
  value: unknown
): JsBlockHostToWorkerMessage | null {
  if (!isRecord(value)) {
    return null;
  }

  if (value.direction !== 'host_to_worker') {
    return null;
  }

  if (value.type === 'init') {
    return {
      direction: 'host_to_worker',
      type: 'init',
      ...(typeof value.requestId === 'string'
        ? { requestId: value.requestId }
        : {})
    };
  }

  if (value.type === 'dispose') {
    return {
      direction: 'host_to_worker',
      type: 'dispose',
      ...(typeof value.requestId === 'string'
        ? { requestId: value.requestId }
        : {})
    };
  }

  if (value.type === 'timeout' && typeof value.requestId === 'string') {
    return {
      direction: 'host_to_worker',
      type: 'timeout',
      requestId: value.requestId
    };
  }

  if (value.type === 'run' && isRecord(value.request)) {
    return {
      direction: 'host_to_worker',
      type: 'run',
      request: value.request as unknown as JsBlockRunRequest
    };
  }

  return null;
}

function readIdentity(value: unknown): BlockContextIdentity | null {
  if (!isRecord(value) || typeof value.id !== 'string') {
    return null;
  }

  return {
    id: value.id,
    ...(typeof value.displayName === 'string'
      ? { displayName: value.displayName }
      : {})
  };
}

function readEntity(value: unknown, fallbackId: string): BlockContextEntity {
  if (!isRecord(value) || typeof value.id !== 'string') {
    return { id: fallbackId };
  }

  return {
    id: value.id,
    ...(typeof value.name === 'string' ? { name: value.name } : {})
  };
}

function readPage(value: unknown): BlockContextPage {
  if (!isRecord(value) || typeof value.id !== 'string') {
    return { id: 'page', route: '' };
  }

  return {
    id: value.id,
    route: typeof value.route === 'string' ? value.route : '',
    ...(typeof value.title === 'string' ? { title: value.title } : {})
  };
}

function readRecord(value: unknown): BlockContextRecord {
  return isRecord(value) ? { ...value } : {};
}

function readTheme(value: unknown): BlockContextTheme {
  if (!isRecord(value)) {
    return { mode: 'light', tokens: {} };
  }

  return {
    mode: value.mode === 'dark' ? 'dark' : 'light',
    tokens: readRecord(value.tokens)
  };
}

function readUi(value: unknown): BlockContextUi {
  if (!isRecord(value)) {
    return {};
  }

  return {
    ...(typeof value.locale === 'string' ? { locale: value.locale } : {}),
    ...(value.density === 'compact' || value.density === 'comfortable'
      ? { density: value.density }
      : {})
  };
}

function getErrorMessage(error: unknown): string {
  if (error instanceof Error && error.message) {
    return error.message;
  }

  return 'unknown error';
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}
