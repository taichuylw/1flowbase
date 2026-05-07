import { describe, expect, test } from 'vitest';

import { mapVariableCacheToVariableGroup } from '../../lib/debug-console/variable-groups';

describe('debug console variable groups', () => {
  test('maps variable cache entries by whole node object instead of flattening fields', () => {
    const group = mapVariableCacheToVariableGroup(
      {
        'node-llm': {
          user_prompt: '你好?',
          __attempt_ids: ['attempt-1'],
          usage: {
            total_tokens: 16
          }
        }
      },
      {
        'node-llm': 'LLM'
      }
    );

    expect(group).toEqual({
      title: 'Variable Cache',
      items: [
        {
          key: 'node-llm',
          label: 'LLM',
          value: {
            user_prompt: '你好?',
            __attempt_ids: ['attempt-1'],
            usage: {
              total_tokens: 16
            }
          }
        }
      ]
    });
  });
});
