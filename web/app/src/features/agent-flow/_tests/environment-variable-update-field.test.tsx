import { render, screen } from '@testing-library/react';
import { describe, expect, test, vi } from 'vitest';

import { EnvironmentVariableUpdateField } from '../components/bindings/EnvironmentVariableUpdateField';
import type { FlowSelectorOption } from '../lib/selector-options';

vi.mock('../components/bindings/TemplatedTextField', () => ({
  TemplatedTextField: ({
    ariaLabel,
    value
  }: {
    ariaLabel: string;
    value: string;
  }) => <input aria-label={ariaLabel} readOnly value={value} />
}));

const selectorOptions: FlowSelectorOption[] = [
  {
    nodeId: 'node-start',
    nodeLabel: 'Start',
    outputKey: 'query',
    outputLabel: 'query',
    valueType: 'string',
    value: ['node-start', 'query'],
    displayLabel: 'Start/query'
  }
];

describe('EnvironmentVariableUpdateField', () => {
  test('uses templated text value editing for string environment variables', () => {
    render(
      <EnvironmentVariableUpdateField
        ariaLabel="Environment Variable Update"
        environmentVariables={[
          {
            name: 'ApiBaseUrl',
            value_type: 'string',
            value: '',
            description: ''
          }
        ]}
        selectorOptions={selectorOptions}
        value={[
          {
            path: ['env', 'ApiBaseUrl'],
            operator: 'set',
            value: {
              kind: 'templated_text',
              value: 'https://{{node-start.query}}/v1'
            }
          }
        ]}
        onChange={vi.fn()}
      />
    );

    expect(
      screen.getByLabelText('Environment Variable Update-0-value')
    ).toHaveValue('https://{{node-start.query}}/v1');
    expect(
      screen.queryByLabelText('Environment Variable Update-0-source')
    ).not.toBeInTheDocument();
  });
});
