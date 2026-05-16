import type { JsBlockWorkerRuntimeScope } from '@1flowbase/page-runtime';

import { attachFrontstageRestrictedBlockWorkerRuntime } from '../lib/restricted-block-worker-runtime';

attachFrontstageRestrictedBlockWorkerRuntime(
  self as unknown as JsBlockWorkerRuntimeScope
);
