import { afterEach, expect, test } from 'vitest';

import { sha256ArrayBuffer } from '../lib/run-archive-hash';

const originalCrypto = window.crypto;

afterEach(() => {
  Object.defineProperty(window, 'crypto', {
    configurable: true,
    value: originalCrypto
  });
});

test('sha256ArrayBuffer computes SHA-256 without web crypto digest', async () => {
  Object.defineProperty(window, 'crypto', {
    configurable: true,
    value: {}
  });

  const bytes = new Uint8Array([0x61, 0x62, 0x63]);

  await expect(sha256ArrayBuffer(bytes.buffer)).resolves.toBe(
    'sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad'
  );
});
