import type { JsBlockWorkerFactory } from '@1flowbase/page-runtime';

import {
  createRestrictedBlockRuntimeHost,
  type RestrictedBlockRuntimeHost,
  type RestrictedBlockRuntimeHostOptions
} from './restricted-block-runtime-host';
import {
  createFrontstageRestrictedBlockWorkerFactory,
  type FrontstageRestrictedBlockWorkerFactoryOptions
} from './restricted-block-worker-factory';

export interface FrontstageRestrictedBlockRuntimeHostOptions
  extends Omit<RestrictedBlockRuntimeHostOptions, 'workerFactory'> {
  workerFactory?: JsBlockWorkerFactory;
  browserWorkerFactoryOptions?: FrontstageRestrictedBlockWorkerFactoryOptions;
}

export function createFrontstageRestrictedBlockRuntimeHost(
  options: FrontstageRestrictedBlockRuntimeHostOptions
): RestrictedBlockRuntimeHost {
  const { browserWorkerFactoryOptions, workerFactory, ...runtimeOptions } =
    options;

  return createRestrictedBlockRuntimeHost({
    ...runtimeOptions,
    workerFactory:
      workerFactory ??
      createFrontstageRestrictedBlockWorkerFactory(browserWorkerFactoryOptions)
  });
}
