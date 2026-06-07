import { render, screen } from '@testing-library/react';
import { describe, expect, test, vi } from 'vitest';

import { VariableAssignmentField } from '../components/bindings/VariableAssignmentField';
import type { FlowSelectorOption } from '../lib/selector-options';

vi.mock('../components/bindings/TemplatedTextField', () => ({
  TemplatedTextField: ({
    ariaLabel,
    label,
    value
  }: {
    ariaLabel: string;
    label: string;
    value: string;
  }) => (
    <label>
      {label}
      <input aria-label={ariaLabel} readOnly value={value} />
    </label>
  )
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

describe('VariableAssignmentField', () => {
  test('uses templated text value editing for string conversation variables', () => {
    render(
      <VariableAssignmentField
        ariaLabel="变量赋值"
        conversationVariables={[
          {
            name: 'ApiBaseUrl',
            valueType: 'string',
            description: ''
          }
        ]}
        selectorOptions={selectorOptions}
        value={[
          {
            path: ['conversation', 'ApiBaseUrl'],
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

    expect(screen.getByLabelText('变量赋值-0-value')).toHaveValue(
      'https://{{node-start.query}}/v1'
    );
    expect(
      screen.getAllByText('conversation.ApiBaseUrl').length
    ).toBeGreaterThan(0);
    expect(screen.queryByText('env.ApiBaseUrl')).not.toBeInTheDocument();
  });
});
