import { describe, expect, test, vi } from 'vitest';
import * as transport from '../transport';

import {
  downloadConsoleOfficialAgentFlowTemplate,
  listConsoleOfficialAgentFlowTemplateCatalog
} from '../console/application-orchestration';

describe('console application orchestration official template client', () => {
  vi.spyOn(transport, 'apiFetch').mockImplementation(
    async (input) => input as never
  );

  test('points official template catalog at the paged backend route', async () => {
    await expect(
      listConsoleOfficialAgentFlowTemplateCatalog(
        { cursor: '2' },
        'https://api.flowbase.test'
      )
    ).resolves.toMatchObject({
      path: '/api/console/applications/orchestration/templates/official-catalog?cursor=2',
      baseUrl: 'https://api.flowbase.test'
    });
  });

  test('downloads official templates through the backend route', async () => {
    await expect(
      downloadConsoleOfficialAgentFlowTemplate(
        'customer/support bot',
        'https://api.flowbase.test'
      )
    ).resolves.toMatchObject({
      path: '/api/console/applications/orchestration/templates/official/customer%2Fsupport%20bot',
      baseUrl: 'https://api.flowbase.test'
    });
  });
});
