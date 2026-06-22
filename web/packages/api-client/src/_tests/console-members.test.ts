import { describe, expect, test, vi } from 'vitest';
import * as transport from '../transport';

import {
  deleteConsoleMember,
  enableConsoleMember,
  updateConsoleMember,
  type UpdateConsoleMemberInput
} from '../console-members';

describe('console members client', () => {
  vi.spyOn(transport, 'apiFetch').mockImplementation(
    async (input) => input as never
  );
  vi.spyOn(transport, 'apiFetchVoid').mockImplementation(
    async (input) => input as never
  );

  test('updates member profile through the member patch route', async () => {
    const input: UpdateConsoleMemberInput = {
      name: 'Root Next',
      nickname: 'Captain Root',
      email: 'root-next@example.com',
      phone: '13900000000',
      introduction: 'updated root profile'
    };

    await expect(
      updateConsoleMember('member-1', input, 'csrf-123')
    ).resolves.toMatchObject({
      path: '/api/console/members/member-1',
      method: 'PATCH',
      csrfToken: 'csrf-123',
      body: input
    });
  });

  test('deletes member through the member delete route', async () => {
    await expect(
      deleteConsoleMember('member-1', 'csrf-123')
    ).resolves.toMatchObject({
      path: '/api/console/members/member-1',
      method: 'DELETE',
      csrfToken: 'csrf-123'
    });
  });

  test('enables member through the member enable route', async () => {
    await expect(
      enableConsoleMember('member-1', 'csrf-123')
    ).resolves.toMatchObject({
      path: '/api/console/members/member-1/actions/enable',
      method: 'POST',
      csrfToken: 'csrf-123'
    });
  });
});
