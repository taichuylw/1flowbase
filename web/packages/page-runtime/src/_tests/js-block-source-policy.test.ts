import { describe, expect, test } from 'vitest';

import { validateJsBlockSource } from '../index';

const validBlockSkeleton = `
import { defineBlock } from '@1flowbase/block-sdk';
import { Stack, Text } from '@1flowbase/antd-facade';

export default defineBlock({
  render() {
    return Stack({ children: [Text({ children: 'Ready' })] });
  }
});
`;

describe('JS block source static policy', () => {
  test('accepts a block skeleton that imports only the first-party SDKs', () => {
    const result = validateJsBlockSource(validBlockSkeleton);

    expect(result).toEqual({
      ok: true,
      source: validBlockSkeleton,
      normalizedSource: validBlockSkeleton.trim(),
      errors: []
    });
  });

  test.each([
    ['react import', "import React from 'react';"],
    ['antd import', "import { Button } from 'antd';"],
    ['dom import', "import { createRoot } from 'react-dom/client';"],
    ['npm package import', "import dayjs from 'dayjs';"]
  ])('rejects denied static import: %s', (_label, source) => {
    const result = validateJsBlockSource(source);

    expect(result.ok).toBe(false);
    expect(result.errors[0]).toMatchObject({
      code: 'import_denied',
      path: 'source.imports[0]'
    });
  });

  test('rejects dynamic import with a stable import error', () => {
    const result = validateJsBlockSource(
      "const mod = await import('@1flowbase/block-sdk');"
    );

    expect(result.ok).toBe(false);
    expect(result.errors[0]).toMatchObject({
      code: 'import_denied',
      path: 'source.imports[0]'
    });
  });

  test.each([
    [
      'require',
      "const sdk = require('@1flowbase/block-sdk');",
      'import_denied'
    ],
    ['eval', "eval('2 + 2');", 'transform_failed'],
    [
      'Function constructor',
      "const fn = new Function('return 1');",
      'transform_failed'
    ]
  ] as const)('rejects executable escape hatch: %s', (_label, source, code) => {
    const result = validateJsBlockSource(source);

    expect(result.ok).toBe(false);
    expect(result.errors[0]).toMatchObject({
      code
    });
  });

  test.each([
    ['fetch', "await fetch('/api/private');"],
    ['XMLHttpRequest', 'const xhr = new XMLHttpRequest();'],
    ['WebSocket', "const socket = new WebSocket('wss://example.com');"],
    ['sendBeacon', "navigator.sendBeacon('/track');"]
  ])('rejects network capability: %s', (_label, source) => {
    const result = validateJsBlockSource(source);

    expect(result.ok).toBe(false);
    expect(result.errors[0]).toMatchObject({
      code: 'transform_failed'
    });
  });

  test.each([
    ['window', 'window.location.href;'],
    ['document', 'document.querySelector("#root");'],
    ['globalThis', 'globalThis.crypto;'],
    ['self', 'self.postMessage({});'],
    ['localStorage', "localStorage.getItem('token');"],
    ['sessionStorage', "sessionStorage.setItem('token', '1');"],
    ['cookie', 'const cookie = document.cookie;']
  ])('rejects DOM or storage capability: %s', (_label, source) => {
    const result = validateJsBlockSource(source);

    expect(result.ok).toBe(false);
    expect(result.errors[0]).toMatchObject({
      code: 'transform_failed'
    });
  });

  test('returns syntax_invalid for malformed source without throwing', () => {
    expect(() =>
      validateJsBlockSource('const value = "unterminated')
    ).not.toThrow();

    const result = validateJsBlockSource('const value = "unterminated');

    expect(result.ok).toBe(false);
    expect(result.errors[0]).toMatchObject({
      code: 'syntax_invalid',
      path: 'source'
    });
  });

  test('returns a structured failure for non-string source without throwing', () => {
    expect(() => validateJsBlockSource(null)).not.toThrow();

    const result = validateJsBlockSource(null);

    expect(result.ok).toBe(false);
    expect(result.errors[0]).toMatchObject({
      code: 'transform_failed',
      path: 'source'
    });
  });
});
