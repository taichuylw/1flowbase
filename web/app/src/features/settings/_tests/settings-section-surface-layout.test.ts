import fs from 'node:fs';
import path from 'node:path';

import { describe, expect, test } from 'vitest';

describe('settings section surface layout CSS', () => {
  test('lets fill settings surfaces allocate remaining height to the body', () => {
    const cssSource = fs.readFileSync(
      path.resolve(
        import.meta.dirname,
        '../components/settings-section-surface.css'
      ),
      'utf8'
    );
    const fillBlock = cssSource.match(
      /\.settings-section-surface--fill\s*\{[\s\S]*?\n\}/
    )?.[0];
    const fillBodyBlock = cssSource.match(
      /\.settings-section-surface--fill \.settings-section-surface__body\s*\{[\s\S]*?\n\}/
    )?.[0];

    expect(fillBlock).toContain('display: flex;');
    expect(fillBlock).toContain('flex-direction: column;');
    expect(fillBlock).toContain('height: 100%;');
    expect(fillBlock).toContain('min-height: 0;');
    expect(fillBlock).not.toContain('grid-template-rows');
    expect(fillBodyBlock).toContain('flex: 1 1 auto;');
    expect(fillBodyBlock).toContain('height: 100%;');
    expect(fillBodyBlock).toContain('min-height: 0;');
  });
});
