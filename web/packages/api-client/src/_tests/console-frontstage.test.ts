import { describe, expect, test, vi } from 'vitest';
import * as transport from '../transport';

import {
  createFrontstageGroup,
  createFrontstagePage,
  deleteFrontstagePageNode,
  listFrontstagePages,
  moveFrontstagePageNode,
  updateFrontstagePageNodeTitle
} from '../console-frontstage';

describe('console-frontstage client', () => {
  const apiFetchSpy = vi
    .spyOn(transport, 'apiFetch')
    .mockImplementation(async (input) => input as never);

  test('transport spy is active', () => {
    expect(apiFetchSpy).toBeDefined();
  });

  test('listFrontstagePages reads the workspace page tree', async () => {
    await expect(listFrontstagePages('workspace-1')).resolves.toMatchObject({
      path: '/api/console/frontstage/workspace-1/pages',
      method: 'GET'
    });
  });

  test('createFrontstageGroup posts group payload with CSRF', async () => {
    await expect(
      createFrontstageGroup(
        'workspace-1',
        { title: '分组 1', parent_id: null, rank: '001000' },
        'csrf-123'
      )
    ).resolves.toMatchObject({
      path: '/api/console/frontstage/workspace-1/pages/groups',
      method: 'POST',
      body: { title: '分组 1', parent_id: null, rank: '001000' },
      csrfToken: 'csrf-123'
    });
  });

  test('createFrontstagePage posts page payload with CSRF', async () => {
    await expect(
      createFrontstagePage(
        'workspace-1',
        { title: '页面 新建 1', parent_id: 'group-1', rank: '002000' },
        'csrf-123'
      )
    ).resolves.toMatchObject({
      path: '/api/console/frontstage/workspace-1/pages',
      method: 'POST',
      body: { title: '页面 新建 1', parent_id: 'group-1', rank: '002000' },
      csrfToken: 'csrf-123'
    });
  });

  test('updateFrontstagePageNodeTitle patches title payload with CSRF', async () => {
    await expect(
      updateFrontstagePageNodeTitle(
        'workspace-1',
        'page-1',
        { title: '页面-已重命名' },
        'csrf-123'
      )
    ).resolves.toMatchObject({
      path: '/api/console/frontstage/workspace-1/pages/page-1',
      method: 'PATCH',
      body: { title: '页面-已重命名' },
      csrfToken: 'csrf-123'
    });
  });

  test('moveFrontstagePageNode posts move payload with CSRF', async () => {
    await expect(
      moveFrontstagePageNode(
        'workspace-1',
        'page-1',
        { parent_id: null, rank: '000000' },
        'csrf-123'
      )
    ).resolves.toMatchObject({
      path: '/api/console/frontstage/workspace-1/pages/page-1/move',
      method: 'POST',
      body: { parent_id: null, rank: '000000' },
      csrfToken: 'csrf-123'
    });
  });

  test('deleteFrontstagePageNode deletes node with CSRF', async () => {
    await expect(
      deleteFrontstagePageNode('workspace-1', 'page-1', 'csrf-123')
    ).resolves.toMatchObject({
      path: '/api/console/frontstage/workspace-1/pages/page-1',
      method: 'DELETE',
      csrfToken: 'csrf-123'
    });
  });
});
