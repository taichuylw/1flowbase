import {
  createJsBlockBrowserWorkerFactory,
  type JsBlockBrowserWorkerConstructor,
  type JsBlockBrowserWorkerFactoryOptions,
  type JsBlockWorkerFactory
} from '@1flowbase/page-runtime';

import frontstageRestrictedBlockWorkerUrl from '../workers/restricted-block-runtime.worker?worker&url';

export const FRONTSTAGE_RESTRICTED_BLOCK_WORKER_NAME =
  'frontstage-restricted-block-runtime';

export interface FrontstageRestrictedBlockWorkerFactoryOptions {
  workerConstructor?: JsBlockBrowserWorkerConstructor | null;
  workerUrl?: string | URL | null;
  workerOptions?: WorkerOptions;
}

export function getFrontstageRestrictedBlockWorkerUrl(): string {
  return frontstageRestrictedBlockWorkerUrl;
}

export function getFrontstageRestrictedBlockWorkerOptions(
  overrides: WorkerOptions = {}
): WorkerOptions {
  return {
    type: 'module',
    name: FRONTSTAGE_RESTRICTED_BLOCK_WORKER_NAME,
    ...overrides
  };
}

export function createFrontstageRestrictedBlockWorkerFactory(
  options: FrontstageRestrictedBlockWorkerFactoryOptions = {}
): JsBlockWorkerFactory {
  const factoryOptions: JsBlockBrowserWorkerFactoryOptions = {
    workerUrl: Object.hasOwn(options, 'workerUrl')
      ? options.workerUrl
      : getFrontstageRestrictedBlockWorkerUrl(),
    workerOptions: getFrontstageRestrictedBlockWorkerOptions(
      options.workerOptions
    )
  };

  if (Object.hasOwn(options, 'workerConstructor')) {
    factoryOptions.workerConstructor = options.workerConstructor;
  }

  return createJsBlockBrowserWorkerFactory(factoryOptions);
}
