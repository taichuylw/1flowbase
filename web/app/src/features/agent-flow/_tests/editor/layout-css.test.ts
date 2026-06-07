import fs from 'node:fs';
import path from 'node:path';

import { describe, expect, test } from 'vitest';

describe('AgentFlow editor layout CSS', () => {
  test('bounds the canvas editor to the parent full-height section', () => {
    const shellCss = fs.readFileSync(
      path.resolve(
        import.meta.dirname,
        '../../components/editor/styles/shell.css'
      ),
      'utf8'
    );
    const editorBlock = shellCss.match(
      /\.agent-flow-editor\s*\{[\s\S]*?\n\}/
    )?.[0];

    expect(editorBlock).toContain('height: 100%;');
    expect(editorBlock).toContain('min-height: 0;');
    expect(editorBlock).not.toContain('min-height: calc(100vh');
  });
});
