import { afterEach, describe, expect, test, vi } from 'vitest';

import copy from 'copy-to-clipboard';
import { copyTextToClipboard } from '../copy-text';

vi.mock('copy-to-clipboard', () => ({
  default: vi.fn()
}));

afterEach(() => {
  vi.restoreAllMocks();
});

describe('copyTextToClipboard', () => {
  test('copies text through the shared clipboard dependency', async () => {
    vi.mocked(copy).mockResolvedValue(true);

    await copyTextToClipboard('hello');

    expect(copy).toHaveBeenCalledWith('hello');
  });

  test('throws when the clipboard dependency reports copy failure', async () => {
    vi.mocked(copy).mockResolvedValue(false);

    await expect(copyTextToClipboard('fallback text')).rejects.toThrow(
      'Copy command failed'
    );
  });
});
