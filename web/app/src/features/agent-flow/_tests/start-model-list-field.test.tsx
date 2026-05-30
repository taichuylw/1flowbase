import { act, fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, test, vi } from 'vitest';

import { StartModelListField } from '../components/detail/fields/StartModelListField';

describe('StartModelListField', () => {
  test('shows only model id and context in the list', () => {
    const onChange = vi.fn();

    render(
      <StartModelListField
        value={[
          {
            id: ' qwen3.6-35b-a3b ',
            name: ' Qwen 3.6 35B ',
            context_window: 128000,
            max_context_window: 200000,
            max_output_tokens: 8192,
            auto_compact_token_limit: 110000,
            capabilities: {
              tool_call: true,
              multimodal: true,
              structured_output: true
            },
            reasoning: {
              default_effort: 'medium',
              supported_efforts: ['low', 'medium']
            }
          },
          ' deepseek-v4-flash '
        ]}
        onChange={onChange}
      />
    );

    expect(screen.getByText('qwen3.6-35b-a3b')).toBeInTheDocument();
    expect(screen.getByText('128K')).toBeInTheDocument();
    expect(screen.getByText('deepseek-v4-flash')).toBeInTheDocument();
    expect(
      screen.queryByLabelText('Model reasoning supported efforts 1')
    ).not.toBeInTheDocument();
    expect(
      screen.queryByPlaceholderText('display name')
    ).not.toBeInTheDocument();
  });

  test('adds the default flowbase model with real defaults after confirmation', () => {
    const onChange = vi.fn();

    render(<StartModelListField value={[]} onChange={onChange} />);

    fireEvent.click(screen.getByLabelText('Add new model'));
    expect(screen.getByLabelText('Model ID input')).toHaveValue('flowbase');
    expect(screen.getByLabelText('Model display name input')).toHaveValue(
      'flowbase'
    );
    expect(screen.getByLabelText('Model context window')).toHaveValue('257');
    expect(screen.getByLabelText('Maximum context window input')).toHaveValue(
      '128'
    );
    expect(screen.getByLabelText('Maximum output tokens input')).toHaveValue(
      '32'
    );
    expect(
      screen.getByLabelText('Auto compact threshold percent input')
    ).toHaveValue('85');
    expect(
      screen.queryByLabelText('External reasoning override switch')
    ).not.toBeInTheDocument();
    expect(screen.getByLabelText('Supported reasoning effort 1')).toHaveValue(
      'minimal'
    );
    expect(screen.getByLabelText('Supported reasoning effort 2')).toHaveValue(
      'low'
    );
    expect(screen.getByLabelText('Supported reasoning effort 3')).toHaveValue(
      'medium'
    );
    expect(screen.getByLabelText('Supported reasoning effort 4')).toHaveValue(
      'high'
    );
    expect(screen.getByLabelText('Supported reasoning effort 5')).toHaveValue(
      'xhigh'
    );
    fireEvent.click(screen.getByLabelText('Save model'));

    expect(onChange).toHaveBeenLastCalledWith([
      {
        id: 'flowbase',
        name: 'flowbase',
        context_window: 257000,
        max_context_window: 128000,
        max_output_tokens: 32000,
        auto_compact_token_limit: 218450,
        capabilities: {
          reasoning: true,
          tool_call: true,
          multimodal: true,
          structured_output: true
        },
        reasoning: {
          default_effort: 'medium',
          supported_efforts: ['minimal', 'low', 'medium', 'high', 'xhigh']
        }
      }
    ]);
  });

  test('saves optional model settings when configured in the form', () => {
    const onChange = vi.fn();

    render(<StartModelListField value={[]} onChange={onChange} />);

    fireEvent.click(screen.getByLabelText('Add new model'));
    fireEvent.change(screen.getByLabelText('Model ID input'), {
      target: { value: 'gpt-5.5' }
    });
    fireEvent.change(screen.getByLabelText('Model display name input'), {
      target: { value: 'GPT 5.5' }
    });
    expect(
      screen.getAllByLabelText('Model context window unit').length
    ).toBeGreaterThan(0);
    fireEvent.change(screen.getByLabelText('Model context window'), {
      target: { value: '256' }
    });
    fireEvent.change(screen.getByLabelText('Maximum context window input'), {
      target: { value: '300' }
    });
    fireEvent.change(screen.getByLabelText('Maximum output tokens input'), {
      target: { value: '64' }
    });
    fireEvent.change(
      screen.getByLabelText('Auto compact threshold percent input'),
      {
        target: { value: '85' }
      }
    );
    fireEvent.click(
      screen.getByLabelText('Remove supported reasoning effort 1')
    );
    fireEvent.click(
      screen.getByLabelText('Remove supported reasoning effort 4')
    );
    expect(screen.getByLabelText('Supported reasoning effort 1')).toHaveValue(
      'low'
    );
    expect(screen.getByLabelText('Supported reasoning effort 2')).toHaveValue(
      'medium'
    );
    expect(screen.getByLabelText('Supported reasoning effort 3')).toHaveValue(
      'high'
    );
    fireEvent.click(screen.getByLabelText('Add supported reasoning effort'));
    fireEvent.change(screen.getByLabelText('Supported reasoning effort 4'), {
      target: { value: 'ultra' }
    });
    expect(
      screen.getAllByLabelText('Default reasoning effort input').length
    ).toBeGreaterThan(0);
    expect(screen.getByLabelText('Tool call capability')).toHaveAttribute(
      'aria-checked',
      'true'
    );
    expect(screen.getByLabelText('Multimodal capability')).toHaveAttribute(
      'aria-checked',
      'true'
    );
    expect(
      screen.getByLabelText('Structured output capability')
    ).toHaveAttribute('aria-checked', 'true');
    fireEvent.click(screen.getByLabelText('Save model'));

    expect(onChange).toHaveBeenLastCalledWith([
      {
        id: 'gpt-5.5',
        name: 'GPT 5.5',
        context_window: 256000,
        max_context_window: 300000,
        max_output_tokens: 64000,
        auto_compact_token_limit: 217600,
        capabilities: {
          reasoning: true,
          tool_call: true,
          multimodal: true,
          structured_output: true
        },
        reasoning: {
          default_effort: 'medium',
          supported_efforts: ['low', 'medium', 'high', 'ultra']
        }
      }
    ]);
  });

  test('keeps focus when editing a newly added reasoning effort', () => {
    const onChange = vi.fn();

    render(<StartModelListField value={[]} onChange={onChange} />);

    fireEvent.click(screen.getByLabelText('Add new model'));
    fireEvent.click(screen.getByLabelText('Add supported reasoning effort'));

    const effortInput = screen.getByLabelText('Supported reasoning effort 6');
    act(() => {
      effortInput.focus();
    });

    fireEvent.change(effortInput, {
      target: { value: 'u' }
    });

    expect(screen.getByLabelText('Supported reasoning effort 6')).toHaveFocus();
  });
});
