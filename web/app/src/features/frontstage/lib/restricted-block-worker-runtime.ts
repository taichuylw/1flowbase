import {
  attachDefaultJsBlockWorkerRuntime,
  type AttachedJsBlockWorkerRuntime,
  type DefaultAttachedJsBlockWorkerRuntimeOptions,
  type JsBlockWorkerRuntimeScope
} from '@1flowbase/page-runtime';

export type FrontstageRestrictedBlockWorkerRuntimeOptions =
  DefaultAttachedJsBlockWorkerRuntimeOptions;

export function attachFrontstageRestrictedBlockWorkerRuntime(
  scope: JsBlockWorkerRuntimeScope,
  options: FrontstageRestrictedBlockWorkerRuntimeOptions = {}
): AttachedJsBlockWorkerRuntime {
  return attachDefaultJsBlockWorkerRuntime(scope, options);
}
