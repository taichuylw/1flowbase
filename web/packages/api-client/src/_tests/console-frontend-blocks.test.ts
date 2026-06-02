import { describe, expect, test, vi } from 'vitest';
import * as transport from '../transport';

import { listConsoleFrontendBlocks } from '../console/frontend-blocks';

describe('console-frontend-blocks client', () => {
  vi.spyOn(transport, 'apiFetch').mockImplementation(async (input) => input as never);

  test('lists frontend block catalog entries from console endpoint', async () => {
    await expect(listConsoleFrontendBlocks()).resolves.toMatchObject({
      path: '/api/console/frontend-blocks',
      method: 'GET'
    });
  });
});
