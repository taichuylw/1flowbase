import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

import { describe, expect, test } from 'vitest';

import {
  getConsoleApplicationConversationMessages as getConsoleApplicationConversationMessagesFromBarrel,
  getConsoleApplicationRunConversationMessages as getConsoleApplicationRunConversationMessagesFromBarrel
} from '../index';
import {
  getConsoleApplicationConversationMessages,
  getConsoleApplicationRunConversationMessages
} from '../console/application-runtime';

const indexSource = readFileSync(
  fileURLToPath(new URL('../index.ts', import.meta.url)),
  'utf8'
);

describe('api client barrel exports', () => {
  test('exposes run-scoped conversation messages from the package entrypoint', () => {
    expect(getConsoleApplicationConversationMessagesFromBarrel).toBe(
      getConsoleApplicationConversationMessages
    );
    expect(getConsoleApplicationRunConversationMessagesFromBarrel).toBe(
      getConsoleApplicationRunConversationMessages
    );
    expect(indexSource).toContain('getConsoleApplicationConversationMessages');
    expect(indexSource).toContain(
      'getConsoleApplicationRunConversationMessages'
    );
  });
});
